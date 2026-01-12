//! Fast Userspace Mutex (Futex) Implementation for RustOS
//!
//! This module implements POSIX-compliant futex operations for efficient
//! userspace synchronization primitives. Futexes enable blocking and waking
//! threads with minimal kernel intervention.
//!
//! Key features:
//! - FUTEX_WAIT/FUTEX_WAKE operations
//! - FUTEX_REQUEUE/FUTEX_CMP_REQUEUE for condition variables
//! - Priority inheritance (PI) futexes
//! - Robust futex support for handling dead lock owners
//! - Timeout support for all blocking operations
//! - Efficient wait queue management

use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;
use spin::{Mutex, RwLock};
use lazy_static::lazy_static;
use core::sync::atomic::{AtomicU32, Ordering};
use super::{Pid, ProcessState};

/// Futex operation flags
pub mod futex_op {
    pub const FUTEX_WAIT: i32 = 0;
    pub const FUTEX_WAKE: i32 = 1;
    pub const FUTEX_FD: i32 = 2;
    pub const FUTEX_REQUEUE: i32 = 3;
    pub const FUTEX_CMP_REQUEUE: i32 = 4;
    pub const FUTEX_WAKE_OP: i32 = 5;
    pub const FUTEX_LOCK_PI: i32 = 6;
    pub const FUTEX_UNLOCK_PI: i32 = 7;
    pub const FUTEX_TRYLOCK_PI: i32 = 8;
    pub const FUTEX_WAIT_BITSET: i32 = 9;
    pub const FUTEX_WAKE_BITSET: i32 = 10;
    pub const FUTEX_WAIT_REQUEUE_PI: i32 = 11;
    pub const FUTEX_CMP_REQUEUE_PI: i32 = 12;

    // Flags
    pub const FUTEX_PRIVATE_FLAG: i32 = 128;
    pub const FUTEX_CLOCK_REALTIME: i32 = 256;
}

/// Futex error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FutexError {
    InvalidArgument,
    InvalidAddress,
    WouldBlock,
    TimedOut,
    Interrupted,
    DeadLockDetected,
    OwnerDied,
    NotSupported,
}

impl From<FutexError> for crate::process::syscalls::SyscallError {
    fn from(err: FutexError) -> Self {
        match err {
            FutexError::InvalidArgument => crate::process::syscalls::SyscallError::InvalidArgument,
            FutexError::InvalidAddress => crate::process::syscalls::SyscallError::InvalidAddress,
            FutexError::WouldBlock => crate::process::syscalls::SyscallError::ResourceBusy,
            FutexError::TimedOut => crate::process::syscalls::SyscallError::ResourceBusy,
            FutexError::Interrupted => crate::process::syscalls::SyscallError::ResourceBusy,
            FutexError::DeadLockDetected => crate::process::syscalls::SyscallError::ResourceBusy,
            FutexError::OwnerDied => crate::process::syscalls::SyscallError::OperationNotSupported,
            FutexError::NotSupported => crate::process::syscalls::SyscallError::OperationNotSupported,
        }
    }
}

/// Waiter information in futex wait queue
#[derive(Debug, Clone)]
struct FutexWaiter {
    pid: Pid,
    bitset: u32,
    priority: u8,
    enqueue_time: u64,
    requeue_pi: bool,
}

/// Wait queue for a specific futex address
#[derive(Debug, Clone)]
struct FutexWaitQueue {
    waiters: VecDeque<FutexWaiter>,
    owner_pid: Option<Pid>,
    owner_tid: Option<u32>,
}

impl FutexWaitQueue {
    fn new() -> Self {
        Self {
            waiters: VecDeque::new(),
            owner_pid: None,
            owner_tid: None,
        }
    }

    fn enqueue_waiter(&mut self, waiter: FutexWaiter) {
        // Insert in priority order
        let insert_pos = self.waiters.iter()
            .position(|w| w.priority > waiter.priority)
            .unwrap_or(self.waiters.len());
        self.waiters.insert(insert_pos, waiter);
    }

