#!/bin/bash
# Quick benchmark script for Day 9 performance validation

CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${CYAN}================================${NC}"
echo -e "${CYAN}   QUICK FHE BENCHMARKS${NC}"
echo -e "${CYAN}================================${NC}"
echo ""

cd /home/deploy/experimental/zyberlink-fhe

# Run benchmarks sequentially and save results
echo -e "${YELLOW}[1/4] CountIf(100) benchmark...${NC}"
cargo test --package zyberlink-prover --lib circuits::voting::tests::test_count_if_performance --release -- --ignored --nocapture 2>&1 | tee countif_100_bench.txt
echo -e "${GREEN}✓ CountIf done${NC}"
echo ""

echo -e "${YELLOW}[2/4] Average(100) benchmark...${NC}"
cargo test --package zyberlink-prover --lib circuits::demographics::tests::test_average_performance_100_values --release -- --ignored --nocapture 2>&1 | tee avg_100_bench.txt
echo -e "${GREEN}✓ Average done${NC}"
echo ""

echo -e "${YELLOW}[3/4] Sum(100) benchmark...${NC}"
cargo test --package zyberlink-prover --lib circuits::census::tests::test_sum_100_inputs_performance --release -- --ignored --nocapture 2>&1 | tee sum_100_bench.txt
echo -e "${GREEN}✓ Sum done${NC}"
echo ""

echo -e "${YELLOW}[4/4] Histogram(50, 4 bins) benchmark...${NC}"
cargo test --package zyberlink-prover --lib circuits::voting::tests::test_histogram_performance --release -- --ignored --nocapture 2>&1 | tee histogram_bench.txt
echo -e "${GREEN}✓ Histogram done${NC}"
echo ""

echo -e "${GREEN}================================${NC}"
echo -e "${GREEN}   ALL BENCHMARKS COMPLETE${NC}"
echo -e "${GREEN}================================${NC}"
echo ""
echo "Results saved to:"
echo "  - countif_100_bench.txt"
echo "  - avg_100_bench.txt"
echo "  - sum_100_bench.txt"
echo "  - histogram_bench.txt"
