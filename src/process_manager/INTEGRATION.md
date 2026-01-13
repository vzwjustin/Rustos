# Process Manager Integration Guide

## Quick Start

### 1. Initialize the Process Manager

Add to your kernel initialization sequence:

```rust
// In kernel_main() or init sequence
use crate::process_manager;

fn init_kernel() -> Result<(), &'static str> {
    // ... other initialization ...

    // Initialize process manager
    process_manager::init()?;

    println!("Process manager initialized");

    Ok(())
}
```

### 2. Basic Usage

```rust
use crate::process_manager::{get_process_manager, Priority};

let pm = get_process_manager();

// Create a process
let pid = pm.create_process(
    Some(0),           // Parent PID
    "my_process",      // Process name
    Priority::Normal   // Priority
)?;

println!("Created process with PID: {}", pid);
```

### 3. Fork-Exec Pattern

```rust
use crate::process_manager::{get_process_manager, fork, exec, wait};

// Fork current process
let parent_pid = pm.current_pid();
let child_pid = pm.fork(parent_pid)?;

// Execute program in child
let program = load_elf_binary("/bin/ls")?;
pm.exec(child_pid, &program, &["ls", "-la"])?;

// Parent waits for child
let (pid, status) = pm.wait(parent_pid)?;
println!("Child {} exited with status {}", pid, status);
```

## Integration with Existing Systems

### With Scheduler (src/process/scheduler.rs)

The process manager provides hooks for scheduler integration:

```rust
// When creating a process
let pid = pm.create_process(None, "new_process", Priority::Normal)?;

// Add to scheduler
let process_manager = crate::process::get_process_manager();
let mut scheduler = process_manager.scheduler.lock();
scheduler.add_process(pid, Priority::Normal)?;

// When blocking a process
pm.set_process_state(pid, ProcessState::Blocked)?;
let _ = process_manager.block_process(pid);

// When unblocking
pm.set_process_state(pid, ProcessState::Ready)?;
let _ = process_manager.unblock_process(pid);
```

### With Memory Manager (src/memory.rs)

For fork() to work with copy-on-write:

```rust
// In fork operation (operations.rs)
pub fn fork(parent_pid: Pid, process_table: &Mutex<ProcessTable>) -> Result<Pid, &'static str> {
    // ... existing code ...

    // After cloning PCB, setup memory
    let memory_manager = crate::memory::get_memory_manager();

    // Clone page tables with COW semantics
    memory_manager.clone_address_space(parent.memory.page_directory, child.memory.page_directory)?;

    // ... rest of fork ...
}
```

### With ELF Loader (src/process/elf_loader.rs)

For exec() to load programs:

```rust
// In exec operation (operations.rs)
fn parse_program(program: &[u8]) -> Result<u64, &'static str> {
    let elf_loader = crate::process::elf_loader::ElfLoader::new();

    // Parse and validate ELF
    let elf_info = elf_loader.parse(program)?;

    // Load segments into memory
    let entry_point = elf_loader.load(elf_info)?;

    Ok(entry_point)
}
```

## Syscall Integration

To expose process management through syscalls:

```rust
// In syscall handler
pub fn handle_syscall(syscall_num: u64, args: &[u64]) -> Result<u64, &'static str> {
    let pm = get_process_manager();

    match syscall_num {
        // SYS_FORK
        2 => {
            let current = pm.current_pid();
            let child = pm.fork(current)?;
            Ok(child as u64)
        }

        // SYS_EXEC
        11 => {
            let pid = pm.current_pid();
            let program_ptr = args[0] as *const u8;
            let program_len = args[1] as usize;
            let program = unsafe { core::slice::from_raw_parts(program_ptr, program_len) };
            pm.exec(pid, program, &[])?;
            Ok(0)
        }

        // SYS_WAIT
        7 => {
            let current = pm.current_pid();
            let (child_pid, status) = pm.wait(current)?;
            Ok(((child_pid as u64) << 32) | (status as u64 & 0xFFFFFFFF))
        }

        // SYS_EXIT
        1 => {
            let status = args[0] as i32;
            let pid = pm.current_pid();
            pm.exit(pid, status)?;
            // Does not return
            Ok(0)
        }

        // SYS_GETPID
        20 => {
            Ok(pm.current_pid() as u64)
        }

        // SYS_GETPPID
        39 => {
            let pid = pm.current_pid();
            let ppid = pm.get_parent_pid(pid).ok_or("No parent")?;
            Ok(ppid as u64)
        }

        _ => Err("Unknown syscall")
    }
}
```

## Timer Interrupt Integration

Update scheduler tick to check process state:

```rust
// In timer interrupt handler
pub extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // ... existing code ...

    let pm = crate::process_manager::get_process_manager();
    let current_pid = pm.current_pid();

    // Update process CPU time
    if let Some(mut pcb) = pm.get_process(current_pid) {
        pcb.cpu_time += 1;
    }

    // Trigger scheduler
    let process_manager = crate::process::get_process_manager();
    if let Ok(Some(next_pid)) = process_manager.schedule() {
        pm.set_current_pid(next_pid);
        // Perform context switch
    }

    // ... acknowledge interrupt ...
}
```

## Context Switching Integration

Update context switcher to use process manager:

