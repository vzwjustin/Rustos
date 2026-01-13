#!/bin/bash

echo "🚀 Starting RustOS with Web VNC Access"
echo "======================================"

# Kill any existing containers
docker kill rustos-desktop 2>/dev/null || true

echo "📦 Starting RustOS with web browser access..."
echo "🌐 Once started, open: http://localhost:6080"
echo "🖥️  VNC direct access: localhost:5900"
echo ""

# Start with both VNC and web access
docker run --rm -it \
  -p 5900:5900 \
  -p 6080:6080 \
  --name rustos-desktop \
  rustos:latest bash -c "
    echo 'Building bootimage...'
    ./create_bootimage.sh
    
    echo 'Starting display server...'
    export DISPLAY=:99
    Xvfb :99 -screen 0 1024x768x24 &
    sleep 3
    
    echo 'Starting VNC server...'
    x11vnc -display :99 -nopw -forever -shared -listen 0.0.0.0 &
    sleep 2
    
    echo ''
    echo '🎉 RustOS Desktop Ready!'
    echo '🌐 Web Access: http://localhost:6080'  
    echo '📺 VNC Client: localhost:5900'
    echo '🔑 Password: NONE (just connect)'
    echo ''
    
    echo 'Starting RustOS...'
    qemu-system-x86_64 \
      -drive format=raw,file=target/x86_64-rustos/debug/bootimage-rustos.bin \
      -m 512M \
      -cpu qemu64 \
      -vga std \
      -display gtk \
      -rtc base=localtime \
      -serial stdio
"
