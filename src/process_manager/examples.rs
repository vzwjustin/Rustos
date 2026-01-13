//! Process Manager Usage Examples
//!
//! Demonstrates how to use the process management APIs in RustOS.

#![allow(dead_code)]

use crate::process_manager::{
    get_process_manager, init, fork, exec, wait, waitpid, exit,
    ProcessState, FileDescriptorType, Priority
};
use crate::process::Pid;

/// Example 1: Create a simple process
pub fn example_create_process() -> Result<(), &'static str> {
    // Initialize process manager
    init()?;

    let pm = get_process_manager();

    // Create a new process
    let pid = pm.create_process(
        None,              // No parent (kernel process)
        "my_process",      // Process name
        Priority::Normal   // Normal priority
    )?;

    println!("Created process with PID: {}", pid);

    // Get process info
    if let Some(pcb) = pm.get_process(pid) {
        println!("Process name: {}", pcb.name_str());
        println!("State: {:?}", pcb.state);
        println!("Priority: {:?}", pcb.priority);
    }

    Ok(())
}

/// Example 2: Fork a process
pub fn example_fork_process(parent_pid: Pid) -> Result<(), &'static str> {
    let pm = get_process_manager();

    println!("Parent PID: {}", parent_pid);

    // Fork the process
    let child_pid = pm.fork(parent_pid)?;

    println!("Created child with PID: {}", child_pid);

    // Check parent's child count
    if let Some(parent) = pm.get_process(parent_pid) {
        println!("Parent now has {} children", parent.child_count);
    }

    // Check child's parent
    if let Some(child) = pm.get_process(child_pid) {
        println!("Child's parent PID: {:?}", child.parent_pid);
    }

    Ok(())
}

/// Example 3: Execute a program
pub fn example_exec_program(pid: Pid, program: &[u8]) -> Result<(), &'static str> {
    let pm = get_process_manager();

    // Arguments to pass to the program
    let args = &["arg1", "arg2", "arg3"];

    println!("Executing program in process {}", pid);

    // Execute the program
    pm.exec(pid, program, args)?;

    println!("Program loaded successfully");

    // Check updated process state
    if let Some(pcb) = pm.get_process(pid) {
        println!("Entry point: 0x{:x}", pcb.entry_point);
        println!("Arguments: {:?}", pcb.get_args());
    }

    Ok(())
}

/// Example 4: Wait for child process
pub fn example_wait_for_child(parent_pid: Pid) -> Result<(), &'static str> {
    let pm = get_process_manager();

    println!("Parent {} waiting for child to exit...", parent_pid);

    // Wait for any child to exit
    match pm.wait(parent_pid) {
        Ok((child_pid, exit_status)) => {
            println!("Child {} exited with status {}", child_pid, exit_status);
            Ok(())
        }
        Err(e) => {
            println!("Wait failed: {}", e);
            Err(e)
        }
    }
}

/// Example 5: Complete fork-exec-wait pattern
pub fn example_fork_exec_wait(parent_pid: Pid, program: &[u8]) -> Result<(), &'static str> {
    let pm = get_process_manager();

    // Fork to create child
    let child_pid = pm.fork(parent_pid)?;
    println!("Forked child process: {}", child_pid);

    // In real implementation, we would check if we're parent or child
    // For now, parent continues here

    // Execute program in child
    pm.exec(child_pid, program, &["--help"])?;
    println!("Loaded program into child");

    // Parent waits for child
    let (pid, status) = pm.wait(parent_pid)?;
    println!("Child {} completed with status {}", pid, status);

    Ok(())
}

/// Example 6: Process hierarchy
pub fn example_process_hierarchy() -> Result<(), &'static str> {
    let pm = get_process_manager();

    // Create parent
    let parent = pm.create_process(Some(0), "parent", Priority::Normal)?;
    println!("Created parent: {}", parent);

    // Create multiple children
    let child1 = pm.fork(parent)?;
    let child2 = pm.fork(parent)?;
    let child3 = pm.fork(parent)?;

    println!("Created children: {}, {}, {}", child1, child2, child3);

    // Create grandchild
    let grandchild = pm.fork(child1)?;
    println!("Created grandchild: {}", grandchild);

    // List all processes
    let processes = pm.list_processes();
    println!("\nProcess hierarchy:");
    for (pid, name, state, priority) in processes {
        if let Some(ppid) = pm.get_parent_pid(pid) {
            println!("  PID {} ({}): parent={} state={:?} priority={:?}",
                pid, name, ppid, state, priority);
        } else {
            println!("  PID {} ({}): [no parent] state={:?} priority={:?}",
                pid, name, state, priority);
        }
    }

    Ok(())
}

