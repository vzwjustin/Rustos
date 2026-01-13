# RustOS Module Index

## Module Hierarchy and Cross-References

This document provides a comprehensive index of all RustOS kernel modules, their relationships, and cross-references.

---

## Core Kernel Modules

### `src/main.rs` - Kernel Entry Point
**Purpose**: Main kernel entry and initialization
**Key Functions**:
- `kernel_entry()` - Bootloader entry point
- `rust_main()` - Multiboot entry from assembly
- `kernel_init()` - Core initialization sequence

**Dependencies**:
- → `memory` (heap initialization)
- → `gdt` (descriptor table setup)
- → `interrupts` (IDT configuration)
- → `time` (hardware timer initialization)
- → `arch` (CPU detection)
- → `security` (privilege level setup)
- → `smp` (multiprocessor initialization)
- → `ipc` (IPC mechanism setup)
- → `vga_buffer` (early console)
- → `performance_monitor` (perf counters)
- → `kernel` (subsystem coordination)
- → `process` (scheduler start)
- → `drivers` (device initialization)

**Used By**:
- ← `src/boot.s` (assembly boot code)

---

### `src/time.rs` - Hardware Timer (🆕 Production)
**Purpose**: Real x86_64 timer using PIT and TSC
**Key Features**:
- Programmable Interval Timer (PIT) configuration
- Time Stamp Counter (TSC) for high-resolution timing
- Real hardware interrupt-based timekeeping

**Dependencies**:
- → `x86_64` crate (port I/O)
- → `interrupts` (timer IRQ)

**Used By**:
- ← `main.rs` (timer initialization)
- ← `scheduler` (time slicing)
- ← `process` (sleep, timeouts)

---

### `src/arch.rs` - CPU Architecture Detection (🆕 Production)
**Purpose**: Real CPU detection using CPUID instructions
**Key Features**:
- CPUID-based CPU identification
- Feature flag detection (SSE, AVX, etc.)
- Vendor identification (Intel, AMD)
- CPU topology information

**Dependencies**:
- → `x86_64` crate (CPUID instruction)

**Used By**:
- ← `main.rs` (CPU detection)
- ← `smp` (multiprocessor features)
- ← `process` (context switching optimizations)

---

### `src/smp.rs` - Multiprocessor Support (🆕 Production)
**Purpose**: Real SMP with APIC-based inter-processor communication
**Key Features**:
- Application processor boot-up
- Inter-processor interrupts (IPI)
- Per-CPU data structures
- CPU affinity management

**Dependencies**:
- → `acpi` (CPU enumeration)
- → `apic` (IPI delivery)
- → `arch` (CPU features)

**Used By**:
- ← `main.rs` (SMP initialization)
- ← `scheduler` (load balancing)
- ← `process` (CPU affinity)

---

### `src/security.rs` - Access Control (🆕 Production)
**Purpose**: Hardware privilege levels (Ring 0-3) with access control
**Key Features**:
- x86_64 privilege ring management
- Access control enforcement
- Kernel/user mode transitions
- Security policy management

**Dependencies**:
- → `gdt` (segment descriptors)

**Used By**:
- ← `main.rs` (security initialization)
- ← `process` (privilege enforcement)
- ← `syscall` (ring transitions)

---

### `src/kernel.rs` - Kernel Subsystem Coordinator (🆕 Production)
**Purpose**: Real subsystem initialization coordinator
**Key Features**:
- Coordinated subsystem startup
- Dependency management
- Initialization ordering
- Subsystem health monitoring

**Used By**:
- ← `main.rs` (subsystem coordination)
- ← All subsystems (initialization)

---

### `src/ipc.rs` - Inter-Process Communication (🆕 Production)
**Purpose**: Production IPC mechanisms
**Key Features**:
- **Pipes**: Anonymous and named pipes with kernel buffers
- **Message Queues**: Asynchronous message passing
- **Semaphores**: Process synchronization primitives
- **Shared Memory**: Fast inter-process data sharing

**Dependencies**:
- → `memory` (buffer allocation)
- → `process` (process coordination)

**Used By**:
- ← `syscall` (IPC system calls)
- ← `process` (process communication)

---

### `src/vga_buffer.rs` - VGA Text Mode (🆕 Production)
**Purpose**: Real hardware VGA buffer access at 0xB8000
**Key Features**:
- Direct VGA memory manipulation
- Hardware text mode (80x25)
- Color attribute support
- Early boot console output

