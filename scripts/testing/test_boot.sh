#!/bin/bash
echo "Testing RustOS boot..."
echo "========================"
qemu-system-x86_64 \
  -drive format=raw,file=rustos.img,index=0,media=disk \
  -serial stdio \
  -nographic &
PID=$!
sleep 2
kill $PID 2>/dev/null
wait $PID 2>/dev/null
echo ""
echo "Boot test complete!"
