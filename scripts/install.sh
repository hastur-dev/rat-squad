#!/usr/bin/env bash
# rat-squad installation script for POSIX systems
# Installs rat-squad as a ratterm extension

set -euo pipefail

echo "rat-squad Extension Installer"
echo "============================="
echo ""

# Get script directory (where Cargo.toml is)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_TOML="$SCRIPT_DIR/Cargo.toml"

if [ ! -f "$CARGO_TOML" ]; then
    echo "Error: Must run from rat-squad repository root"
    exit 1
fi

# Build release binary
echo "[1/4] Building release binary..."
cd "$SCRIPT_DIR"
cargo build --release
echo "Build successful!"

# Create extension directory
EXT_DIR="$HOME/.ratterm/extensions/rat-squad"
echo "[2/4] Creating extension directory: $EXT_DIR"
mkdir -p "$EXT_DIR"
echo "Directory created!"

# Copy files
echo "[3/4] Copying extension files..."
BINARY_PATH="$SCRIPT_DIR/target/release/rat-squad"
MANIFEST_PATH="$SCRIPT_DIR/extension.toml"
CONFIG_EXAMPLE_PATH="$SCRIPT_DIR/config.yaml.example"

cp "$BINARY_PATH" "$EXT_DIR/"
cp "$MANIFEST_PATH" "$EXT_DIR/"
cp "$CONFIG_EXAMPLE_PATH" "$EXT_DIR/"
chmod +x "$EXT_DIR/rat-squad"
echo "Files copied!"

# Create data directory
DATA_DIR="$HOME/.rat-squad"
echo "[4/4] Creating configuration directory: $DATA_DIR"
mkdir -p "$DATA_DIR"

# Copy config example if no config exists
CONFIG_PATH="$DATA_DIR/config.yaml"
if [ ! -f "$CONFIG_PATH" ]; then
    cp "$CONFIG_EXAMPLE_PATH" "$CONFIG_PATH"
    echo "Created default configuration at $CONFIG_PATH"
else
    echo "Configuration already exists at $CONFIG_PATH"
fi

echo ""
echo "Installation complete!"
echo ""
echo "Next steps:"
echo "  1. Restart ratterm to discover the extension"
echo "  2. Approve the extension when prompted"
echo "  3. Use 'rat-squad' commands in ratterm"
echo ""
echo "Configuration file: $CONFIG_PATH"
echo "Extension directory: $EXT_DIR"
