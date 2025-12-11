#!/bin/bash
set -e

echo "Building MCP Rust Server for macOS..."

# Build release binary
cargo build --release

# Create app bundle
APP_DIR="$HOME/.rmcp_servers/MCPServer.app"
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

# Copy binary (crate binary name: rmcp_memex)
cp target/release/rmcp_memex "$APP_DIR/Contents/MacOS/"

# Create Info.plist
cat > "$APP_DIR/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
 "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>com.maciejgad.rmcp-rust</string>
    <key>CFBundleExecutable</key>
    <string>rmcp_memex</string>
    <key>CFBundleName</key>
    <string>MCP Rust Server</string>
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
    <key>LSUIElement</key>
    <true/>
    <key>LSBackgroundOnly</key>
    <true/>
    <key>NSHighResolutionCapable</key>
    <false/>
    <key>NSSupportsAutomaticTermination</key>
    <false/>
</dict>
</plist>
EOF

# Sign the app
codesign --force --deep --sign - "$APP_DIR"

echo "Done! App bundle created at: $APP_DIR"
echo ""
echo "To use with an MCP host, add to config:"
echo '  "command": "'"$APP_DIR/Contents/MacOS/rmcp_memex"'"'
