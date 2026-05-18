//
//  IPCProducer.swift
//  AORATA
//
//  Host app IPC producer (PROT_WRITE side of unidirectional IPC)
//

import Foundation
import NetworkExtension
import os.log

/// IPC Producer (Host App → Tunnel Extension)
///
/// ## Architecture
/// - **Producer**: PROT_WRITE (this class)
/// - **Consumer**: PROT_READ only (extension)
/// - **SPSC**: Quetzalcoatl lock-free ring buffer
/// - **Security**: TOCTOU prevention via MMU enforcement
///
class IPCProducer {

    // MARK: - Constants

    static let defaultBufferSize = 2 * 1024 * 1024  // 2MB

    // MARK: - Properties

    private let basePointer: UnsafeMutableRawPointer
    private let bufferSize: Int
    private var ringBuffer: SPSCRingBufferProducer
    private let logger = Logger(subsystem: "com.lorislab.aorata", category: "IPCProducer")

    // MARK: - Initialization

    /// Initialize IPC producer
    ///
    /// ## Setup Flow
    /// 1. Allocate mmap with PROT_WRITE
    /// 2. Send Mach port to extension via handleAppMessage
    /// 3. Initialize SPSC ring buffer producer
    ///
    init(appGroup: String) throws {
        logger.info("🔧 Initializing IPC producer (App Group: \(appGroup))")

        // Resolve App Group container
        guard let containerURL = FileManager.default.containerURL(
            forSecurityApplicationGroupIdentifier: appGroup
        ) else {
            throw NSError(domain: "IPCProducer", code: -1, userInfo: [
                NSLocalizedDescriptionKey: "App Group not found: \(appGroup)"
            ])
        }

        logger.debug("📁 App Group container: \(containerURL.path)")

        // PRODUCTION TODO: Use Mach memory entry
        // For now, use file-backed mmap (placeholder)
        let mmapPath = containerURL.appendingPathComponent("aorata_ipc.mmap").path

        self.bufferSize = Self.defaultBufferSize

        // Create and map file
        self.basePointer = try Self.createSharedMemory(path: mmapPath, size: bufferSize)

        logger.info("✅ mmap created: \(self.bufferSize) bytes @ \(String(describing: self.basePointer))")

        // Initialize SPSC ring buffer producer
        self.ringBuffer = SPSCRingBufferProducer(basePointer: basePointer, size: bufferSize)

        logger.info("✅ IPC producer ready (SPSC ring buffer)")
    }

    deinit {
        munmap(basePointer, bufferSize)
        logger.info("🗑️ IPC producer destroyed (munmap)")
    }

    // MARK: - Memory Management

    /// Create shared memory with PROT_WRITE (producer side)
    ///
    /// **CRITICAL**: This is placeholder file-backed mmap. Production must use:
    /// 1. `mach_make_memory_entry_64` to create Mach memory object
    /// 2. `mmap` with PROT_WRITE
    /// 3. Send Mach port to extension via `NETunnelProviderSession.sendProviderMessage`
    ///
    private static func createSharedMemory(path: String, size: Int) throws -> UnsafeMutableRawPointer {
        // Create/open file
        let fd = open(path, O_RDWR | O_CREAT, 0o666)
        guard fd >= 0 else {
            throw NSError(domain: NSPOSIXErrorDomain, code: Int(errno))
        }
        defer { close(fd) }

        // Set file size
        guard ftruncate(fd, off_t(size)) == 0 else {
            throw NSError(domain: NSPOSIXErrorDomain, code: Int(errno))
        }

        // mmap with PROT_WRITE
        let ptr = mmap(nil, size, PROT_WRITE | PROT_READ, MAP_SHARED, fd, 0)
        guard ptr != MAP_FAILED else {
            throw NSError(domain: NSPOSIXErrorDomain, code: Int(errno))
        }

        return ptr!
    }

    // MARK: - Packet Operations

    /// Push packet to SPSC ring buffer (zero-copy)
    ///
    /// ## Zero-Copy via Typestate
    /// - `reserve()` allocates slot in ring
    /// - Write packet directly into slot
    /// - `commit()` publishes with Release memory ordering
    ///
    func pushPacket(_ data: Data) -> Bool {
        return ringBuffer.push(data)
    }

    /// Send Mach port to extension (production TODO)
    ///
    /// This method should be called after creating the VPN session to bootstrap IPC.
    ///
    func sendMachPortToExtension(session: NETunnelProviderSession) {
        // PRODUCTION TODO:
        // 1. Extract Mach port from mach_make_memory_entry_64
        // 2. Serialize port as Data
        // 3. Send via session.sendProviderMessage()

        logger.warning("⚠️ sendMachPortToExtension not implemented (file-backed mmap placeholder)")
    }
}

/// SPSC Ring Buffer Producer (placeholder — real impl uses quetzalcoatl Rust FFI)
///
/// ## Architecture
/// - **Head**: Consumer read position (remote, read-only)
/// - **Tail**: Producer write position (local, Acquire/Release)
/// - **Cache-line padding**: 64 bytes between Head/Tail
///
/// **TODO**: Replace with FFI to Rust `quetzalcoatl::buffer::spsc::Producer`
///
private struct SPSCRingBufferProducer {
    let basePointer: UnsafeMutableRawPointer
    let size: Int

    /// Push packet (placeholder)
    func push(_ data: Data) -> Bool {
        // TODO: Atomic load Tail with Acquire ordering
        // TODO: Check space available (Tail vs Head)
        // TODO: Write packet size + data
        // TODO: Atomic store Tail with Release ordering
        return true  // Mock success
    }
}