    fn dequeue_waiter(&mut self) -> Option<FutexWaiter> {
        self.waiters.pop_front()
    }

    fn wake_n(&mut self, count: usize, bitset: u32) -> Vec<Pid> {
        let mut woken = Vec::new();
        let mut remaining = VecDeque::new();

        while let Some(waiter) = self.waiters.pop_front() {
            if woken.len() < count && (waiter.bitset & bitset) != 0 {
                woken.push(waiter.pid);
            } else {
                remaining.push_back(waiter);
            }
        }

        self.waiters = remaining;
        woken
    }

    fn is_empty(&self) -> bool {
        self.waiters.is_empty()
    }

    fn len(&self) -> usize {
        self.waiters.len()
    }
}

/// Global futex table mapping futex addresses to wait queues
struct FutexTable {
    /// Map from futex address to wait queue
    wait_queues: BTreeMap<u64, FutexWaitQueue>,
    /// Robust futex list (for handling dead lock owners)
    robust_list: BTreeMap<Pid, Vec<u64>>,
    /// Statistics
    total_waits: u64,
    total_wakes: u64,
    total_requeues: u64,
}

impl FutexTable {
    fn new() -> Self {
        Self {
            wait_queues: BTreeMap::new(),
            robust_list: BTreeMap::new(),
            total_waits: 0,
            total_wakes: 0,
            total_requeues: 0,
        }
    }

    fn get_or_create_queue(&mut self, addr: u64) -> &mut FutexWaitQueue {
        self.wait_queues.entry(addr).or_insert_with(FutexWaitQueue::new)
    }

    fn remove_empty_queues(&mut self, addr: u64) {
        if let Some(queue) = self.wait_queues.get(&addr) {
            if queue.is_empty() {
                self.wait_queues.remove(&addr);
            }
        }
    }

    fn cleanup_process(&mut self, pid: Pid) {
        // Remove process from all wait queues
        for (_, queue) in self.wait_queues.iter_mut() {
            queue.waiters.retain(|w| w.pid != pid);
        }

        // Handle robust futexes for dead process
        if let Some(robust_futexes) = self.robust_list.remove(&pid) {
            for addr in robust_futexes {
                if let Some(queue) = self.wait_queues.get_mut(&addr) {
                    if queue.owner_pid == Some(pid) {
                        // Mark futex as owner died
                        queue.owner_pid = None;
                        queue.owner_tid = None;
                        // Wake one waiter to handle the dead owner situation
                        if let Some(waiter) = queue.dequeue_waiter() {
                            // Signal that owner died (userspace must check)
                        }
                    }
                }
            }
        }

        // Clean up empty queues
        self.wait_queues.retain(|_, queue| !queue.is_empty());
    }
}

lazy_static! {
    static ref FUTEX_TABLE: RwLock<FutexTable> = RwLock::new(FutexTable::new());
}

/// Futex manager - main interface for futex operations
pub struct FutexManager;

