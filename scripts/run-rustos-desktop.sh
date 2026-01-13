#!/bin/bash

# RustOS Desktop Mode Runner
# This script runs RustOS in Docker with VNC GUI access

set -e

echo "🚀 RustOS Desktop Mode"
echo "==================="
echo ""
echo "This will start RustOS with a graphical desktop you can view in your browser or VNC client."
echo ""

# Check if Docker is running
if ! docker info >/dev/null 2>&1; then
    echo "❌ Docker is not running. Please start Docker Desktop first."
    exit 1
fi

# Check if image exists
if ! docker image inspect rustos:latest >/dev/null 2>&1; then
    echo "🔨 Building RustOS Docker image..."
    docker build -t rustos:latest .
fi

echo "🖥️  Starting RustOS with GUI..."
echo ""
echo "🌐 VNC Access Methods:"
echo "   1. VNC Viewer: Connect to localhost:5900 (no password)"
echo "   2. Web Browser: http://localhost:6080 (if using noVNC)"
echo ""
echo "⌨️  Controls:"
echo "   - Ctrl+Alt+G: Release mouse from QEMU"
echo "   - Ctrl+Alt+2: QEMU monitor console"
echo "   - Ctrl+C: Stop the system"
echo ""
echo "Press Enter to continue or Ctrl+C to cancel..."
read -r

# Run RustOS with VNC port exposed
docker run --rm -it \
    -p 5900:5900 \
    -p 6080:6080 \
    --name rustos-desktop \
    rustos:latest ./run_desktop.sh
