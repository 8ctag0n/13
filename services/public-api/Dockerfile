# ZyberLink Public API Gateway Dockerfile
# Uses pre-built binary (build locally first with: cargo build --release -p public-api)

FROM docker.io/library/debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy pre-built binary
COPY target/release/public-api /app/server

# Environment defaults
ENV RUST_LOG=info
ENV HOST=0.0.0.0
ENV PORT=3000
ENV X402_URL=http://x402:8081
ENV BLINK_URL=http://blink:8080
ENV RATE_LIMIT_PER_MIN=100

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=3s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

CMD ["/app/server"]
