//
//  AORATACore.swift
//  AORATA
//
//  Swift wrapper for Rust FFI (cleaner API)
//

import Foundation

/// AORATA Core — Swift wrapper for Rust FFI
///
/// Provides type-safe Swift API over raw C FFI functions
///
enum AORATACore {

    // MARK: - Transport

    /// MPQUIC Transport wrapper
    class Transport {
        private var context: OpaquePointer?

        init(localAddress: String) throws {
            var ctx: OpaquePointer?
            let result = localAddress.withCString { addr in
                transport_create(addr, &ctx)
            }

            guard result == 0, let context = ctx else {
                throw AORATAError.transportInitFailed
            }

            self.context = context
        }

        deinit {
            if let ctx = context {
                transport_destroy(ctx)
            }
        }

        /// Connect to remote peer (returns 32-byte PeerId as connection handle)
        func connect(to remoteAddress: String) throws -> Data {
            guard let ctx = context else {
                throw AORATAError.invalidContext
            }

            var connectionHandle = Data(count: 32)
            let result = remoteAddress.withCString { addr in
                connectionHandle.withUnsafeMutableBytes { handlePtr in
                    transport_connect(
                        ctx,
                        addr,
                        handlePtr.baseAddress!.assumingMemoryBound(to: UInt8.self)
                    )
                }
            }

            guard result == 0 else {
                throw AORATAError.connectionFailed
            }

            return connectionHandle
        }

        /// Disconnect from peer
        func disconnect(connectionHandle: Data) throws {
            guard let ctx = context else {
                throw AORATAError.invalidContext
            }

            guard connectionHandle.count == 32 else {
                throw AORATAError.invalidInput
            }

            let result = connectionHandle.withUnsafeBytes { handlePtr in
                transport_disconnect(
                    ctx,
                    handlePtr.baseAddress!.assumingMemoryBound(to: UInt8.self)
                )
            }

            guard result == 0 else {
                throw AORATAError.disconnectFailed
            }
        }

        /// Send data to peer (encrypted via MPQUIC)
        func send(_ data: Data, to connectionHandle: Data) throws {
            guard let ctx = context else {
                throw AORATAError.invalidContext
            }

            guard connectionHandle.count == 32 else {
                throw AORATAError.invalidInput
            }

            let result = connectionHandle.withUnsafeBytes { handlePtr in
                data.withUnsafeBytes { dataPtr in
                    transport_send(
                        ctx,
                        handlePtr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                        dataPtr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                        data.count
                    )
                }
            }

            guard result == 0 else {
                throw AORATAError.sendFailed
            }
        }

        /// Receive data from any peer (decrypted via MPQUIC)
        func receive(timeout: TimeInterval = 1.0) throws -> Data {
            guard let ctx = context else {
                throw AORATAError.invalidContext
            }

            var buffer = Data(count: 65536) // 64KB max packet
            var outLen: Int = 0
            let timeoutMs = UInt64(timeout * 1000)
            let bufferCapacity = buffer.count

            let result = buffer.withUnsafeMutableBytes { bufPtr in
                transport_recv(
                    ctx,
                    bufPtr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                    bufferCapacity,
                    &outLen,
                    timeoutMs
                )
            }

            guard result == 0 else {
                if result == -9 { // TransportTimeout
                    throw AORATAError.timeout
                }
                throw AORATAError.recvFailed
            }

            return buffer.prefix(outLen)
        }

        // MARK: - Legacy test functions

        func encrypt(_ plaintext: Data) throws -> Data {
            guard let ctx = context else {
                throw AORATAError.invalidContext
            }

            var ciphertext = Data(count: plaintext.count + 32)
            var outLen: Int = 0
            let plaintextLen = plaintext.count
            let ciphertextCapacity = ciphertext.count

            let result = plaintext.withUnsafeBytes { plainPtr in
                ciphertext.withUnsafeMutableBytes { cipherPtr in
                    transport_encrypt(
                        ctx,
                        plainPtr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                        plaintextLen,
                        cipherPtr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                        ciphertextCapacity,
                        &outLen
                    )
                }
            }

            guard result == 0 else {
                throw AORATAError.encryptionFailed
            }

            return ciphertext.prefix(outLen)
        }

