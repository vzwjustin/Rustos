# Safety Invariants

This document is the single source of truth for unsafe invariants. Every unsafe
block or function must reference one of the sections below.

## io-port-access
- Target is x86_64 with I/O port support.
- Port addresses are valid for the device being accessed (e.g., COM1 at 0x3F8).
- Callers understand that port I/O has side effects and may block if hardware
  is not present or ready.

## vga-text-buffer
- VGA text buffer at 0xB8000 is mapped and the system is in VGA text mode.
- Writes stay within the visible 80x25 character grid (80 * 25 * 2 bytes).
- No concurrent writers without a higher-level lock.

## halt-loop
- `hlt` is executed only in idle loops or fatal error paths.
- The CPU may remain halted until an interrupt occurs; no forward progress is
  required.
- No memory or device access is performed inside the halted loop.

## bootinfo-use
- `BootInfo` must be provided by the bootloader crate entry path.
- It must not be zero-initialized or fabricated.
- The memory map and physical memory offset are treated as trusted inputs from
  the bootloader and validated before use.

## gdt-tss-setup
- The TSS and GDT descriptors remain in static storage for the lifetime of the
  kernel.
- IST stacks are properly aligned and sized for fault handlers.

## idt-load
- The IDT points to valid handler functions and uses the correct IST index for
  double faults.
- Interrupts are enabled only after IDT and handlers are installed.
