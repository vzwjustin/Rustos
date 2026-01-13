# RustOS Project Structure

## Root Directory
```
├── Cargo.toml              # Main project configuration
├── Makefile                # Build system with comprehensive targets
├── build_rustos.sh         # Primary build script (bash)
├── build.rs                # Cargo build script
├── rust-toolchain.toml     # Rust nightly toolchain specification
└── src/                    # Source code
```

## Source Code Organization (`src/`)

### Entry Points
- `main.rs` - Primary kernel entry point with desktop selection
- `main_*.rs` - Alternative entry points for different configurations
- `lib.rs` - Kernel library exports (if present)

### Core Kernel Modules
```
├── intrinsics.rs           # Compiler intrinsics for missing symbols
├── vga_buffer.rs           # VGA text mode output
├── print.rs                # Print macros and formatting
├── gdt.rs                  # Global Descriptor Table
├── interrupts.rs           # Interrupt handling and IDT
├── memory.rs               # Advanced memory management
├── memory_basic.rs         # Basic memory management fallback
├── serial.rs               # Serial port communication
└── keyboard.rs             # Keyboard input handling
```

### Hardware Abstraction
```
├── acpi/                   # ACPI subsystem
│   └── mod.rs              # ACPI table parsing, hardware discovery
├── apic/                   # Advanced Programmable Interrupt Controller
│   └── mod.rs              # Local APIC and IO APIC management
└── pci/                    # PCI bus management
    ├── mod.rs              # PCI enumeration and device management
    ├── config.rs           # PCI configuration space access
    ├── database.rs         # Hardware device database
    └── detection.rs        # Device detection and classification
```

### System Services
```
├── scheduler/              # Process scheduling
│   └── mod.rs              # Preemptive scheduler with SMP support
├── syscall/                # System call interface
│   └── mod.rs              # POSIX-compatible syscall dispatch
├── process/                # Process management
│   ├── mod.rs              # Process lifecycle management
│   ├── scheduler.rs        # Process scheduling algorithms
│   ├── context.rs          # Process context switching
│   ├── sync.rs             # Process synchronization primitives
│   ├── syscalls.rs         # Process-related system calls
│   └── integration.rs      # Integration with kernel systems
└── fs/                     # Virtual File System
    ├── mod.rs              # VFS layer and filesystem abstraction
    ├── ramfs.rs            # RAM-based filesystem
    ├── devfs.rs            # Device filesystem
    └── vfs.rs              # Virtual filesystem interface
```

### Networking
```
└── net/                    # Network stack (TCP/IP)
    ├── mod.rs              # Network subsystem coordination
    ├── ethernet.rs         # Ethernet frame processing
    ├── ip.rs               # IPv4 protocol implementation
    ├── tcp.rs              # TCP protocol with connection management
    ├── udp.rs              # UDP protocol implementation
    ├── socket.rs           # Socket interface and management
    ├── device.rs           # Network device abstraction
    ├── arp.rs              # Address Resolution Protocol
    ├── dhcp.rs             # DHCP client implementation
    ├── dns.rs              # DNS resolver
    └── buffer.rs           # Zero-copy network buffers
```

### Graphics & Desktop
```
├── graphics/               # Graphics and framebuffer
│   ├── mod.rs              # Graphics subsystem
│   └── framebuffer.rs      # Hardware-accelerated framebuffer
├── desktop/                # Desktop environment
│   ├── mod.rs              # Desktop system management
│   └── window_manager.rs   # Window management and compositing
├── simple_desktop.rs       # Text-based desktop (MS-DOS style)
└── gpu/                    # GPU acceleration and graphics
    ├── mod.rs              # GPU subsystem coordination
    ├── accel.rs            # Hardware acceleration interface
    ├── memory.rs           # GPU memory management
    └── opensource/         # Open source driver integration
        ├── mod.rs          # Driver registry and management
        ├── drm_compat.rs   # Linux DRM compatibility layer
        ├── mesa_compat.rs  # Mesa3D integration
        ├── nouveau.rs      # NVIDIA open source driver
        ├── amdgpu.rs       # AMD open source driver
        └── i915.rs         # Intel open source driver
```

### Device Drivers
```
└── drivers/                # Device driver framework
    ├── mod.rs              # Driver management and registration
    ├── pci.rs              # PCI device drivers
    ├── hotplug.rs          # Hot-plug device support
    ├── vbe.rs              # VESA BIOS Extensions
    ├── network/            # Network device drivers
    └── storage/            # Storage device drivers
```

### Testing & Development
```
├── testing/                # Testing framework
│   ├── testing_framework.rs
│   ├── integration_tests.rs
│   ├── security_tests.rs
│   ├── stress_tests.rs
│   └── benchmarking.rs
├── boot_display.rs         # Visual boot sequence
└── performance_monitor.rs  # System performance monitoring
```

## Architecture Patterns
- **Modular Design**: Each subsystem in separate module with clear interfaces
- **Hardware Abstraction**: Clean separation between hardware-specific and generic code
- **No-std Environment**: All code must work without standard library
- **Error Handling**: Result types for fallible operations, panic for unrecoverable errors
- **Memory Safety**: Rust ownership model + careful unsafe blocks for hardware access
- **Concurrency**: Spin locks and atomic operations for kernel synchronization