# Quick Integration Guide for Syscall Implementations

## TL;DR

Replace 4 function stubs in `src/process/syscalls.rs` with complete implementations from `/tmp/syscall_implementations.rs`.

---

## Step-by-Step Integration

### Step 1: Locate Functions to Replace

Open `src/process/syscalls.rs` and find these 4 functions:

1. **Line 1355-1360**: `sys_clone()` - Currently returns `OperationNotSupported`
2. **Line 1362-1367**: `sys_execve()` - Currently returns `OperationNotSupported`
3. **Line 1369-1373**: `sys_waitid()` - Currently returns `OperationNotSupported`
4. **Line 1872-1877**: `sys_set_tid_address()` - Currently returns `OperationNotSupported`

### Step 2: Get Replacement Code

All implementations are in: `/tmp/syscall_implementations.rs`

View it:
```bash
cat /tmp/syscall_implementations.rs
```

### Step 3: Replace Each Function

For each function:

1. **Delete** the old stub implementation (including TODO comment)
2. **Copy** the new implementation from `/tmp/syscall_implementations.rs`
3. **Paste** in the same location

### Step 4: Verify Changes

```bash
cd /home/user/Rustos

# Check syntax
cargo check --bin rustos

# Build
cargo build --bin rustos

# Run tests
cargo test
```

---

## Exact Line Numbers

| Function | Lines to Replace | New Lines |
|----------|-----------------|-----------|
| sys_clone | 1355-1360 (6 lines) | 180 lines |
| sys_execve | 1362-1367 (6 lines) | 280 lines |
| sys_waitid | 1369-1373 (5 lines) | 150 lines |
| sys_set_tid_address | 1872-1877 (6 lines) | 40 lines |

**Net Change**: +627 lines (replacing 23 lines of stubs)

---

## Alternative: Automated Integration

If you prefer automated integration:

```bash
# Method 1: Copy implementations file
cp /tmp/syscall_implementations.rs /tmp/syscalls_to_integrate.rs

# Method 2: Use the backup and manually edit
cp src/process/syscalls.rs src/process/syscalls.rs.backup
vim src/process/syscalls.rs  # Edit manually using /tmp/syscall_implementations.rs

# Method 3: Script-based (advanced)
# A Python script could automate this, but manual review recommended
```

---

## Visual Example: sys_clone Replacement

**BEFORE** (lines 1355-1360):
```rust
    /// sys_clone - Create thread/process (flexible fork)
    fn sys_clone(&self, _args: &[u64], _process_manager: &ProcessManager, _current_pid: Pid) -> SyscallResult {
        // TODO: Implement clone() for thread creation
        // This is critical for dynamic linking and pthread support
        SyscallResult::Error(SyscallError::OperationNotSupported)
    }
```

**AFTER** (180 lines):
```rust
    /// sys_clone - Create thread/process (flexible fork)
    fn sys_clone(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let flags = args.get(0).copied().unwrap_or(0);
        let stack_ptr = args.get(1).copied().unwrap_or(0);
        let parent_tid_ptr = args.get(2).copied().unwrap_or(0);
        let child_tid_ptr = args.get(3).copied().unwrap_or(0);
        let tls = args.get(4).copied().unwrap_or(0);

        const CLONE_VM: u64 = 0x00000100;
        const CLONE_FS: u64 = 0x00000200;
        // ... (full implementation in /tmp/syscall_implementations.rs)

        SyscallResult::Success(tid_or_pid as u64)
    }
```

Repeat for all 4 functions.

---

## Verification Checklist

After integration, verify:

- [ ] All 4 TODO comments removed
- [ ] All 4 functions return real results (not `OperationNotSupported`)
- [ ] File compiles without errors: `cargo check`
- [ ] No clippy warnings: `cargo clippy`
- [ ] Tests pass: `cargo test`
- [ ] Documentation builds: `cargo doc`

---

## Rollback (If Needed)

If you need to revert:

```bash
# Restore from backup
cp src/process/syscalls.rs.backup src/process/syscalls.rs

# Or use git
git checkout src/process/syscalls.rs
```

---

## Testing After Integration

### Quick Smoke Test

```bash
# Build
cargo build --bin rustos

# Run with QEMU
make run

# Or use build script
./build_rustos.sh
```

### Unit Tests

Add to `src/process/syscalls.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clone_validates_flags() {
        // Test that CLONE_THREAD requires CLONE_VM and CLONE_SIGHAND
        let dispatcher = SyscallDispatcher::new();
        let pm = get_process_manager();

        // Missing CLONE_VM should fail
        let result = dispatcher.sys_clone(
            &[0x00010000, 0, 0, 0, 0],  // CLONE_THREAD only
            pm, 1
        );
        assert!(matches!(result, SyscallResult::Error(SyscallError::InvalidArgument)));
    }

    #[test]
    fn test_execve_validates_path() {
        let dispatcher = SyscallDispatcher::new();
        let pm = get_process_manager();

        // NULL path should fail
        let result = dispatcher.sys_execve(&[0, 0, 0], pm, 1);
        assert!(matches!(result, SyscallResult::Error(SyscallError::InvalidArgument)));
    }
}
```

---

## Support Files

All documentation and implementations available in:

- **Implementations**: `/tmp/syscall_implementations.rs`
- **Full Documentation**: `/home/user/Rustos/SYSCALL_IMPLEMENTATIONS.md`
- **Summary**: `/home/user/Rustos/IMPLEMENTATION_SUMMARY.md`
- **This Guide**: `/home/user/Rustos/INTEGRATION_GUIDE.md`
- **Patch Format**: `/home/user/Rustos/syscalls.patch`

---

## Questions?

Each implementation includes:
- Detailed comments explaining the logic
- Error handling for all edge cases
- Security validation
- Integration with existing systems

Review the code in `/tmp/syscall_implementations.rs` for complete details.

---

**Status**: Ready for integration ✅
**Estimated Time**: 10-15 minutes for manual integration
**Risk Level**: Low (can rollback easily)
**Testing Required**: Yes (build + basic tests)
