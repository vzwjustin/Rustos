//! Production SMP (Symmetric MultiProcessing) support for RustOS
//!
//! Real multiprocessor support using APIC and x86_64 features

use core::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use x86_64::VirtAddr;
use spin::Mutex;

/// Maximum number of CPUs supported
pub const MAX_CPUS: usize = 256;

/// Per-CPU data structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CpuData {
    pub cpu_id: u32,
    pub apic_id: u32,
    pub online: bool,
    pub idle_time: u64,
    pub kernel_stack: VirtAddr,
    pub tss_selector: u16,
}

impl CpuData {
    const fn new() -> Self {
        Self {
            cpu_id: 0,
            apic_id: 0,
            online: false,
            idle_time: 0,
            kernel_stack: VirtAddr::zero(),
            tss_selector: 0,
        }
    }
}

/// Global CPU data array
static CPU_DATA: Mutex<[CpuData; MAX_CPUS]> = Mutex::new([CpuData::new(); MAX_CPUS]);
/// Number of detected CPUs
static CPU_COUNT: AtomicU32 = AtomicU32::new(0);
/// Number of online CPUs
static ONLINE_CPUS: AtomicU32 = AtomicU32::new(1); // BSP is always online
/// SMP initialized flag
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Local APIC base address
static LOCAL_APIC_BASE: AtomicU64 = AtomicU64::new(0);

use core::sync::atomic::AtomicU64;

/// APIC register offsets
mod apic_regs {
    pub const APIC_ID: u32 = 0x20;
    pub const APIC_VERSION: u32 = 0x30;
    pub const APIC_TPR: u32 = 0x80;
    pub const APIC_EOI: u32 = 0xB0;
    pub const APIC_SPURIOUS: u32 = 0xF0;
    pub const APIC_ICR_LOW: u32 = 0x300;
    pub const APIC_ICR_HIGH: u32 = 0x310;
    pub const APIC_TIMER_LVT: u32 = 0x320;
    pub const APIC_TIMER_INITIAL: u32 = 0x380;
    pub const APIC_TIMER_CURRENT: u32 = 0x390;
    pub const APIC_TIMER_DIVIDE: u32 = 0x3E0;
}

/// Initialize SMP subsystem
pub fn init() -> Result<(), &'static str> {
    // Check if already initialized
    if INITIALIZED.load(Ordering::Acquire) {
        return Ok(());
    }
    
    // Get Local APIC base from MSR
    let apic_base = unsafe { read_msr(0x1B) };
    if apic_base & (1 << 11) == 0 {
        return Err("APIC not enabled");
    }
    
    let apic_phys = apic_base & 0xFFFF_F000;
    LOCAL_APIC_BASE.store(apic_phys, Ordering::Release);
    
    // Initialize BSP (Bootstrap Processor) data
    let mut cpu_data = CPU_DATA.lock();
    cpu_data[0] = CpuData {
        cpu_id: 0,
        apic_id: get_apic_id(),
        online: true,
        idle_time: 0,
        kernel_stack: VirtAddr::zero(), // Will be set by memory manager
        tss_selector: 0, // Will be set by GDT
    };
    
    CPU_COUNT.store(1, Ordering::Release);
    ONLINE_CPUS.store(1, Ordering::Release);
    INITIALIZED.store(true, Ordering::Release);
    
    Ok(())
}

/// Get current CPU's APIC ID
pub fn get_apic_id() -> u32 {
    if let Some(base) = get_apic_base() {
        unsafe { read_apic(base, apic_regs::APIC_ID) >> 24 }
    } else {
        // Fallback to CPUID
        unsafe {
            let result = core::arch::x86_64::__cpuid(1);
            (result.ebx >> 24) as u32
        }
    }
}

/// Get current CPU ID
pub fn current_cpu() -> u32 {
    let apic_id = get_apic_id();
    let cpu_data = CPU_DATA.lock();
    
    for i in 0..CPU_COUNT.load(Ordering::Acquire) as usize {
        if cpu_data[i].apic_id == apic_id {
            return cpu_data[i].cpu_id;
        }
    }
    
    // Default to 0 if not found (shouldn't happen)
    0
}

