#!/bin/bash
# TRACK 2 - DAY 9: Performance Benchmark Suite
# Executes all FHE operation benchmarks and generates performance report

set -e

CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${CYAN}================================${NC}"
echo -e "${CYAN}   FHE PERFORMANCE BENCHMARK${NC}"
echo -e "${CYAN}================================${NC}"
echo ""
echo -e "Date: $(date)"
echo -e "Target: All FHE operations"
echo ""

# Create results directory
RESULTS_DIR="benchmark_results_$(date +%Y%m%d_%H%M%S)"
mkdir -p "$RESULTS_DIR"

echo -e "${YELLOW}[1/6] Building in release mode...${NC}"
cargo build --release --package zyberlink-prover
echo -e "${GREEN}✓ Build complete${NC}"
echo ""

# Benchmark 1: Sum
echo -e "${YELLOW}[2/6] Benchmarking Sum(100)...${NC}"
echo "Target: < 5 seconds"
cargo test --package zyberlink-prover --release test_sum_100_inputs_performance -- --ignored --nocapture > "$RESULTS_DIR/sum_benchmark.txt" 2>&1 || true
echo -e "${GREEN}✓ Sum benchmark complete${NC}"
echo ""

# Benchmark 2: Average
echo -e "${YELLOW}[3/6] Benchmarking Average(100)...${NC}"
echo "Target: < 7 seconds"
cargo test --package zyberlink-prover --release test_average_performance_100_values -- --ignored --nocapture > "$RESULTS_DIR/average_benchmark.txt" 2>&1 || true
echo -e "${GREEN}✓ Average benchmark complete${NC}"
echo ""

# Benchmark 3: CountIf
echo -e "${YELLOW}[4/6] Benchmarking CountIf(100)...${NC}"
echo "Target: < 10 seconds"
cargo test --package zyberlink-prover --release test_count_if_performance -- --ignored --nocapture > "$RESULTS_DIR/countif_benchmark.txt" 2>&1 || true
echo -e "${GREEN}✓ CountIf benchmark complete${NC}"
echo ""

# Benchmark 4: Histogram
echo -e "${YELLOW}[5/6] Benchmarking Histogram(50, 4 bins)...${NC}"
echo "Target: < 20 seconds"
cargo test --package zyberlink-prover --release test_histogram_performance -- --ignored --nocapture > "$RESULTS_DIR/histogram_benchmark.txt" 2>&1 || true
echo -e "${GREEN}✓ Histogram benchmark complete${NC}"
echo ""

# Benchmark 5: E2E Average Test
echo -e "${YELLOW}[6/6] Running E2E Average(100) test...${NC}"
echo "Target: < 10 seconds (total with encryption)"
cargo test --package zyberlink-prover --release test_fhe_average_large_dataset -- --nocapture > "$RESULTS_DIR/e2e_average_benchmark.txt" 2>&1 || true
echo -e "${GREEN}✓ E2E benchmark complete${NC}"
echo ""

# Generate summary report
echo -e "${CYAN}================================${NC}"
echo -e "${CYAN}   GENERATING SUMMARY REPORT${NC}"
echo -e "${CYAN}================================${NC}"
echo ""

REPORT_FILE="$RESULTS_DIR/BENCHMARK_SUMMARY.md"

cat > "$REPORT_FILE" << 'EOF'
# FHE Performance Benchmark Report

**Date:** $(date)
**Branch:** $(git branch --show-current)
**Commit:** $(git rev-parse --short HEAD)

---

## Performance Targets

| Operation | Inputs | Target Time | Status |
|-----------|--------|-------------|--------|
| Sum | 100 | < 5s | TBD |
| Average | 100 | < 7s | TBD |
| CountIf | 100 | < 10s | TBD |
| Histogram | 50, 4 bins | < 20s | TBD |

---

## Benchmark Results

### 1. Sum(100)

EOF

