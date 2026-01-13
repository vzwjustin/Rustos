# RustOS Driver Development Guide

## Table of Contents
1. [Driver Framework Overview](#driver-framework-overview)
2. [Driver Types and Categories](#driver-types-and-categories)
3. [Creating a New Driver](#creating-a-new-driver)
4. [Network Driver Development](#network-driver-development)
5. [Storage Driver Development](#storage-driver-development)
6. [GPU Driver Development](#gpu-driver-development)
7. [Driver Testing and Debugging](#driver-testing-and-debugging)
8. [Hardware Database Integration](#hardware-database-integration)
9. [Best Practices](#best-practices)

---

## Driver Framework Overview

### Architecture

RustOS uses a unified driver framework that provides:
- Automatic device detection and driver loading
- Hot-plug support for dynamic device insertion/removal
- Standardized driver interface for all device types
- Resource management and conflict resolution
- Performance monitoring and error reporting

### Core Driver Interface

All drivers must implement the `DriverOps` trait:

```rust
pub trait DriverOps: Send + Sync {
    /// Probe if this driver can handle the given device
    fn probe(&mut self, device: &PciDevice) -> Result<(), DriverError>;

    /// Initialize the driver and device
    fn init(&mut self) -> Result<(), DriverError>;

    /// Clean up resources when driver is unloaded
    fn cleanup(&mut self);

    /// Handle power management suspend
    fn suspend(&mut self) -> Result<(), DriverError>;

    /// Handle power management resume
    fn resume(&mut self) -> Result<(), DriverError>;

    /// Handle hardware interrupts
    fn interrupt_handler(&mut self, irq: u8);

    /// Get driver information
    fn get_info(&self) -> DriverInfo {
        DriverInfo {
            name: self.name(),
            version: self.version(),
            author: self.author(),
            supported_devices: self.supported_devices(),
        }
    }
}
```

### Driver Registration

```rust
pub fn register_driver(
    driver: Box<dyn DriverOps>,
    supported_devices: &[(u16, u16)]  // (vendor_id, device_id) pairs
) -> Result<DriverHandle, DriverError> {
    let driver_id = allocate_driver_id();

    // Register device ID mappings
    for &(vendor_id, device_id) in supported_devices {
        DRIVER_DATABASE.lock().insert((vendor_id, device_id), driver_id);
    }

    // Store driver instance
    DRIVERS.lock().insert(driver_id, driver);

    Ok(DriverHandle { id: driver_id })
}
```

---

## Driver Types and Categories

### Network Drivers

Implement the `NetworkDevice` trait for network interface cards:

```rust
pub trait NetworkDevice: Send + Sync {
    fn transmit(&mut self, packet: &[u8]) -> Result<(), NetworkError>;
    fn receive(&mut self) -> Option<Vec<u8>>;
    fn get_mac_address(&self) -> MacAddress;
    fn get_mtu(&self) -> usize;
    fn get_stats(&self) -> NetworkStats;
    fn set_promiscuous(&mut self, enabled: bool);
    fn add_multicast(&mut self, addr: MacAddress);
    fn remove_multicast(&mut self, addr: MacAddress);
}
```

### Storage Drivers

Implement the `BlockDevice` trait for storage devices:

```rust
pub trait BlockDevice: Send + Sync {
    fn read_blocks(&mut self, start_block: u64,
                   blocks: &mut [Block]) -> Result<(), StorageError>;
    fn write_blocks(&mut self, start_block: u64,
                    blocks: &[Block]) -> Result<(), StorageError>;
    fn get_block_size(&self) -> usize;
    fn get_block_count(&self) -> u64;
    fn flush(&mut self) -> Result<(), StorageError>;
    fn get_device_info(&self) -> StorageDeviceInfo;
}
```

### GPU Drivers

Implement GPU-specific interfaces:

```rust
pub trait GpuDevice: Send + Sync {
    fn initialize_display(&mut self) -> Result<(), GpuError>;
    fn set_mode(&mut self, mode: DisplayMode) -> Result<(), GpuError>;
    fn allocate_framebuffer(&mut self, size: usize) -> Result<FramebufferId, GpuError>;
    fn create_surface(&mut self, width: u32, height: u32) -> Result<SurfaceId, GpuError>;
    fn submit_commands(&mut self, commands: &[GpuCommand]) -> Result<(), GpuError>;
}
```

---

## Creating a New Driver

### 1. Driver Structure Template

```rust
use crate::drivers::{DriverOps, DriverError, DriverInfo};
use crate::pci::PciDevice;
use alloc::string::String;
use spin::Mutex;

pub struct MyDriver {
    device: Option<PciDevice>,
    mmio_base: PhysAddr,
    irq: Option<u8>,
    initialized: bool,
    // Driver-specific fields
}

impl MyDriver {
    pub fn new() -> Self {
        MyDriver {
            device: None,
            mmio_base: PhysAddr::zero(),
            irq: None,
            initialized: false,
        }
    }

    // Helper methods
    fn read_register(&self, offset: u32) -> u32 {
        unsafe {
            let addr = self.mmio_base + offset;
            core::ptr::read_volatile(addr.as_ptr::<u32>())
        }
    }

    fn write_register(&self, offset: u32, value: u32) {
        unsafe {
            let addr = self.mmio_base + offset;
            core::ptr::write_volatile(addr.as_mut_ptr::<u32>(), value);
        }
    }
}

impl DriverOps for MyDriver {
    fn probe(&mut self, device: &PciDevice) -> Result<(), DriverError> {
        // Check if this driver supports the device
        match (device.vendor_id, device.device_id) {
            (0x1234, 0x5678) => {
                self.device = Some(*device);
                Ok(())
            }
            _ => Err(DriverError::UnsupportedDevice),
        }
    }

    fn init(&mut self) -> Result<(), DriverError> {
        let device = self.device.ok_or(DriverError::DeviceNotFound)?;

        // Map device memory
        self.mmio_base = map_device_memory(
            device.get_bar(0)?,
            device.get_bar_size(0)?
        )?;

        // Setup interrupt handler
        if let Some(irq) = device.get_irq() {
            register_interrupt_handler(irq, my_driver_interrupt_handler)?;
            self.irq = Some(irq);
        }

        // Initialize device hardware
        self.reset_device()?;
        self.configure_device()?;
        self.enable_device()?;

        self.initialized = true;
        Ok(())
    }

    fn cleanup(&mut self) {
        if self.initialized {
            self.disable_device();

            if let Some(irq) = self.irq {
                unregister_interrupt_handler(irq);
            }

            if !self.mmio_base.is_zero() {
                unmap_device_memory(self.mmio_base);
            }
        }
    }

    fn interrupt_handler(&mut self, irq: u8) {
        // Read interrupt status
        let status = self.read_register(INTERRUPT_STATUS_REG);

        // Handle different interrupt types
        if status & INT_RECEIVE != 0 {
            self.handle_receive_interrupt();
        }

        if status & INT_TRANSMIT != 0 {
            self.handle_transmit_interrupt();
        }

        if status & INT_ERROR != 0 {
            self.handle_error_interrupt();
        }

        // Clear interrupt status
        self.write_register(INTERRUPT_STATUS_REG, status);
    }

    fn suspend(&mut self) -> Result<(), DriverError> {
        // Save device state
        self.save_device_state()?;

        // Put device in low power mode
        self.set_power_state(PowerState::D3)?;

        Ok(())
    }

    fn resume(&mut self) -> Result<(), DriverError> {
        // Restore full power
        self.set_power_state(PowerState::D0)?;

        // Restore device state
        self.restore_device_state()?;

        Ok(())
    }
}
```

### 2. Device-Specific Implementation

```rust
impl MyDriver {
    fn reset_device(&self) -> Result<(), DriverError> {
        // Send reset command
        self.write_register(CONTROL_REG, RESET_BIT);

        // Wait for reset completion
        let timeout = 1000; // milliseconds
        for _ in 0..timeout {
            if self.read_register(STATUS_REG) & RESET_COMPLETE != 0 {
                return Ok(());
            }
            sleep_ms(1);
        }

        Err(DriverError::DeviceTimeout)
    }

    fn configure_device(&self) -> Result<(), DriverError> {
        // Device-specific configuration
        self.write_register(CONFIG_REG, DEFAULT_CONFIG);

        // Setup DMA if needed
        self.setup_dma()?;

        // Configure interrupts
        self.write_register(INTERRUPT_MASK_REG, ENABLED_INTERRUPTS);

        Ok(())
    }

    fn enable_device(&self) -> Result<(), DriverError> {
        let mut control = self.read_register(CONTROL_REG);
        control |= ENABLE_BIT;
        self.write_register(CONTROL_REG, control);

        Ok(())
    }
}
```

### 3. Registration and Loading

```rust
// In your driver module's init function
pub fn init() -> Result<(), DriverError> {
    let driver = Box::new(MyDriver::new());

    // List of supported devices (vendor_id, device_id)
    let supported_devices = &[
        (0x1234, 0x5678), // MyDevice Model A
        (0x1234, 0x5679), // MyDevice Model B
    ];

    register_driver(driver, supported_devices)?;

    info!("MyDriver registered successfully");
    Ok(())
}
```

---

## Network Driver Development

### Comprehensive Example: Intel E1000 Driver

```rust
use crate::net::{NetworkDevice, NetworkError, NetworkStats, MacAddress};
use crate::drivers::{DriverOps, DriverError};
use alloc::vec::Vec;

pub struct E1000Driver {
    device: PciDevice,
    mmio_base: PhysAddr,
    irq: u8,
    mac_address: MacAddress,

    // Transmit ring
    tx_descriptors: Vec<TxDescriptor>,
    tx_buffers: Vec<DmaBuffer>,
    tx_head: usize,
    tx_tail: usize,

    // Receive ring
    rx_descriptors: Vec<RxDescriptor>,
    rx_buffers: Vec<DmaBuffer>,
    rx_head: usize,
    rx_tail: usize,

    // Statistics
    stats: NetworkStats,
}

impl E1000Driver {
    pub fn new() -> Self {
        E1000Driver {
            device: PciDevice::default(),
            mmio_base: PhysAddr::zero(),
            irq: 0,
            mac_address: MacAddress::zero(),
            tx_descriptors: Vec::new(),
            tx_buffers: Vec::new(),
            tx_head: 0,
            tx_tail: 0,
            rx_descriptors: Vec::new(),
            rx_buffers: Vec::new(),
            rx_head: 0,
            rx_tail: 0,
            stats: NetworkStats::new(),
        }
    }

    fn setup_transmit_ring(&mut self) -> Result<(), DriverError> {
        const TX_RING_SIZE: usize = 256;

        // Allocate descriptors
        self.tx_descriptors = allocate_dma_memory(
            TX_RING_SIZE * size_of::<TxDescriptor>()
        )?;

        // Allocate buffers
        for _ in 0..TX_RING_SIZE {
            let buffer = allocate_dma_buffer(MAX_PACKET_SIZE)?;
            self.tx_buffers.push(buffer);
        }

        // Configure transmit registers
        let desc_addr = self.tx_descriptors.as_ptr() as u64;
        self.write_register(E1000_TDBAL, (desc_addr & 0xFFFFFFFF) as u32);
        self.write_register(E1000_TDBAH, (desc_addr >> 32) as u32);
        self.write_register(E1000_TDLEN,
                           (TX_RING_SIZE * size_of::<TxDescriptor>()) as u32);
        self.write_register(E1000_TDH, 0);
        self.write_register(E1000_TDT, 0);

        // Enable transmit
        let tctl = E1000_TCTL_EN | E1000_TCTL_PSP |
                   (0x10 << E1000_TCTL_CT_SHIFT) |
                   (0x40 << E1000_TCTL_COLD_SHIFT);
        self.write_register(E1000_TCTL, tctl);

        Ok(())
    }

    fn setup_receive_ring(&mut self) -> Result<(), DriverError> {
        const RX_RING_SIZE: usize = 256;

        // Allocate descriptors
        self.rx_descriptors = allocate_dma_memory(
            RX_RING_SIZE * size_of::<RxDescriptor>()
        )?;

        // Allocate and setup receive buffers
        for i in 0..RX_RING_SIZE {
            let buffer = allocate_dma_buffer(MAX_PACKET_SIZE)?;
            self.rx_descriptors[i].buffer_addr = buffer.physical_addr();
            self.rx_descriptors[i].status = 0;
            self.rx_buffers.push(buffer);
        }

        // Configure receive registers
        let desc_addr = self.rx_descriptors.as_ptr() as u64;
        self.write_register(E1000_RDBAL, (desc_addr & 0xFFFFFFFF) as u32);
        self.write_register(E1000_RDBAH, (desc_addr >> 32) as u32);
        self.write_register(E1000_RDLEN,
                           (RX_RING_SIZE * size_of::<RxDescriptor>()) as u32);
        self.write_register(E1000_RDH, 0);
        self.write_register(E1000_RDT, RX_RING_SIZE as u32 - 1);

        // Enable receive
        let rctl = E1000_RCTL_EN | E1000_RCTL_BAM | E1000_RCTL_SZ_2048 |
                   E1000_RCTL_SECRC;
        self.write_register(E1000_RCTL, rctl);

        Ok(())
    }

    fn read_mac_address(&mut self) -> Result<(), DriverError> {
        // Try to read from EEPROM first
        if let Ok(mac) = self.read_mac_from_eeprom() {
            self.mac_address = mac;
            return Ok(());
        }

        // Fall back to reading from registers
        let low = self.read_register(E1000_RAL);
        let high = self.read_register(E1000_RAH);

        self.mac_address = MacAddress([
            (low & 0xFF) as u8,
            ((low >> 8) & 0xFF) as u8,
            ((low >> 16) & 0xFF) as u8,
            ((low >> 24) & 0xFF) as u8,
            (high & 0xFF) as u8,
            ((high >> 8) & 0xFF) as u8,
        ]);

        Ok(())
    }
}

impl DriverOps for E1000Driver {
    fn probe(&mut self, device: &PciDevice) -> Result<(), DriverError> {
        // Check if this is a supported E1000 device
        match (device.vendor_id, device.device_id) {
            (0x8086, 0x100E) | // 82540EM
            (0x8086, 0x100F) | // 82545EM
            (0x8086, 0x10D3) | // 82574L
            (0x8086, 0x1533) => { // I210
                self.device = *device;
                Ok(())
            }
            _ => Err(DriverError::UnsupportedDevice),
        }
    }

    fn init(&mut self) -> Result<(), DriverError> {
        // Map device memory
        let bar0 = self.device.get_bar(0)?;
        self.mmio_base = map_device_memory(bar0, 0x20000)?;

        // Reset device
        self.write_register(E1000_CTRL, E1000_CTRL_RST);
        sleep_ms(10);

        // Read MAC address
        self.read_mac_address()?;

        // Setup transmit and receive rings
        self.setup_transmit_ring()?;
        self.setup_receive_ring()?;

        // Setup interrupt handler
        self.irq = self.device.get_irq().ok_or(DriverError::NoIrq)?;
        register_interrupt_handler(self.irq, e1000_interrupt_handler)?;

        // Enable interrupts
        let ims = E1000_IMS_RXT0 | E1000_IMS_TXDW | E1000_IMS_LSC;
        self.write_register(E1000_IMS, ims);

        info!("E1000 driver initialized: MAC={}", self.mac_address);
        Ok(())
    }

    fn interrupt_handler(&mut self, _irq: u8) {
        let icr = self.read_register(E1000_ICR);

        if icr & E1000_ICR_RXT0 != 0 {
            self.handle_receive_interrupt();
        }

        if icr & E1000_ICR_TXDW != 0 {
            self.handle_transmit_done_interrupt();
        }

        if icr & E1000_ICR_LSC != 0 {
            self.handle_link_status_change();
        }
    }
}

impl NetworkDevice for E1000Driver {
    fn transmit(&mut self, packet: &[u8]) -> Result<(), NetworkError> {
        if packet.len() > MAX_PACKET_SIZE {
            return Err(NetworkError::PacketTooLarge);
        }

        // Check if transmit ring has space
        let next_tail = (self.tx_tail + 1) % self.tx_descriptors.len();
        if next_tail == self.tx_head {
            return Err(NetworkError::TransmitRingFull);
        }

        // Copy packet to buffer
        let buffer = &mut self.tx_buffers[self.tx_tail];
        buffer.copy_from_slice(packet);

        // Setup descriptor
        let descriptor = &mut self.tx_descriptors[self.tx_tail];
        descriptor.buffer_addr = buffer.physical_addr();
        descriptor.length = packet.len() as u16;
        descriptor.cso = 0;
        descriptor.cmd = TXD_CMD_EOP | TXD_CMD_IFCS | TXD_CMD_RS;
        descriptor.status = 0;
        descriptor.css = 0;
        descriptor.special = 0;

        // Update tail pointer
        self.tx_tail = next_tail;
        self.write_register(E1000_TDT, self.tx_tail as u32);

        self.stats.packets_transmitted += 1;
        self.stats.bytes_transmitted += packet.len() as u64;

        Ok(())
    }

    fn receive(&mut self) -> Option<Vec<u8>> {
        let descriptor = &self.rx_descriptors[self.rx_head];

        // Check if packet is ready
        if descriptor.status & RXD_STAT_DD == 0 {
            return None;
        }

        // Copy packet from buffer
        let buffer = &self.rx_buffers[self.rx_head];
        let packet_len = descriptor.length as usize;
        let packet = buffer.as_slice()[..packet_len].to_vec();

        // Reset descriptor
        let descriptor = &mut self.rx_descriptors[self.rx_head];
        descriptor.status = 0;

        // Update head pointer
        self.rx_head = (self.rx_head + 1) % self.rx_descriptors.len();

        // Update tail pointer to give buffer back to hardware
        let new_tail = (self.rx_tail + 1) % self.rx_descriptors.len();
        self.rx_tail = new_tail;
        self.write_register(E1000_RDT, self.rx_tail as u32);

        self.stats.packets_received += 1;
        self.stats.bytes_received += packet_len as u64;

        Some(packet)
    }

    fn get_mac_address(&self) -> MacAddress {
        self.mac_address
    }

    fn get_mtu(&self) -> usize {
        1500 // Standard Ethernet MTU
    }

    fn get_stats(&self) -> NetworkStats {
        self.stats.clone()
    }

    fn set_promiscuous(&mut self, enabled: bool) {
        let mut rctl = self.read_register(E1000_RCTL);
        if enabled {
            rctl |= E1000_RCTL_UPE | E1000_RCTL_MPE;
        } else {
            rctl &= !(E1000_RCTL_UPE | E1000_RCTL_MPE);
        }
        self.write_register(E1000_RCTL, rctl);
    }
}
```

---

## Storage Driver Development

### AHCI SATA Driver Example

```rust
use crate::storage::{BlockDevice, StorageError, StorageDeviceInfo, Block};

pub struct AhciDriver {
    device: PciDevice,
    abar: PhysAddr,  // AHCI Base Address Register
    ports: Vec<AhciPort>,
    command_slots: usize,
}

pub struct AhciPort {
    port_num: u8,
    port_regs: *mut AhciPortRegs,
    command_list: Vec<CommandHeader>,
    received_fis: ReceivedFis,
    command_tables: Vec<CommandTable>,
    active_commands: BitSet,
}

impl AhciDriver {
    fn detect_drives(&mut self) -> Result<(), DriverError> {
        let pi = unsafe { (*self.hba_mem).ports_implemented };

        for port in 0..32 {
            if pi & (1 << port) != 0 {
                if self.check_drive_type(port) == DriveType::Sata {
                    self.init_port(port)?;
                }
            }
        }

        Ok(())
    }

    fn init_port(&mut self, port_num: u8) -> Result<(), DriverError> {
        let port_regs = unsafe {
            &mut (*self.hba_mem).ports[port_num as usize]
        };

        // Stop command and FIS reception
        port_regs.cmd &= !(HBA_PxCMD_ST | HBA_PxCMD_FRE);

        // Wait for completion
        while port_regs.cmd & (HBA_PxCMD_FR | HBA_PxCMD_CR) != 0 {
            // Wait
        }

        // Allocate command list
        let command_list = allocate_dma_memory(
            size_of::<CommandHeader>() * self.command_slots
        )?;

        port_regs.clb = command_list.physical_addr() as u32;
        port_regs.clbu = (command_list.physical_addr() >> 32) as u32;

        // Allocate received FIS structure
        let received_fis = allocate_dma_memory(size_of::<ReceivedFis>())?;
        port_regs.fb = received_fis.physical_addr() as u32;
        port_regs.fbu = (received_fis.physical_addr() >> 32) as u32;

        // Start command and FIS reception
        port_regs.cmd |= HBA_PxCMD_FRE;
        port_regs.cmd |= HBA_PxCMD_ST;

        let port = AhciPort::new(port_num, port_regs, command_list, received_fis);
        self.ports.push(port);

        Ok(())
    }

    fn send_command(&mut self, port: u8, fis: &HostToDeviceFis,
                    buffer: Option<&mut [u8]>) -> Result<(), StorageError> {
        let port = &mut self.ports[port as usize];

        // Find free command slot
        let slot = port.find_free_slot().ok_or(StorageError::NoFreeSlots)?;

        // Setup command header
        let command_header = &mut port.command_list[slot];
        command_header.cfl = size_of::<HostToDeviceFis>() / 4; // FIS length in DWORDs
        command_header.w = 0; // Read from device
        command_header.prdtl = if buffer.is_some() { 1 } else { 0 };

        // Setup command table
        let command_table = &mut port.command_tables[slot];
        command_table.cfis = *fis;

        if let Some(buf) = buffer {
            command_table.prdt[0].dba = buf.as_ptr() as u64;
            command_table.prdt[0].dbc = buf.len() as u32 - 1;
            command_table.prdt[0].i = 0; // No interrupt on completion for this PRDT entry
        }

        // Issue command
        port.port_regs.ci |= 1 << slot;

        // Wait for completion
        while port.port_regs.ci & (1 << slot) != 0 {
            if port.port_regs.is & HBA_PxIS_TFES != 0 {
                return Err(StorageError::CommandFailed);
            }
        }

        port.active_commands.set(slot, false);
        Ok(())
    }
}

impl BlockDevice for AhciDriver {
    fn read_blocks(&mut self, start_block: u64,
                   blocks: &mut [Block]) -> Result<(), StorageError> {
        let port = 0; // Use first port for simplicity

        for (i, block) in blocks.iter_mut().enumerate() {
            let lba = start_block + i as u64;

            let fis = HostToDeviceFis {
                fis_type: FIS_TYPE_REG_H2D,
                pmport: 0,
                c: 1, // Command
                command: ATA_CMD_READ_DMA_EX,
                lba0: (lba & 0xFF) as u8,
                lba1: ((lba >> 8) & 0xFF) as u8,
                lba2: ((lba >> 16) & 0xFF) as u8,
                device: 1 << 6, // LBA mode
                lba3: ((lba >> 24) & 0xFF) as u8,
                lba4: ((lba >> 32) & 0xFF) as u8,
                lba5: ((lba >> 40) & 0xFF) as u8,
                features_low: 0,
                count_low: 1, // Read 1 sector
                count_high: 0,
                features_high: 0,
                control: 0,
                aux: 0,
            };

            self.send_command(port, &fis, Some(block.as_mut_slice()))?;
        }

        Ok(())
    }

    fn write_blocks(&mut self, start_block: u64,
                    blocks: &[Block]) -> Result<(), StorageError> {
        // Similar to read_blocks but with write command
        // Implementation details...
        Ok(())
    }

    fn get_block_size(&self) -> usize {
        512 // Standard sector size
    }

    fn get_block_count(&self) -> u64 {
        // Read from device identify information
        self.device_info.sectors
    }
}
```

---

## GPU Driver Development

### Intel i915 Driver Framework

```rust
use crate::gpu::{GpuDevice, GpuError, DisplayMode, FramebufferId};

pub struct I915Driver {
    device: PciDevice,
    mmio_base: PhysAddr,
    gtt_base: PhysAddr,
    stolen_base: PhysAddr,

    // Display pipes
    pipes: [DisplayPipe; 3],

    // Memory management
    gtt: GlobalGraphicsTranslationTable,
    gem_objects: HashMap<u32, GemObject>,

    // Command submission
    ring_buffers: [RingBuffer; 3], // Render, BSD, BLT
}

impl I915Driver {
    fn init_graphics_translation_table(&mut self) -> Result<(), GpuError> {
        // Setup GTT for GPU virtual memory
        let gtt_size = self.get_gtt_size();
        let stolen_size = self.get_stolen_memory_size();

        self.gtt = GlobalGraphicsTranslationTable::new(
            self.gtt_base,
            gtt_size,
            stolen_size
        )?;

        // Map stolen memory
        self.gtt.map_stolen_memory(self.stolen_base, stolen_size)?;

        Ok(())
    }

    fn init_display_pipes(&mut self) -> Result<(), GpuError> {
        for i in 0..3 {
            self.pipes[i] = DisplayPipe::new(i as u8);
            self.pipes[i].init(self.mmio_base)?;
        }

        Ok(())
    }

    fn init_ring_buffers(&mut self) -> Result<(), GpuError> {
        // Render ring
        self.ring_buffers[0] = RingBuffer::new(
            RingType::Render,
            RING_SIZE,
            &mut self.gtt
        )?;

        // BSD (video decode) ring
        self.ring_buffers[1] = RingBuffer::new(
            RingType::Bsd,
            RING_SIZE,
            &mut self.gtt
        )?;

        // BLT (copy) ring
        self.ring_buffers[2] = RingBuffer::new(
            RingType::Blt,
            RING_SIZE,
            &mut self.gtt
        )?;

        Ok(())
    }
}

impl GpuDevice for I915Driver {
    fn initialize_display(&mut self) -> Result<(), GpuError> {
        // Power on display wells
        self.enable_display_power()?;

        // Initialize display PLLs
        self.init_display_clocks()?;

        // Setup display pipes
        self.init_display_pipes()?;

        Ok(())
    }

    fn set_mode(&mut self, mode: DisplayMode) -> Result<(), GpuError> {
        let pipe = 0; // Use pipe A

        // Configure CRTC timing
        self.pipes[pipe].set_timing(
            mode.width,
            mode.height,
            mode.refresh_rate
        )?;

        // Configure plane
        self.pipes[pipe].set_plane_config(
            mode.width,
            mode.height,
            mode.pixel_format
        )?;

        // Enable pipe and plane
        self.pipes[pipe].enable()?;

        Ok(())
    }

    fn allocate_framebuffer(&mut self, size: usize) -> Result<FramebufferId, GpuError> {
        // Allocate GEM object for framebuffer
        let gem_object = self.gtt.allocate_object(size, GemObjectType::Framebuffer)?;

        let fb_id = self.next_framebuffer_id();
        self.gem_objects.insert(fb_id, gem_object);

        Ok(FramebufferId(fb_id))
    }

    fn submit_commands(&mut self, commands: &[GpuCommand]) -> Result<(), GpuError> {
        let ring = &mut self.ring_buffers[0]; // Use render ring

        for command in commands {
            match command {
                GpuCommand::DrawTriangles { vertices, indices } => {
                    ring.emit_draw_command(vertices, indices)?;
                }
                GpuCommand::Clear { color } => {
                    ring.emit_clear_command(*color)?;
                }
                GpuCommand::SetTexture { texture_id } => {
                    ring.emit_set_texture_command(*texture_id)?;
                }
            }
        }

        // Submit batch
        ring.submit()?;

        Ok(())
    }
}
```

---

## Driver Testing and Debugging

### Unit Testing Framework

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_framework::{TestDevice, MockPciDevice};

    #[test]
    fn test_driver_probe() {
        let mut driver = MyDriver::new();
        let device = MockPciDevice::new(0x1234, 0x5678);

        assert!(driver.probe(&device).is_ok());
    }

    #[test]
    fn test_driver_initialization() {
        let mut driver = MyDriver::new();
        let device = TestDevice::create_virtual_device();

        driver.probe(&device).unwrap();
        assert!(driver.init().is_ok());
        assert!(driver.initialized);
    }

    #[test]
    fn test_interrupt_handling() {
        let mut driver = setup_test_driver();

        // Simulate interrupt
        driver.interrupt_handler(5);

        // Verify interrupt was handled correctly
        assert_eq!(driver.interrupt_count, 1);
    }
}
```

### Integration Testing

```rust
#[integration_test]
fn test_network_driver_integration() {
    let mut driver = E1000Driver::new();
    let test_device = create_e1000_test_device();

    // Initialize driver
    driver.probe(&test_device).unwrap();
    driver.init().unwrap();

    // Test packet transmission
    let test_packet = vec![0xAA; 64];
    assert!(driver.transmit(&test_packet).is_ok());

    // Test packet reception
    inject_test_packet(&test_device, &test_packet);
    let received = driver.receive();
    assert_eq!(received, Some(test_packet));
}
```

### Debugging Techniques

#### 1. Register Debugging

```rust
impl MyDriver {
    fn dump_registers(&self) {
        debug!("Device Registers:");
        debug!("  CONTROL: 0x{:08X}", self.read_register(CONTROL_REG));
        debug!("  STATUS:  0x{:08X}", self.read_register(STATUS_REG));
        debug!("  CONFIG:  0x{:08X}", self.read_register(CONFIG_REG));
    }

    fn validate_register_values(&self) -> Result<(), DriverError> {
        let status = self.read_register(STATUS_REG);
        if status & ERROR_BITS != 0 {
            error!("Device error detected: 0x{:08X}", status);
            return Err(DriverError::DeviceError);
        }
        Ok(())
    }
}
```

#### 2. DMA Debugging

```rust
fn debug_dma_descriptors(&self) {
    for (i, desc) in self.rx_descriptors.iter().enumerate() {
        debug!("RX Desc {}: addr=0x{:016X}, status=0x{:04X}, length={}",
               i, desc.buffer_addr, desc.status, desc.length);
    }
}
```

#### 3. Performance Monitoring

```rust
impl MyDriver {
    fn update_performance_stats(&mut self) {
        self.stats.interrupts_per_second =
            self.interrupt_count / self.uptime_seconds;
        self.stats.throughput_mbps =
            (self.bytes_transferred * 8) / (self.uptime_seconds * 1_000_000);
    }
}
```

---

## Hardware Database Integration

### Adding New Device Support

```rust
// In src/pci/database.rs
pub static PCI_DEVICE_DATABASE: &[(u16, u16, &str, DeviceClass)] = &[
    // Existing entries...

    // Add your new devices here
    (0x1234, 0x5678, "MyCompany MyDevice Model A", DeviceClass::Network),
    (0x1234, 0x5679, "MyCompany MyDevice Model B", DeviceClass::Network),
    (0x1234, 0x567A, "MyCompany MyDevice Model C", DeviceClass::Storage),
];
```

### Device Classification

```rust
pub enum DeviceClass {
    Network,
    Storage,
    Display,
    Multimedia,
    Bridge,
    Communication,
    Input,
    Dock,
    Processor,
    SerialBus,
    Unknown,
}

impl DeviceClass {
    pub fn from_pci_class(class_code: u8, subclass: u8) -> Self {
        match class_code {
            0x01 => DeviceClass::Storage,
            0x02 => DeviceClass::Network,
            0x03 => DeviceClass::Display,
            0x04 => DeviceClass::Multimedia,
            0x06 => DeviceClass::Bridge,
            0x07 => DeviceClass::Communication,
            0x09 => DeviceClass::Input,
            0x0A => DeviceClass::Dock,
            0x0B => DeviceClass::Processor,
            0x0C => DeviceClass::SerialBus,
            _ => DeviceClass::Unknown,
        }
    }
}
```

---

## Best Practices

### 1. Error Handling

```rust
// Use specific error types
#[derive(Debug)]
pub enum MyDriverError {
    DeviceNotFound,
    InitializationFailed,
    DmaAllocationFailed,
    TimeoutError,
    HardwareError(u32),
}

// Implement proper error propagation
impl From<DmaError> for MyDriverError {
    fn from(err: DmaError) -> Self {
        MyDriverError::DmaAllocationFailed
    }
}
```

### 2. Resource Management

```rust
impl Drop for MyDriver {
    fn drop(&mut self) {
        // Ensure cleanup is called
        if self.initialized {
            self.cleanup();
        }
    }
}

// Use RAII for resource management
pub struct DmaBuffer {
    virtual_addr: VirtAddr,
    physical_addr: PhysAddr,
    size: usize,
}

impl Drop for DmaBuffer {
    fn drop(&mut self) {
        unsafe {
            deallocate_dma_memory(self.virtual_addr, self.size);
        }
    }
}
```

### 3. Thread Safety

```rust
// Use appropriate synchronization
pub struct ThreadSafeDriver {
    inner: Mutex<MyDriver>,
}

impl ThreadSafeDriver {
    pub fn transmit(&self, packet: &[u8]) -> Result<(), NetworkError> {
        let mut driver = self.inner.lock();
        driver.transmit(packet)
    }
}
```

### 4. Performance Optimization

```rust
// Use lock-free techniques where possible
pub struct LockFreeRingBuffer {
    head: AtomicUsize,
    tail: AtomicUsize,
    buffer: Vec<Entry>,
}

// Batch operations
impl MyDriver {
    pub fn transmit_batch(&mut self, packets: &[&[u8]]) -> Result<(), NetworkError> {
        // Process multiple packets in one call
        for packet in packets {
            self.queue_packet(packet)?;
        }
        self.flush_transmit_queue()
    }
}
```

### 5. Documentation

```rust
/// High-performance network driver for MyDevice family
///
/// This driver supports:
/// - Hardware TSO/UFO offload
/// - MSI-X interrupts
/// - SR-IOV virtualization
/// - DPDK compatibility
///
/// # Example
/// ```
/// let mut driver = MyDriver::new();
/// driver.probe(&device)?;
/// driver.init()?;
/// driver.transmit(&packet)?;
/// ```
pub struct MyDriver {
    // ...
}
```

### 6. Testing Strategy

1. **Unit Tests**: Test individual functions
2. **Integration Tests**: Test driver with mock hardware
3. **Hardware Tests**: Test with real hardware
4. **Stress Tests**: Test under high load
5. **Regression Tests**: Ensure no functionality breaks

### 7. Debugging Guidelines

1. **Logging**: Use structured logging with appropriate levels
2. **Assertions**: Use debug assertions for invariants
3. **Register Dumps**: Provide register debugging capabilities
4. **Statistics**: Maintain performance and error statistics
5. **Tracing**: Use tracing for complex state transitions

---

## Conclusion

This guide provides the foundation for developing high-quality device drivers for RustOS. Remember to:

- Follow the unified driver interface
- Implement proper error handling and resource management
- Write comprehensive tests
- Document your driver thoroughly
- Consider performance and security implications

For specific questions or advanced topics, refer to:
- [Architecture Overview](ARCHITECTURE.md)
- [API Reference](API_REFERENCE.md)
- [Module Index](MODULE_INDEX.md)
- [Subsystem Documentation](SUBSYSTEMS.md)

Happy driver development!