#!/bin/bash
# Validate all JSON rule files are syntactically correct

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

echo "🔍 Validating rule files..."

FAIL_COUNT=0
PASS_COUNT=0

# Find all .json files in proxy/rules
while IFS= read -r -d '' file; do
    if jq empty "$file" 2>/dev/null; then
        echo -e "${GREEN}✓${NC} $file"
        ((PASS_COUNT++))
    else
        echo -e "${RED}✗${NC} $file - Invalid JSON"
        ((FAIL_COUNT++))
    fi
done < <(find proxy/rules -name "*.json" -type f -print0)

echo ""
echo "Results: $PASS_COUNT passed, $FAIL_COUNT failed"

if [ $FAIL_COUNT -gt 0 ]; then
    exit 1
fi

echo -e "${GREEN}✅ All rule files are valid${NC}"