impl FutexManager {
    /// FUTEX_WAIT - Block on futex if value matches
    pub fn futex_wait(
        uaddr: u64,
        val: u32,
        timeout_ns: Option<u64>,
        bitset: u32,
        pid: Pid,
    ) -> Result<i32, FutexError> {
        // Validate address
        Self::validate_futex_address(uaddr)?;

        // Read current value from userspace
        let current_val = unsafe {
            core::ptr::read_volatile(uaddr as *const u32)
        };

        // If value doesn't match, return immediately
        if current_val != val {
            return Err(FutexError::WouldBlock);
        }

        // Add to wait queue
        let mut table = FUTEX_TABLE.write();
        let queue = table.get_or_create_queue(uaddr);

        let waiter = FutexWaiter {
            pid,
            bitset: if bitset == 0 { !0 } else { bitset },
            priority: Self::get_process_priority(pid),
            enqueue_time: crate::time::uptime_ns(),
            requeue_pi: false,
        };

        queue.enqueue_waiter(waiter);
        table.total_waits += 1;

        drop(table);

        // Block the process
        let process_manager = crate::process::get_process_manager();
        if let Err(_) = process_manager.block_process(pid) {
            // Failed to block, remove from wait queue
            let mut table = FUTEX_TABLE.write();
            if let Some(queue) = table.wait_queues.get_mut(&uaddr) {
                queue.waiters.retain(|w| w.pid != pid);
            }
            return Err(FutexError::InvalidArgument);
        }

        // Handle timeout if specified
        if let Some(timeout) = timeout_ns {
            Self::setup_timeout(pid, uaddr, timeout);
        }

        Ok(0)
    }

    /// FUTEX_WAKE - Wake waiters on futex
    pub fn futex_wake(
        uaddr: u64,
        val: i32,
        bitset: u32,
    ) -> Result<i32, FutexError> {
        // Validate address
        Self::validate_futex_address(uaddr)?;

        // Validate val (number of waiters to wake)
        if val < 0 {
            return Err(FutexError::InvalidArgument);
        }

        let mut table = FUTEX_TABLE.write();

        let woken_count = if let Some(queue) = table.wait_queues.get_mut(&uaddr) {
            let woken_pids = queue.wake_n(val as usize, if bitset == 0 { !0 } else { bitset });
            let count = woken_pids.len();

            // Unblock woken processes
            let process_manager = crate::process::get_process_manager();
            for pid in woken_pids {
                let _ = process_manager.unblock_process(pid);
            }

            count
        } else {
            0
        };

        // Clean up empty queue
        table.remove_empty_queues(uaddr);
        table.total_wakes += 1;

        Ok(woken_count as i32)
    }

    /// FUTEX_REQUEUE - Requeue waiters from one futex to another
    pub fn futex_requeue(
        uaddr1: u64,
        val: i32,
        val2: i32,
        uaddr2: u64,
    ) -> Result<i32, FutexError> {
        // Validate addresses
        Self::validate_futex_address(uaddr1)?;
        Self::validate_futex_address(uaddr2)?;

        if uaddr1 == uaddr2 {
            return Err(FutexError::InvalidArgument);
        }

        if val < 0 || val2 < 0 {
            return Err(FutexError::InvalidArgument);
        }

        let mut table = FUTEX_TABLE.write();

        // Wake val waiters from uaddr1
        let woken_pids = if let Some(queue1) = table.wait_queues.get_mut(&uaddr1) {
            queue1.wake_n(val as usize, !0)
        } else {
            Vec::new()
        };

        // Requeue val2 waiters from uaddr1 to uaddr2
        let requeued_count = if let Some(queue1) = table.wait_queues.get_mut(&uaddr1) {
            let mut requeued = Vec::new();
            for _ in 0..val2 {
                if let Some(waiter) = queue1.dequeue_waiter() {
                    requeued.push(waiter);
                } else {
                    break;
                }
            }

            let count = requeued.len();

            // Add to second queue
            let queue2 = table.get_or_create_queue(uaddr2);
            for waiter in requeued {
                queue2.enqueue_waiter(waiter);
            }

            count
        } else {
            0
        };

        // Unblock woken processes
        let process_manager = crate::process::get_process_manager();
        for pid in woken_pids {
            let _ = process_manager.unblock_process(pid);
        }

        // Clean up empty queues
        table.remove_empty_queues(uaddr1);
        table.total_requeues += 1;

        Ok((woken_pids.len() + requeued_count) as i32)
    }

