# Linux Compatibility Guide

## Overview

RustOS is an **independent operating system kernel** written in Rust, not a Linux distribution or Linux-based system. However, RustOS aims to achieve compatibility with Linux software through standardized interfaces and binary compatibility layers.

## Important: RustOS is NOT Linux

### Common Misconceptions

❌ **Misconception**: RustOS is based on Linux  
✅ **Reality**: RustOS is a completely separate kernel written from scratch in Rust

❌ **Misconception**: RustOS can be merged into the Linux kernel  
✅ **Reality**: RustOS and Linux are separate operating systems with different codebases, like FreeBSD and Linux

❌ **Misconception**: RustOS can run all Linux software automatically  
✅ **Reality**: RustOS implements compatible interfaces but requires specific porting efforts

### What RustOS Actually Is

RustOS is an **independent kernel** that:
- Is written entirely in Rust (with minimal assembly for boot)
- Has its own architecture, design, and implementation
- Implements POSIX-compatible interfaces where possible
- Can run compatible ELF binaries through proper support layers

## Linux Compatibility Approach

### Strategy: Interface Compatibility, Not Code Sharing

RustOS achieves Linux software compatibility through **interface compatibility**:

```
Linux Software
      ↓
  POSIX API (open, read, write, fork, exec, etc.)
      ↓
┌─────────────────┬─────────────────┐
│   Linux Kernel  │   RustOS Kernel │
│  (implements    │   (implements   │
│   POSIX)        │    POSIX)       │
└─────────────────┴─────────────────┘
```

Both systems implement the same **standard interfaces**, allowing portable software to run on either.

## Current Compatibility Status

### ✅ Implemented Features

#### 1. POSIX System Call Interface
**Status**: Core calls implemented, extended calls in progress

**Location**: `src/syscall/mod.rs`, `src/process/syscalls.rs`

**Implemented System Calls**:
- Process Management: `exit`, `fork`, `exec`, `wait`, `getpid`, `getppid`, `kill`
- File I/O: `open`, `close`, `read`, `write`, `seek`, `stat`
- Memory Management: `mmap`, `munmap`, `brk`, `sbrk`
- Process Communication: `pipe`, `signal`
- System Information: `uname`, `gettime`, `settime`

**Example**: Standard POSIX programs can make these calls and expect Linux-compatible behavior.

#### 2. ELF Binary Loading
**Status**: Core ELF64 support complete

**Location**: `src/process/elf_loader.rs`

**Features**:
- Parses ELF64 headers and program headers
- Loads PT_LOAD segments into memory
- Sets up stack and heap
- Validates entry points
- Supports static binaries

**Example**: RustOS can load and execute statically-linked ELF binaries compiled for x86_64.

#### 3. Process Management
**Status**: Core functionality complete

**Location**: `src/process/mod.rs`, `src/process/scheduler.rs`

**Features**:
- Fork/exec model compatible with POSIX
- Process lifecycle management
- Parent-child relationships
- Process states (Ready, Running, Blocked, Zombie)
- Context switching

#### 4. Network Stack
**Status**: TCP/IP stack complete

**Location**: `src/net/`

**Features**:
- Full TCP/IP protocol suite (Ethernet, IPv4, TCP, UDP)
- POSIX-compatible socket interface
- Standard network protocols (ARP, ICMP, DHCP, DNS)
- Can communicate with Linux systems

**Compatibility**: Network protocols are standard, so RustOS can interoperate with Linux systems over the network.

#### 5. Virtual File System
**Status**: Core VFS implemented, filesystem support limited

**Location**: `src/fs/`

**Features**:
- Linux-inspired VFS layer
- RamFS (temporary in-memory filesystem)
- DevFS (device filesystem for /dev)
- Standard file operations interface

### 🚧 Partial Implementation

#### Dynamic Linking
**Status**: In progress

**Current Limitation**: Only static binaries fully supported  
**Future Work**: Dynamic linker, shared library support

#### Extended System Calls
**Status**: Core calls done, many extended calls pending

**Examples of Missing Calls**: 
- Advanced IPC (message queues, semaphores, shared memory)
- Extended file operations (ioctl, fcntl variants)
- Advanced process controls