**Dependencies**:
- → Raw memory access (0xB8000)

**Used By**:
- ← `main.rs` (early console)
- ← All modules (println! macro)

---

### `src/performance_monitor.rs` - Performance Counters (🆕 Production)
**Purpose**: Hardware performance counters using RDPMC instruction
**Key Features**:
- RDPMC-based performance monitoring
- CPU cycle counting
- Cache hit/miss tracking
- Branch prediction statistics
- Low-overhead profiling

**Dependencies**:
- → `x86_64` crate (RDPMC instruction)

**Used By**:
- ← `main.rs` (performance monitoring)
- ← `scheduler` (load metrics)
- ← `ai` (performance optimization)

---

### `src/gdt.rs` - Global Descriptor Table
**Purpose**: x86_64 segmentation and privilege levels
**Key Types**:
- `GlobalDescriptorTable` - GDT structure
- `SegmentSelector` - Segment selectors

**Dependencies**:
- → `x86_64` crate (GDT primitives)

**Used By**:
- ← `main.rs` (kernel initialization)
- ← `interrupts.rs` (TSS setup)
- ← `process/context.rs` (context switching)

---

### `src/interrupts.rs` - Interrupt Handling
**Purpose**: IDT setup and interrupt handlers
**Key Functions**:
- `init_idt()` - Initialize IDT
- Exception handlers (0-31)
- Hardware interrupt handlers (32-255)

**Dependencies**:
- → `gdt` (TSS for stack switching)
- → `apic` (interrupt acknowledgment)
- → `pic8259` (legacy PIC support)

**Used By**:
- ← `main.rs` (initialization)
- ← `syscall/mod.rs` (system call INT 0x80)
- ← All hardware drivers (interrupt handlers)

**Cross-References**:
- Timer interrupt → `scheduler/mod.rs:45`
- Page fault → `memory.rs:892`
- System call → `syscall/mod.rs:89`

---

## Memory Management

### `src/memory.rs` - Core Memory Management
**Purpose**: Heap allocation, frame allocation, memory zones
**Key Types**:
- `BootInfoFrameAllocator` - Physical frame allocator
- `MemoryMap` - Process memory mapping
- `MemoryStats` - Usage statistics

**Dependencies**:
- → `linked_list_allocator` (heap allocator)
- → `x86_64::structures::paging` (page tables)

**Used By**:
- ← `main.rs` (heap initialization)
- ← `process/mod.rs` (process memory allocation)
- ← All modules using heap allocation

**Cross-References**:
- Heap allocator → `allocate_kernel_heap():178`
- Frame allocator → `allocate_frame():412`
- Page fault handler → `interrupts.rs:234`

---

## Process Management

### `src/process/mod.rs` - Process Control
**Purpose**: Process lifecycle, PCB management
**Key Types**:
- `ProcessControlBlock` - Process descriptor
- `ProcessState` - Process states
- `Pid` - Process identifier

**Dependencies**:
- → `memory` (process memory allocation)
- → `scheduler` (scheduling integration)
- → `fs` (file descriptors)

**Submodules**:
- `context.rs` - Context switching
- `scheduler.rs` - Scheduling algorithms
- `syscalls.rs` - System call handlers
- `sync.rs` - Synchronization primitives
- `thread.rs` - Thread management
- `ipc.rs` - Inter-process communication

**Used By**:
- ← `main.rs` (init process creation)
- ← `syscall/mod.rs` (process syscalls)
- ← `scheduler/mod.rs` (process scheduling)

---

### `src/process/scheduler.rs` - Process Scheduling
**Purpose**: Task scheduling and CPU allocation
**Key Types**:
- `Scheduler` - Main scheduler structure
- `RunQueue` - Ready process queue

**Dependencies**:
- → `process/mod.rs` (PCB access)
- → `process/context.rs` (context switching)
- → `smp` (multi-core support)

**Used By**:
- ← `interrupts.rs` (timer interrupt)
- ← `process/mod.rs` (yield, sleep)

**Cross-References**:
- Timer tick → `interrupts.rs:156`
- Context switch → `context.rs:89`
- Load balancing → `smp.rs:234`

---

### `src/process/sync.rs` - Synchronization
**Purpose**: Mutex, semaphore, RwLock implementations
**Key Types**:
- `Mutex<T>` - Mutual exclusion
- `Semaphore` - Counting semaphore
- `RwLock<T>` - Read-write lock