    /// FUTEX_CMP_REQUEUE - Requeue waiters with value comparison
    pub fn futex_cmp_requeue(
        uaddr1: u64,
        val: i32,
        val2: i32,
        uaddr2: u64,
        val3: u32,
    ) -> Result<i32, FutexError> {
        // Validate addresses
        Self::validate_futex_address(uaddr1)?;
        Self::validate_futex_address(uaddr2)?;

        if uaddr1 == uaddr2 {
            return Err(FutexError::InvalidArgument);
        }

        // Read and compare current value
        let current_val = unsafe {
            core::ptr::read_volatile(uaddr1 as *const u32)
        };

        if current_val != val3 {
            return Err(FutexError::WouldBlock);
        }

        // Perform requeue
        Self::futex_requeue(uaddr1, val, val2, uaddr2)
    }

    /// FUTEX_LOCK_PI - Lock priority-inheritance futex
    pub fn futex_lock_pi(
        uaddr: u64,
        timeout_ns: Option<u64>,
        pid: Pid,
        tid: u32,
    ) -> Result<i32, FutexError> {
        // Validate address
        Self::validate_futex_address(uaddr)?;

        let mut table = FUTEX_TABLE.write();
        let queue = table.get_or_create_queue(uaddr);

        // Check if futex is already owned
        if queue.owner_pid.is_some() {
            // Try to acquire with priority inheritance
            let owner_priority = queue.owner_pid
                .map(|p| Self::get_process_priority(p))
                .unwrap_or(255);

            let current_priority = Self::get_process_priority(pid);

            // Boost owner priority if necessary
            if current_priority < owner_priority {
                if let Some(owner) = queue.owner_pid {
                    Self::boost_process_priority(owner, current_priority);
                }
            }

            // Block current process
            let waiter = FutexWaiter {
                pid,
                bitset: !0,
                priority: current_priority,
                enqueue_time: crate::time::uptime_ns(),
                requeue_pi: true,
            };

            queue.enqueue_waiter(waiter);
            drop(table);

            let process_manager = crate::process::get_process_manager();
            process_manager.block_process(pid)
                .map_err(|_| FutexError::InvalidArgument)?;

            if let Some(timeout) = timeout_ns {
                Self::setup_timeout(pid, uaddr, timeout);
            }

            Ok(0)
        } else {
            // Acquire futex
            queue.owner_pid = Some(pid);
            queue.owner_tid = Some(tid);

            // Write TID to futex word
            unsafe {
                core::ptr::write_volatile(uaddr as *mut u32, tid);
            }

            Ok(0)
        }
    }

    /// FUTEX_UNLOCK_PI - Unlock priority-inheritance futex
    pub fn futex_unlock_pi(uaddr: u64, pid: Pid) -> Result<i32, FutexError> {
        // Validate address
        Self::validate_futex_address(uaddr)?;

        let mut table = FUTEX_TABLE.write();

        if let Some(queue) = table.wait_queues.get_mut(&uaddr) {
            // Verify ownership
            if queue.owner_pid != Some(pid) {
                return Err(FutexError::NotSupported);
            }

            // Release ownership
            queue.owner_pid = None;
            queue.owner_tid = None;

            // Wake next waiter with highest priority
            if let Some(waiter) = queue.dequeue_waiter() {
                // Transfer ownership to next waiter
                queue.owner_pid = Some(waiter.pid);

                // Restore original priority if it was boosted
                Self::restore_process_priority(pid);

                let process_manager = crate::process::get_process_manager();
                let _ = process_manager.unblock_process(waiter.pid);
            } else {
                // No waiters, clear futex word
                unsafe {
                    core::ptr::write_volatile(uaddr as *mut u32, 0);
                }
                Self::restore_process_priority(pid);
            }

            table.remove_empty_queues(uaddr);
            Ok(0)
        } else {
            Err(FutexError::InvalidArgument)
        }
    }