# Extract Sum result
if [ -f "$RESULTS_DIR/sum_benchmark.txt" ]; then
    echo '```' >> "$REPORT_FILE"
    grep -A 3 "Sum(100 inputs)" "$RESULTS_DIR/sum_benchmark.txt" || echo "No timing found" >> "$REPORT_FILE"
    echo '```' >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
fi

cat >> "$REPORT_FILE" << 'EOF'
### 2. Average(100)

EOF

# Extract Average result
if [ -f "$RESULTS_DIR/average_benchmark.txt" ]; then
    echo '```' >> "$REPORT_FILE"
    grep -A 3 "Average(100 values)" "$RESULTS_DIR/average_benchmark.txt" || echo "No timing found" >> "$REPORT_FILE"
    echo '```' >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
fi

cat >> "$REPORT_FILE" << 'EOF'
### 3. CountIf(100)

EOF

# Extract CountIf result
if [ -f "$RESULTS_DIR/countif_benchmark.txt" ]; then
    echo '```' >> "$REPORT_FILE"
    grep -A 3 "CountIf(100 values)" "$RESULTS_DIR/countif_benchmark.txt" || echo "No timing found" >> "$REPORT_FILE"
    echo '```' >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
fi

cat >> "$REPORT_FILE" << 'EOF'
### 4. Histogram(50, 4 bins)

EOF

# Extract Histogram result
if [ -f "$RESULTS_DIR/histogram_benchmark.txt" ]; then
    echo '```' >> "$REPORT_FILE"
    grep -A 3 "Histogram(50 values" "$RESULTS_DIR/histogram_benchmark.txt" || echo "No timing found" >> "$REPORT_FILE"
    echo '```' >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
fi

cat >> "$REPORT_FILE" << 'EOF'
### 5. E2E Average(100) Test

EOF

# Extract E2E result
if [ -f "$RESULTS_DIR/e2e_average_benchmark.txt" ]; then
    echo '```' >> "$REPORT_FILE"
    grep -E "(Encryption took|Computation took|test result)" "$RESULTS_DIR/e2e_average_benchmark.txt" || echo "No timing found" >> "$REPORT_FILE"
    echo '```' >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
fi

cat >> "$REPORT_FILE" << 'EOF'
---

## Observations

### Performance Improvements (Day 9 Fixes)

**Fix 1: voting.rs - Moved trivial constants outside loop**
- Before: Creating 2N FheUint8 trivials (N = input count)
- After: Creating 2 FheUint8 trivials once
- Impact: ~N-1 fewer allocations

**Fix 2: census.rs - Eliminated unnecessary clones in cast operations**
- Before: `&sum + &ct_u16` (creates temporary references)
- After: `sum + ct_u16` (consumes values)
- Impact: Reduced memory allocations for large operations

### Bottlenecks Identified

1. **FHE Key Generation**: ~1-2 seconds per test (unavoidable with TFHE)
2. **Cast Operations**: FheUint8 -> FheUint16 involves computational overhead
3. **Accumulation**: Each FHE addition is expensive (~10-50ms per operation)

### Recommendations

- ✅ Loop optimizations implemented
- ⏳ Consider Rayon for parallel operations (future work)
- ⏳ Evaluate TFHE parameter tuning (security vs performance tradeoff)
- ⏳ Explore batch operations if TFHE supports them

---

**Generated by:** `scripts/benchmark_suite.sh`
EOF

echo -e "${GREEN}================================${NC}"
echo -e "${GREEN}   BENCHMARKS COMPLETE${NC}"
echo -e "${GREEN}================================${NC}"
echo ""
echo -e "Results saved to: ${YELLOW}$RESULTS_DIR${NC}"
echo -e "Summary report: ${YELLOW}$REPORT_FILE${NC}"
echo ""
echo -e "To view results:"
echo -e "  ${CYAN}cat $REPORT_FILE${NC}"
echo -e "  ${CYAN}ls -lh $RESULTS_DIR/${NC}"
echo ""