#### File System Compatibility
**Status**: Limited

**Current**: RamFS, DevFS  
**Needed**: ext4, FAT32, NTFS readers/writers for Linux filesystem compatibility

### ❌ Not Implemented

#### Linux Kernel Modules
**Reason**: Completely different kernel architecture  
**Alternative**: RustOS has its own driver framework

#### Linux-Specific APIs
**Examples**: 
- Netlink sockets
- SystemD APIs
- D-Bus (requires userspace implementation)
- eBPF
- Linux kernel debugfs/procfs/sysfs (planning compatible versions)

#### Package Manager Compatibility
**Current Status**: Cannot use Linux package managers (apt, dnf, pacman) directly

**Reason**: 
- Packages expect Linux kernel
- Dependencies on Linux-specific libraries
- Different binary formats and ABIs

**Future Approach**: RustOS needs its own package system or adaptation layer

## Running Linux Software on RustOS

### What Can Run

✅ **Statically-linked POSIX programs**
- Simple command-line tools
- Programs using standard POSIX APIs
- No external dependencies

✅ **Network services**
- Programs using standard sockets
- TCP/UDP servers/clients
- HTTP servers (if dependencies met)

✅ **Simple C programs**
- When compiled as static binaries
- Using only implemented syscalls

### What Cannot Run (Yet)

❌ **Dynamically-linked binaries**
- Most Linux binaries use dynamic linking
- Requires shared library loader

❌ **Complex applications**
- GUI applications (no X11/Wayland yet)
- Applications expecting systemd
- Programs using unimplemented syscalls

❌ **Kernel modules or drivers**
- Linux kernel modules are Linux-specific
- Must be rewritten for RustOS

❌ **Distribution packages**
- .deb, .rpm, etc. cannot be installed directly
- Require Linux kernel and ecosystem
- **See [LINUX_APP_SUPPORT.md](LINUX_APP_SUPPORT.md) for detailed requirements**

### Compatibility Testing

To test if a Linux binary might work on RustOS:

1. **Check if statically linked**:
   ```bash
   file mybinary
   # Should show: "statically linked"
   ldd mybinary
   # Should show: "not a dynamic executable"
   ```

2. **Check system calls used**:
   ```bash
   strace -c mybinary
   # Review syscalls - check if RustOS implements them
   ```

3. **Try running in RustOS QEMU**:
   ```bash
   # Load binary into RustOS filesystem
   # Attempt execution
   # Check kernel logs for unsupported syscalls
   ```

## Development Roadmap for Compatibility

### Phase 1: Core POSIX (Current)
- ✅ Basic syscall interface
- ✅ ELF loading
- ✅ Process management
- ✅ Basic file I/O
- 🚧 Extended syscalls

### Phase 2: Enhanced Compatibility (Next 6 months)
- Dynamic linking support
- Shared library loading
- Extended POSIX syscalls
- procfs/sysfs compatibility
- C standard library port

### Phase 3: Binary Compatibility (6-12 months)
- Linux ABI compatibility layer
- glibc compatibility
- More complete syscall coverage
- Filesystem compatibility (ext4 read/write)

### Phase 4: Application Support (12+ months)
- GUI framework support
- Container runtime
- Package management adaptation
- Wide application compatibility

## How to Contribute

### Improving Linux Compatibility

1. **Implement Missing Syscalls**
   - Check `src/syscall/mod.rs` for TODOs
   - Add syscall handlers following existing patterns
   - Test with real Linux binaries

2. **Enhance ELF Loader**
   - Add dynamic linking support
   - Implement shared library loading
   - Support more ELF features

3. **Port C Library**
   - Consider musl libc port to RustOS
   - Implement missing libc functions
   - Test with standard C programs

4. **File System Support**
   - Implement ext4 driver
   - Add FAT32 support
   - Enable mounting Linux filesystems

5. **Testing and Documentation**
   - Test Linux binaries on RustOS
   - Document compatibility issues
   - Create compatibility test suite

### Testing Workflow

