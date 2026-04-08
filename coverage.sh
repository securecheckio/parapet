#!/bin/bash
# Test Coverage Script for Sol-Shield
# Generates coverage reports using cargo-llvm-cov

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Sol-Shield Test Coverage ===${NC}\n"

# Check if cargo-llvm-cov is installed
if ! command -v cargo-llvm-cov &> /dev/null; then
    echo -e "${YELLOW}cargo-llvm-cov not found. Installing...${NC}"
    cargo install cargo-llvm-cov
fi

# Check if WASM target is installed
if ! rustup target list | grep -q "wasm32-unknown-unknown (installed)"; then
    echo -e "${YELLOW}Installing WASM target...${NC}"
    rustup target add wasm32-unknown-unknown
fi

# Build WASM mock if it exists
if [ -d "core/tests/wasm_mock" ]; then
    echo -e "${BLUE}Building WASM mock...${NC}"
    cd core/tests/wasm_mock
    cargo build --target wasm32-unknown-unknown --release
    cd ../../..
fi

# Parse command line arguments
OUTPUT_FORMAT="html"
OPEN_REPORT=false
SUMMARY_ONLY=false
PACKAGE=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --html)
            OUTPUT_FORMAT="html"
            shift
            ;;
        --lcov)
            OUTPUT_FORMAT="lcov"
            shift
            ;;
        --json)
            OUTPUT_FORMAT="json"
            shift
            ;;
        --open)
            OPEN_REPORT=true
            shift
            ;;
        --summary)
            SUMMARY_ONLY=true
            shift
            ;;
        --package|-p)
            PACKAGE="$2"
            shift 2
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --html          Generate HTML report (default)"
            echo "  --lcov          Generate LCOV report"
            echo "  --json          Generate JSON report"
            echo "  --open          Open HTML report in browser"
            echo "  --summary       Show summary only (no report generation)"
            echo "  --package, -p   Run coverage for specific package"
            echo "  --help, -h      Show this help message"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# Build package filter
PACKAGE_FILTER=""
if [ -n "$PACKAGE" ]; then
    PACKAGE_FILTER="--package $PACKAGE"
    echo -e "${BLUE}Running coverage for package: ${PACKAGE}${NC}\n"
fi

# Run coverage based on options
if [ "$SUMMARY_ONLY" = true ]; then
    echo -e "${BLUE}Generating coverage summary...${NC}\n"
    cargo llvm-cov $PACKAGE_FILTER --workspace --all-features --summary-only
else
    case $OUTPUT_FORMAT in
        html)
            echo -e "${BLUE}Generating HTML coverage report...${NC}\n"
            cargo llvm-cov $PACKAGE_FILTER --workspace --all-features --html
            
            echo -e "\n${GREEN}✅ HTML report generated at: target/llvm-cov/html/index.html${NC}"
            
            if [ "$OPEN_REPORT" = true ]; then
                if command -v xdg-open &> /dev/null; then
                    xdg-open target/llvm-cov/html/index.html
                elif command -v open &> /dev/null; then
                    open target/llvm-cov/html/index.html
                else
                    echo -e "${YELLOW}Could not open browser automatically${NC}"
                fi
            fi
            ;;
        lcov)
            echo -e "${BLUE}Generating LCOV coverage report...${NC}\n"
            cargo llvm-cov $PACKAGE_FILTER --workspace --all-features --lcov --output-path lcov.info
            echo -e "\n${GREEN}✅ LCOV report generated at: lcov.info${NC}"
            ;;
        json)
            echo -e "${BLUE}Generating JSON coverage report...${NC}\n"
            cargo llvm-cov $PACKAGE_FILTER --workspace --all-features --json --output-path coverage.json
            echo -e "\n${GREEN}✅ JSON report generated at: coverage.json${NC}"
            ;;
    esac
    
    # Show summary
    echo -e "\n${BLUE}Coverage Summary:${NC}"
    cargo llvm-cov $PACKAGE_FILTER --workspace --all-features --summary-only
fi

# Check coverage threshold
echo -e "\n${BLUE}Checking coverage threshold...${NC}"
COVERAGE=$(cargo llvm-cov $PACKAGE_FILTER --workspace --all-features --summary-only | grep -oP 'TOTAL.*?\K\d+\.\d+(?=%)' || echo "0")

THRESHOLD=70.0

if (( $(echo "$COVERAGE < $THRESHOLD" | bc -l 2>/dev/null || echo "0") )); then
    echo -e "${RED}❌ Coverage ${COVERAGE}% is below threshold ${THRESHOLD}%${NC}"
    exit 1
else
    echo -e "${GREEN}✅ Coverage ${COVERAGE}% meets threshold ${THRESHOLD}%${NC}"
fi