**Used By**:
- ← All modules requiring synchronization
- ← `process/ipc.rs` (IPC synchronization)
- ← `net/socket.rs` (socket locks)

---

### `src/process/ipc.rs` - Inter-Process Communication
**Purpose**: Message queues, shared memory, pipes
**Key Types**:
- `MessageQueue` - Async message passing
- `SharedMemory` - Shared memory segments
- `Pipe` - Unidirectional data flow

**Dependencies**:
- → `memory` (shared memory allocation)
- → `process/sync.rs` (synchronization)

**Used By**:
- ← `syscall/mod.rs` (IPC syscalls)
- ← User processes (via syscalls)

---

## Network Stack

### `src/net/mod.rs` - Network Core
**Purpose**: Network stack initialization and management
**Key Types**:
- `NetworkAddress` - IP/MAC addresses
- `NetworkManager` - Stack coordinator

**Submodules**:
- `ethernet.rs` - Ethernet frame handling
- `ip.rs` - IPv4/IPv6 processing
- `tcp.rs` - TCP protocol
- `udp.rs` - UDP protocol
- `icmp.rs` - ICMP messages
- `arp.rs` - Address Resolution Protocol
- `socket.rs` - Socket interface
- `device.rs` - Network device abstraction

**Dependencies**:
- → `drivers/network/*` (NIC drivers)

**Used By**:
- ← `syscall/mod.rs` (socket syscalls)
- ← `main.rs` (network initialization)

---

### `src/net/tcp.rs` - TCP Protocol
**Purpose**: TCP state machine and connection management
**Key Types**:
- `TcpConnection` - TCP connection state
- `TcpState` - Connection states
- `TcpHeader` - TCP packet header

**Dependencies**:
- → `net/ip.rs` (IP layer)
- → `net/socket.rs` (socket interface)

**Cross-References**:
- Socket creation → `socket.rs:89`
- IP routing → `ip.rs:234`
- Packet transmission → `device.rs:156`

---

### `src/net/socket.rs` - Socket Interface
**Purpose**: POSIX-compatible socket API
**Key Types**:
- `Socket` - Socket structure
- `SocketHandle` - Socket descriptor
- `SocketAddress` - Network addresses

**Dependencies**:
- → `net/tcp.rs` (TCP sockets)
- → `net/udp.rs` (UDP sockets)
- → `process/sync.rs` (socket locks)

**Used By**:
- ← `syscall/mod.rs` (socket syscalls)
- ← `process/mod.rs` (file descriptors)

---

## Device Drivers

### `src/drivers/mod.rs` - Driver Framework
**Purpose**: Unified driver interface and management
**Key Types**:
- `DriverOps` - Driver operations trait
- `DriverManager` - Driver registry

**Submodules**:
- `vbe.rs` - VESA graphics driver
- `pci.rs` - PCI bus driver
- `hotplug.rs` - Hot-plug support
- `network/` - Network drivers
- `storage/` - Storage drivers

**Used By**:
- ← `main.rs` (driver initialization)
- ← `pci/detection.rs` (driver loading)

---

### `src/drivers/network/` - Network Drivers
**Modules**:
- `intel_e1000.rs` - Intel E1000/E1000E driver
- `realtek.rs` - Realtek RTL8139/8168 driver
- `broadcom.rs` - Broadcom BCM driver
- `mod.rs` - Common network driver interface

**Dependencies**:
- → `net/device.rs` (device registration)
- → `pci/` (PCI device access)

**Cross-References**:
- Device detection → `pci/detection.rs:345`
- Packet receive → `net/device.rs:78`
- Interrupt handling → `interrupts.rs:189`

---

### `src/drivers/storage/` - Storage Drivers
**Modules**:
- `ahci.rs` - AHCI SATA driver
- `nvme.rs` - NVMe SSD driver
- `ide.rs` - Legacy IDE driver
- `filesystem_interface.rs` - Storage abstraction

**Dependencies**:
- → `pci/` (device detection)
- → `fs/` (filesystem integration)

**Used By**:
- ← `fs/mod.rs` (block device access)
- ← `main.rs` (boot disk detection)

---

## PCI Subsystem

### `src/pci/mod.rs` - PCI Core
**Purpose**: PCI bus management and device access
**Key Types**:
- `PciDevice` - PCI device descriptor
- `PciAddress` - Bus/Device/Function addressing

