//! Basic Memory Management for RustOS
//!
//! Simple memory management without heap allocation (for incremental development)

use bootloader::bootinfo::MemoryRegion;

/// Page size constants
pub const PAGE_SIZE: usize = 4096;

/// Memory layout constants for virtual address space
pub const KERNEL_HEAP_START: usize = 0x_4444_4444_0000;
pub const KERNEL_HEAP_SIZE: usize = 100 * 1024 * 1024; // 100 MiB

/// Simple memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_memory: usize,
    pub usable_memory: usize,
    pub memory_regions: usize,
}

/// Analyze memory map from bootloader
pub fn analyze_memory_map(memory_regions: &[MemoryRegion]) -> MemoryStats {
    let mut total_memory = 0;
    let mut usable_memory = 0;
    let memory_regions_count = memory_regions.len();

    for region in memory_regions {
        let size = region.range.end_addr() - region.range.start_addr();
        total_memory += size as usize;

        if region.region_type == bootloader::bootinfo::MemoryRegionType::Usable {
            usable_memory += size as usize;
        }
    }

    MemoryStats {
        total_memory,
        usable_memory,
        memory_regions: memory_regions_count,
    }
}

/// Initialize memory management system (simplified)
pub fn init_memory(
    memory_regions: &[MemoryRegion],
    _physical_memory_offset: x86_64::VirtAddr,
) -> Result<MemoryStats, &'static str> {
    // For now, just analyze the memory map without setting up heap
    let stats = analyze_memory_map(memory_regions);
    
    // Store stats for health monitoring
    store_memory_stats(stats.clone());
    
    Ok(stats)
}

/// Utility function to align up to nearest boundary
pub fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

/// Utility function to align down to nearest boundary
pub fn align_down(addr: usize, align: usize) -> usize {
    addr & !(align - 1)
}

/// Global memory statistics for health monitoring
static mut GLOBAL_MEMORY_STATS: Option<MemoryStats> = None;

/// Store memory statistics for later retrieval
pub fn store_memory_stats(stats: MemoryStats) {
    unsafe {
        GLOBAL_MEMORY_STATS = Some(stats);
    }
}

/// Get current memory statistics for health monitoring
pub fn get_memory_stats() -> Result<MemoryStats, &'static str> {
    unsafe {
        GLOBAL_MEMORY_STATS.clone().ok_or("Memory statistics not available")
    }
}

/// Initialize the kernel heap allocator
pub fn init_heap(allocator: &linked_list_allocator::LockedHeap) -> Result<(), &'static str> {
    unsafe {
        allocator.lock().init(KERNEL_HEAP_START, KERNEL_HEAP_SIZE);
    }
    Ok(())
}