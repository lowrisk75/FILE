//
//  ContentView.swift
//  AORATA
//
//  The most beautiful VPN UI ever created
//  Design: visionOS glassmorphism + network visualization
//

import SwiftUI
import Charts

struct ContentView: View {
    @State private var isConnected = false
    @State private var connectionProgress: Double = 0.0
    @State private var currentLatency: Double = 12.4
    @State private var bandwidth: Double = 245.8
    @State private var activePeers: Int = 3
    @State private var encryptionStatus: EncryptionStatus = .quantum

    // Animation states
    @State private var pulseScale: CGFloat = 1.0
    @State private var rotationAngle: Double = 0

    var body: some View {
        ZStack {
            // Background gradient
            LinearGradient(
                colors: [
                    Color(red: 0.05, green: 0.05, blue: 0.1),
                    Color(red: 0.1, green: 0.05, blue: 0.15)
                ],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )
            .ignoresSafeArea()

            // Main content
            VStack(spacing: 0) {
                // Header
                headerView
                    .padding(.horizontal, 32)
                    .padding(.top, 20)

                Spacer()

                // Central connection orb
                connectionOrbView
                    .padding(.vertical, 40)

                Spacer()

                // Stats grid
                statsGridView
                    .padding(.horizontal, 32)
                    .padding(.bottom, 32)

                // Network visualization
                networkVisualizationView
                    .frame(height: 200)
                    .padding(.horizontal, 32)
                    .padding(.bottom, 32)
            }
        }
        .preferredColorScheme(.dark)
        .onAppear {
            startAnimations()
        }
    }

    // MARK: - Header

    private var headerView: some View {
        HStack {
            VStack(alignment: .leading, spacing: 4) {
                Text("AORATA")
                    .font(.system(size: 32, weight: .bold, design: .rounded))
                    .foregroundStyle(
                        LinearGradient(
                            colors: [.white, Color(white: 0.7)],
                            startPoint: .topLeading,
                            endPoint: .bottomTrailing
                        )
                    )

                Text(isConnected ? "Zero Trust Active" : "Disconnected")
                    .font(.system(size: 14, weight: .medium))
                    .foregroundColor(isConnected ? .green : .secondary)
            }

            Spacer()

            // Security badge
            securityBadge
        }
    }

    private var securityBadge: some View {
        HStack(spacing: 8) {
            Image(systemName: "shield.checkered")
                .font(.system(size: 16, weight: .semibold))
                .symbolEffect(.pulse, options: .repeating, value: isConnected)

            Text(encryptionStatus.rawValue)
                .font(.system(size: 12, weight: .semibold))
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
        .background(.ultraThinMaterial, in: Capsule())
        .overlay(
            Capsule()
                .strokeBorder(
                    LinearGradient(
                        colors: [
                            encryptionStatus.color.opacity(0.6),
                            encryptionStatus.color.opacity(0.2)
                        ],
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    ),
                    lineWidth: 1
                )
        )
        .foregroundColor(encryptionStatus.color)
    }

    // MARK: - Connection Orb

    private var connectionOrbView: some View {
        ZStack {
            // Outer glow rings
            ForEach(0..<3) { index in
                GlowRing(index: index, isConnected: isConnected, pulseScale: pulseScale)
            }

            // Main orb
            ZStack {
                // Glassmorphic background
                Circle()
                    .fill(.ultraThinMaterial)
                    .frame(width: 280, height: 280)

                // Gradient overlay
                Circle()
                    .fill(
                        RadialGradient(
                            colors: isConnected ? [
                                Color.blue.opacity(0.4),
                                Color.purple.opacity(0.3),
                                Color.clear
                            ] : [
                                Color.gray.opacity(0.2),
                                Color.clear
                            ],
                            center: .center,
                            startRadius: 0,
                            endRadius: 140
                        )
                    )
                    .frame(width: 280, height: 280)

                // Border
                Circle()
                    .strokeBorder(
                        LinearGradient(
                            colors: isConnected ? [
                                Color.blue.opacity(0.8),
                                Color.purple.opacity(0.6)
                            ] : [
                                Color.gray.opacity(0.3),
                                Color.gray.opacity(0.1)
                            ],
                            startPoint: .topLeading,
                            endPoint: .bottomTrailing
                        ),
                        lineWidth: 2
                    )
                    .frame(width: 280, height: 280)

                // Content
                VStack(spacing: 16) {
                    // Icon
                    ZStack {
                        Image(systemName: isConnected ? "network" : "network.slash")
                            .font(.system(size: 64, weight: .light))
                            .foregroundStyle(
                                LinearGradient(
                                    colors: isConnected ? [.blue, .purple] : [.gray, .gray.opacity(0.5)],
                                    startPoint: .topLeading,
                                    endPoint: .bottomTrailing
                                )
                            )
                            .symbolEffect(.variableColor, options: .repeating, value: isConnected)

                        if !isConnected {
                            Circle()
                                .trim(from: 0, to: connectionProgress)
                                .stroke(Color.blue, lineWidth: 3)
                                .frame(width: 90, height: 90)
                                .rotationEffect(.degrees(-90))
                                .opacity(connectionProgress > 0 ? 1 : 0)
                        }
                    }

                    // Status text
                    Text(isConnected ? "PROTECTED" : "TAP TO CONNECT")
                        .font(.system(size: 14, weight: .bold, design: .rounded))
                        .tracking(2)
                        .foregroundColor(isConnected ? .white : .secondary)

                    // Latency (when connected)
                    if isConnected {
                        HStack(spacing: 4) {
                            Image(systemName: "timer")
                                .font(.system(size: 12))
                            Text(String(format: "%.1f ms", currentLatency))
                                .font(.system(size: 16, weight: .semibold, design: .rounded))
                        }
                        .foregroundStyle(.secondary)
                        .transition(.opacity.combined(with: .scale))
                    }
                }
            }
            .onTapGesture {
                toggleConnection()
            }
            #if os(iOS)
            .hoverEffect()
            #endif
        }
    }

    // MARK: - Stats Grid

    private var statsGridView: some View {
        LazyVGrid(columns: [
            GridItem(.flexible()),
            GridItem(.flexible()),
            GridItem(.flexible())
        ], spacing: 16) {
            StatCard(
                icon: "arrow.up.arrow.down",
                value: String(format: "%.1f", bandwidth),
                unit: "Mbps",
                label: "Bandwidth",
                color: .blue
            )

            StatCard(
                icon: "person.3.fill",
                value: "\(activePeers)",
                unit: "peers",
                label: "Active Peers",
                color: .purple
            )

            StatCard(
                icon: "cpu",
                value: "ANE",
                unit: "tier 2",
                label: "AI Routing",
                color: .green
            )
        }
    }

    // MARK: - Network Visualization

    private var networkVisualizationView: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Network Activity")
                .font(.system(size: 14, weight: .semibold))
                .foregroundColor(.secondary)

            // Real-time bandwidth chart
            Chart {
                ForEach(0..<30, id: \.self) { index in
                    LineMark(
                        x: .value("Time", index),
                        y: .value("Speed", sin(Double(index) * 0.5) * 100 + bandwidth)
                    )
                    .foregroundStyle(
                        LinearGradient(
                            colors: [.blue, .purple],
                            startPoint: .leading,
                            endPoint: .trailing
                        )
                    )
                    .interpolationMethod(.catmullRom)

                    AreaMark(
                        x: .value("Time", index),
                        y: .value("Speed", sin(Double(index) * 0.5) * 100 + bandwidth)
                    )
                    .foregroundStyle(
                        LinearGradient(
                            colors: [
                                Color.blue.opacity(0.3),
                                Color.purple.opacity(0.1)
                            ],
                            startPoint: .top,
                            endPoint: .bottom
                        )
                    )
                    .interpolationMethod(.catmullRom)
                }
            }
            .chartXAxis(.hidden)
            .chartYAxis(.hidden)
            .frame(height: 120)
            .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 16))
        }
    }

    // MARK: - Actions

    private func toggleConnection() {
        if isConnected {
            // Disconnect
            withAnimation(.spring(response: 0.6, dampingFraction: 0.7)) {
                isConnected = false
                connectionProgress = 0
            }
        } else {
            // Connect with progress animation
            connectionProgress = 0
            withAnimation(.linear(duration: 1.5)) {
                connectionProgress = 1.0
            }

            DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) {
                withAnimation(.spring(response: 0.6, dampingFraction: 0.7)) {
                    isConnected = true
                    connectionProgress = 0
                }
            }
        }
    }

    private func startAnimations() {
        withAnimation(.easeInOut(duration: 2.0).repeatForever(autoreverses: true)) {
            pulseScale = 1.1
        }
    }
}

