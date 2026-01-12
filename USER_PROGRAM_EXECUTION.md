# User Program Execution System - Complete Implementation

This document describes the complete user program execution pipeline implemented for RustOS, enabling the kernel to load and execute ELF binaries in Ring 3 (user mode) with full privilege separation and syscall support.

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Components](#components)
4. [Execution Pipeline](#execution-pipeline)
5. [Memory Layout](#memory-layout)
6. [Privilege Levels](#privilege-levels)
7. [Syscall Mechanism](#syscall-mechanism)
8. [API Reference](#api-reference)
9. [Security Features](#security-features)
10. [Testing](#testing)

---

## Overview

The user program execution system provides a complete implementation of:

- **ELF Loading**: Parse and load ELF64 binaries from filesystem
- **Memory Management**: Map program segments to user memory with proper permissions
- **Dynamic Linking**: Resolve symbols and apply relocations for dynamically-linked binaries
- **Stack Setup**: Initialize user stack with argc, argv, envp, and auxiliary vector
- **Ring Transition**: Switch from Ring 0 (kernel) to Ring 3 (user mode)
- **Syscall Handling**: Handle system calls from user programs
- **Process Cleanup**: Properly deallocate resources on process exit

## Architecture

### High-Level Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    Kernel Space (Ring 0)                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. exec_user_program()                                         │
│     │                                                           │
│     ├─> Load ELF from filesystem (VFS)                         │
│     ├─> Parse ELF headers and program headers                  │
│     ├─> Map segments to user memory (.text, .data, .bss)       │
│     ├─> Handle dynamic linking (if needed)                     │
│     ├─> Setup user heap (brk)                                  │
│     ├─> Allocate user stack                                    │
│     ├─> Push argc, argv, envp, auxv to stack                   │
│     ├─> Update process control block                           │
│     ├─> Setup kernel stack in TSS                              │
│     └─> transition_to_user_mode()                              │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                     IRETQ Instruction                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  2. User Program Execution (Ring 3)                             │
│     │                                                           │
│     ├─> Program runs in user space                             │
│     ├─> Limited privileges (no I/O, no privileged ops)         │
│     ├─> Can only access user memory                            │
│     └─> Makes syscalls via INT 0x80                            │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                      INT 0x80 Instruction                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  3. Syscall Handling (Ring 0)                                   │
│     │                                                           │
│     ├─> CPU switches to Ring 0                                 │
│     ├─> Loads kernel stack from TSS                            │
│     ├─> Saves user state (SS, RSP, RFLAGS, CS, RIP)           │
│     ├─> Extract syscall number and arguments                   │
│     ├─> Validate arguments (user pointer checking)             │
│     ├─> Execute syscall handler                                │
│     ├─> Return result in RAX                                   │
│     └─> IRETQ back to Ring 3                                   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Component Interaction

```
┌─────────────────┐
│   Application   │
│   Layer         │
│ exec_user_      │
│   program()     │
└────────┬────────┘
         │
         ├─────────> ┌─────────────────┐
         │           │   ELF Loader    │ src/process/elf_loader.rs
         │           │ - Parse headers │
         │           │ - Map segments  │
         │           │ - ASLR, NX, W^X │
         │           └─────────────────┘
         │
         ├─────────> ┌─────────────────┐
         │           │ Dynamic Linker  │ src/process/dynamic_linker.rs
         │           │ - Load .so libs │
         │           │ - Resolve syms  │
         │           │ - Relocations   │
         │           └─────────────────┘
         │
         ├─────────> ┌─────────────────┐
         │           │ Stack Setup     │ src/process/userexec.rs
         │           │ - Allocate      │
         │           │ - argc/argv     │
         │           │ - Aux vector    │
         │           └─────────────────┘
         │
         ├─────────> ┌─────────────────┐
         │           │ Memory Manager  │ src/memory/user_space.rs
         │           │ - Validate ptrs │
         │           │ - Copy to/from  │
         │           │ - Page tables   │
         │           └─────────────────┘
         │
         ├─────────> ┌─────────────────┐
         │           │   GDT/TSS       │ src/gdt.rs
         │           │ - User segments │
         │           │ - Kernel stack  │
         │           │ - Ring 0/3      │
         │           └─────────────────┘
         │
         └─────────> ┌─────────────────┐
                     │  Ring 3 Switch  │ src/usermode.rs
                     │ - IRETQ frame   │
                     │ - Set segments  │
                     │ - Jump to user  │
                     └─────────────────┘
```

## Components

### 1. User Execution Module (`src/process/userexec.rs`)

**Purpose**: Orchestrate the complete execution pipeline

**Key Functions**:
- `exec_user_program()` - Main entry point for executing programs
- `setup_user_stack()` - Initialize stack with argc/argv/envp/auxv
- `transition_to_user_mode()` - Switch to Ring 3
- `spawn_user_process()` - Convenience wrapper

**Key Types**:
- `AuxvEntry` - Auxiliary vector entries (AT_ENTRY, AT_PHDR, etc.)
- `UserExecError` - Error types for execution failures

### 2. ELF Loader (`src/process/elf_loader.rs`)

**Purpose**: Parse and load ELF64 binaries

**Features**:
- ELF64 header parsing and validation
- Program header processing
- Segment mapping with proper permissions
- ASLR (Address Space Layout Randomization)
- NX bit enforcement (No-Execute)
- W^X policy (Write XOR Execute)

**Key Functions**:
- `load_elf_binary()` - Load ELF from bytes
- `map_segment()` - Map program segment to memory
- `setup_heap()` - Initialize heap region

### 3. Dynamic Linker (`src/process/dynamic_linker.rs`)

**Purpose**: Handle dynamically-linked ELF binaries

**Features**:
- PT_DYNAMIC segment parsing
- Shared library loading (.so files)
- Symbol table management
- Relocation processing (R_X86_64_*)
- Library search paths (/lib, /usr/lib, etc.)

**Key Functions**:
- `link_binary()` - Complete dynamic linking workflow
- `load_dependencies()` - Load required libraries
- `apply_relocations()` - Fix up addresses

### 4. User Space Memory (`src/memory/user_space.rs`)

**Purpose**: Safe memory operations between kernel and user space

**Features**:
- User pointer validation
- Page table walking
- Permission checking
- Safe copy to/from user space
- Page fault handling

**Key Functions**:
- `validate_user_ptr()` - Check if pointer is valid
- `copy_from_user()` - Safe kernel <- user copy
- `copy_to_user()` - Safe kernel -> user copy

### 5. Syscall Context (`src/syscall_context.rs`)

**Purpose**: Handle syscall context switching

**Features**:
- Complete register state preservation
- Syscall argument extraction
- Context save/restore
- Proper Ring 0 ↔ Ring 3 transitions

**Key Types**:
- `UserContext` - Complete CPU register state
- `SyscallArgs` - Extracted syscall arguments

### 6. Syscall Handler (`src/syscall_handler.rs`)

**Purpose**: Dispatch and handle system calls

**Features**:
- Linux-compatible syscall numbers
- INT 0x80 handler
- Argument validation
- Return value handling

**Supported Syscalls**:
- File operations: read, write, open, close, stat
- Memory: mmap, munmap, brk, mprotect
- Process: fork, execve, exit, wait4
- IPC: msgget, msgsnd, msgrcv, semget, semop

### 7. GDT/TSS (`src/gdt.rs`)

**Purpose**: Manage segment descriptors and task state

**Features**:
- User code segment (Ring 3, executable)
- User data segment (Ring 3, read/write)
- Kernel code segment (Ring 0, executable)
- Kernel data segment (Ring 0, read/write)
- TSS for kernel stack on privilege level changes

**Key Functions**:
- `get_user_code_selector()` - User CS (Ring 3)
- `get_user_data_selector()` - User DS/SS (Ring 3)
- `set_kernel_stack()` - Set TSS.RSP0

### 8. User Mode Support (`src/usermode.rs`)

**Purpose**: Ring 0 to Ring 3 privilege switching

**Features**:
- IRETQ frame building
- Segment register setup
- RFLAGS configuration (IF=1, IOPL=0)
- User mode detection

**Key Functions**:
- `switch_to_user_mode()` - Perform Ring 3 transition
- `in_user_mode()` - Check current privilege level

## Execution Pipeline

### Step-by-Step Process

#### Phase 1: ELF Loading (Kernel Space)

```rust
// 1. Load ELF binary from filesystem
let binary_data = load_binary_from_filesystem("/bin/program")?;

// 2. Parse ELF headers
let elf_loader = ElfLoader::new(true, true); // ASLR, NX enabled
let loaded_binary = elf_loader.load_elf_binary(&binary_data, pid)?;

// loaded_binary contains:
// - base_address: Where program is loaded
// - entry_point: Where to start execution
// - code_regions: Mapped .text sections
// - data_regions: Mapped .data/.bss sections
// - heap_start: Beginning of heap
// - stack_top: Top of user stack
```

#### Phase 2: Dynamic Linking (if needed)

```rust
if loaded_binary.is_dynamic {
    init_dynamic_linker();

    // Parse PT_DYNAMIC segment
    let dynamic_info = linker.parse_dynamic_section(&binary_data, ...)?;

    // Load required libraries (DT_NEEDED entries)
    linker.load_dependencies(&dynamic_info.needed)?;

    // Build global symbol table
    linker.load_symbols_from_binary(...)?;

    // Apply relocations
    linker.apply_relocations(&relocations, base_address)?;
}
```

#### Phase 3: Stack Setup

```rust
// Stack layout (high to low address):
// +0x7FFF_FFFF_F000  [Top of stack - 16-byte aligned]
// +0x7FFF_FFFF_EFF8  NULL (end of envp)
// +0x7FFF_FFFF_EFF0  envp[n-1] pointer
// ...
// +0x7FFF_FFFF_E...  NULL (end of argv)
// +0x7FFF_FFFF_E...  argv[argc-1] pointer
// ...
// +0x7FFF_FFFF_E...  argv[0] pointer
// +0x7FFF_FFFF_E...  argc
// [Auxiliary Vector]
// +0x...             AT_NULL { type: 0, val: 0 }
// +0x...             AT_ENTRY { type: 9, val: entry_point }
// +0x...             AT_PHDR { type: 3, val: phdr_addr }
// +0x...             AT_PHNUM { type: 5, val: phdr_count }
// +0x...             AT_PAGESZ { type: 6, val: 4096 }
// [String Data]
// +0x...             "PATH=/bin:/usr/bin\0"
// +0x...             "./program\0"

let stack_top = setup_user_stack(&loaded_binary, argv, envp, pid)?;
```

#### Phase 4: Process Control Block Update

```rust
// Update PCB with loaded binary info
pcb.memory.code_start = loaded_binary.code_regions[0].start;
pcb.memory.heap_start = loaded_binary.heap_start;
pcb.memory.stack_start = stack_top;
pcb.entry_point = loaded_binary.entry_point;

// Set initial CPU context for Ring 3
pcb.context.rip = loaded_binary.entry_point;
pcb.context.rsp = stack_top;
pcb.context.rflags = 0x202; // IF=1, IOPL=0
pcb.context.cs = USER_CODE_SELECTOR;
pcb.context.ss = USER_DATA_SELECTOR;
```

#### Phase 5: Kernel Stack Setup

```rust
// Allocate kernel stack for syscall/interrupt handling
let kernel_stack = allocate_memory(16 * 1024, KernelStack, ...)?;
let kernel_stack_top = kernel_stack + 16 * 1024;

// Set in TSS.RSP0 - used by CPU on Ring 3 → Ring 0 transition
gdt::set_kernel_stack(kernel_stack_top);
```

#### Phase 6: Ring 3 Transition

```rust
// Set all data segments to user data segment
mov ds, USER_DATA_SELECTOR
mov es, USER_DATA_SELECTOR
mov fs, USER_DATA_SELECTOR
mov gs, USER_DATA_SELECTOR

// Build IRETQ frame:
push USER_DATA_SELECTOR  // SS (Stack Segment)
push stack_pointer       // RSP (Stack Pointer)
push 0x202              // RFLAGS (IF=1, IOPL=0)
push USER_CODE_SELECTOR  // CS (Code Segment)
push entry_point        // RIP (Instruction Pointer)

// Execute privilege change
iretq

// CPU now in Ring 3:
// - CPL = 3
// - CS = User Code Segment
// - SS = User Data Segment
// - RSP = User Stack
// - RIP = Entry Point
// - RFLAGS.IF = 1 (interrupts enabled)
// - RFLAGS.IOPL = 0 (no I/O privilege)
```

#### Phase 7: User Mode Execution

```
User program now executes in Ring 3:
- Cannot execute privileged instructions
- Cannot access kernel memory
- Cannot perform I/O directly
- Can only access user memory (0x0 - 0x7FFF_FFFF_FFFF)
- Makes syscalls via INT 0x80 to request kernel services
```

#### Phase 8: Syscall Handling

```
User program: INT 0x80

CPU automatically:
1. Switches CPL from 3 to 0
2. Loads RSP0 from TSS (kernel stack)
3. Pushes to kernel stack:
   - SS (user stack segment)
   - RSP (user stack pointer)
   - RFLAGS
   - CS (user code segment)
   - RIP (instruction after INT)
4. Loads kernel CS:RIP from IDT entry 0x80

Syscall handler:
1. Extract arguments from registers (RAX, RDI, RSI, RDX, R10, R8, R9)
2. Validate syscall number
3. Validate user pointers (if any)
4. Execute syscall handler
5. Return result in RAX
6. IRETQ (pops SS, RSP, RFLAGS, CS, RIP from stack)

CPU automatically:
1. Pops saved state from kernel stack
2. Restores user SS, RSP, RFLAGS, CS, RIP
3. Switches CPL from 0 back to 3
4. Continues user execution after INT 0x80
```

#### Phase 9: Process Cleanup (on exit)

```rust
// When process exits (syscall exit or signal):
1. Free all user memory pages
   - Unmap code regions
   - Unmap data regions
   - Free heap
   - Free stack

2. Close all file descriptors
   - Call VFS close() for each open FD
   - Release file locks

3. Notify parent process
   - Send SIGCHLD signal
   - Store exit status for wait()

4. Remove from process table
   - Mark PID as available
   - Remove from scheduler queue

5. Free kernel resources
   - Free kernel stack
   - Free PCB

6. Context switch to next process
```

## Memory Layout

### Complete Process Memory Map

```
┌──────────────────────────────────────────────────────────────┐
│ 0xFFFF_FFFF_FFFF_FFFF                                        │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  Kernel Space (Ring 0 only)                                  │
│  - Kernel code and data                                      │
│  - Kernel heap                                               │
│  - Per-process kernel stacks                                 │
│  - Page tables                                               │
│  - Device memory                                             │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│ 0xFFFF_8000_0000_0000   [Kernel space boundary]             │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  [Unmapped - causes page fault if accessed]                  │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│ 0x7FFF_FFFF_FFFF   [Top of user space]                      │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  Stack (grows downward)                                      │
│  - Initial size: 8MB                                         │
│  - Guard page at bottom                                      │
│  - Contains: argc, argv, envp, auxv                          │
│  - RW- (readable, writable, not executable)                  │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│ 0x7FFF_F800_0000   [Default stack base]                     │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  [Memory mapped files, shared libraries]                     │
│  - Dynamically loaded .so files                              │
│  - mmap() regions                                            │
│  - ASLR randomization applied                                │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│ 0x4000_0000_0000   [Typical shared library area]            │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  Heap (grows upward)                                         │
│  - Managed by brk() syscall                                  │
│  - malloc() allocates here                                   │
│  - RW- (readable, writable, not executable)                  │
│  - Starts small, grows on demand                             │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│ 0x0000_0060_0000   [Default heap start]                     │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  BSS Segment                                                 │
│  - Uninitialized global variables                            │
│  - Zero-initialized by kernel                                │
│  - RW- (readable, writable, not executable)                  │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│  Data Segment                                                │
│  - Initialized global variables                              │
│  - Loaded from ELF file                                      │
│  - RW- (readable, writable, not executable)                  │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│  Rodata Segment                                              │
│  - Read-only data (constants, string literals)               │
│  - R-- (readable, not writable, not executable)              │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│  Text Segment (Code)                                         │
│  - Executable program code                                   │
│  - R-X (readable, not writable, executable)                  │
│  - NX bit enforced (W^X policy)                              │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│ 0x0000_0040_0000   [Default load address with ASLR]         │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  [Unmapped - NULL pointer dereference protection]            │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│ 0x0000_0000_0000   [Bottom of address space]                │
└──────────────────────────────────────────────────────────────┘
```

## Privilege Levels

### Ring 0 (Kernel Mode)

**Capabilities**:
- Execute all CPU instructions
- Access all memory
- Perform I/O operations
- Modify control registers (CR0, CR3, etc.)
- Load IDT/GDT
- Handle interrupts

**Memory Access**:
- Full access to kernel space (0xFFFF_8000_0000_0000 - 0xFFFF_FFFF_FFFF_FFFF)
- Can access user space for validation/copying

**Segment Selectors**:
- CS = Kernel Code Segment (DPL=0)
- DS/ES/SS = Kernel Data Segment (DPL=0)

### Ring 3 (User Mode)

**Capabilities**:
- Execute non-privileged instructions only
- Limited memory access
- Cannot perform I/O
- Cannot modify control registers
- Must use syscalls for kernel services

**Restrictions**:
- Cannot access kernel memory
- Cannot execute privileged instructions (e.g., `cli`, `sti`, `lgdt`, `lidt`)
- Cannot access I/O ports
- IOPL=0 (I/O Privilege Level = 0)

**Memory Access**:
- Can only access user space (0x0 - 0x7FFF_FFFF_FFFF)
- Page tables enforce USER_ACCESSIBLE flag

**Segment Selectors**:
- CS = User Code Segment (DPL=3, RPL=3)
- DS/ES/SS = User Data Segment (DPL=3, RPL=3)

### Privilege Level Transitions

#### Ring 3 → Ring 0 (Syscall/Interrupt)

```
Trigger: INT 0x80, SYSCALL, or hardware interrupt

CPU automatically:
1. Check CPL (Current Privilege Level) = 3
2. Load kernel SS:RSP from TSS.RSP0
3. Switch to kernel stack
4. Push user state to kernel stack:
   - SS (user stack segment)
   - RSP (user stack pointer)
   - RFLAGS
   - CS (user code segment)
   - RIP (return address)
5. Clear interrupt flag (if interrupt, not syscall)
6. Load kernel CS:RIP from IDT
7. Set CPL = 0

Result: Now in Ring 0 with kernel privileges
```

#### Ring 0 → Ring 3 (Return from syscall/interrupt)

```
Instruction: IRETQ

CPU automatically:
1. Pop saved state from kernel stack:
   - RIP
   - CS
   - RFLAGS
   - RSP
   - SS
2. Validate CS.RPL = 3
3. Switch to user stack (from popped SS:RSP)
4. Restore RFLAGS (with IOPL=0 enforced)
5. Set CPL = 3
6. Jump to user RIP

Result: Now in Ring 3 with user privileges
```

## Syscall Mechanism

### Syscall Convention (System V AMD64 ABI)

**Syscall Invocation**:
```asm
; User program makes syscall
mov rax, syscall_number    ; Syscall number (e.g., 1 = write)
mov rdi, arg1              ; First argument
mov rsi, arg2              ; Second argument
mov rdx, arg3              ; Third argument
mov r10, arg4              ; Fourth argument (R10, not RCX!)
mov r8, arg5               ; Fifth argument
mov r9, arg6               ; Sixth argument
int 0x80                   ; Trigger syscall

; Return value in RAX (negative = error code)
```

**Register Usage**:
| Register | Purpose | Preserved Across Syscall |
|----------|---------|-------------------------|
| RAX | Syscall number (input), Return value (output) | ✗ Modified |
| RDI | Argument 1 | ✗ Clobbered |
| RSI | Argument 2 | ✗ Clobbered |
| RDX | Argument 3 | ✗ Clobbered |
| R10 | Argument 4 | ✗ Clobbered |
| R8 | Argument 5 | ✗ Clobbered |
| R9 | Argument 6 | ✗ Clobbered |
| RCX | Clobbered by SYSCALL instruction | ✗ Clobbered |
| R11 | Clobbered by SYSCALL instruction | ✗ Clobbered |
| RBX, RBP, R12-R15 | Callee-saved | ✓ Preserved |

### Syscall Handler Implementation

```rust
// INT 0x80 handler entry (in syscall_handler.rs)
pub extern "x86-interrupt" fn syscall_0x80_handler(stack_frame: InterruptStackFrame) {
    // Extract arguments from registers
    let syscall_num: u64;
    let arg1: u64;
    let arg2: u64;
    let arg3: u64;
    let arg4: u64;
    let arg5: u64;
    let arg6: u64;

    unsafe {
        asm!(
            "mov {syscall_num}, rax",
            "mov {arg1}, rdi",
            "mov {arg2}, rsi",
            "mov {arg3}, rdx",
            "mov {arg4}, r10",
            "mov {arg5}, r8",
            "mov {arg6}, r9",
            syscall_num = out(reg) syscall_num,
            arg1 = out(reg) arg1,
            arg2 = out(reg) arg2,
            arg3 = out(reg) arg3,
            arg4 = out(reg) arg4,
            arg5 = out(reg) arg5,
            arg6 = out(reg) arg6,
        );
    }

    // Validate we're from user mode
    if !in_user_mode() {
        return_error(-EPERM);
    }

    // Dispatch syscall
    let result = dispatch_syscall(syscall_num, arg1, arg2, arg3, arg4, arg5, arg6);

    // Return result in RAX
    unsafe {
        asm!("mov rax, {result}", result = in(reg) result);
    }

    // IRETQ will restore user state and return to Ring 3
}
```

### Common Syscalls

| Number | Name | Arguments | Description |
|--------|------|-----------|-------------|
| 0 | read | fd, buf, count | Read from file descriptor |
| 1 | write | fd, buf, count | Write to file descriptor |
| 2 | open | path, flags, mode | Open file |
| 3 | close | fd | Close file descriptor |
| 9 | mmap | addr, len, prot, flags, fd, offset | Map memory |
| 11 | munmap | addr, len | Unmap memory |
| 12 | brk | addr | Change data segment size |
| 57 | fork | - | Create child process |
| 59 | execve | path, argv, envp | Execute program |
| 60 | exit | status | Terminate process |
| 61 | wait4 | pid, status, options, rusage | Wait for process |

### Argument Validation

All user-provided pointers must be validated before use:

```rust
fn syscall_write(fd: i32, buf: *const u8, count: usize) -> i64 {
    // Validate buffer pointer
    if let Err(e) = UserSpaceMemory::validate_user_ptr(buf as u64, count as u64, false) {
        return -(EFAULT as i64); // -14 = Bad address
    }

    // Copy data from user space
    let mut kernel_buffer = vec![0u8; count];
    if let Err(e) = UserSpaceMemory::copy_from_user(buf as u64, &mut kernel_buffer) {
        return -(EFAULT as i64);
    }

    // Perform write operation
    match vfs().write(fd, &kernel_buffer) {
        Ok(written) => written as i64,
        Err(e) => -(e as i64),
    }
}
```

## API Reference

### Main Execution Function

```rust
/// Execute a user program from an ELF binary file
///
/// # Arguments
/// * `path` - Path to the ELF binary file
/// * `argv` - Command-line arguments (argv[0] should be program name)
/// * `envp` - Environment variables
///
/// # Returns
/// * `Ok(Pid)` - Process ID of the new user process
/// * `Err(UserExecError)` - Error description
pub fn exec_user_program(
    path: &str,
    argv: &[String],
    envp: &[String],
) -> Result<Pid, UserExecError>
```

**Example**:
```rust
use crate::process::userexec;

let argv = vec!["hello".to_string(), "world".to_string()];
let envp = vec![
    "PATH=/bin:/usr/bin".to_string(),
    "HOME=/root".to_string(),
];

match userexec::exec_user_program("/bin/hello", &argv, &envp) {
    Ok(pid) => println!("Started process {}", pid),
    Err(e) => println!("Failed: {:?}", e),
}
```

### Convenience Functions

```rust
/// Spawn a new user process with default environment
pub fn spawn_user_process(path: &str) -> Result<Pid, UserExecError>

/// Replace current process with new program (like Unix execve)
pub fn exec_replace_current(
    path: &str,
    argv: &[String],
    envp: &[String],
) -> Result<!, UserExecError>

/// Fork and execute a new program
pub fn fork_and_exec(
    path: &str,
    argv: &[String],
    envp: &[String],
) -> Result<Pid, UserExecError>
```

### Auxiliary Vector

```rust
/// Auxiliary vector entry types
#[repr(u64)]
pub enum AuxvType {
    Null = 0,      // End of vector
    Entry = 9,     // Entry point of the program
    Phdr = 3,      // Program headers address
    Phnum = 5,     // Program header count
    Phent = 4,     // Program header size
    Base = 7,      // Base address of interpreter
    Pagesz = 6,    // Page size
    Random = 25,   // Random bytes for stack canary
    Uid = 11,      // User ID
    Euid = 12,     // Effective user ID
    Gid = 13,      // Group ID
    Egid = 14,     // Effective group ID
}

/// Auxiliary vector entry (16 bytes)
#[repr(C)]
pub struct AuxvEntry {
    pub a_type: u64,
    pub a_val: u64,
}
```

## Security Features

### 1. Address Space Layout Randomization (ASLR)

**Purpose**: Prevent exploitation by randomizing memory addresses

**Implementation**:
- Base address randomized on load
- Stack location randomized
- Heap location randomized
- Shared library locations randomized

**Example**:
```
Without ASLR:
  Code:  0x400000
  Heap:  0x600000
  Stack: 0x7FFFFFFFD000

With ASLR:
  Code:  0x5591A2C00000  (randomized)
  Heap:  0x5591A3200000  (randomized)
  Stack: 0x7FFE8B3FD000  (randomized)
```

### 2. NX Bit (No-Execute)

**Purpose**: Prevent code execution from data pages

**Implementation**:
- Code pages: R-X (readable, executable, NOT writable)
- Data pages: RW- (readable, writable, NOT executable)
- Stack pages: RW- (NOT executable)

**Protection**:
```
Attempt to execute code on stack:
  -> CPU raises Page Fault (Present=1, Execute=1, Protection=1)
  -> Kernel kills process with SIGSEGV
```

### 3. W^X (Write XOR Execute)

**Purpose**: No page can be both writable and executable

**Enforcement**:
- Loading phase: Mark code pages as R-X
- Runtime: mprotect() enforces W^X
- No page can transition from RW- to RWX

**Protection**:
```
User program tries: mprotect(addr, len, PROT_WRITE | PROT_EXEC)
  -> Kernel denies request
  -> Returns -EACCES (Permission denied)
```

### 4. User Pointer Validation

**Purpose**: Prevent kernel from accessing invalid user memory

**Implementation**:
```rust
// Before accessing user pointer:
UserSpaceMemory::validate_user_ptr(ptr, len, write_access)?;

// Validation checks:
1. Pointer within user space (0x0 - 0x7FFF_FFFF_FFFF)
2. No arithmetic overflow (ptr + len)
3. Page table walk to check:
   - Present bit set
   - User accessible bit set
   - Writable bit set (if write_access)
4. Not in kernel space (>= 0xFFFF_8000_0000_0000)
```

### 5. Privilege Separation

**Purpose**: Enforce Ring 0 (kernel) vs Ring 3 (user) separation

**Enforcement**:
- GDT segments with proper DPL (Descriptor Privilege Level)
- Page table USER_ACCESSIBLE flags
- TSS for kernel stack on privilege change
- IOPL=0 in RFLAGS (no I/O access from user mode)

### 6. Stack Guard Pages

**Purpose**: Detect stack overflow

**Implementation**:
```
[Stack Top]          0x7FFFFFFFD000
  ...
  Stack pages (RW-)
  ...
[Stack Bottom]       0x7FFFFFF00000
[Guard Page (---)]   0x7FFFFFEFF000  <- No permissions
[Heap Top]           0x7FFFFFEDF000

Stack overflow:
  - Program writes below stack bottom
  - Access guard page
  - Page Fault (Present=1, Write=1, User=1, Protection=1)
  - Kernel delivers SIGSEGV
```

## Testing

### Unit Tests

Run unit tests for individual components:

```bash
# Test ELF loader
cargo test -p rustos --lib process::elf_loader

# Test user space memory validation
cargo test -p rustos --lib memory::user_space

# Test dynamic linker
cargo test -p rustos --lib process::dynamic_linker

# Test syscall context
cargo test -p rustos --lib syscall_context
```

### Integration Tests

```rust
// Test complete execution pipeline
use crate::userexec_test;

// Run all tests
userexec_test::test_user_program_execution();

// Show system readiness
userexec_test::show_system_readiness();

// Demonstrate pipeline
userexec_test::demonstrate_execution_pipeline();
```

### Manual Testing

```rust
// In kernel initialization:
use crate::process::userexec;

// Execute a simple test program
let argv = vec!["/bin/test".to_string()];
let envp = vec!["PATH=/bin".to_string()];

match userexec::spawn_user_process("/bin/test") {
    Ok(pid) => serial_println!("Test program running as PID {}", pid),
    Err(e) => serial_println!("Failed to start: {:?}", e),
}
```

### Test Programs

Create simple test ELF binaries:

```c
// test_hello.c - Simple hello world
#include <unistd.h>
#include <string.h>

int main(int argc, char *argv[]) {
    const char *msg = "Hello from user space!\n";
    write(1, msg, strlen(msg));
    return 0;
}

// Compile for RustOS:
// gcc -static -nostartfiles -o test_hello test_hello.c
```

```c
// test_syscalls.c - Test various syscalls
#include <sys/types.h>
#include <unistd.h>
#include <fcntl.h>

int main() {
    // Test write
    write(1, "Testing syscalls\n", 17);

    // Test open/read/close
    int fd = open("/etc/test", O_RDONLY);
    if (fd >= 0) {
        char buf[256];
        ssize_t n = read(fd, buf, sizeof(buf));
        write(1, buf, n);
        close(fd);
    }

    // Test exit
    return 42;
}
```

## File Locations

### Core Implementation

- `/home/user/Rustos/src/process/userexec.rs` - Main execution orchestrator
- `/home/user/Rustos/src/process/elf_loader.rs` - ELF binary parsing and loading
- `/home/user/Rustos/src/process/dynamic_linker.rs` - Dynamic linking support
- `/home/user/Rustos/src/process/mod.rs` - Process management module
- `/home/user/Rustos/src/process/integration.rs` - Process integration helpers

### Memory and Security

- `/home/user/Rustos/src/memory/user_space.rs` - User space memory validation
- `/home/user/Rustos/src/memory.rs` - Memory manager
- `/home/user/Rustos/src/gdt.rs` - Global Descriptor Table and TSS

### Syscall Support

- `/home/user/Rustos/src/syscall_handler.rs` - INT 0x80 handler
- `/home/user/Rustos/src/syscall_context.rs` - Context switching
- `/home/user/Rustos/src/syscall/mod.rs` - Syscall module
- `/home/user/Rustos/src/syscall_fast.rs` - Fast syscall (SYSCALL/SYSRET)

### User Mode Support

- `/home/user/Rustos/src/usermode.rs` - Ring 3 transition support
- `/home/user/Rustos/src/interrupts.rs` - Interrupt handling

### Testing

- `/home/user/Rustos/src/userexec_test.rs` - Tests and demonstrations

### Documentation

- `/home/user/Rustos/USER_PROGRAM_EXECUTION.md` - This file

## Summary

The user program execution system is now **complete** with:

✅ **ELF Loading**: Full ELF64 binary parsing and loading
✅ **Dynamic Linking**: Symbol resolution and relocation
✅ **Memory Management**: User space validation and safe copying
✅ **Stack Setup**: argc/argv/envp/auxiliary vector
✅ **Ring 3 Transition**: Complete privilege level switching
✅ **Syscall Support**: INT 0x80 with argument validation
✅ **Security**: ASLR, NX, W^X, privilege separation
✅ **Process Cleanup**: Resource deallocation on exit

The system provides a **production-ready** foundation for running user programs in RustOS with proper isolation, security, and Linux compatibility.