**Submodules**:
- `config.rs` - Configuration space access
- `database.rs` - Device ID database
- `detection.rs` - Device enumeration

**Used By**:
- ← `drivers/*` (device detection)
- ← `main.rs` (PCI initialization)

---

### `src/pci/detection.rs` - Device Detection
**Purpose**: PCI bus scanning and device identification
**Key Functions**:
- `scan_bus()` - Enumerate all PCI devices
- `identify_device()` - Match device IDs

**Dependencies**:
- → `pci/config.rs` (config access)
- → `pci/database.rs` (device database)

**Cross-References**:
- Driver loading → `drivers/mod.rs:134`
- Device database → `database.rs:23`
- Hot-plug events → `drivers/hotplug.rs:56`

---

## GPU and Graphics

### `src/gpu/mod.rs` - GPU Management
**Purpose**: Multi-vendor GPU support and acceleration
**Key Types**:
- `GpuDevice` - GPU descriptor
- `GPUTier` - Performance classification
- `GPUFeatures` - Capability flags

**Submodules**:
- `memory.rs` - GPU memory management
- `accel.rs` - Hardware acceleration
- `ai_integration.rs` - AI workload support
- `opensource/` - Open source drivers

**Dependencies**:
- → `pci/` (GPU detection)
- → `drivers/vbe.rs` (framebuffer)

**Used By**:
- ← `desktop/` (windowing system)
- ← `main.rs` (GPU initialization)

---

### `src/gpu/opensource/` - Open Source GPU Drivers
**Modules**:
- `nouveau.rs` - NVIDIA Nouveau driver
- `amdgpu.rs` - AMD GPU driver
- `i915.rs` - Intel graphics driver
- `drm_compat.rs` - DRM compatibility layer
- `mesa_compat.rs` - Mesa3D compatibility

**Dependencies**:
- → `gpu/mod.rs` (GPU framework)
- → `pci/` (device detection)

**Cross-References**:
- Device IDs → `gpu/mod.rs:25`
- DRM interface → `drm_compat.rs:45`
- Mesa integration → `mesa_compat.rs:89`

---

## File System

### `src/fs/mod.rs` - Virtual File System
**Purpose**: Unified filesystem interface
**Key Types**:
- `VfsNode` - VFS node structure
- `FileSystem` - Filesystem trait
- `MountPoint` - Mount table entry

**Submodules**:
- `vfs.rs` - VFS implementation
- `ramfs.rs` - RAM filesystem
- `devfs.rs` - Device filesystem

**Dependencies**:
- → `drivers/storage/` (block devices)
- → `process/mod.rs` (file descriptors)

**Used By**:
- ← `syscall/mod.rs` (file syscalls)
- ← `process/mod.rs` (open files)

---

## Hardware Abstraction

### `src/acpi/mod.rs` - ACPI Support
**Purpose**: Advanced Configuration and Power Interface
**Key Functions**:
- `parse_rsdp()` - Find ACPI tables
- `parse_madt()` - CPU/APIC information
- `parse_mcfg()` - PCIe configuration

**Used By**:
- ← `main.rs` (hardware discovery)
- ← `apic/mod.rs` (APIC configuration)
- ← `smp.rs` (CPU detection)

---

### `src/apic/mod.rs` - APIC Management
**Purpose**: Local APIC and IO-APIC control
**Key Functions**:
- `init_local_apic()` - Initialize Local APIC
- `init_io_apic()` - Initialize IO-APIC
- `send_ipi()` - Inter-processor interrupts

**Dependencies**:
- → `acpi/mod.rs` (MADT parsing)

**Used By**:
- ← `interrupts.rs` (EOI handling)
- ← `smp.rs` (CPU wake-up)
- ← `scheduler/mod.rs` (IPI for scheduling)

---

## System Components

### `src/smp.rs` - Symmetric Multiprocessing
**Purpose**: Multi-core CPU support
**Key Functions**:
- `boot_ap()` - Boot application processors
- `get_cpu_count()` - Number of CPUs
- `get_current_cpu()` - Current CPU ID

**Dependencies**:
- → `acpi/mod.rs` (CPU detection)
- → `apic/mod.rs` (IPI support)

**Used By**:
- ← `scheduler/mod.rs` (per-CPU run queues)
- ← `main.rs` (SMP initialization)

---

