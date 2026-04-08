#!/bin/bash
# Wrapper script for update-safe-lists tool

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Binary path (in parapet/target, not scanner/target)
BINARY_PATH="$SCRIPT_DIR/../target/release/update-safe-lists"

# Build if needed
if [ ! -f "$BINARY_PATH" ]; then
    echo -e "${YELLOW}Building update-safe-lists tool...${NC}"
    cd "$SCRIPT_DIR/.."
    cargo build --release --bin update-safe-lists
    cd - > /dev/null
fi

# Check if binary exists
if [ ! -f "$BINARY_PATH" ]; then
    echo -e "${RED}Error: Failed to build update-safe-lists binary${NC}"
    exit 1
fi

# Show what we're doing
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}        Parapet Safe Lists Update Tool${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo

# Run the binary with all arguments
"$BINARY_PATH" "$@"
EXIT_CODE=$?

echo
if [ $EXIT_CODE -eq 0 ]; then
    echo -e "${GREEN}✨ Update process completed successfully${NC}"
else
    echo -e "${RED}❌ Update process failed with exit code: $EXIT_CODE${NC}"
fi

exit $EXIT_CODE
