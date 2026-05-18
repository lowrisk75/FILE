//
//  UnidirectionalIPC.swift
//  AORATATunnelExtension
//
//  Unidirectional IPC for TOCTOU prevention
//  Producer (app) has PROT_WRITE, Consumer (extension) has PROT_READ only
//

import Foundation
import os.log

/// Unidirectional IPC error types
enum UnidirectionalIPCError: Error {
    case appGroupNotFound
    case mmapFailed(errno: Int32)
    case invalidBufferSize
    case portTransferFailed
}

/// Unidirectional IPC Bridge
///
/// ## Architecture
///
/// - **Producer** (Host App): PROT_WRITE via mmap
/// - **Consumer** (Extension): PROT_READ only
/// - **SPSC**: Quetzalcoatl lock-free ring buffer
/// - **App Groups**: Mach port via `com.apple.security.application-groups`
///
/// ## Memory Safety
///
/// - **TOCTOU Prevention**: Consumer cannot modify shared memory (MMU-enforced)
/// - **Single Fetch**: Data read once, copied to local stack, then validated
/// - **MIE Ready**: Hardware tagging prevents buffer overflows
///
class UnidirectionalIPC {

    // MARK: - Constants

    /// Default ring buffer size (2MB — fits Jetsam <15MB)
    static let defaultBufferSize: Int = 2 * 1024 * 1024

    // MARK: - Properties

    /// Shared memory base pointer (PROT_READ only for extension)
    private let basePointer: UnsafeMutableRawPointer

    /// Ring buffer size
    private let bufferSize: Int

    /// SPSC ring buffer handle
    private var ringBuffer: SPSCRingBuffer

    /// Logger
    private let logger = Logger(subsystem: "com.lorislab.aorata.tunnel", category: "IPC")

    // MARK: - Initialization

    /// Initialize unidirectional IPC
    ///
    /// ## Setup Flow
    /// 1. App Group container URL resolution
    /// 2. Mach port retrieval (via XPC or handleAppMessage bootstrap)
    /// 3. mmap attach with PROT_READ only
    /// 4. SPSC ring buffer initialization
    ///
    /// - Parameter appGroup: App Group identifier (e.g., "group.com.lorislab.aorata")
    /// - Throws: `UnidirectionalIPCError` if setup fails
    ///
    init(appGroup: String) throws {
        logger.info("🔧 Initializing unidirectional IPC (App Group: \(appGroup))")

        // Step 1: Resolve App Group container
        guard let containerURL = FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: appGroup) else {
            throw UnidirectionalIPCError.appGroupNotFound
        }

        logger.debug("📁 App Group container: \(containerURL.path)")

        // Step 2: TODO - Retrieve Mach port from host app via handleAppMessage
        // For now, use a placeholder file-based mmap (not production-ready)
        let mmapPath = containerURL.appendingPathComponent("aorata_ipc.mmap").path

        // Step 3: Attach mmap with PROT_READ only
        self.bufferSize = Self.defaultBufferSize
        self.basePointer = try Self.attachSharedMemory(path: mmapPath, size: bufferSize)

        logger.info("✅ mmap attached: \(self.bufferSize) bytes @ \(String(describing: self.basePointer))")

        // Step 4: Initialize SPSC ring buffer
        self.ringBuffer = SPSCRingBuffer(basePointer: basePointer, size: bufferSize)

        logger.info("✅ Unidirectional IPC ready (SPSC ring buffer)")
    }

    deinit {
        // munmap on deallocation
        munmap(basePointer, bufferSize)
        logger.info("🗑️ Unidirectional IPC destroyed (munmap)")
    }

    // MARK: - Memory Management

    /// Attach shared memory with PROT_READ only (consumer side)
    ///
    /// **CRITICAL**: This is placeholder file-backed mmap. Production must use:
    /// 1. `mach_make_memory_entry_64` on producer (host app)
    /// 2. Mach port transfer via XPC/handleAppMessage
    /// 3. `mach_vm_map` with VM_PROT_READ on consumer (extension)
    ///
    private static func attachSharedMemory(path: String, size: Int) throws -> UnsafeMutableRawPointer {
        // Open or create file (O_CREAT | O_RDWR for producer, O_RDONLY for consumer)
        // For extension (consumer), we open O_RDONLY
        let fd = open(path, O_RDONLY | O_CREAT, 0o666)
        guard fd >= 0 else {
            throw UnidirectionalIPCError.mmapFailed(errno: errno)
        }
        defer { close(fd) }

        // Ensure file size (producer should have done this)
        // Consumer just attaches, but we check size
        var stat = stat()
        guard fstat(fd, &stat) == 0 else {
            throw UnidirectionalIPCError.mmapFailed(errno: errno)
        }

        if stat.st_size < size {
            // File too small — producer not ready
            throw UnidirectionalIPCError.invalidBufferSize
        }

        // mmap with PROT_READ only (unidirectional consumer)
        let ptr = mmap(nil, size, PROT_READ, MAP_SHARED, fd, 0)
        guard ptr != MAP_FAILED else {
            throw UnidirectionalIPCError.mmapFailed(errno: errno)
        }

        return ptr!
    }

    // MARK: - Packet Operations

    /// Pop packet from SPSC ring buffer (zero-copy)
    ///
    /// ## Zero-Copy via Typestate
    /// - `pop_ref()` returns immutable view into shared memory
    /// - No heap allocation, no memcpy
    /// - Single fetch (TOCTOU-safe)
    ///
    /// - Returns: Packet data (copied to local buffer for validation)
    ///
    func popPacket() -> Data? {
        return ringBuffer.pop()
    }
}

/// SPSC Ring Buffer (placeholder — real impl uses quetzalcoatl Rust crate)
///
/// ## Architecture
/// - **Head**: Consumer read position (Acquire ordering)
/// - **Tail**: Producer write position (Release ordering)
/// - **Cache-line padding**: 64 bytes to prevent false sharing
///
/// **TODO**: Replace with FFI to Rust `quetzalcoatl::buffer::spsc::Buffer`
///
private struct SPSCRingBuffer {
    let basePointer: UnsafeMutableRawPointer
    let size: Int

    /// Pop packet (placeholder)
    func pop() -> Data? {
        // TODO: Atomic load Head with Acquire ordering
        // TODO: Compare Head vs Tail
        // TODO: Read packet size, copy to local Data (single fetch)
        // TODO: Atomic store Head with Release ordering
        return nil
    }
}
