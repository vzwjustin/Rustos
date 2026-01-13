#!/bin/bash

# RustOS X11 Mode Runner (for macOS with XQuartz)
# This script runs RustOS in Docker with X11 forwarding

set -e

echo "🚀 RustOS X11 Desktop Mode"
echo "========================"
echo ""

# Check if XQuartz is running (macOS)
if [[ "$OSTYPE" == "darwin"* ]]; then
    if ! pgrep -x "Xquartz" > /dev/null; then
        echo "⚠️  XQuartz not detected. Please:"
        echo "   1. Install XQuartz: brew install --cask xquartz"
        echo "   2. Start XQuartz"
        echo "   3. In XQuartz preferences, enable 'Allow connections from network clients'"
        echo "   4. Run: xhost + localhost"
        echo ""
        echo "Or use the VNC version: ./run-rustos-desktop.sh"
        exit 1
    fi
    
    echo "✅ XQuartz detected"
    IP=$(ifconfig en0 | grep inet | awk '$1=="inet" {print $2}')
    export DISPLAY="$IP:0"
    xhost + "$IP" 2>/dev/null || true
else
    export DISPLAY=":0"
fi

echo "🖥️  Starting RustOS with GUI..."
echo "Display: $DISPLAY"
echo ""

# Run RustOS with X11 forwarding
docker run --rm -it \
    -e DISPLAY="$DISPLAY" \
    -e GUI_MODE=1 \
    -v /tmp/.X11-unix:/tmp/.X11-unix:rw \
    --name rustos-x11 \
    rustos:latest ./run_qemu.sh