        func decrypt(_ ciphertext: Data) throws -> Data {
            guard let ctx = context else {
                throw AORATAError.invalidContext
            }

            var plaintext = Data(count: ciphertext.count)
            var outLen: Int = 0
            let ciphertextLen = ciphertext.count
            let plaintextCapacity = plaintext.count

            let result = ciphertext.withUnsafeBytes { cipherPtr in
                plaintext.withUnsafeMutableBytes { plainPtr in
                    transport_decrypt(
                        ctx,
                        cipherPtr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                        ciphertextLen,
                        plainPtr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                        plaintextCapacity,
                        &outLen
                    )
                }
            }

            guard result == 0 else {
                throw AORATAError.decryptionFailed
            }

            return plaintext.prefix(outLen)
        }
    }

    // MARK: - Noise Protocol

    /// Noise XXfallback handshake + transport encryption
    class NoiseSession {
        private var session: OpaquePointer?

        /// Create initiator (client side)
        /// - Parameters:
        ///   - localStaticKey: Local 32-byte X25519 static key
        ///   - remoteStaticKey: Remote 32-byte X25519 static public key (optional for IK pattern)
        init(asInitiator localStaticKey: Data, remoteStaticKey: Data? = nil) throws {
            guard localStaticKey.count == 32 else {
                throw AORATAError.invalidInput
            }
            if let remote = remoteStaticKey, remote.count != 32 {
                throw AORATAError.invalidInput
            }

            var ctx: OpaquePointer?
            let result = localStaticKey.withUnsafeBytes { localPtr in
                if let remote = remoteStaticKey {
                    return remote.withUnsafeBytes { remotePtr in
                        noise_create_initiator(
                            localPtr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                            remotePtr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                            &ctx
                        )
                    }
                } else {
                    return noise_create_initiator(
                        localPtr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                        nil,
                        &ctx
                    )
                }
            }

            guard result == 0, let session = ctx else {
                throw AORATAError.noiseInitFailed
            }

            self.session = session
        }

        /// Create responder (server side)
        init(asResponder localStaticKey: Data) throws {
            guard localStaticKey.count == 32 else {
                throw AORATAError.invalidInput
            }

            var ctx: OpaquePointer?
            let result = localStaticKey.withUnsafeBytes { localPtr in
                noise_create_responder(
                    localPtr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                    &ctx
                )
            }

            guard result == 0, let session = ctx else {
                throw AORATAError.noiseInitFailed
            }

            self.session = session
        }

        deinit {
            if let ctx = session {
                noise_destroy(ctx)
            }
        }

        /// Write handshake message (0-RTT capable)
        func writeHandshake(payload: Data = Data()) throws -> Data {
            guard let ctx = session else {
                throw AORATAError.invalidContext
            }

            var message = Data(count: 1024)
            var outLen: Int = 0
            let payloadLen = payload.count
            let messageCapacity = message.count

            let result = payload.withUnsafeBytes { payloadPtr in
                message.withUnsafeMutableBytes { msgPtr in
                    noise_handshake_write(
                        ctx,
                        payloadPtr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                        payloadLen,
                        msgPtr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                        messageCapacity,
                        &outLen
                    )
                }
            }

            guard result == 0 else {
                throw AORATAError.noiseInitFailed
            }

            return message.prefix(outLen)
        }

        /// Read handshake message
        func readHandshake(message: Data) throws -> Data {
            guard let ctx = session else {
                throw AORATAError.invalidContext
            }

            var payload = Data(count: 1024)
            var outLen: Int = 0
            let messageLen = message.count
            let payloadCapacity = payload.count

            let result = message.withUnsafeBytes { msgPtr in
                payload.withUnsafeMutableBytes { payloadPtr in
                    noise_handshake_read(
                        ctx,
                        msgPtr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                        messageLen,
                        payloadPtr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                        payloadCapacity,
                        &outLen
                    )
                }
            }

            guard result == 0 else {
                throw AORATAError.noiseInitFailed
            }

            return payload.prefix(outLen)
        }

        /// Check if handshake is complete and transition to transport mode
        func isHandshakeComplete() -> Bool {
            guard let ctx = session else { return false }
            return noise_is_handshake_complete(ctx)
        }