/// Get number of CPUs
pub fn cpu_count() -> u32 {
    CPU_COUNT.load(Ordering::Acquire)
}

/// Get number of online CPUs
pub fn online_cpus() -> u32 {
    ONLINE_CPUS.load(Ordering::Acquire)
}

/// Check if SMP is initialized
pub fn is_initialized() -> bool {
    INITIALIZED.load(Ordering::Acquire)
}

/// Send Inter-Processor Interrupt
pub fn send_ipi(target_cpu: u32, vector: u8) -> Result<(), &'static str> {
    let cpu_data = CPU_DATA.lock();
    
    if target_cpu >= CPU_COUNT.load(Ordering::Acquire) {
        return Err("Invalid CPU ID");
    }
    
    if !cpu_data[target_cpu as usize].online {
        return Err("Target CPU not online");
    }
    
    let apic_id = cpu_data[target_cpu as usize].apic_id;
    drop(cpu_data);
    
    if let Some(base) = get_apic_base() {
        unsafe {
            // Set target APIC ID in ICR high
            write_apic(base, apic_regs::APIC_ICR_HIGH, apic_id << 24);
            // Send IPI with vector in ICR low
            write_apic(base, apic_regs::APIC_ICR_LOW, vector as u32);
        }
        Ok(())
    } else {
        Err("APIC not mapped")
    }
}

/// Broadcast IPI to all CPUs except self
pub fn broadcast_ipi(vector: u8) -> Result<(), &'static str> {
    if let Some(base) = get_apic_base() {
        unsafe {
            // Set broadcast mode in ICR high
            write_apic(base, apic_regs::APIC_ICR_HIGH, 0);
            // Send IPI with broadcast flag (bit 19) and all except self (bit 18)
            write_apic(base, apic_regs::APIC_ICR_LOW, 
                      (vector as u32) | (3 << 18));
        }
        Ok(())
    } else {
        Err("APIC not mapped")
    }
}

/// Signal End Of Interrupt
pub fn eoi() {
    if let Some(base) = get_apic_base() {
        unsafe {
            write_apic(base, apic_regs::APIC_EOI, 0);
        }
    }
}

/// Get Local APIC base address
fn get_apic_base() -> Option<VirtAddr> {
    let phys = LOCAL_APIC_BASE.load(Ordering::Acquire);
    if phys != 0 {
        // In production, this should be properly mapped by memory manager
        // For now, return identity-mapped address
        Some(VirtAddr::new(phys))
    } else {
        None
    }
}

/// Read APIC register
unsafe fn read_apic(base: VirtAddr, offset: u32) -> u32 {
    let addr = (base.as_u64() + offset as u64) as *const u32;
    addr.read_volatile()
}

/// Write APIC register
unsafe fn write_apic(base: VirtAddr, offset: u32, value: u32) {
    let addr = (base.as_u64() + offset as u64) as *mut u32;
    addr.write_volatile(value);
}

/// Read Model-Specific Register
unsafe fn read_msr(msr: u32) -> u64 {
    let (high, low): (u32, u32);
    core::arch::asm!(
        "rdmsr",
        in("ecx") msr,
        out("eax") low,
        out("edx") high,
        options(nomem, nostack, preserves_flags)
    );
    ((high as u64) << 32) | (low as u64)
}

/// Write Model-Specific Register
unsafe fn write_msr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    core::arch::asm!(
        "wrmsr",
        in("ecx") msr,
        in("eax") low,
        in("edx") high,
        options(nomem, nostack, preserves_flags)
    );
}

// =============================================================================
// STUB FUNCTIONS - TODO: Implement production versions
// =============================================================================

/// TODO: Implement SMP detection
/// Check if Symmetric Multiprocessing is available and initialized
/// Currently returns false - needs CPU count detection via ACPI/APIC
pub fn smp_available() -> bool {
    // TODO: Check if more than one CPU is available
    // TODO: Verify SMP initialization completed successfully
    cpu_count() > 1
}
