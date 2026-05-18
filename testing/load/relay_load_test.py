#!/usr/bin/env python3
"""
FILE Relay Server - Load Testing Tool

Generates realistic load patterns for performance testing and capacity validation.
"""

import argparse
import asyncio
import hashlib
import json
import random
import socket
import struct
import sys
import time
from collections import defaultdict
from dataclasses import dataclass, asdict
from datetime import datetime
from typing import Dict, List, Optional, Tuple

import aiohttp


@dataclass
class TestConfig:
    """Load test configuration."""
    host: str
    port: int
    metrics_port: int
    num_peers: int
    duration_seconds: int
    packet_rate: float  # packets/second per peer
    peer_churn_rate: float  # peers joining/leaving per second
    capow_difficulty: int  # Number of leading zero bits
    seed: Optional[int] = None


@dataclass
class TestResults:
    """Load test results."""
    start_time: float
    end_time: float
    duration: float

    # Peer metrics
    peers_registered: int
    peers_rejected: int
    peers_timeout: int

    # Packet metrics
    packets_sent: int
    packets_relayed: int
    packets_lost: int

    # Latency metrics (ms)
    latency_min: float
    latency_max: float
    latency_avg: float
    latency_p50: float
    latency_p95: float
    latency_p99: float

    # Throughput
    packets_per_second: float
    bytes_per_second: float

    # Server metrics (from /metrics endpoint)
    server_active_peers: int
    server_packets_forwarded: int
    server_capow_rejected: int
    server_rate_limited: int

    # Success rate
    registration_success_rate: float
    packet_delivery_rate: float


class CAPoWGenerator:
    """Computational Amortized Proof of Work generator (MTP-Argon2id)."""

    def __init__(self, difficulty: int = 16):
        """
        Initialize CAPoW generator.

        Args:
            difficulty: Number of leading zero bits required (default: 16)
        """
        self.difficulty = difficulty
        self.target = (1 << (256 - difficulty)) - 1

    def generate_proof(self, peer_id: bytes) -> bytes:
        """
        Generate a valid CAPoW proof for a peer ID.

        Args:
            peer_id: 32-byte peer identifier

        Returns:
            Valid proof bytes

        Note: This is a simplified version. Production should use MTP-Argon2id.
        """
        nonce = 0
        timestamp = int(time.time())

        while True:
            # Build input: peer_id || timestamp || nonce
            input_data = peer_id + struct.pack('>Q', timestamp) + struct.pack('>Q', nonce)

            # Hash with SHA-256 (simplified - production uses Argon2id)
            hash_output = hashlib.sha256(input_data).digest()
            hash_value = int.from_bytes(hash_output, byteorder='big')

            # Check if meets difficulty target
            if hash_value <= self.target:
                # Return proof: timestamp || nonce || hash
                return struct.pack('>Q', timestamp) + struct.pack('>Q', nonce) + hash_output

            nonce += 1

            # Timeout after 100ms worth of attempts (roughly)
            if nonce > 10000:
                raise TimeoutError(f"CAPoW generation timeout after {nonce} attempts")


class UDPPeer:
    """Simulated UDP peer."""

    def __init__(self, peer_id: bytes, relay_addr: Tuple[str, int], capow_gen: CAPoWGenerator):
        self.peer_id = peer_id
        self.relay_addr = relay_addr
        self.capow_gen = capow_gen
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        self.sock.setblocking(False)
        self.registered = False
        self.last_heartbeat = 0.0

        # Metrics
        self.packets_sent = 0
        self.packets_received = 0
        self.registration_attempts = 0
        self.registration_time: Optional[float] = None

    async def register(self) -> bool:
        """
        Register with relay server.

        Returns:
            True if registration succeeded, False otherwise
        """
        self.registration_attempts += 1
        start = time.time()

        try:
            # Generate CAPoW proof
            proof = self.capow_gen.generate_proof(self.peer_id)

            # Build REGISTER message
            # Format: type (1) | peer_id (32) | proof_len (2) | proof
            msg = struct.pack('B', 1) + self.peer_id + struct.pack('>H', len(proof)) + proof

            # Send registration
            self.sock.sendto(msg, self.relay_addr)
            self.packets_sent += 1

            # Wait for REGISTER_ACK (with timeout)
            self.sock.settimeout(1.0)
            try:
                data, _ = self.sock.recvfrom(4096)
                msg_type = struct.unpack('B', data[:1])[0]

                if msg_type == 2:  # REGISTER_ACK
                    self.registered = True
                    self.registration_time = time.time() - start
                    return True
                elif msg_type == 3:  # REGISTER_REJECT
                    return False
            except socket.timeout:
                return False
            finally:
                self.sock.setblocking(False)

        except Exception as e:
            print(f"Registration error: {e}", file=sys.stderr)
            return False

        return False

    async def send_packet(self, target_peer_id: bytes, payload: bytes) -> float:
        """
        Send a packet to another peer via relay.

        Args:
            target_peer_id: Destination peer ID
            payload: Packet payload

        Returns:
            Send timestamp for latency measurement
        """
        # Build RELAY_PACKET message
        # Format: type (1) | target_peer_id (32) | payload_len (2) | payload
        msg = (struct.pack('B', 5) + target_peer_id +
               struct.pack('>H', len(payload)) + payload)

        send_time = time.time()
        self.sock.sendto(msg, self.relay_addr)
        self.packets_sent += 1

        return send_time

    async def heartbeat(self) -> None:
        """Send heartbeat to relay server."""
        if not self.registered:
            return

        # Build HEARTBEAT message
        msg = struct.pack('B', 4) + self.peer_id
        self.sock.sendto(msg, self.relay_addr)
        self.packets_sent += 1
        self.last_heartbeat = time.time()

    def close(self) -> None:
        """Close peer socket."""
        self.sock.close()


