# Aptos Development Environment
# Similar a cairo-dev pero para contratos Move
#
# Build:
#   podman build -t aptos-dev:local docker/aptos/
#
# Run:
#   podman run --rm -it -v $(pwd):/workspace aptos-dev:local

FROM docker.io/library/ubuntu:22.04

# Avoid interactive prompts
ENV DEBIAN_FRONTEND=noninteractive

# Install system dependencies
RUN apt-get update && apt-get install -y \
    curl \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    jq \
    unzip \
    python3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install Aptos CLI (latest stable)
RUN curl -fsSL "https://aptos.dev/scripts/install_cli.py" | python3
ENV PATH="/root/.local/bin:${PATH}"

# Verify installation
RUN aptos --version

# Create workspace
WORKDIR /workspace

# Default shell
CMD ["/bin/bash"]
