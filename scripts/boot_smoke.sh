#!/bin/bash
# Headless boot smoke test for RustOS (serial log check)

set -euo pipefail

BOOTIMAGE="${BOOTIMAGE_PATH:-target/x86_64-rustos/debug/bootimage-rustos.bin}"
PATTERN="${RUSTOS_BOOT_LOG_PATTERN:-RustOS: Kernel entry point reached!}"
TIMEOUT="${RUSTOS_BOOT_TIMEOUT_SEC:-10}"

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
    echo "Error: qemu-system-x86_64 not found. Install QEMU (brew install qemu)."
    exit 1
fi

if [ ! -f "$BOOTIMAGE" ]; then
    echo "Error: bootimage not found at $BOOTIMAGE"
    echo "Run: make bootimage"
    exit 1
fi

echo "Running boot smoke test..."
echo "Bootimage: $BOOTIMAGE"
echo "Pattern: $PATTERN"
echo "Timeout: ${TIMEOUT}s"

BOOTIMAGE="$BOOTIMAGE" PATTERN="$PATTERN" TIMEOUT="$TIMEOUT" python3 - <<'PY'
import os
import subprocess
import sys

bootimage = os.environ["BOOTIMAGE"]
pattern = os.environ["PATTERN"]
timeout = float(os.environ["TIMEOUT"])

cmd = [
    "qemu-system-x86_64",
    "-drive", f"format=raw,file={bootimage}",
    "-m", "512M",
    "-serial", "stdio",
    "-display", "none",
    "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04",
    "-machine", "q35,accel=tcg",
    "-cpu", "qemu64,+apic",
    "-no-reboot",
    "-no-shutdown",
]

proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
try:
    out, _ = proc.communicate(timeout=timeout)
except subprocess.TimeoutExpired:
    proc.terminate()
    try:
        out, _ = proc.communicate(timeout=2)
    except subprocess.TimeoutExpired:
        proc.kill()
        out, _ = proc.communicate()

print(out)

if pattern not in out:
    sys.stderr.write("Boot smoke test failed: expected log line not found.\n")
    sys.exit(1)

print("Boot smoke test passed.")
PY