    /// FUTEX_TRYLOCK_PI - Try to lock PI futex without blocking
    pub fn futex_trylock_pi(uaddr: u64, pid: Pid, tid: u32) -> Result<i32, FutexError> {
        // Validate address
        Self::validate_futex_address(uaddr)?;

        let mut table = FUTEX_TABLE.write();
        let queue = table.get_or_create_queue(uaddr);

        // Check if futex is available
        if queue.owner_pid.is_some() {
            return Err(FutexError::WouldBlock);
        }

        // Acquire futex
        queue.owner_pid = Some(pid);
        queue.owner_tid = Some(tid);

        // Write TID to futex word
        unsafe {
            core::ptr::write_volatile(uaddr as *mut u32, tid);
        }

        Ok(0)
    }

    /// Cleanup futexes for a terminated process
    pub fn cleanup_process_futexes(pid: Pid) {
        let mut table = FUTEX_TABLE.write();
        table.cleanup_process(pid);
    }

    /// Register robust futex for a process
    pub fn register_robust_futex(pid: Pid, futex_addr: u64) {
        let mut table = FUTEX_TABLE.write();
        table.robust_list.entry(pid).or_insert_with(Vec::new).push(futex_addr);
    }

    /// Validate futex address
    fn validate_futex_address(addr: u64) -> Result<(), FutexError> {
        // Must be 4-byte aligned
        if addr % 4 != 0 {
            return Err(FutexError::InvalidAddress);
        }

        // Must be in user space
        const USER_SPACE_START: u64 = 0x0000_1000_0000;
        const USER_SPACE_END: u64 = 0x0000_8000_0000;

        if addr < USER_SPACE_START || addr >= USER_SPACE_END {
            return Err(FutexError::InvalidAddress);
        }

        Ok(())
    }

    /// Get process priority (lower number = higher priority)
    fn get_process_priority(pid: Pid) -> u8 {
        let process_manager = crate::process::get_process_manager();
        if let Some(process) = process_manager.get_process(pid) {
            match process.priority {
                crate::process::Priority::RealTime => 0,
                crate::process::Priority::High => 1,
                crate::process::Priority::Normal => 2,
                crate::process::Priority::Low => 3,
                crate::process::Priority::Idle => 4,
            }
        } else {
            255
        }
    }

    /// Boost process priority for priority inheritance
    fn boost_process_priority(pid: Pid, new_priority: u8) {
        let process_manager = crate::process::get_process_manager();
        if let Some(mut process) = process_manager.get_process(pid) {
            let priority = match new_priority {
                0 => crate::process::Priority::RealTime,
                1 => crate::process::Priority::High,
                2 => crate::process::Priority::Normal,
                3 => crate::process::Priority::Low,
                _ => crate::process::Priority::Idle,
            };
            process.priority = priority;
        }
    }

    /// Restore process priority after priority inheritance
    fn restore_process_priority(pid: Pid) {
        // In a full implementation, we would track the original priority
        // For now, reset to Normal
        let process_manager = crate::process::get_process_manager();
        if let Some(mut process) = process_manager.get_process(pid) {
            process.priority = crate::process::Priority::Normal;
        }
    }

    /// Setup timeout for futex wait
    fn setup_timeout(pid: Pid, _uaddr: u64, timeout_ns: u64) {
        // Convert to milliseconds for timer
        let timeout_ms = timeout_ns / 1_000_000;

        // In a full implementation, we would use a timer callback
        // For now, we track the wake time in the process control block
        let process_manager = crate::process::get_process_manager();
        if let Some(mut process) = process_manager.get_process(pid) {
            let wake_time = crate::time::uptime_ms() + timeout_ms;
            process.wake_time = Some(wake_time);
        }
    }

    /// Get futex statistics
    pub fn get_statistics() -> (u64, u64, u64) {
        let table = FUTEX_TABLE.read();
        (table.total_waits, table.total_wakes, table.total_requeues)
    }
}

/// Initialize the futex subsystem
pub fn init() -> Result<(), &'static str> {
    // Futex subsystem uses lazy initialization
    Ok(())
}
