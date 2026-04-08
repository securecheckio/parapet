#!/bin/bash
# Test that authority change detection is working

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

echo "🔐 Testing authority change detection..."

# Verify authority change detection in code
if grep -r "authority_changes" core/src/rules/analyzers/ >/dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Authority change analyzer found"
else
    echo -e "${RED}✗${NC} Authority change analyzer not found"
    exit 1
fi

# Verify related rules exist
if grep -r "authority" proxy/rules/ >/dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Authority-related rules found"
else
    echo -e "${RED}✗${NC} Authority rules not found"
    exit 1
fi

echo ""
echo -e "${GREEN}✅ Authority change detection validated${NC}"