```bash
# 1. Build RustOS
make build

# 2. Create test binary (on Linux)
cat > test.c << 'EOF'
#include <stdio.h>
int main() {
    printf("Hello from Linux binary!\n");
    return 0;
}
EOF

# 3. Compile statically
gcc -static test.c -o test

# 4. Load into RustOS (future: via filesystem)
# 5. Run in RustOS QEMU environment
make run

# 6. Check compatibility
# - Does it load?
# - Do syscalls work?
# - Does it execute correctly?
```

## Comparison with Linux

| Feature | Linux | RustOS | Compatibility |
|---------|-------|--------|---------------|
| **Language** | C | Rust | N/A |
| **POSIX Syscalls** | Full | Partial | ~60% |
| **ELF Loading** | Full | Core | Static only |
| **Dynamic Linking** | Yes | No (yet) | 0% |
| **File Systems** | Many | 2 basic | Limited |
| **Network Stack** | Full | TCP/IP | Good |
| **Process Model** | Full | Core | Good |
| **Device Drivers** | Thousands | Few | Limited |
| **Package System** | Yes | No | 0% |

## Real-World Compatibility Examples

### Example 1: Simple Echo Server

A simple TCP echo server using standard POSIX sockets:

```c
// This COULD work on RustOS if compiled statically
#include <sys/socket.h>
#include <netinet/in.h>
#include <unistd.h>

int main() {
    int sock = socket(AF_INET, SOCK_STREAM, 0);
    // ... standard socket code ...
    bind(sock, ...);
    listen(sock, 5);
    accept(sock, ...);
    // ...
}
```

**Compatibility**: ✅ Likely to work (uses implemented syscalls)

### Example 2: GUI Application

A typical Linux GUI application:

```c
#include <gtk/gtk.h>  // Requires GTK, X11, etc.

int main() {
    gtk_init(...);
    // ...
}
```

**Compatibility**: ❌ Won't work (needs dynamic linking, graphics stack, many libraries)

### Example 3: Command Line Tool

A simple file processing tool:

```c
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv) {
    FILE *f = fopen(argv[1], "r");
    // ... process file ...
    fclose(f);
}
```

**Compatibility**: ✅ May work if statically compiled with implemented syscalls

## Frequently Asked Questions

### Can I install Ubuntu packages on RustOS?

No. Ubuntu packages (.deb files) are designed for the Linux kernel and include Linux-specific dependencies. RustOS would need a complete compatibility layer and package adaptation system.

**For a detailed technical guide on what would be required**, see [LINUX_APP_SUPPORT.md](LINUX_APP_SUPPORT.md), which covers:
- Exact components needed (dynamic linker, libc, syscalls, filesystems)
- Step-by-step implementation roadmap
- Realistic effort estimates (15-20 months)
- Alternative approaches

### Will Wine/Proton work on RustOS?

Not currently. Wine requires extensive Linux syscall support and libraries. This is a long-term goal once core compatibility is mature.

### Can RustOS run Docker containers?

Future goal. RustOS would need to implement:
- Container runtime (like containerd)
- Namespace support
- Cgroups equivalent
- Overlay filesystem

This is technically possible but requires significant development.

### Should I use RustOS or Linux?

**Use Linux if you need**:
- Production system
- Wide software compatibility  
- Hardware support
- Mature ecosystem

**Use RustOS for**:
- Learning OS development
- Research projects
- Rust-based system programming
- Experimental platforms

## Conclusion

**Bottom Line**: RustOS cannot be "merged into Linux" - they are separate operating systems. However, RustOS is actively working toward **interface compatibility** through POSIX standards, allowing properly-written portable software to run on both systems.

The path forward is implementing standard interfaces, not merging code. This is similar to how FreeBSD, OpenBSD, and other Unix-like systems achieve Linux software compatibility - through standard APIs and compatibility layers, not code merging.

For the latest compatibility status, see:
- [FAQ](FAQ.md) - General questions
- [ROADMAP.md](ROADMAP.md) - Development plans
- [ARCHITECTURE.md](ARCHITECTURE.md) - Technical details
- GitHub Issues - Compatibility tracking
