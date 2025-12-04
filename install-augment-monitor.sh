#!/bin/bash

# Augment Credit Monitor - Safe Installation Script
# This script helps install the app while handling macOS security restrictions

set -e

echo "🚀 Augment Credit Monitor - Installation Script"
echo "================================================"
echo ""

# Check if running on macOS
if [[ "$OSTYPE" != "darwin"* ]]; then
    echo "❌ This script is for macOS only"
    exit 1
fi

# Check if DMG file exists
DMG_FILE="Augment Credit Monitor_1.0.0_aarch64.dmg"
if [ ! -f "$DMG_FILE" ]; then
    echo "❌ DMG file not found: $DMG_FILE"
    echo "Please download the DMG file first from:"
    echo "https://github.com/codavidgarcia/augment-credit-monitor/releases"
    exit 1
fi

echo "✅ Found DMG file: $DMG_FILE"
echo ""

# Mount the DMG
echo "📦 Mounting DMG..."
MOUNT_POINT=$(hdiutil attach "$DMG_FILE" | grep "/Volumes" | awk '{print $3}')

if [ -z "$MOUNT_POINT" ]; then
    echo "❌ Failed to mount DMG"
    exit 1
fi

echo "✅ Mounted at: $MOUNT_POINT"

# Copy app to Applications
echo "📋 Installing to Applications folder..."
if [ -d "/Applications/Augment Credit Monitor.app" ]; then
    echo "⚠️  Existing installation found. Removing..."
    rm -rf "/Applications/Augment Credit Monitor.app"
fi

cp -R "$MOUNT_POINT/Augment Credit Monitor.app" "/Applications/"

# Unmount DMG
echo "📤 Unmounting DMG..."
hdiutil detach "$MOUNT_POINT" -quiet

# Remove quarantine attribute
echo "🔓 Removing quarantine attribute..."
xattr -d com.apple.quarantine "/Applications/Augment Credit Monitor.app" 2>/dev/null || true

# Set executable permissions
echo "🔧 Setting permissions..."
chmod +x "/Applications/Augment Credit Monitor.app/Contents/MacOS/augment-credit-monitor"

echo ""
echo "✅ Installation completed successfully!"
echo ""
echo "🎯 Next steps:"
echo "1. Open Applications folder"
echo "2. Double-click 'Augment Credit Monitor'"
echo "3. If you see a security warning, click 'Open'"
echo "4. Follow the setup instructions in the app"
echo ""
echo "📖 For help and documentation:"
echo "https://github.com/codavidgarcia/augment-credit-monitor"
echo ""
echo "🎉 Enjoy monitoring your Augment credits!"
