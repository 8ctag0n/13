# ZyberLink Backend Dockerfile
# Uses pre-built binary (build locally first with: cargo build --release --bin blink-server)

FROM docker.io/library/debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy pre-built binary
COPY target/release/blink-server /app/server

# Copy migrations (backend loads from ./blink-server/migrations)
COPY src/blink-server/migrations /app/blink-server/migrations

# Environment defaults
ENV RUST_LOG=info
ENV HOST=0.0.0.0
ENV PORT=8080

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["/app/server"]
