# Builder stage
FROM rust:latest as builder

WORKDIR /usr/src/app
COPY . .

# Build the release binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install necessary runtime dependencies (OpenSSL is needed for lettre)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /usr/src/app/target/release/gmail-mcp-server /usr/local/bin/gmail-mcp-server

# Set the entrypoint
ENTRYPOINT ["gmail-mcp-server"]
