# ZyberLink X402 Anti-Spam Gateway Dockerfile
# Uses pre-built binary (build locally first with: cargo build --release -p x402-server)

FROM docker.io/library/debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy pre-built binary
COPY target/release/x402-server /app/server

# Environment defaults
ENV RUST_LOG=info
ENV HOST=0.0.0.0
ENV PORT=8081
ENV BLINK_URL=http://blink:8080
ENV DATABASE_URL=postgresql://zyberlink:dev_password@postgres:5432/zyberlink

EXPOSE 8081

HEALTHCHECK --interval=30s --timeout=3s --retries=3 \
    CMD curl -f http://localhost:8081/health || exit 1

CMD ["/app/server"]
