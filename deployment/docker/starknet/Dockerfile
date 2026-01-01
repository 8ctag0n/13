FROM docker.io/library/rust:1.75-slim

# Install dependencies
RUN apt-get update && apt-get install -y \
    curl \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    jq \
    && rm -rf /var/lib/apt/lists/*

# Install Scarb (Cairo package manager)
RUN curl --proto '=https' --tlsv1.2 -sSf https://docs.swmansion.com/scarb/install.sh | bash -s -- -v 2.8.4

# Install Starkli
RUN curl https://get.starkli.sh | bash
RUN /root/.starkli/bin/starkliup

# Add tools to PATH
ENV PATH="/root/.local/bin:/root/.scarb/bin:/root/.starkli/bin:${PATH}"

WORKDIR /workspace

CMD ["/bin/bash"]