class LoadTester:
    """FILE Relay Server load tester."""

    def __init__(self, config: TestConfig):
        self.config = config
        self.capow_gen = CAPoWGenerator(difficulty=config.capow_difficulty)

        self.peers: List[UDPPeer] = []
        self.active_peers: List[UDPPeer] = []

        # Metrics
        self.latencies: List[float] = []
        self.registration_times: List[float] = []
        self.packets_sent = 0
        self.packets_received = 0
        self.peers_registered = 0
        self.peers_rejected = 0

        # Random seed
        if config.seed is not None:
            random.seed(config.seed)

    async def fetch_metrics(self) -> Dict:
        """Fetch current metrics from relay server."""
        try:
            async with aiohttp.ClientSession() as session:
                async with session.get(
                    f"http://{self.config.host}:{self.config.metrics_port}/metrics",
                    timeout=aiohttp.ClientTimeout(total=5)
                ) as resp:
                    if resp.status == 200:
                        text = await resp.text()
                        return self._parse_prometheus_metrics(text)
        except Exception as e:
            print(f"Failed to fetch metrics: {e}", file=sys.stderr)

        return {}

    def _parse_prometheus_metrics(self, text: str) -> Dict:
        """Parse Prometheus metrics format."""
        metrics = {}
        for line in text.splitlines():
            if line.startswith('#') or not line.strip():
                continue

            parts = line.split()
            if len(parts) >= 2:
                name = parts[0]
                value = parts[1]
                try:
                    metrics[name] = float(value)
                except ValueError:
                    pass

        return metrics

    async def create_peer(self) -> UDPPeer:
        """Create a new simulated peer."""
        peer_id = random.randbytes(32)
        peer = UDPPeer(
            peer_id=peer_id,
            relay_addr=(self.config.host, self.config.port),
            capow_gen=self.capow_gen
        )
        return peer

    async def register_peer(self, peer: UDPPeer) -> bool:
        """Register a peer with the relay server."""
        success = await peer.register()

        if success:
            self.peers_registered += 1
            self.active_peers.append(peer)
            if peer.registration_time:
                self.registration_times.append(peer.registration_time)
        else:
            self.peers_rejected += 1

        return success

    async def send_packets(self, duration: float) -> None:
        """Send packets at configured rate for specified duration."""
        start = time.time()
        packets_per_peer = self.config.packet_rate
        interval = 1.0 / packets_per_peer if packets_per_peer > 0 else 1.0

        while time.time() - start < duration:
            if len(self.active_peers) < 2:
                await asyncio.sleep(0.1)
                continue

            # Pick random sender and receiver
            sender = random.choice(self.active_peers)
            receiver = random.choice([p for p in self.active_peers if p != sender])

            # Send packet with timestamp payload
            timestamp = struct.pack('>d', time.time())
            payload = timestamp + b'X' * 100  # 108-byte payload

            await sender.send_packet(receiver.peer_id, payload)
            self.packets_sent += 1

            await asyncio.sleep(interval)

    async def heartbeat_loop(self) -> None:
        """Send periodic heartbeats for all active peers."""
        while True:
            for peer in self.active_peers:
                if time.time() - peer.last_heartbeat > 30:
                    await peer.heartbeat()

            await asyncio.sleep(10)

    async def churn_loop(self, duration: float) -> None:
        """Simulate peer churn (joining/leaving)."""
        start = time.time()
        interval = 1.0 / self.config.peer_churn_rate if self.config.peer_churn_rate > 0 else 10.0

        while time.time() - start < duration:
            # 50% chance to add, 50% to remove
            if random.random() < 0.5 and len(self.active_peers) < self.config.num_peers:
                # Add peer
                peer = await self.create_peer()
                await self.register_peer(peer)
            elif len(self.active_peers) > 1:
                # Remove peer
                peer = random.choice(self.active_peers)
                self.active_peers.remove(peer)
                peer.close()

            await asyncio.sleep(interval)

    async def run(self) -> TestResults:
        """Run load test."""
        print(f"Starting load test:")
        print(f"  Target: {self.config.host}:{self.config.port}")
        print(f"  Peers: {self.config.num_peers}")
        print(f"  Duration: {self.config.duration_seconds}s")
        print(f"  Packet rate: {self.config.packet_rate}/s per peer")
        print(f"  Churn rate: {self.config.peer_churn_rate}/s")
        print()

        start_time = time.time()

        # Fetch initial metrics
        initial_metrics = await self.fetch_metrics()

        # Phase 1: Ramp up - register peers
        print("Phase 1: Registering peers...")
        for i in range(self.config.num_peers):
            peer = await self.create_peer()
            await self.register_peer(peer)

            if (i + 1) % 10 == 0:
                print(f"  Registered: {i + 1}/{self.config.num_peers}")

        print(f"  Success: {self.peers_registered}/{self.config.num_peers}")
        print()

        # Phase 2: Steady state - send packets
        print("Phase 2: Sending packets...")

        tasks = [
            asyncio.create_task(self.send_packets(self.config.duration_seconds)),
            asyncio.create_task(self.heartbeat_loop()),
        ]

        # Add churn if configured
        if self.config.peer_churn_rate > 0:
            tasks.append(asyncio.create_task(self.churn_loop(self.config.duration_seconds)))

        # Wait for packet sending to complete
        await tasks[0]

        # Cancel background tasks
        for task in tasks[1:]:
            task.cancel()

        # Phase 3: Cool down
        print("Phase 3: Collecting final metrics...")
        await asyncio.sleep(2)

        end_time = time.time()

        # Fetch final metrics
        final_metrics = await self.fetch_metrics()

        # Calculate results
        results = self._calculate_results(start_time, end_time, initial_metrics, final_metrics)

        # Cleanup
        for peer in self.active_peers:
            peer.close()

        return results

    def _calculate_results(
        self,
        start_time: float,
        end_time: float,
        initial_metrics: Dict,
        final_metrics: Dict
    ) -> TestResults:
        """Calculate test results from collected metrics."""
        duration = end_time - start_time

        # Server metrics delta
        server_packets = final_metrics.get('file_relay_packets_forwarded_total', 0) - \
                        initial_metrics.get('file_relay_packets_forwarded_total', 0)

        # Calculate latencies (if we collected any)
        if self.latencies:
            sorted_latencies = sorted(self.latencies)
            latency_min = sorted_latencies[0]
            latency_max = sorted_latencies[-1]
            latency_avg = sum(self.latencies) / len(self.latencies)
            latency_p50 = sorted_latencies[len(sorted_latencies) // 2]
            latency_p95 = sorted_latencies[int(len(sorted_latencies) * 0.95)]
            latency_p99 = sorted_latencies[int(len(sorted_latencies) * 0.99)]
        else:
            latency_min = latency_max = latency_avg = 0.0
            latency_p50 = latency_p95 = latency_p99 = 0.0

        # Throughput
        packets_per_second = self.packets_sent / duration if duration > 0 else 0
        avg_packet_size = 150  # Approximate
        bytes_per_second = packets_per_second * avg_packet_size

        # Success rates
        total_registration_attempts = self.peers_registered + self.peers_rejected
        registration_success_rate = (
            self.peers_registered / total_registration_attempts * 100
            if total_registration_attempts > 0 else 0
        )

        packet_delivery_rate = (
            server_packets / self.packets_sent * 100
            if self.packets_sent > 0 else 0
        )

        return TestResults(
            start_time=start_time,
            end_time=end_time,
            duration=duration,
            peers_registered=self.peers_registered,
            peers_rejected=self.peers_rejected,
            peers_timeout=0,  # Not tracked in this implementation
            packets_sent=self.packets_sent,
            packets_relayed=int(server_packets),
            packets_lost=self.packets_sent - int(server_packets),
            latency_min=latency_min * 1000,  # Convert to ms
            latency_max=latency_max * 1000,
            latency_avg=latency_avg * 1000,
            latency_p50=latency_p50 * 1000,
            latency_p95=latency_p95 * 1000,
            latency_p99=latency_p99 * 1000,
            packets_per_second=packets_per_second,
            bytes_per_second=bytes_per_second,
            server_active_peers=int(final_metrics.get('file_relay_active_peers', 0)),
            server_packets_forwarded=int(server_packets),
            server_capow_rejected=int(final_metrics.get('file_relay_capow_rejected_total', 0)),
            server_rate_limited=int(final_metrics.get('file_relay_rate_limited_total', 0)),
            registration_success_rate=registration_success_rate,
            packet_delivery_rate=packet_delivery_rate
        )


def print_results(results: TestResults) -> None:
    """Print test results in human-readable format."""
    print()
    print("=" * 70)
    print("LOAD TEST RESULTS")
    print("=" * 70)
    print()

    print(f"Duration: {results.duration:.2f}s")
    print()

    print("PEER METRICS:")
    print(f"  Registered:     {results.peers_registered}")
    print(f"  Rejected:       {results.peers_rejected}")
    print(f"  Success rate:   {results.registration_success_rate:.1f}%")
    print()

    print("PACKET METRICS:")
    print(f"  Sent:           {results.packets_sent}")
    print(f"  Relayed:        {results.packets_relayed}")
    print(f"  Lost:           {results.packets_lost}")
    print(f"  Delivery rate:  {results.packet_delivery_rate:.1f}%")
    print()

    print("THROUGHPUT:")
    print(f"  Packets/sec:    {results.packets_per_second:.2f}")
    print(f"  Bytes/sec:      {results.bytes_per_second / 1024:.2f} KB/s")
    print()

    if results.latency_avg > 0:
        print("LATENCY (ms):")
        print(f"  Min:            {results.latency_min:.2f}")
        print(f"  Average:        {results.latency_avg:.2f}")
        print(f"  Median (p50):   {results.latency_p50:.2f}")
        print(f"  p95:            {results.latency_p95:.2f}")
        print(f"  p99:            {results.latency_p99:.2f}")
        print(f"  Max:            {results.latency_max:.2f}")
        print()

    print("SERVER METRICS:")
    print(f"  Active peers:   {results.server_active_peers}")
    print(f"  Packets fwd:    {results.server_packets_forwarded}")
    print(f"  CAPoW rejected: {results.server_capow_rejected}")
    print(f"  Rate limited:   {results.server_rate_limited}")
    print()

    print("=" * 70)


def save_results_json(results: TestResults, filename: str) -> None:
    """Save results to JSON file."""
    with open(filename, 'w') as f:
        json.dump(asdict(results), f, indent=2)
    print(f"Results saved to: {filename}")


def main():
    parser = argparse.ArgumentParser(
        description="FILE Relay Server Load Testing Tool",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Quick test: 10 peers, 30 seconds
  %(prog)s --host relay.example.com --peers 10 --duration 30

  # Stress test: 1000 peers, 5 minutes, high packet rate
  %(prog)s --host relay.example.com --peers 1000 --duration 300 --packet-rate 10

  # Churn test: 100 peers, constant joining/leaving
  %(prog)s --host relay.example.com --peers 100 --duration 120 --churn-rate 2.0

  # Save results to JSON
  %(prog)s --host relay.example.com --peers 100 --output results.json
        """
    )

    parser.add_argument('--host', required=True, help='Relay server hostname or IP')
    parser.add_argument('--port', type=int, default=8080, help='Relay server UDP port (default: 8080)')
    parser.add_argument('--metrics-port', type=int, default=8081, help='Metrics HTTP port (default: 8081)')
    parser.add_argument('--peers', type=int, default=100, help='Number of peers (default: 100)')
    parser.add_argument('--duration', type=int, default=60, help='Test duration in seconds (default: 60)')
    parser.add_argument('--packet-rate', type=float, default=1.0, help='Packets/sec per peer (default: 1.0)')
    parser.add_argument('--churn-rate', type=float, default=0.0, help='Peer churn rate (joins+leaves)/sec (default: 0)')
    parser.add_argument('--capow-difficulty', type=int, default=16, help='CAPoW difficulty (default: 16)')
    parser.add_argument('--seed', type=int, help='Random seed for reproducibility')
    parser.add_argument('--output', '-o', help='Save results to JSON file')

    args = parser.parse_args()

    config = TestConfig(
        host=args.host,
        port=args.port,
        metrics_port=args.metrics_port,
        num_peers=args.peers,
        duration_seconds=args.duration,
        packet_rate=args.packet_rate,
        peer_churn_rate=args.churn_rate,
        capow_difficulty=args.capow_difficulty,
        seed=args.seed
    )

    # Run test
    tester = LoadTester(config)
    try:
        results = asyncio.run(tester.run())
    except KeyboardInterrupt:
        print("\n\nTest interrupted by user", file=sys.stderr)
        sys.exit(1)

    # Print results
    print_results(results)

    # Save to file if requested
    if args.output:
        save_results_json(results, args.output)


if __name__ == '__main__':
    main()