/// Example 7: File descriptor management
pub fn example_file_descriptors(pid: Pid) -> Result<(), &'static str> {
    let pm = get_process_manager();

    println!("Managing file descriptors for PID {}", pid);

    // Check standard file descriptors
    println!("Standard file descriptors:");
    for fd in 0..3 {
        if let Some(descriptor) = pm.get_fd(pid, fd) {
            println!("  FD {}: {:?}", fd, descriptor.fd_type);
        }
    }

    // Allocate new file descriptor for a file
    let mut file_path = [0u8; 256];
    let path = b"/home/user/test.txt";
    file_path[..path.len()].copy_from_slice(path);

    let fd = pm.allocate_fd(pid, FileDescriptorType::File { path: file_path })?;
    println!("Allocated file descriptor: {}", fd);

    // Allocate file descriptor for a pipe
    let pipe_fd = pm.allocate_fd(pid, FileDescriptorType::Pipe {
        pipe_id: 1,
        read_end: true,
    })?;
    println!("Allocated pipe descriptor: {}", pipe_fd);

    // List all file descriptors
    if let Some(pcb) = pm.get_process(pid) {
        println!("\nAll file descriptors for process:");
        for (fd, descriptor) in &pcb.fd_table {
            println!("  FD {}: {:?}", fd, descriptor.fd_type);
        }
    }

    // Close file descriptor
    pm.close_fd(pid, fd)?;
    println!("Closed file descriptor {}", fd);

    Ok(())
}

/// Example 8: Process termination and cleanup
pub fn example_process_termination(pid: Pid) -> Result<(), &'static str> {
    let pm = get_process_manager();

    println!("Terminating process {}", pid);

    // Get parent before termination
    let parent_pid = pm.get_parent_pid(pid);

    // Exit the process with status code
    pm.exit(pid, 42)?;
    println!("Process {} exited with status 42", pid);

    // Check process state
    if let Some(pcb) = pm.get_process(pid) {
        println!("Process state: {:?}", pcb.state);
        println!("Exit status: {:?}", pcb.exit_status);
    }

    // If process has parent, parent can wait for it
    if let Some(ppid) = parent_pid {
        println!("Parent {} can now wait for child", ppid);
        match pm.waitpid(ppid, pid) {
            Ok(status) => {
                println!("Parent collected exit status: {}", status);
            }
            Err(e) => {
                println!("Wait failed: {}", e);
            }
        }
    }

    Ok(())
}

/// Example 9: Process state management
pub fn example_process_states(pid: Pid) -> Result<(), &'static str> {
    let pm = get_process_manager();

    println!("Managing states for process {}", pid);

    // Get initial state
    if let Some(pcb) = pm.get_process(pid) {
        println!("Initial state: {:?}", pcb.state);
    }

    // Transition to running
    pm.set_process_state(pid, ProcessState::Running)?;
    println!("Changed to Running");

    // Transition to blocked
    pm.set_process_state(pid, ProcessState::Blocked)?;
    println!("Changed to Blocked");

    // Transition back to ready
    pm.set_process_state(pid, ProcessState::Ready)?;
    println!("Changed to Ready");

    // Get final state
    if let Some(pcb) = pm.get_process(pid) {
        println!("Final state: {:?}", pcb.state);
    }

    Ok(())
}

/// Example 10: Monitoring multiple processes
pub fn example_monitor_processes() -> Result<(), &'static str> {
    let pm = get_process_manager();

    println!("Process Manager Statistics");
    println!("==========================");

    // Get total process count
    let total = pm.process_count();
    println!("Total processes: {}", total);

    // List all processes grouped by state
    let processes = pm.list_processes();

    let mut ready = 0;
    let mut running = 0;
    let mut blocked = 0;
    let mut zombie = 0;

    for (_, _, state, _) in &processes {
        match state {
            ProcessState::Ready => ready += 1,
            ProcessState::Running => running += 1,
            ProcessState::Blocked => blocked += 1,
            ProcessState::Zombie => zombie += 1,
            ProcessState::Dead => {}
        }
    }

    println!("\nProcesses by state:");
    println!("  Ready:   {}", ready);
    println!("  Running: {}", running);
    println!("  Blocked: {}", blocked);
    println!("  Zombie:  {}", zombie);

    println!("\nProcess details:");
    for (pid, name, state, priority) in processes {
        if let Some(parent_pid) = pm.get_parent_pid(pid) {
            println!("  [{}] {} (parent: {}) - {:?}/{:?}",
                pid, name, parent_pid, state, priority);
        } else {
            println!("  [{}] {} (no parent) - {:?}/{:?}",
                pid, name, state, priority);
        }
    }

    Ok(())
}

/// Complete example: Shell-like process spawning
pub fn example_shell_spawn() -> Result<(), &'static str> {
    let pm = get_process_manager();

    println!("Shell: Spawning command");

    // Simulate shell (PID 10)
    let shell_pid = pm.create_process(Some(1), "shell", Priority::Normal)?;

    // Fork to create child for command
    let cmd_pid = pm.fork(shell_pid)?;
    println!("Shell forked child: {}", cmd_pid);

    // Load program into child (simulated)
    let program = &[0x7f, b'E', b'L', b'F', 0]; // Minimal ELF header
    pm.exec(cmd_pid, program, &["ls", "-la"])?;
    println!("Loaded 'ls' into child process");

    // Simulate child execution and exit
    pm.set_process_state(cmd_pid, ProcessState::Running)?;
    pm.exit(cmd_pid, 0)?;
    println!("Child exited with status 0");

    // Shell waits for child
    let (pid, status) = pm.wait(shell_pid)?;
    println!("Shell: Command {} finished with status {}", pid, status);

    Ok(())
}