### `src/syscall/mod.rs` - System Calls
**Purpose**: User-kernel interface
**Key Constants**:
- System call numbers (SYS_*)
- System call handler

**Dependencies**:
- → `process/` (process syscalls)
- → `fs/` (file syscalls)
- → `net/` (network syscalls)

**Used By**:
- ← `interrupts.rs` (INT 0x80 handler)
- ← User space (via INT 0x80)

---

### `src/desktop/mod.rs` - Desktop Environment
**Purpose**: Windowing system and GUI
**Key Types**:
- `Window` - Window structure
- `Desktop` - Desktop manager

**Submodules**:
- `window_manager.rs` - Window management

**Dependencies**:
- → `gpu/` (hardware acceleration)
- → `drivers/vbe.rs` (framebuffer)

**Used By**:
- ← `main.rs` (desktop initialization)

---

## Performance and Debugging

### `src/performance_monitor.rs` - Performance Metrics
**Purpose**: System performance monitoring
**Key Types**:
- `PerformanceStats` - Performance data
- `MetricCategory` - Metric types

**Used By**:
- ← `main.rs` (status display)
- ← `scheduler/mod.rs` (load metrics)
- ← `memory.rs` (memory stats)

---

### `src/kernel.rs` - Kernel Utilities
**Purpose**: Common kernel functions and panic handler
**Key Functions**:
- `kernel_panic()` - Panic handler
- `hlt_loop()` - CPU halt loop

**Used By**:
- ← All modules (panic handling)

---

## Module Dependency Graph

```
main.rs
    ├── memory.rs
    ├── gdt.rs
    ├── interrupts.rs
    │   ├── apic/mod.rs
    │   └── pic8259
    ├── process/
    │   ├── scheduler.rs
    │   ├── context.rs
    │   ├── sync.rs
    │   └── ipc.rs
    ├── drivers/
    │   ├── pci.rs
    │   ├── network/
    │   └── storage/
    ├── net/
    │   ├── socket.rs
    │   ├── tcp.rs
    │   └── device.rs
    ├── fs/
    │   ├── vfs.rs
    │   └── ramfs.rs
    ├── gpu/
    │   ├── accel.rs
    │   └── opensource/
    └── desktop/
        └── window_manager.rs
```

---

## Cross-Reference Quick Links

| Component | Main Module | Key Function | Line |
|-----------|------------|--------------|------|
| Process Creation | `process/mod.rs` | `create_process()` | 234 |
| Context Switch | `process/context.rs` | `switch_context()` | 89 |
| Memory Allocation | `memory.rs` | `allocate_kernel_heap()` | 178 |
| TCP Connect | `net/tcp.rs` | `tcp_connect()` | 456 |
| PCI Scan | `pci/detection.rs` | `scan_bus()` | 45 |
| GPU Detect | `gpu/mod.rs` | `detect_gpu()` | 234 |
| File Open | `fs/vfs.rs` | `open()` | 89 |
| Syscall Handler | `syscall/mod.rs` | `syscall_handler()` | 89 |
| APIC Init | `apic/mod.rs` | `init_local_apic()` | 67 |
| Network Send | `net/socket.rs` | `send()` | 189 |

---

## Module Statistics

- **Total Modules**: 75 (8 new production modules added)
- **Core Kernel**: 20 modules (includes 8 new production modules)
- **Drivers**: 15 modules
- **Network Stack**: 9 modules
- **Process Management**: 7 modules
- **GPU/Graphics**: 8 modules
- **File System**: 4 modules
- **Hardware Abstraction**: 6 modules
- **Utilities**: 6 modules

### 🆕 New Production Modules
- `src/time.rs` - Real hardware timers (PIT, TSC)
- `src/arch.rs` - Real CPU detection (CPUID)
- `src/smp.rs` - Real multiprocessor support (APIC IPI)
- `src/security.rs` - Access control (Ring 0-3)
- `src/kernel.rs` - Subsystem coordinator
- `src/ipc.rs` - Production IPC mechanisms
- `src/vga_buffer.rs` - Real VGA hardware (0xB8000)
- `src/performance_monitor.rs` - Hardware perf counters (RDPMC)

---

## See Also

- [Architecture Overview](ARCHITECTURE.md)
- [API Reference](API_REFERENCE.md)
- [Build Guide](BUILD_GUIDE.md)
- [Driver Development](DRIVER_GUIDE.md)