        /// Encrypt data (post-handshake)
        func encrypt(_ plaintext: Data) throws -> Data {
            guard let ctx = session else {
                throw AORATAError.invalidContext
            }

            var ciphertext = Data(count: plaintext.count + 16) // ChaCha20-Poly1305 overhead
            var outLen: Int = 0
            let plaintextLen = plaintext.count
            let ciphertextCapacity = ciphertext.count

            let result = plaintext.withUnsafeBytes { plainPtr in
                ciphertext.withUnsafeMutableBytes { cipherPtr in
                    noise_encrypt(
                        ctx,
                        plainPtr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                        plaintextLen,
                        cipherPtr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                        ciphertextCapacity,
                        &outLen
                    )
                }
            }

            guard result == 0 else {
                throw AORATAError.encryptionFailed
            }

            return ciphertext.prefix(outLen)
        }

        /// Decrypt data (post-handshake)
        func decrypt(_ ciphertext: Data) throws -> Data {
            guard let ctx = session else {
                throw AORATAError.invalidContext
            }

            var plaintext = Data(count: ciphertext.count)
            var outLen: Int = 0
            let ciphertextLen = ciphertext.count
            let plaintextCapacity = plaintext.count

            let result = ciphertext.withUnsafeBytes { cipherPtr in
                plaintext.withUnsafeMutableBytes { plainPtr in
                    noise_decrypt(
                        ctx,
                        cipherPtr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                        ciphertextLen,
                        plainPtr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                        plaintextCapacity,
                        &outLen
                    )
                }
            }

            guard result == 0 else {
                throw AORATAError.decryptionFailed
            }

            return plaintext.prefix(outLen)
        }
    }

    // MARK: - VFS Guard

    /// Anti-Ransomware guard
    class VFSGuard {
        func checkWrite(path: String, data: Data) throws {
            let result = path.withCString { pathPtr in
                data.withUnsafeBytes { dataPtr in
                    vfs_check_write(
                        pathPtr,
                        dataPtr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                        data.count
                    )
                }
            }

            if result == -2 {  // GuardHighEntropyDetected
                throw AORATAError.highEntropyDetected
            } else if result != 0 {
                throw AORATAError.vfsCheckFailed
            }
        }

        func calculateEntropy(_ data: Data) -> Double {
            return data.withUnsafeBytes { ptr in
                vfs_calculate_entropy(
                    ptr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                    data.count,
                    false  // SIMD disabled for now
                )
            }
        }
    }

    // MARK: - Edge AI

    /// AI Routing backend
    class AIBackend {
        private var backend: OpaquePointer?

        init() throws {
            var ctx: OpaquePointer?
            let result = ai_create(&ctx)

            guard result == 0, let backend = ctx else {
                throw AORATAError.aiBackendNotAvailable
            }

            self.backend = backend
        }

        deinit {
            if let ctx = backend {
                ai_destroy(ctx)
            }
        }

        func getTier() -> UInt8 {
            guard let ctx = backend else { return 0 }
            return ai_get_tier(ctx)
        }

        func getBackendName() -> String {
            guard let ctx = backend else { return "None" }

            var nameBuffer = [UInt8](repeating: 0, count: 64)
            let len = ai_get_backend_name(ctx, &nameBuffer, 64)

            guard len > 0 else { return "Unknown" }
            return String(bytes: nameBuffer.prefix(Int(len)), encoding: .utf8) ?? "Unknown"
        }
    }

    // MARK: - CAPoW

    /// Proof-of-Work challenge/proof
    class CAPoW {
        func generateChallenge(difficulty: UInt32 = 20) throws -> CapowChallenge {
            var challenge = CapowChallenge()
            let result = capow_generate_challenge(difficulty, &challenge)

            guard result == 0 else {
                throw AORATAError.capowFailed
            }

            return challenge
        }

        func computeProof(challenge: CapowChallenge) throws -> CapowProof {
            var challenge = challenge
            var proof = CapowProof()

            let result = capow_compute_proof(&challenge, &proof)

            guard result == 0 else {
                throw AORATAError.capowFailed
            }

            return proof
        }

        func verifyProof(challenge: CapowChallenge, proof: CapowProof) -> Bool {
            var challenge = challenge
            var proof = proof

            return capow_verify_proof(&challenge, &proof) == 1
        }
    }
}

// MARK: - Errors

enum AORATAError: Error {
    case transportInitFailed
    case invalidContext
    case invalidInput
    case connectionFailed
    case disconnectFailed
    case sendFailed
    case recvFailed
    case timeout
    case encryptionFailed
    case decryptionFailed
    case highEntropyDetected
    case vfsCheckFailed
    case aiBackendNotAvailable
    case capowFailed
    case noiseInitFailed
}
