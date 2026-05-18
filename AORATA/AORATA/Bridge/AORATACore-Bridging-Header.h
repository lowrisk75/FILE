//
//  AORATACore-Bridging-Header.h
//  AORATA
//
//  Swift-to-C bridging header for Rust FFI
//

#ifndef AORATACore_Bridging_Header_h
#define AORATACore_Bridging_Header_h

#import <Foundation/Foundation.h>
#import <stdint.h>
#import <stdbool.h>

#pragma mark - Core Transport (MPQUIC)

typedef struct TransportContext TransportContext;

typedef enum {
    TransportSuccess = 0,
    TransportInvalidInput = -1,
    TransportConnectionFailed = -2,
    TransportSendFailed = -3,
    TransportRecvFailed = -4,
    TransportRuntimeError = -5,
    TransportAddressParseFailed = -6,
    TransportEndpointInitFailed = -7,
    TransportPeerNotFound = -8,
    TransportTimeout = -9,
} TransportError;

// Endpoint lifecycle
int transport_create(const char *local_addr, TransportContext **out_context);
void transport_destroy(TransportContext *context);

// Connection management (returns 32-byte PeerId as connection handle)
int transport_connect(TransportContext *context, const char *remote_addr, uint8_t *out_connection_handle);
int transport_disconnect(TransportContext *context, const uint8_t *connection_handle);

// Send/Recv (real MPQUIC encrypted transport)
int transport_send(TransportContext *context, const uint8_t *connection_handle,
                   const uint8_t *data, size_t len);
int transport_recv(TransportContext *context, uint8_t *out_data, size_t capacity,
                   size_t *out_len, uint64_t timeout_ms);

// Legacy test functions (kept for compatibility)
int transport_encrypt(TransportContext *context, const uint8_t *plaintext, size_t plaintext_len,
                      uint8_t *ciphertext, size_t ciphertext_capacity, size_t *out_len);
int transport_decrypt(TransportContext *context, const uint8_t *ciphertext, size_t ciphertext_len,
                      uint8_t *plaintext, size_t plaintext_capacity, size_t *out_len);

#pragma mark - Crypto Fabric (Noise)

typedef struct NoiseSession NoiseSession;

typedef enum {
    NoiseSuccess = 0,
    NoiseInvalidInput = -1,
    NoiseHandshakeFailed = -2,
    NoiseEncryptionFailed = -3,
    NoiseDecryptionFailed = -4,
} NoiseError;

int noise_create_initiator(const uint8_t *local_static_key, const uint8_t *remote_static_key,
                           NoiseSession **out_session);
int noise_create_responder(const uint8_t *local_static_key, NoiseSession **out_session);
void noise_destroy(NoiseSession *session);
int noise_handshake_write(NoiseSession *session, const uint8_t *payload, size_t payload_len,
                           uint8_t *out_message, size_t message_capacity, size_t *out_len);
int noise_handshake_read(NoiseSession *session, const uint8_t *message, size_t message_len,
                          uint8_t *out_payload, size_t payload_capacity, size_t *out_len);
bool noise_is_handshake_complete(NoiseSession *session);
int noise_encrypt(NoiseSession *session, const uint8_t *plaintext, size_t plaintext_len,
                  uint8_t *ciphertext, size_t ciphertext_capacity, size_t *out_len);
int noise_decrypt(NoiseSession *session, const uint8_t *ciphertext, size_t ciphertext_len,
                  uint8_t *plaintext, size_t plaintext_capacity, size_t *out_len);

#pragma mark - VFS Guard (Anti-Ransomware)

typedef enum {
    GuardSuccess = 0,
    GuardInvalidInput = -1,
    GuardHighEntropyDetected = -2,
    GuardConfigError = -3,
} GuardError;

double vfs_calculate_entropy(const uint8_t *data, size_t len, bool use_simd);
int vfs_is_high_entropy(const uint8_t *data, size_t len, double threshold, bool use_simd);
int vfs_is_extension_whitelisted(const char *path);
int vfs_check_write(const char *path, const uint8_t *data, size_t len);

#pragma mark - Edge AI (Routing)

typedef struct AiBackend AiBackend;

typedef struct {
    uint8_t avg_ttl;
    uint32_t asn;
    uint32_t iat_variance_us;
    uint8_t cpu_load;
    uint8_t num_paths;
} NetworkContextFFI;

typedef enum {
    AiSuccess = 0,
    AiInvalidInput = -1,
    AiBackendNotAvailable = -2,
    AiInferenceFailed = -3,
} AiError;

int ai_create(AiBackend **out_backend);
void ai_destroy(AiBackend *backend);
int ai_get_backend_name(AiBackend *backend, uint8_t *out_name, size_t capacity);
int64_t ai_update_lora(AiBackend *backend, const NetworkContextFFI *context);
int64_t ai_infer(AiBackend *backend, const float *input, size_t input_len,
                 float *output, size_t output_capacity, size_t *out_len);
uint8_t ai_get_tier(AiBackend *backend);

#pragma mark - Relay Daemon (CAPoW)

typedef struct {
    uint32_t difficulty;
    uint8_t nonce[32];
    uint64_t timestamp;
} CapowChallenge;

typedef struct {
    uint8_t solution[32];
    uint64_t compute_time_ms;
} CapowProof;

typedef enum {
    CapowSuccess = 0,
    CapowInvalidInput = -1,
    CapowComputeFailed = -2,
    CapowVerifyFailed = -3,
} CapowError;

typedef struct {
    uint32_t ttl;
    uint32_t asn;
    uint64_t iat_ms;
    float cpu_load;
} CapowNetworkContext;

int capow_generate_challenge(uint32_t difficulty, CapowChallenge *out_challenge);
int capow_compute_proof(const CapowChallenge *challenge, CapowProof *out_proof);
int capow_verify_proof(const CapowChallenge *challenge, const CapowProof *proof);
uint32_t capow_recommended_difficulty(const CapowNetworkContext *context);

#endif /* AORATACore_Bridging_Header_h */
