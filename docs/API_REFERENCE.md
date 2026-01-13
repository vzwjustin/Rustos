# RustOS API Reference

## Table of Contents
1. [Process Management APIs](#process-management-apis)
2. [Memory Management APIs](#memory-management-apis)
3. [Network Stack APIs](#network-stack-apis)
4. [File System APIs](#file-system-apis)
5. [Device Driver APIs](#device-driver-apis)
6. [GPU/Graphics APIs](#gpugraphics-apis)
7. [System Call APIs](#system-call-apis)
8. [Synchronization APIs](#synchronization-apis)

---

## Process Management APIs

### Core Process Functions (`src/process/mod.rs`)

#### `create_process`
```rust
pub fn create_process(
    name: &str,
    entry_point: VirtAddr,
    priority: u8
) -> Result<Pid, ProcessError>
```
Creates a new process with the specified name and entry point.
- **Parameters**:
  - `name`: Process name for identification
  - `entry_point`: Virtual address of process entry function
  - `priority`: Process priority (0-255, higher = more priority)
- **Returns**: Process ID on success, error on failure
- **Example**: `src/process/mod.rs:234`

#### `terminate_process`
```rust
pub fn terminate_process(pid: Pid) -> Result<(), ProcessError>
```
Terminates a process and cleans up its resources.
- **Parameters**: `pid` - Process ID to terminate
- **Returns**: Ok(()) on success
- **Location**: `src/process/mod.rs:289`

#### `get_current_process`
```rust
pub fn get_current_process() -> Option<Arc<RwLock<ProcessControlBlock>>>
```
Gets the currently running process on this CPU core.
- **Returns**: Current process PCB or None
- **Thread-safe**: Yes (returns Arc<RwLock>)
- **Location**: `src/process/mod.rs:156`

### Process State Management

#### `ProcessState` Enum
```rust
pub enum ProcessState {
    Ready,      // Ready to run
    Running,    // Currently executing
    Blocked,    // Waiting for I/O or resource
    Zombie,     // Terminated, awaiting cleanup
    Dead,       // Fully cleaned up
}
```
Location: `src/process/mod.rs:28`

#### `set_process_state`
```rust
pub fn set_process_state(pid: Pid, state: ProcessState) -> Result<(), ProcessError>
```
Changes the state of a process.
- **Location**: `src/process/mod.rs:312`

### Thread Management (`src/process/thread.rs`)

#### `create_thread`
```rust
pub fn create_thread(
    process: Pid,
    entry: VirtAddr,
    stack_size: usize
) -> Result<ThreadId, ThreadError>
```
Creates a new thread within a process.
- **Parameters**:
  - `process`: Parent process ID
  - `entry`: Thread entry point
  - `stack_size`: Stack size in bytes
- **Location**: `src/process/thread.rs:45`

---

## Memory Management APIs

### Heap Allocation (`src/memory.rs`)

#### `allocate_kernel_heap`
```rust
pub fn allocate_kernel_heap(size: usize) -> Result<*mut u8, MemoryError>
```
Allocates memory from the kernel heap.
- **Parameters**: `size` - Number of bytes to allocate
- **Returns**: Pointer to allocated memory
- **Thread-safe**: Yes (uses locked allocator)
- **Location**: `src/memory.rs:178`

#### `deallocate_kernel_heap`
```rust
pub unsafe fn deallocate_kernel_heap(ptr: *mut u8, size: usize)
```
Frees previously allocated kernel heap memory.
- **Safety**: Caller must ensure pointer validity
- **Location**: `src/memory.rs:195`

### Physical Memory Management

#### `allocate_frame`
```rust
pub fn allocate_frame() -> Option<PhysFrame>
```
Allocates a physical memory frame (4KB).
- **Returns**: Physical frame or None if out of memory
- **Location**: `src/memory.rs:412`

#### `free_frame`
```rust
pub fn free_frame(frame: PhysFrame)
```
Releases a physical memory frame.
- **Location**: `src/memory.rs:428`

### Memory Statistics

#### `get_memory_stats`
```rust
pub fn get_memory_stats() -> MemoryStats {
    total_memory: usize,
    used_memory: usize,
    free_memory: usize,
    heap_usage: usize,
    frame_usage: usize,
}
```
Returns current memory usage statistics.
- **Location**: `src/memory.rs:567`

---

## Network Stack APIs

### Socket Interface (`src/net/socket.rs`)

#### `create_socket`
```rust
pub fn create_socket(
    domain: AddressFamily,
    socket_type: SocketType,
    protocol: Protocol
) -> Result<SocketHandle, SocketError>
```
Creates a new network socket.
- **Parameters**:
  - `domain`: AF_INET or AF_INET6
  - `socket_type`: SOCK_STREAM (TCP) or SOCK_DGRAM (UDP)
  - `protocol`: Protocol number
- **Returns**: Socket handle for further operations
- **Location**: `src/net/socket.rs:89`

#### `bind`
```rust
pub fn bind(
    socket: SocketHandle,
    address: SocketAddress
) -> Result<(), SocketError>
```
Binds a socket to a local address.
- **Location**: `src/net/socket.rs:134`

#### `connect`
```rust
pub fn connect(
    socket: SocketHandle,
    address: SocketAddress
) -> Result<(), SocketError>
```
Connects a socket to a remote address.
- **Location**: `src/net/socket.rs:156`

#### `send`
```rust
pub fn send(
    socket: SocketHandle,
    data: &[u8],
    flags: SendFlags
) -> Result<usize, SocketError>
```
Sends data through a connected socket.
- **Returns**: Number of bytes sent
- **Location**: `src/net/socket.rs:189`

#### `receive`
```rust
pub fn receive(
    socket: SocketHandle,
    buffer: &mut [u8],
    flags: RecvFlags
) -> Result<usize, SocketError>
```
Receives data from a socket.
- **Returns**: Number of bytes received
- **Location**: `src/net/socket.rs:212`

### TCP Protocol (`src/net/tcp.rs`)

#### `TcpConnection`
```rust
pub struct TcpConnection {
    pub state: TcpState,
    pub local_addr: SocketAddress,
    pub remote_addr: SocketAddress,
    pub send_buffer: CircularBuffer,
    pub recv_buffer: CircularBuffer,
    // ...
}
```
Represents a TCP connection with full state machine.
- **Location**: `src/net/tcp.rs:45`

#### TCP States
```rust
pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}
```
Location: `src/net/tcp.rs:23`

### Network Device Management (`src/net/device.rs`)

#### `register_network_device`
```rust
pub fn register_network_device(
    device: Box<dyn NetworkDevice>
) -> Result<DeviceId, NetworkError>
```
Registers a network interface with the stack.
- **Location**: `src/net/device.rs:78`

#### `NetworkDevice` Trait
```rust
pub trait NetworkDevice: Send + Sync {
    fn transmit(&mut self, packet: &[u8]) -> Result<(), NetworkError>;
    fn receive(&mut self) -> Option<Vec<u8>>;
    fn get_mac_address(&self) -> MacAddress;
    fn get_mtu(&self) -> usize;
}
```
Interface that network drivers must implement.
- **Location**: `src/net/device.rs:34`

---

## File System APIs

### Virtual File System (`src/fs/vfs.rs`)

#### `open`
```rust
pub fn open(
    path: &str,
    flags: OpenFlags
) -> Result<FileDescriptor, FsError>
```
Opens a file and returns a file descriptor.
- **Parameters**:
  - `path`: File path
  - `flags`: O_RDONLY, O_WRONLY, O_RDWR, O_CREAT, etc.
- **Location**: `src/fs/vfs.rs:89`

#### `read`
```rust
pub fn read(
    fd: FileDescriptor,
    buffer: &mut [u8]
) -> Result<usize, FsError>
```
Reads data from an open file.
- **Returns**: Number of bytes read
- **Location**: `src/fs/vfs.rs:123`

#### `write`
```rust
pub fn write(
    fd: FileDescriptor,
    data: &[u8]
) -> Result<usize, FsError>
```
Writes data to an open file.
- **Returns**: Number of bytes written
- **Location**: `src/fs/vfs.rs:145`

#### `close`
```rust
pub fn close(fd: FileDescriptor) -> Result<(), FsError>
```
Closes an open file descriptor.
- **Location**: `src/fs/vfs.rs:167`

### File System Mounting

#### `mount`
```rust
pub fn mount(
    fs_type: &str,
    device: Option<&str>,
    mount_point: &str,
    options: MountOptions
) -> Result<(), FsError>
```
Mounts a file system at the specified mount point.
- **Location**: `src/fs/vfs.rs:234`

---

## Device Driver APIs

### PCI Device Management (`src/pci/mod.rs`)

#### `scan_pci_bus`
```rust
pub fn scan_pci_bus() -> Vec<PciDevice>
```
Scans the PCI bus and returns all detected devices.
- **Location**: `src/pci/detection.rs:45`

#### `PciDevice`
```rust
pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_code: u8,
    pub subclass: u8,
    pub prog_if: u8,
}
```
Represents a PCI device on the bus.
- **Location**: `src/pci/mod.rs:23`

### Driver Framework (`src/drivers/mod.rs`)

#### `DriverOps` Trait
```rust
pub trait DriverOps: Send + Sync {
    fn probe(&mut self, device: &PciDevice) -> Result<(), DriverError>;
    fn init(&mut self) -> Result<(), DriverError>;
    fn cleanup(&mut self);
    fn interrupt_handler(&mut self, irq: u8);
}
```
Interface that all device drivers must implement.
- **Location**: `src/drivers/mod.rs:67`

#### `register_driver`
```rust
pub fn register_driver(
    driver: Box<dyn DriverOps>,
    device_ids: &[(u16, u16)]  // (vendor_id, device_id) pairs
) -> Result<DriverHandle, DriverError>
```
Registers a device driver with the kernel.
- **Location**: `src/drivers/mod.rs:134`

### Hot-Plug Support (`src/drivers/hotplug.rs`)

#### `HotplugEvent`
```rust
pub enum HotplugEvent {
    DeviceAdded(HotplugDevice),
    DeviceRemoved(HotplugDevice),
    DeviceChanged(HotplugDevice),
}
```
Hot-plug event types.
- **Location**: `src/drivers/hotplug.rs:34`

---

## GPU/Graphics APIs

### GPU Device Management (`src/gpu/mod.rs`)

#### `detect_gpu`
```rust
pub fn detect_gpu() -> Vec<GpuDevice>
```
Detects all GPU devices in the system.
- **Returns**: List of detected GPUs with capabilities
- **Location**: `src/gpu/mod.rs:234`

#### `GpuDevice`
```rust
pub struct GpuDevice {
    pub vendor: GpuVendor,
    pub device_id: u16,
    pub name: String,
    pub tier: GPUTier,
    pub features: GPUFeatures,
    pub vram_size: usize,
}
```
Represents a GPU device.
- **Location**: `src/gpu/mod.rs:89`

### Graphics Acceleration (`src/gpu/accel.rs`)

#### `create_framebuffer`
```rust
pub fn create_framebuffer(
    width: u32,
    height: u32,
    format: PixelFormat
) -> Result<FramebufferId, GraphicsError>
```
Creates a GPU-backed framebuffer.
- **Location**: `src/gpu/accel.rs:67`

#### `draw_rectangle`
```rust
pub fn draw_rectangle(
    fb: FramebufferId,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    color: Color
) -> Result<(), GraphicsError>
```
Hardware-accelerated rectangle drawing.
- **Location**: `src/gpu/accel.rs:123`

### Shader Support

#### `compile_shader`
```rust
pub fn compile_shader(
    source: &str,
    shader_type: ShaderType
) -> Result<ShaderId, ShaderError>
```
Compiles a GPU shader.
- **Parameters**:
  - `source`: Shader source code
  - `shader_type`: Vertex, Fragment, or Compute
- **Location**: `src/gpu/accel.rs:234`

---

## System Call APIs

### System Call Interface (`src/syscall/mod.rs`)

#### System Call Numbers
```rust
pub const SYS_EXIT: usize = 1;
pub const SYS_FORK: usize = 2;
pub const SYS_READ: usize = 3;
pub const SYS_WRITE: usize = 4;
pub const SYS_OPEN: usize = 5;
pub const SYS_CLOSE: usize = 6;
pub const SYS_WAIT: usize = 7;
pub const SYS_EXEC: usize = 11;
pub const SYS_GETPID: usize = 20;
pub const SYS_MOUNT: usize = 21;
pub const SYS_UMOUNT: usize = 22;
// ... more syscalls
```
Location: `src/syscall/mod.rs:12`

#### `syscall_handler`
```rust
pub fn syscall_handler(
    syscall_num: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize
) -> isize
```
Main system call dispatcher.
- **Called from**: Interrupt handler for INT 0x80
- **Location**: `src/syscall/mod.rs:89`

---

## Synchronization APIs

### Mutex (`src/process/sync.rs`)

#### `Mutex<T>`
```rust
pub struct Mutex<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    pub fn new(data: T) -> Self;
    pub fn lock(&self) -> MutexGuard<T>;
    pub fn try_lock(&self) -> Option<MutexGuard<T>>;
}
```
Kernel mutex for mutual exclusion.
- **Location**: `src/process/sync.rs:34`

### Semaphore

#### `Semaphore`
```rust
pub struct Semaphore {
    count: AtomicUsize,
    waiters: VecDeque<Pid>,
}

impl Semaphore {
    pub fn new(initial: usize) -> Self;
    pub fn wait(&mut self);
    pub fn signal(&mut self);
}
```
Counting semaphore for resource management.
- **Location**: `src/process/sync.rs:89`

### Read-Write Lock

#### `RwLock<T>`
```rust
pub struct RwLock<T> {
    readers: AtomicUsize,
    writer: AtomicBool,
    data: UnsafeCell<T>,
}

impl<T> RwLock<T> {
    pub fn read(&self) -> RwLockReadGuard<T>;
    pub fn write(&self) -> RwLockWriteGuard<T>;
}
```
Allows multiple readers or single writer.
- **Location**: `src/process/sync.rs:145`

---

## Inter-Process Communication APIs

### Message Queue (`src/process/ipc.rs`)

#### `create_message_queue`
```rust
pub fn create_message_queue(
    name: &str,
    max_messages: usize
) -> Result<MessageQueueId, IpcError>
```
Creates a new message queue.
- **Location**: `src/process/ipc.rs:56`

#### `send_message`
```rust
pub fn send_message(
    queue: MessageQueueId,
    message: &[u8],
    priority: u8
) -> Result<(), IpcError>
```
Sends a message to a queue.
- **Location**: `src/process/ipc.rs:89`

### Shared Memory

#### `create_shared_memory`
```rust
pub fn create_shared_memory(
    name: &str,
    size: usize
) -> Result<SharedMemoryId, IpcError>
```
Creates a shared memory segment.
- **Location**: `src/process/ipc.rs:134`

#### `attach_shared_memory`
```rust
pub fn attach_shared_memory(
    shmid: SharedMemoryId
) -> Result<*mut u8, IpcError>
```
Attaches shared memory to process address space.
- **Location**: `src/process/ipc.rs:156`

---

## Error Types

### Common Error Types

```rust
pub enum ProcessError {
    NotFound,
    AlreadyExists,
    InvalidState,
    PermissionDenied,
    OutOfMemory,
}

pub enum MemoryError {
    OutOfMemory,
    InvalidAddress,
    AlignmentError,
    PermissionDenied,
}

pub enum NetworkError {
    ConnectionRefused,
    ConnectionReset,
    Timeout,
    InvalidAddress,
    BufferFull,
}

pub enum FsError {
    FileNotFound,
    PermissionDenied,
    IsDirectory,
    NotDirectory,
    DiskFull,
}
```

---

## Usage Examples

### Creating a Process
```rust
use rustos::process::{create_process, wait_for_process};

let pid = create_process(
    "worker",
    VirtAddr::new(0x100000),
    128  // Normal priority
)?;

wait_for_process(pid)?;
```

### Network Socket
```rust
use rustos::net::{create_socket, bind, listen, accept};

let socket = create_socket(AF_INET, SOCK_STREAM, 0)?;
bind(socket, SocketAddress::new("0.0.0.0", 8080))?;
listen(socket, 10)?;

loop {
    let client = accept(socket)?;
    // Handle client connection
}
```

### File Operations
```rust
use rustos::fs::{open, read, write, close};

let fd = open("/data/file.txt", O_RDWR | O_CREAT)?;
write(fd, b"Hello, RustOS!")?;

let mut buffer = [0u8; 1024];
let bytes_read = read(fd, &mut buffer)?;
close(fd)?;
```

---

## API Stability

### Stable APIs
APIs marked as stable will maintain backward compatibility.

### Unstable APIs
APIs in active development may change between versions.

### Deprecated APIs
Deprecated APIs will be maintained for at least 2 major versions.

---

## Further Documentation

- Architecture Overview: `docs/ARCHITECTURE.md`
- Module Reference: `docs/MODULE_INDEX.md`
- Driver Development: `docs/DRIVER_GUIDE.md`
- Build Instructions: `docs/BUILD_GUIDE.md`