#!/bin/bash
set -e

# Parapet Rebrand Automation Script (Portable Version)
# Uses standard Unix tools (find + sed) for maximum compatibility

echo "🚀 Starting Parapet rebrand automation (portable version)..."
echo ""

cd "$(dirname "$0")/.."

# Function to replace text in files
replace_in_files() {
    local pattern="$1"
    local replacement="$2"
    local file_pattern="$3"
    
    find . -type f -name "$file_pattern" ! -path "*/target/*" ! -path "*/.git/*" ! -path "*/node_modules/*" -exec sed -i "s|$pattern|$replacement|g" {} +
}

# Phase 1: Update Cargo.toml files
echo "📦 Phase 1: Updating Cargo.toml files..."
replace_in_files "sol-shield-core" "parapet-core" "Cargo.toml"
replace_in_files "sol-shield-proxy" "parapet-proxy" "Cargo.toml"
replace_in_files "sol-shield-scanner" "parapet-scanner" "Cargo.toml"
replace_in_files "sol-shield-api" "parapet-api" "Cargo.toml"
replace_in_files "sol-shield-mcp" "parapet-mcp" "Cargo.toml"
replace_in_files "sol-shield-rpc-perf" "parapet-rpc-perf" "Cargo.toml"
replace_in_files "Sol-Shield Team" "SecureCheck Team" "Cargo.toml"
replace_in_files "github.com/securecheck/sol-shield" "github.com/securecheckio/parapet" "Cargo.toml"
echo "✅ Cargo.toml files updated"

# Phase 2: Update Rust files
echo "🦀 Phase 2: Updating Rust source files..."
find . -type f -name "*.rs" ! -path "*/target/*" ! -path "*/.git/*" -exec sed -i \
    -e "s/sol_shield_core/parapet_core/g" \
    -e "s/sol_shield_proxy/parapet_proxy/g" \
    -e "s/sol_shield_scanner/parapet_scanner/g" \
    -e "s/sol_shield_api/parapet_api/g" \
    -e "s/sol_shield_mcp/parapet_mcp/g" \
    -e "s/SolShield/Parapet/g" \
    -e "s/SOL_SHIELD_/PARAPET_/g" \
    -e "s|// Sol-Shield|// Parapet|g" \
    -e "s|/// Sol-Shield|/// Parapet|g" \
    {} +
echo "✅ Rust files updated"

# Phase 3: Update package.json files
echo "📦 Phase 3: Updating package.json files..."
find . -type f -name "package.json" ! -path "*/node_modules/*" ! -path "*/.git/*" -exec sed -i \
    -e "s/@securecheck\/sol-shield/@securecheck\/parapet/g" \
    -e "s/sol-shield/parapet/g" \
    {} +
echo "✅ package.json files updated"

# Phase 4: Update TypeScript/JavaScript files
echo "📝 Phase 4: Updating TypeScript/JavaScript files..."
find . -type f \( -name "*.ts" -o -name "*.js" \) ! -path "*/node_modules/*" ! -path "*/.git/*" ! -path "*/target/*" -exec sed -i \
    -e "s/sol-shield/parapet/g" \
    -e "s/SolShield/Parapet/g" \
    -e "s|// Sol-Shield|// Parapet|g" \
    {} +
echo "✅ TypeScript/JavaScript files updated"

# Phase 5: Update markdown files
echo "📄 Phase 5: Updating markdown files..."
find . -type f -name "*.md" ! -path "*/.git/*" ! -path "*/target/*" ! -path "*/node_modules/*" -exec sed -i \
    -e "s/Sol-Shield/Parapet/g" \
    -e "s/sol-shield/parapet/g" \
    -e "s/SolShield/Parapet/g" \
    -e "s|github.com/securecheckio/sol-shield|github.com/securecheckio/parapet|g" \
    -e "s|github.com/securecheck/sol-shield|github.com/securecheckio/parapet|g" \
    {} +
echo "✅ Markdown files updated"

# Phase 6: Update config files
echo "⚙️  Phase 6: Updating config files..."
find . -type f \( -name "*.toml" -o -name "*.yaml" -o -name "*.yml" -o -name "*.env" -o -name ".env.*" \) \
    ! -path "*/target/*" ! -path "*/.git/*" ! -path "*/node_modules/*" ! -name "Cargo.toml" -exec sed -i \
    -e "s/sol-shield/parapet/g" \
    -e "s/Sol-Shield/Parapet/g" \
    -e "s/SOL_SHIELD_/PARAPET_/g" \
    {} +
echo "✅ Config files updated"

# Phase 7: Update Docker files
echo "🐳 Phase 7: Updating Docker files..."
find . -type f \( -name "Dockerfile*" -o -name "docker-compose*.yml" -o -name ".dockerignore" \) \
    ! -path "*/.git/*" -exec sed -i "s/sol-shield/parapet/g" {} +
echo "✅ Docker files updated"

# Phase 8: Update test files
echo "🧪 Phase 8: Updating test files..."
find . -path "*/tests/*" -type f \( -name "*.rs" -o -name "*.json" -o -name "*.toml" \) \
    ! -path "*/target/*" ! -path "*/.git/*" -exec sed -i \
    -e "s/sol_shield/parapet/g" \
    -e "s/SolShield/Parapet/g" \
    -e "s/sol-shield/parapet/g" \
    {} +
echo "✅ Test files updated"

echo ""
echo "🎉 Bulk renaming complete!"
echo ""
echo "⚠️  Next steps:"
echo "1. Run: cargo check --workspace"
echo "2. Run: cargo test --workspace"
echo "3. Review critical files manually"
echo "4. Run performance benchmarks to verify no regression"
echo ""