// MARK: - Stat Card Component

struct StatCard: View {
    let icon: String
    let value: String
    let unit: String
    let label: String
    let color: Color

    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: icon)
                .font(.system(size: 24))
                .foregroundStyle(
                    LinearGradient(
                        colors: [color, color.opacity(0.6)],
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    )
                )
                .symbolEffect(.bounce, value: value)

            VStack(spacing: 2) {
                HStack(alignment: .lastTextBaseline, spacing: 4) {
                    Text(value)
                        .font(.system(size: 24, weight: .bold, design: .rounded))
                    Text(unit)
                        .font(.system(size: 12, weight: .medium))
                        .foregroundColor(.secondary)
                }

                Text(label)
                    .font(.system(size: 11, weight: .medium))
                    .foregroundColor(.secondary)
            }
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 20)
        .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 16))
        .overlay(
            RoundedRectangle(cornerRadius: 16)
                .strokeBorder(
                    LinearGradient(
                        colors: [
                            color.opacity(0.3),
                            Color.clear
                        ],
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    ),
                    lineWidth: 1
                )
        )
    }
}

// MARK: - Supporting Types

enum EncryptionStatus: String {
    case quantum = "ML-KEM-768"
    case classic = "AES-256"
    case none = "None"

    var color: Color {
        switch self {
        case .quantum: return .green
        case .classic: return .blue
        case .none: return .gray
        }
    }
}

// MARK: - Glow Ring Component

struct GlowRing: View {
    let index: Int
    let isConnected: Bool
    let pulseScale: CGFloat

    var body: some View {
        Circle()
            .strokeBorder(
                LinearGradient(
                    colors: [
                        isConnected ? Color.blue.opacity(0.4) : Color.gray.opacity(0.2),
                        Color.clear
                    ],
                    startPoint: .top,
                    endPoint: .bottom
                ),
                lineWidth: 2
            )
            .frame(width: 280 + CGFloat(index * 40), height: 280 + CGFloat(index * 40))
            .scaleEffect(pulseScale)
            .opacity(isConnected ? 0.6 - Double(index) * 0.2 : 0.2)
            .animation(
                .easeInOut(duration: 2.0 + Double(index) * 0.5)
                .repeatForever(autoreverses: true)
                .delay(Double(index) * 0.2),
                value: pulseScale
            )
    }
}

#Preview {
    ContentView()
        .frame(width: 900, height: 700)
}
