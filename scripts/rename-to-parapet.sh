#!/bin/bash
set -e

# Parapet Rebrand Automation Script
# This script performs bulk renaming operations for the Sol-Shield → Parapet rebrand

echo "🚀 Starting Parapet rebrand automation..."
echo ""

# Check if fd and sd are installed
if ! command -v fd &> /dev/null; then
    echo "❌ Error: 'fd' command not found. Please install fd-find:"
    echo "   Ubuntu/Debian: sudo apt install fd-find"
    echo "   macOS: brew install fd"
    exit 1
fi

if ! command -v sd &> /dev/null; then
    echo "❌ Error: 'sd' command not found. Please install sd:"
    echo "   Ubuntu/Debian: cargo install sd"
    echo "   macOS: brew install sd"
    exit 1
fi

echo "✅ Prerequisites check passed"
echo ""

# Phase 1: Rename Rust crate names in Cargo.toml files
echo "📦 Phase 1: Updating Cargo.toml files..."
fd -e toml 'Cargo.toml' -x sd 'sol-shield-core' 'parapet-core' {}
fd -e toml 'Cargo.toml' -x sd 'sol-shield-proxy' 'parapet-proxy' {}
fd -e toml 'Cargo.toml' -x sd 'sol-shield-scanner' 'parapet-scanner' {}
fd -e toml 'Cargo.toml' -x sd 'sol-shield-api' 'parapet-api' {}
fd -e toml 'Cargo.toml' -x sd 'sol-shield-mcp' 'parapet-mcp' {}
fd -e toml 'Cargo.toml' -x sd 'sol-shield-rpc-perf' 'parapet-rpc-perf' {}
fd -e toml 'Cargo.toml' -x sd 'Sol-Shield Team' 'SecureCheck Team' {}
fd -e toml 'Cargo.toml' -x sd 'github.com/securecheck/sol-shield' 'github.com/securecheckio/parapet' {}
echo "✅ Cargo.toml files updated"
echo ""

# Phase 2: Rename Rust module imports
echo "🦀 Phase 2: Updating Rust module imports..."
fd -e rs -x sd 'sol_shield_core' 'parapet_core' {}
fd -e rs -x sd 'sol_shield_proxy' 'parapet_proxy' {}
fd -e rs -x sd 'sol_shield_scanner' 'parapet_scanner' {}
fd -e rs -x sd 'sol_shield_api' 'parapet_api' {}
fd -e rs -x sd 'sol_shield_mcp' 'parapet_mcp' {}
echo "✅ Rust module imports updated"
echo ""

# Phase 3: Rename struct/enum names
echo "🏗️  Phase 3: Updating struct/enum names..."
fd -e rs -x sd 'SolShield' 'Parapet' {}
echo "✅ Struct/enum names updated"
echo ""

# Phase 4: Rename environment variables
echo "🌍 Phase 4: Updating environment variables..."
fd -e rs -e toml -e md -e env -e yaml -e yml -x sd 'SOL_SHIELD_' 'PARAPET_' {}
echo "✅ Environment variables updated"
echo ""

# Phase 5: Update package.json files (TypeScript/JavaScript)
echo "📦 Phase 5: Updating package.json files..."
fd 'package.json' -x sd '@securecheck/sol-shield' '@securecheck/parapet' {}
fd 'package.json' -x sd 'sol-shield' 'parapet' {}
echo "✅ package.json files updated"
echo ""

# Phase 6: Update TypeScript/JavaScript code
echo "📝 Phase 6: Updating TypeScript/JavaScript code..."
fd -e ts -e js -x sd 'sol-shield' 'parapet' {}
fd -e ts -e js -x sd 'SolShield' 'Parapet' {}
echo "✅ TypeScript/JavaScript code updated"
echo ""

# Phase 7: Update markdown files
echo "📄 Phase 7: Updating markdown files..."
fd -e md -x sd 'Sol-Shield' 'Parapet' {}
fd -e md -x sd 'sol-shield' 'parapet' {}
fd -e md -x sd 'SolShield' 'Parapet' {}
echo "✅ Markdown files updated"
echo ""

# Phase 8: Update code comments
echo "💬 Phase 8: Updating code comments..."
fd -e rs -e ts -e js -x sd '// Sol-Shield' '// Parapet' {}
fd -e rs -e ts -e js -x sd '/// Sol-Shield' '/// Parapet' {}
echo "✅ Code comments updated"
echo ""

# Phase 9: Update URLs
echo "🔗 Phase 9: Updating URLs..."
fd -e md -e rs -e toml -e ts -e js -x sd 'github.com/securecheckio/sol-shield' 'github.com/securecheckio/parapet' {}
fd -e md -e rs -e toml -e ts -e js -x sd 'github.com/securecheck/sol-shield' 'github.com/securecheckio/parapet' {}
echo "✅ URLs updated"
echo ""

# Phase 10: Update config files
echo "⚙️  Phase 10: Updating config files..."
fd -e toml -e yaml -e yml -e env -x sd 'sol-shield' 'parapet' {}
fd -e toml -e yaml -e yml -e env -x sd 'Sol-Shield' 'Parapet' {}
echo "✅ Config files updated"
echo ""

# Phase 11: Update docker files
echo "🐳 Phase 11: Updating Docker files..."
fd 'Dockerfile' -x sd 'sol-shield' 'parapet' {}
fd 'docker-compose.yml' -x sd 'sol-shield' 'parapet' {}
fd '.dockerignore' -x sd 'sol-shield' 'parapet' {}
echo "✅ Docker files updated"
echo ""

# Phase 12: Update test files
echo "🧪 Phase 12: Updating test files..."
fd -e rs -p tests -x sd 'sol_shield' 'parapet' {}
fd -e rs -p tests -x sd 'SolShield' 'Parapet' {}
fd -e json -e toml -p tests -x sd 'sol-shield' 'parapet' {}
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