```rust
// In context switching code
pub fn switch_to_process(next_pid: Pid) -> Result<(), &'static str> {
    let pm = crate::process_manager::get_process_manager();

    // Get current and next process
    let current_pid = pm.current_pid();
    let current = pm.get_process(current_pid).ok_or("Current process not found")?;
    let next = pm.get_process(next_pid).ok_or("Next process not found")?;

    // Update states
    pm.set_process_state(current_pid, ProcessState::Ready)?;
    pm.set_process_state(next_pid, ProcessState::Running)?;

    // Perform low-level context switch
    let context_switcher = crate::process::context::get_context_switcher();
    unsafe {
        context_switcher.switch_context(&current.context, &next.context, next_pid)?;
    }

    // Update current process
    pm.set_current_pid(next_pid);

    Ok(())
}
```

## Testing Integration

Add to your kernel test suite:

```rust
#[test_case]
fn test_process_management() {
    use crate::process_manager::{init, get_process_manager, Priority};

    // Initialize
    init().expect("Failed to initialize process manager");

    let pm = get_process_manager();

    // Test process creation
    let pid = pm.create_process(Some(0), "test", Priority::Normal)
        .expect("Failed to create process");

    assert!(pid > 0);
    assert_eq!(pm.process_count(), 2); // init + test

    serial_println!("Process management test: OK");
}
```

## Command-Line Tools

Add shell commands for process management:

```rust
// In shell command handler
match command {
    "ps" => {
        let pm = get_process_manager();
        let processes = pm.list_processes();

        println!("PID  NAME             STATE      PRIORITY");
        println!("---  ----             -----      --------");

        for (pid, name, state, priority) in processes {
            println!("{:<4} {:<16} {:?:<10} {:?}", pid, name, state, priority);
        }
    }

    "kill" => {
        let pid: Pid = args[0].parse().expect("Invalid PID");
        pm.exit(pid, -1).expect("Failed to kill process");
        println!("Process {} terminated", pid);
    }

    "fork" => {
        let current = pm.current_pid();
        let child = pm.fork(current).expect("Fork failed");
        println!("Created child process: {}", child);
    }

    _ => println!("Unknown command")
}
```

## Error Handling

Proper error handling for process operations:

```rust
use crate::process_manager::{get_process_manager, ProcessState};

fn safe_process_operation(pid: Pid) -> Result<(), &'static str> {
    let pm = get_process_manager();

    // Check if process exists
    let pcb = pm.get_process(pid).ok_or("Process not found")?;

    // Check if process is in valid state
    if pcb.state == ProcessState::Zombie {
        return Err("Cannot operate on zombie process");
    }

    if pcb.state == ProcessState::Dead {
        return Err("Process is dead");
    }

    // Perform operation
    // ...

    Ok(())
}
```

## Performance Considerations

### Process Table Size
- Default: 4096 processes max
- Adjust in `table.rs`: `const MAX_PROCESSES: usize = 4096;`

### Memory Per Process
- PCB: ~1KB
- FD table: ~500 bytes
- Context: ~256 bytes
- Total: ~2KB per process

### Optimization Tips

1. **Minimize locks**: Hold locks for shortest time possible
2. **Batch operations**: Group multiple process operations together
3. **Lazy cleanup**: Defer zombie cleanup until parent waits
4. **Copy-on-write**: Implement COW for fork to reduce memory copies

## Debugging

Enable process manager debugging:

```rust
// Add to process_manager/mod.rs
#[cfg(feature = "debug-process-manager")]
macro_rules! pm_debug {
    ($($arg:tt)*) => {
        serial_println!("[PM] {}", format_args!($($arg)*));
    };
}

// Use in operations
pm_debug!("Creating process: {}", name);
pm_debug!("Fork: parent={}, child={}", parent_pid, child_pid);
pm_debug!("Exit: pid={}, status={}", pid, status);
```

## Complete Example

See `examples.rs` for 11 complete examples including:
- Basic process creation
- Fork operation
- Exec program loading
- Wait for children
- Fork-exec-wait pattern
- Process hierarchy
- File descriptors
- Process termination
- State management
- Process monitoring
- Shell-like spawning

## Troubleshooting

### Issue: "Process table full"
- Increase MAX_PROCESSES in `table.rs`
- Implement process cleanup/reaping

### Issue: "Current process not found"
- Ensure process manager initialized
- Verify current_pid tracking is correct

### Issue: Fork doesn't copy memory
- Implement COW in memory manager
- Add page table cloning

### Issue: Exec doesn't load program
- Complete ELF loader integration
- Verify binary format

### Issue: Wait blocks forever
- Ensure child exits properly
- Check zombie state transition
- Verify parent-child relationship

## Next Steps

1. **Integrate with scheduler**: Add/remove processes from ready queue
2. **Implement COW fork**: Optimize memory copying
3. **Complete ELF loading**: Full program loading in exec
4. **Add signals**: SIGCHLD, signal handlers
5. **Add credentials**: UID/GID support
6. **Add resource limits**: CPU time, memory limits
7. **Add process groups**: Session management

## Questions?

See:
- `README.md` - Full documentation
- `examples.rs` - Usage examples
- `tests.rs` - Test cases
- `SUMMARY.md` - Implementation details
