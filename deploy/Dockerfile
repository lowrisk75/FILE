# FILE Relay Server - Multi-stage Docker build
# Build: docker build -t file-relay-server:latest -f deploy/Dockerfile .
# Run:   docker run -p 8080:8080/udp -p 8081:8081 file-relay-server:latest

# Stage 1: Build environment
FROM rust:1.83-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /build

# Copy workspace manifests
COPY Cargo.toml Cargo.lock ./
COPY crates/relay_server/Cargo.toml ./crates/relay_server/
COPY crates/relay_daemon/Cargo.toml ./crates/relay_daemon/
COPY crates/relay_common/Cargo.toml ./crates/relay_common/

# Create dummy source files to cache dependencies
RUN mkdir -p crates/relay_server/src && \
    echo "fn main() {}" > crates/relay_server/src/main.rs && \
    mkdir -p crates/relay_daemon/src && \
    echo "pub fn dummy() {}" > crates/relay_daemon/src/lib.rs && \
    mkdir -p crates/relay_common/src && \
    echo "pub fn dummy() {}" > crates/relay_common/src/lib.rs

# Build dependencies (cached layer)
RUN cargo build --release --package relay_server

# Remove dummy artifacts
RUN rm -rf target/release/.fingerprint/relay_* \
    target/release/deps/relay_* \
    target/release/file-relay

# Copy real source code
COPY crates/ ./crates/

# Build release binary
RUN cargo build --release --package relay_server

# Strip binary for smaller size
RUN strip target/release/file-relay

# Stage 2: Runtime environment
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -u 1000 -m -s /bin/bash relay && \
    mkdir -p /var/lib/relay && \
    chown -R relay:relay /var/lib/relay

# Copy binary from builder
COPY --from=builder /build/target/release/file-relay /usr/local/bin/file-relay

# Set ownership
RUN chown relay:relay /usr/local/bin/file-relay

# Switch to non-root user
USER relay

# Expose ports
EXPOSE 8080/udp
EXPOSE 8081/tcp

# Set working directory
WORKDIR /var/lib/relay

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8081/health || exit 1

# Default command
ENTRYPOINT ["/usr/local/bin/file-relay"]
CMD ["--bind", "0.0.0.0:8080", "--health-port", "8081", "--max-peers", "1000"]

# Labels
LABEL org.opencontainers.image.title="FILE Relay Server"
LABEL org.opencontainers.image.description="Zero-trust MPQUIC relay server with CAPoW DoS protection"
LABEL org.opencontainers.image.version="0.1.0"
LABEL org.opencontainers.image.licenses="MIT"
LABEL org.opencontainers.image.source="https://github.com/file-network/relay-server"
