#!/usr/bin/env bash
# Performance benchmark runner for Sol-Shield
# Usage: ./scripts/benchmark.sh [quick|standard|extended|concurrent]

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BENCHMARK_DIR="$PROJECT_ROOT/docs/benchmarks"
OUTPUT_FILE="$BENCHMARK_DIR/rpc-perf-$(date +%Y-%m-%d).md"

# Default mode
MODE="${1:-standard}"

echo -e "${BLUE}Sol-Shield Performance Benchmark${NC}"
echo "=================================="
echo ""

# Check if we're in the right directory
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    echo -e "${RED}Error: Must be run from sol-shield directory${NC}"
    exit 1
fi

# Parse mode and set parameters
case "$MODE" in
    quick)
        ITERATIONS=100
        WARMUP=20
        CONCURRENCY=1
        RELEASE=""
        echo -e "${YELLOW}Mode: Quick smoke test${NC}"
        echo "Iterations: $ITERATIONS (warmup: $WARMUP)"
        echo "Time: ~1 minute"
        ;;
    standard)
        ITERATIONS=500
        WARMUP=50
        CONCURRENCY=1
        RELEASE="--release"
        echo -e "${GREEN}Mode: Standard benchmark${NC}"
        echo "Iterations: $ITERATIONS (warmup: $WARMUP)"
        echo "Time: ~6-7 minutes"
        ;;
    extended)
        ITERATIONS=1000
        WARMUP=100
        CONCURRENCY=1
        RELEASE="--release"
        echo -e "${GREEN}Mode: Extended benchmark${NC}"
        echo "Iterations: $ITERATIONS (warmup: $WARMUP)"
        echo "Time: ~12-15 minutes"
        ;;
    concurrent)
        ITERATIONS=500
        WARMUP=50
        CONCURRENCY=4
        RELEASE="--release"
        echo -e "${GREEN}Mode: Concurrency test${NC}"
        echo "Iterations: $ITERATIONS (warmup: $WARMUP)"
        echo "Concurrency: $CONCURRENCY"
        echo "Time: ~3-4 minutes"
        ;;
    *)
        echo -e "${RED}Error: Invalid mode '$MODE'${NC}"
        echo "Usage: $0 [quick|standard|extended|concurrent]"
        exit 1
        ;;
esac

echo ""
echo "Starting benchmark..."
echo ""

# Run the benchmark
cd "$PROJECT_ROOT"

if [ -n "$RELEASE" ]; then
    cargo run -p rpc-perf --release -- \
        --iterations "$ITERATIONS" \
        --warmup "$WARMUP" \
        --concurrency "$CONCURRENCY" | tee /tmp/benchmark-output.txt
else
    cargo run -p rpc-perf -- \
        --iterations "$ITERATIONS" \
        --warmup "$WARMUP" \
        --concurrency "$CONCURRENCY" | tee /tmp/benchmark-output.txt
fi

echo ""
echo -e "${GREEN}Benchmark completed!${NC}"
echo ""

# Extract key metrics from output
if grep -q "rpc-perf summary" /tmp/benchmark-output.txt; then
    echo -e "${BLUE}Key Results:${NC}"
    echo ""
    
    # Extract the results table
    sed -n '/case.*expect.*p50ms/,/^$/p' /tmp/benchmark-output.txt | head -n 10
    
    echo ""
    
    # Compare to baseline if it exists
    if [ -f "$BENCHMARK_DIR/baseline.md" ]; then
        echo -e "${BLUE}Comparing to baseline...${NC}"
        echo ""
        
        # Extract baseline p50 for sol-transfer-pass (simple comparison)
        BASELINE_P50=$(grep "sol-transfer-pass" "$BENCHMARK_DIR/baseline.md" | grep -oP '\| pass \| \K[0-9.]+' | head -1)
        CURRENT_P50=$(grep "sol-transfer-pass" /tmp/benchmark-output.txt | awk '{print $3}')
        
        if [ -n "$BASELINE_P50" ] && [ -n "$CURRENT_P50" ]; then
            # Calculate percentage change (using bc if available, otherwise awk)
            if command -v bc &> /dev/null; then
                CHANGE=$(echo "scale=1; (($CURRENT_P50 - $BASELINE_P50) / $BASELINE_P50) * 100" | bc)
            else
                CHANGE=$(awk "BEGIN {printf \"%.1f\", (($CURRENT_P50 - $BASELINE_P50) / $BASELINE_P50) * 100}")
            fi
            
            echo "sol-transfer-pass p50:"
            echo "  Baseline: ${BASELINE_P50}ms"
            echo "  Current:  ${CURRENT_P50}ms"
            
            # Color code the change
            if (( $(echo "$CHANGE < 10" | bc -l 2>/dev/null || echo "1") )); then
                echo -e "  Change:   ${GREEN}${CHANGE}%${NC} ✅ Good"
            elif (( $(echo "$CHANGE < 25" | bc -l 2>/dev/null || echo "0") )); then
                echo -e "  Change:   ${YELLOW}${CHANGE}%${NC} ⚠️  Warning - investigate"
            else
                echo -e "  Change:   ${RED}${CHANGE}%${NC} 🚨 Critical - requires fix"
            fi
        fi
        
        echo ""
    fi
    
    # Offer to save results
    if [ "$MODE" = "standard" ] || [ "$MODE" = "extended" ]; then
        echo -e "${BLUE}Save results?${NC}"
        echo "This will create: $OUTPUT_FILE"
        read -p "Save results? (y/N): " -n 1 -r
        echo
        
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            # Create benchmark directory if it doesn't exist
            mkdir -p "$BENCHMARK_DIR"
            
            # Copy output to file (user should edit to add full analysis)
            echo "# RPC Performance Benchmark - $(date +%Y-%m-%d)" > "$OUTPUT_FILE"
            echo "" >> "$OUTPUT_FILE"
            echo "## Raw Results" >> "$OUTPUT_FILE"
            echo "" >> "$OUTPUT_FILE"
            echo '```' >> "$OUTPUT_FILE"
            cat /tmp/benchmark-output.txt >> "$OUTPUT_FILE"
            echo '```' >> "$OUTPUT_FILE"
            echo "" >> "$OUTPUT_FILE"
            echo "## Analysis" >> "$OUTPUT_FILE"
            echo "" >> "$OUTPUT_FILE"
            echo "TODO: Add analysis, comparison to baseline, and conclusions." >> "$OUTPUT_FILE"
            echo "See docs/benchmarks/rpc-perf-2026-04-07.md for template." >> "$OUTPUT_FILE"
            
            echo -e "${GREEN}Results saved to: $OUTPUT_FILE${NC}"
            echo ""
            echo "Next steps:"
            echo "1. Edit $OUTPUT_FILE to add analysis"
            echo "2. Compare results to baseline.md"
            echo "3. Commit results: git add docs/benchmarks/ && git commit -m 'perf: benchmark results $(date +%Y-%m-%d)'"
            
            # Ask about updating baseline
            if [ -f "$BENCHMARK_DIR/baseline.md" ]; then
                echo ""
                read -p "Update baseline.md with these results? (y/N): " -n 1 -r
                echo
                
                if [[ $REPLY =~ ^[Yy]$ ]]; then
                    cp "$OUTPUT_FILE" "$BENCHMARK_DIR/baseline.md"
                    echo -e "${GREEN}Baseline updated!${NC}"
                fi
            fi
        fi
    fi
else
    echo -e "${RED}Error: Could not parse benchmark results${NC}"
    exit 1
fi

# Cleanup
rm -f /tmp/benchmark-output.txt

echo ""
echo -e "${GREEN}Done!${NC}"
