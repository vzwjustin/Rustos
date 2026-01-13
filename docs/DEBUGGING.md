# Debugging Guide

## Baseline Boot (Primary Path)
Command:
```
make build
make run
```
Expected output:
- Serial includes `RustOS: Kernel entry point reached!`
- VGA shows `KERNEL STARTED!`

Failure triage:
- No serial output: confirm `-serial stdio` in QEMU and that COM1 is available.
- Bootimage missing: run `cargo bootimage --bin rustos` or `make bootimage`.
- Immediate reset: check `docs/SAFETY.md#bootinfo-use` and memory map handling.
- On macOS, set `RUSTOS_QEMU_DISPLAY=cocoa` if GTK is unavailable.

## Boot Smoke Test (Headless)
Command:
```
make boot-smoke
```
Expected output:
- Serial includes `RustOS: Kernel entry point reached!`

Failure triage:
- `qemu-system-x86_64` missing: install QEMU (`brew install qemu`).
- Bootimage missing: run `make bootimage`.

## Direct QEMU Run
Command:
```
scripts/run_qemu.sh
```
Expected output:
- Same early serial line as above.

Failure triage:
- `bootimage-rustos.bin` not found: run `cargo bootimage --bin rustos`.
- QEMU errors about KVM: remove `-enable-kvm` or run on a host with KVM.

## Verbose Boot Capture
Command:
```
scripts/debug_boot.sh
```
Expected output:
- `qemu.log` with CPU reset info
- `boot_output.log` with serial output

Failure triage:
- No logs created: check script permissions and QEMU availability.

## Experimental Scripts
Multiboot and standalone-kernel scripts are gated to avoid accidental use.
Set `ALLOW_MULTIBOOT=1` or `ALLOW_EXPERIMENTAL=1` explicitly if you intend to run them.

## Common Signals
- `RustOS: Kernel entry point reached!`: bootloader handoff succeeded.
- No VGA or serial output: suspect entry path or QEMU flags.
- Page fault loop: check paging and memory map assumptions.
