# Process Manager Module

Complete process management system for RustOS with POSIX-like APIs.

## Overview

This module provides comprehensive process management functionality including:
- Process creation and termination
- Process forking (copy parent process)
- Process execution (load new program)
- Process waiting and synchronization
- Process state management
- File descriptor management
- Process hierarchy (parent-child relationships)

## Architecture

### Components

1. **ProcessControlBlock (PCB)** - `pcb.rs`
   - Stores all process state information
   - Manages file descriptors
   - Tracks CPU context and memory info
   - Handles process lifecycle states

2. **ProcessTable** - `table.rs`
   - Central registry of all processes
   - PID allocation and management
   - Process lookup and iteration
   - Process relationship tracking

3. **Process Operations** - `operations.rs`
   - `fork()` - Create child process (copy parent)
   - `exec()` - Execute new program
   - `wait()` - Wait for child to exit
   - `waitpid()` - Wait for specific child
   - `exit()` - Terminate process
   - `getpid()` - Get process ID
   - `getppid()` - Get parent process ID

4. **ProcessManager** - `mod.rs`
   - High-level coordination
   - Thread-safe access to process table
   - Integration with scheduler and memory manager

## API Reference

### Process Creation

```rust
use rustos::process_manager::{get_process_manager, Priority};

let pm = get_process_manager();

// Create new process
let pid = pm.create_process(
    Some(parent_pid),  // Parent PID (None for kernel processes)
    "my_process",      // Process name
    Priority::Normal   // Priority level
)?;
```

### Fork - Create Child Process

```rust
use rustos::process_manager::fork;

// Fork current process
let child_pid = fork(parent_pid)?;

// Child is exact copy of parent at fork point
// Returns child PID to parent, 0 to child
```

### Exec - Load New Program

```rust
use rustos::process_manager::exec;

// Execute new program in process
let program_binary = &elf_binary_data;
let args = &["arg1", "arg2"];

exec(pid, program_binary, args)?;

// Process image is replaced with new program
// Does not return on success (process continues at new entry point)
```

### Wait - Synchronize with Children

```rust
use rustos::process_manager::wait;

// Wait for any child to exit
let (child_pid, exit_status) = wait(parent_pid)?;

println!("Child {} exited with status {}", child_pid, exit_status);
```

### Waitpid - Wait for Specific Child

```rust
use rustos::process_manager::waitpid;

// Wait for specific child
let exit_status = waitpid(parent_pid, child_pid)?;

println!("Child exited with status {}", exit_status);
```

### Exit - Terminate Process

```rust
use rustos::process_manager::exit;

// Terminate process with exit status
exit(pid, 0)?;  // 0 = success

// Process becomes zombie, waiting for parent to collect status
```

### Get Process Info

```rust
use rustos::process_manager::get_process_manager;

let pm = get_process_manager();

// Get current process ID
let pid = pm.current_pid();

// Get process control block
let pcb = pm.get_process(pid)?;

println!("Process: {} (PID {})", pcb.name_str(), pcb.pid);
println!("Parent: {:?}", pcb.parent_pid);
println!("State: {:?}", pcb.state);
```

## Process States

Processes transition through these states:

```
Ready → Running → [Blocked] → Ready → Running → Zombie → Dead
  ↑                   ↓
  └───────────────────┘
```

- **Ready**: Waiting to be scheduled
- **Running**: Currently executing on CPU
- **Blocked**: Waiting for I/O or resources
- **Zombie**: Terminated but waiting for parent to collect exit status
- **Dead**: Fully cleaned up and removed

## File Descriptors

Each process has a file descriptor table:

```rust
// Standard file descriptors (always present)
// 0 = stdin
// 1 = stdout
// 2 = stderr

// Allocate new file descriptor
let fd = pm.allocate_fd(pid, FileDescriptorType::File { path })?;

// Use file descriptor...

// Close file descriptor
pm.close_fd(pid, fd)?;
```

## Process Hierarchy

```
init (PID 0 or 1)
├── shell (PID 10)
│   ├── ls (PID 20)
│   └── cat (PID 21)
└── daemon (PID 11)
    └── worker (PID 30)
```

- Each process (except init) has a parent
- Parents can wait for children to exit
- Orphaned processes are reparented to init
- Zombie processes remain until parent collects exit status

## Integration

### With Scheduler

The process manager integrates with the scheduler for:
- Adding newly created processes to ready queue
- Removing terminated processes from scheduling
- Blocking/unblocking processes

### With Memory Manager

Integration for:
- Allocating memory for new processes
- Copy-on-write semantics for fork
- Freeing memory on process exit
- Page table management

### With ELF Loader

For `exec()` operation:
- Parse ELF binary format
- Load program segments into memory
- Setup initial stack and heap
- Resolve relocations
- Jump to program entry point

## Thread Safety

All operations are thread-safe:
- ProcessTable protected by Mutex
- Atomic operations for PID allocation
- Safe concurrent access to process information

## Performance

- O(log n) process lookup (BTreeMap)
- O(1) PID allocation (atomic counter)
- O(n) iteration over processes
- Efficient copy-on-write for fork

## Future Enhancements

1. **Signal System**
   - SIGCHLD on child exit
   - Signal handlers
   - Signal masking

2. **Session Management**
   - Process groups
   - Session leaders
   - Job control

3. **Resource Limits**
   - CPU time limits
   - Memory limits
   - File descriptor limits

4. **Credentials**
   - User ID (UID)
   - Group ID (GID)
   - Effective vs real IDs

5. **Process Statistics**
   - CPU usage
   - Memory usage
   - I/O statistics

## Examples

### Simple Process Creation

```rust
use rustos::process_manager::{init, get_process_manager, Priority};

// Initialize process manager
init()?;

let pm = get_process_manager();

// Create process
let pid = pm.create_process(None, "my_app", Priority::Normal)?;
println!("Created process with PID {}", pid);
```

### Fork and Execute Pattern

```rust
use rustos::process_manager::{fork, exec, wait};

// Fork to create child
let child_pid = fork(parent_pid)?;

if child_pid == 0 {
    // Child process
    let program = load_program("/bin/ls");
    exec(current_pid(), &program, &["-l"])?;
    // Does not return
} else {
    // Parent process
    let (pid, status) = wait(parent_pid)?;
    println!("Child {} exited with {}", pid, status);
}
```

### Process Monitoring

```rust
use rustos::process_manager::get_process_manager;

let pm = get_process_manager();

// List all processes
let processes = pm.list_processes();

for (pid, name, state, priority) in processes {
    println!("PID {}: {} [{:?}] priority={:?}",
        pid, name, state, priority);
}

// Get statistics
let count = pm.process_count();
println!("Total processes: {}", count);
```

## Testing

Run tests with:
```bash
cargo test -p rustos --lib process_manager
```

Tests cover:
- Process creation and termination
- Fork operation
- Wait and waitpid
- File descriptor management
- Process state transitions
- Process hierarchy
- Edge cases and error conditions
