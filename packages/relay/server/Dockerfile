# Multi-stage build for smaller final image
FROM rust:1.83-slim as builder

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY packages/ packages/

# Build only the relay server (release mode for smaller binary)
RUN cargo build --release -p chadreview_relay_server

# Runtime stage - minimal image
FROM debian:bookworm-slim

# Install CA certificates for HTTPS
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/relay-server /usr/local/bin/relay-server

# Expose the port
EXPOSE 8080

# Run the relay server
CMD ["relay-server"]
