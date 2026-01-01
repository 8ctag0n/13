#!/bin/bash
# Build and run Cairo tests using Docker

set -e

echo "Building Cairo test container..."
docker build -t pbtcfi-cairo-test -f Dockerfile.test .

echo "Running Cairo tests..."
docker run --rm -v $(pwd):/workspace pbtcfi-cairo-test scarb test

echo "Tests completed successfully!"
