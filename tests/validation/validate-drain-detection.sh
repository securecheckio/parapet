#!/bin/bash
# Test that drain detection rules are working

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

echo "🛡️  Testing drain detection capabilities..."

# Verify unlimited delegation rule exists
if grep -r "unlimited.*delegation" proxy/rules/ >/dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Unlimited delegation rule found"
else
    echo -e "${RED}✗${NC} Unlimited delegation rule not found"
    exit 1
fi

# Verify delegation-related analyzers exist in code
if grep -r "delegation_is_unlimited" core/src/rules/analyzers/ >/dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Delegation analyzer found in code"
else
    echo -e "${RED}✗${NC} Delegation analyzer not found"
    exit 1
fi

# Verify test coverage
if cargo test --lib delegation 2>&1 | grep -q "test result: ok"; then
    echo -e "${GREEN}✓${NC} Delegation tests pass"
else
    echo -e "${RED}✗${NC} Delegation tests failed"
    exit 1
fi

echo ""
echo -e "${GREEN}✅ Drain detection validation passed${NC}"
