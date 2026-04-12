#!/bin/bash
# Build release binaries for all platforms

set -e

VERSION=${1:-$(cargo pkgid | cut -d# -f2 | cut -d: -f2)}
DIST_DIR="dist"

echo "Building Sol-Shield MCP Server v$VERSION for all platforms..."

# Clean dist directory
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

# Build for each target
TARGETS=(
    "x86_64-unknown-linux-gnu:sol-shield-mcp:linux-x86_64"
    "x86_64-apple-darwin:sol-shield-mcp:macos-x86_64"
    "aarch64-apple-darwin:sol-shield-mcp:macos-arm64"
    "x86_64-pc-windows-msvc:sol-shield-mcp.exe:windows-x86_64"
)

for target_info in "${TARGETS[@]}"; do
    IFS=":" read -r target binary_name platform_name <<< "$target_info"
    
    echo ""
    echo "Building for $target..."
    
    # Install target if not present
    rustup target add "$target" 2>/dev/null || true
    
    # Build
    cargo build --release --target "$target"
    
    # Package
    BINARY_PATH="target/$target/release/$binary_name"
    ARCHIVE_NAME="sol-shield-mcp-$platform_name-v$VERSION"
    
    if [ -f "$BINARY_PATH" ]; then
        echo "Packaging $ARCHIVE_NAME..."
        
        cd "$(dirname "$BINARY_PATH")"
        tar -czf "$ARCHIVE_NAME.tar.gz" "$(basename "$BINARY_PATH")"
        cd - > /dev/null
        
        mv "target/$target/release/$ARCHIVE_NAME.tar.gz" "$DIST_DIR/"
        
        # Calculate SHA256
        sha256sum "$DIST_DIR/$ARCHIVE_NAME.tar.gz" > "$DIST_DIR/$ARCHIVE_NAME.tar.gz.sha256"
        
        echo "✅ Created $ARCHIVE_NAME.tar.gz"
    else
        echo "⚠️  Binary not found: $BINARY_PATH (target may not be available on this host)"
    fi
done

echo ""
echo "Build complete! Artifacts in $DIST_DIR/"
echo ""
echo "To create a GitHub release:"
echo "  gh release create mcp-v$VERSION $DIST_DIR/* --title \"MCP Server v$VERSION\" --notes \"See CHANGELOG.md\""
echo ""
echo "To publish to crates.io:"
echo "  cargo publish"
