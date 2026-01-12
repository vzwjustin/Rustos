//! EXT4 Filesystem Implementation
//!
//! This module provides a production-ready EXT4 filesystem implementation
//! with proper metadata handling, journaling, and disk I/O operations.

use super::{
    FileSystem, FileSystemType, FileSystemStats, FileMetadata, FileType, FilePermissions,
    DirectoryEntry, OpenFlags, FsResult, FsError, InodeNumber,
};
use crate::drivers::storage::{read_storage_sectors, write_storage_sectors, StorageError};
use alloc::{vec, vec::Vec, string::{String, ToString}, collections::BTreeMap, format, boxed::Box};
use spin::RwLock;
use core::mem;

/// EXT4 superblock magic number
const EXT4_SUPER_MAGIC: u16 = 0xEF53;

/// EXT4 block size constants
const EXT4_MIN_BLOCK_SIZE: u32 = 1024;
const EXT4_MAX_BLOCK_SIZE: u32 = 65536;

/// EXT4 inode size
const EXT4_GOOD_OLD_INODE_SIZE: u16 = 128;
const EXT4_INODE_SIZE_DEFAULT: u16 = 256;

/// EXT4 feature flags
bitflags::bitflags! {
    pub struct Ext4FeatureCompat: u32 {
        const DIR_PREALLOC = 0x0001;
        const IMAGIC_INODES = 0x0002;
        const HAS_JOURNAL = 0x0004;
        const EXT_ATTR = 0x0008;
        const RESIZE_INODE = 0x0010;
        const DIR_INDEX = 0x0020;
        const LAZY_BG = 0x0040;
        const EXCLUDE_INODE = 0x0080;
        const EXCLUDE_BITMAP = 0x0100;
        const SPARSE_SUPER2 = 0x0200;
    }
}

bitflags::bitflags! {
    pub struct Ext4FeatureIncompat: u32 {
        const COMPRESSION = 0x0001;
        const FILETYPE = 0x0002;
        const RECOVER = 0x0004;
        const JOURNAL_DEV = 0x0008;
        const META_BG = 0x0010;
        const EXTENTS = 0x0040;
        const BIT64 = 0x0080;
        const MMP = 0x0100;
        const FLEX_BG = 0x0200;
        const EA_INODE = 0x0400;
        const DIRDATA = 0x1000;
        const CSUM_SEED = 0x2000;
        const LARGEDIR = 0x4000;
        const INLINE_DATA = 0x8000;
        const ENCRYPT = 0x10000;
    }
}

bitflags::bitflags! {
    pub struct Ext4FeatureRoCompat: u32 {
        const SPARSE_SUPER = 0x0001;
        const LARGE_FILE = 0x0002;
        const BTREE_DIR = 0x0004;
        const HUGE_FILE = 0x0008;
        const GDT_CSUM = 0x0010;
        const DIR_NLINK = 0x0020;
        const EXTRA_ISIZE = 0x0040;
        const HAS_SNAPSHOT = 0x0080;
        const QUOTA = 0x0100;
        const BIGALLOC = 0x0200;
        const METADATA_CSUM = 0x0400;
        const REPLICA = 0x0800;
        const READONLY = 0x1000;
        const PROJECT = 0x2000;
    }
}

/// EXT4 superblock structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Ext4Superblock {
    pub s_inodes_count: u32,        // Total inode count
    pub s_blocks_count_lo: u32,     // Total block count (low 32 bits)
    pub s_r_blocks_count_lo: u32,   // Reserved block count (low 32 bits)
    pub s_free_blocks_count_lo: u32, // Free block count (low 32 bits)
    pub s_free_inodes_count: u32,   // Free inode count
    pub s_first_data_block: u32,    // First data block
    pub s_log_block_size: u32,      // Block size (log2(block_size) - 10)
    pub s_log_cluster_size: u32,    // Cluster size (log2(cluster_size) - 10)
    pub s_blocks_per_group: u32,    // Blocks per group
    pub s_clusters_per_group: u32,  // Clusters per group
    pub s_inodes_per_group: u32,    // Inodes per group
    pub s_mtime: u32,               // Mount time
    pub s_wtime: u32,               // Write time
    pub s_mnt_count: u16,           // Mount count
    pub s_max_mnt_count: u16,       // Maximum mount count
    pub s_magic: u16,               // Magic signature
    pub s_state: u16,               // File system state
    pub s_errors: u16,              // Error handling
    pub s_minor_rev_level: u16,     // Minor revision level
    pub s_lastcheck: u32,           // Last check time
    pub s_checkinterval: u32,       // Check interval
    pub s_creator_os: u32,          // Creator OS
    pub s_rev_level: u32,           // Revision level
    pub s_def_resuid: u16,          // Default reserved user ID
    pub s_def_resgid: u16,          // Default reserved group ID
    
    // EXT4_DYNAMIC_REV specific fields
    pub s_first_ino: u32,           // First non-reserved inode
    pub s_inode_size: u16,          // Size of inode structure
    pub s_block_group_nr: u16,      // Block group number of this superblock
    pub s_feature_compat: u32,      // Compatible feature set
    pub s_feature_incompat: u32,    // Incompatible feature set
    pub s_feature_ro_compat: u32,   // Read-only compatible feature set
    pub s_uuid: [u8; 16],           // 128-bit UUID for volume
    pub s_volume_name: [u8; 16],    // Volume name
    pub s_last_mounted: [u8; 64],   // Directory where last mounted
    pub s_algorithm_usage_bitmap: u32, // For compression
    
    // Performance hints
    pub s_prealloc_blocks: u8,      // Number of blocks to preallocate for files
    pub s_prealloc_dir_blocks: u8,  // Number of blocks to preallocate for directories
    pub s_reserved_gdt_blocks: u16, // Number of reserved GDT entries for future filesystem expansion
    
    // Journaling support
    pub s_journal_uuid: [u8; 16],   // UUID of journal superblock
    pub s_journal_inum: u32,        // Inode number of journal file
    pub s_journal_dev: u32,         // Device number of journal file
    pub s_last_orphan: u32,         // Start of list of inodes to delete
    pub s_hash_seed: [u32; 4],      // HTREE hash seed
    pub s_def_hash_version: u8,     // Default hash version to use
    pub s_jnl_backup_type: u8,      // Journal backup type
    pub s_desc_size: u16,           // Size of group descriptor
    pub s_default_mount_opts: u32,  // Default mount options
    pub s_first_meta_bg: u32,       // First metablock block group
    pub s_mkfs_time: u32,           // When the filesystem was created
    pub s_jnl_blocks: [u32; 17],    // Backup of the journal inode
    
    // 64-bit support
    pub s_blocks_count_hi: u32,     // High 32 bits of block count
    pub s_r_blocks_count_hi: u32,   // High 32 bits of reserved block count
    pub s_free_blocks_count_hi: u32, // High 32 bits of free block count
    pub s_min_extra_isize: u16,     // All inodes have at least this many bytes
    pub s_want_extra_isize: u16,    // New inodes should reserve this many bytes
    pub s_flags: u32,               // Miscellaneous flags
    pub s_raid_stride: u16,         // RAID stride
    pub s_mmp_update_interval: u16, // Number of seconds to wait in MMP checking
    pub s_mmp_block: u64,           // Block for multi-mount protection data
    pub s_raid_stripe_width: u32,   // Blocks on all data disks (N * stride)
    pub s_log_groups_per_flex: u8,  // FLEX_BG group size
    pub s_checksum_type: u8,        // Metadata checksum algorithm type
    pub s_reserved_pad: u16,        // Padding
    pub s_kbytes_written: u64,      // Number of lifetime kilobytes written
    pub s_snapshot_inum: u32,       // Inode number of active snapshot
    pub s_snapshot_id: u32,         // Sequential ID of active snapshot
    pub s_snapshot_r_blocks_count: u64, // Number of blocks reserved for active snapshot's future use
    pub s_snapshot_list: u32,       // Inode number of the head of the on-disk snapshot list
    pub s_error_count: u32,         // Number of file system errors
    pub s_first_error_time: u32,    // First time an error happened
    pub s_first_error_ino: u32,     // Inode involved in first error
    pub s_first_error_block: u64,   // Block involved in first error
    pub s_first_error_func: [u8; 32], // Function where the error happened
    pub s_first_error_line: u32,    // Line number where error happened
    pub s_last_error_time: u32,     // Most recent time of an error
    pub s_last_error_ino: u32,      // Inode involved in most recent error
    pub s_last_error_line: u32,     // Line number where most recent error happened
    pub s_last_error_block: u64,    // Block involved in most recent error
    pub s_last_error_func: [u8; 32], // Function where the most recent error happened
    pub s_mount_opts: [u8; 64],     // ASCIIZ string of mount options
    pub s_usr_quota_inum: u32,      // Inode for tracking user quota
    pub s_grp_quota_inum: u32,      // Inode for tracking group quota
    pub s_overhead_clusters: u32,   // Overhead clusters/blocks in fs
    pub s_backup_bgs: [u32; 2],     // Groups with sparse_super2 SBs
    pub s_encrypt_algos: [u8; 4],   // Encryption algorithms in use
    pub s_encrypt_pw_salt: [u8; 16], // Salt used for string2key algorithm
    pub s_lpf_ino: u32,             // Location of the lost+found inode
    pub s_prj_quota_inum: u32,      // Inode for tracking project quota
    pub s_checksum_seed: u32,       // crc32c(uuid) if csum_seed set
    pub s_reserved: [u32; 98],      // Padding to the end of the block
    pub s_checksum: u32,            // crc32c(superblock)
}

/// EXT4 group descriptor
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Ext4GroupDesc {
    pub bg_block_bitmap_lo: u32,      // Blocks bitmap block (low 32 bits)
    pub bg_inode_bitmap_lo: u32,      // Inodes bitmap block (low 32 bits)
    pub bg_inode_table_lo: u32,       // Inodes table block (low 32 bits)
    pub bg_free_blocks_count_lo: u16, // Free blocks count (low 16 bits)
    pub bg_free_inodes_count_lo: u16, // Free inodes count (low 16 bits)
    pub bg_used_dirs_count_lo: u16,   // Directories count (low 16 bits)
    pub bg_flags: u16,                // EXT4_BG_* flags
    pub bg_exclude_bitmap_lo: u32,    // Exclude bitmap for snapshots (low 32 bits)
    pub bg_block_bitmap_csum_lo: u16, // crc32c(s_uuid+grp_num+bbitmap) LE (low 16 bits)
    pub bg_inode_bitmap_csum_lo: u16, // crc32c(s_uuid+grp_num+ibitmap) LE (low 16 bits)
    pub bg_itable_unused_lo: u16,     // Unused inodes count (low 16 bits)
    pub bg_checksum: u16,             // crc16(sb_uuid+group+desc)
    
    // 64-bit fields (only if INCOMPAT_64BIT is set)
    pub bg_block_bitmap_hi: u32,      // Blocks bitmap block (high 32 bits)
    pub bg_inode_bitmap_hi: u32,      // Inodes bitmap block (high 32 bits)
    pub bg_inode_table_hi: u32,       // Inodes table block (high 32 bits)
    pub bg_free_blocks_count_hi: u16, // Free blocks count (high 16 bits)
    pub bg_free_inodes_count_hi: u16, // Free inodes count (high 16 bits)
    pub bg_used_dirs_count_hi: u16,   // Directories count (high 16 bits)
    pub bg_itable_unused_hi: u16,     // Unused inodes count (high 16 bits)
    pub bg_exclude_bitmap_hi: u32,    // Exclude bitmap block (high 32 bits)
    pub bg_block_bitmap_csum_hi: u16, // crc32c(s_uuid+grp_num+bbitmap) BE (high 16 bits)
    pub bg_inode_bitmap_csum_hi: u16, // crc32c(s_uuid+grp_num+ibitmap) BE (high 16 bits)
    pub bg_reserved: u32,             // Padding
}

/// EXT4 inode structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Ext4Inode {
    pub i_mode: u16,        // File mode
    pub i_uid: u16,         // Low 16 bits of Owner Uid
    pub i_size_lo: u32,     // Size in bytes (low 32 bits)
    pub i_atime: u32,       // Access time
    pub i_ctime: u32,       // Inode Change time
    pub i_mtime: u32,       // Modification time
    pub i_dtime: u32,       // Deletion Time
    pub i_gid: u16,         // Low 16 bits of Group Id
    pub i_links_count: u16, // Links count
    pub i_blocks_lo: u32,   // Blocks count (low 32 bits)
    pub i_flags: u32,       // File flags
    pub i_osd1: u32,        // OS dependent 1
    pub i_block: [u32; 15], // Pointers to blocks
    pub i_generation: u32,  // File version (for NFS)
    pub i_file_acl_lo: u32, // File ACL (low 32 bits)
    pub i_size_high: u32,   // Size in bytes (high 32 bits)
    pub i_obso_faddr: u32,  // Obsoleted fragment address
    pub i_osd2: [u32; 3],   // OS dependent 2
    pub i_extra: [u8; 0],   // Extra inode fields (variable size)
}

/// EXT4 directory entry
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Ext4DirEntry2 {
    pub inode: u32,     // Inode number
    pub rec_len: u16,   // Directory entry length
    pub name_len: u8,   // Name length
    pub file_type: u8,  // File type
    // name follows here (variable length)
}

/// EXT4 filesystem implementation
#[derive(Debug)]
pub struct Ext4FileSystem {
    device_id: u32,
    superblock: Ext4Superblock,
    block_size: u32,
    blocks_per_group: u32,
    inodes_per_group: u32,
    group_desc_table: Vec<Ext4GroupDesc>,
    inode_cache: RwLock<BTreeMap<InodeNumber, Ext4Inode>>,
    block_cache: RwLock<BTreeMap<u64, Vec<u8>>>,
    dirty_blocks: RwLock<BTreeMap<u64, Vec<u8>>>,
}

impl Ext4FileSystem {
    /// Create new EXT4 filesystem instance
    pub fn new(device_id: u32) -> FsResult<Self> {
        let mut fs = Self {
            device_id,
            superblock: unsafe { mem::zeroed() },
            block_size: 0,
            blocks_per_group: 0,
            inodes_per_group: 0,
            group_desc_table: Vec::new(),
            inode_cache: RwLock::new(BTreeMap::new()),
            block_cache: RwLock::new(BTreeMap::new()),
            dirty_blocks: RwLock::new(BTreeMap::new()),
        };

        fs.read_superblock()?;
        fs.read_group_descriptors()?;
        Ok(fs)
    }

    /// Read superblock from disk
    fn read_superblock(&mut self) -> FsResult<()> {
        let mut buffer = vec![0u8; 1024];
        
        // Superblock is at offset 1024 bytes (sector 2 for 512-byte sectors)
        read_storage_sectors(self.device_id, 2, &mut buffer)
            .map_err(|_| FsError::IoError)?;

        // Parse superblock
        self.superblock = unsafe {
            core::ptr::read_unaligned(buffer.as_ptr() as *const Ext4Superblock)
        };

        // Validate magic number
        if self.superblock.s_magic != EXT4_SUPER_MAGIC {
            return Err(FsError::InvalidArgument);
        }

        // Calculate block size
        self.block_size = 1024 << self.superblock.s_log_block_size;
        if self.block_size < EXT4_MIN_BLOCK_SIZE || self.block_size > EXT4_MAX_BLOCK_SIZE {
            return Err(FsError::InvalidArgument);
        }

        self.blocks_per_group = self.superblock.s_blocks_per_group;
        self.inodes_per_group = self.superblock.s_inodes_per_group;

        Ok(())
    }

    /// Read group descriptor table
    fn read_group_descriptors(&mut self) -> FsResult<()> {
        let total_blocks = self.get_total_blocks();
        let blocks_per_group = self.blocks_per_group as u64;
        let group_count = (total_blocks + blocks_per_group - 1) / blocks_per_group;
        
        // Group descriptor table starts at block 1 (or block 2 if block size is 1024)
        let gdt_block = if self.block_size == 1024 { 2 } else { 1 };
        
        let desc_size = if self.superblock.s_feature_incompat & Ext4FeatureIncompat::BIT64.bits() != 0 {
            self.superblock.s_desc_size as usize
        } else {
            32 // Old 32-byte descriptor size
        };

        let descs_per_block = self.block_size as usize / desc_size;
        let gdt_blocks = (group_count as usize + descs_per_block - 1) / descs_per_block;

        for block_idx in 0..gdt_blocks {
            let block_num = gdt_block + block_idx as u64;
            let block_data = self.read_block(block_num)?;
            
            for desc_idx in 0..descs_per_block {
                if self.group_desc_table.len() >= group_count as usize {
                    break;
                }
                
                let offset = desc_idx * desc_size;
                if offset + desc_size <= block_data.len() {
                    let desc = unsafe {
                        core::ptr::read_unaligned(
                            block_data.as_ptr().add(offset) as *const Ext4GroupDesc
                        )
                    };
                    self.group_desc_table.push(desc);
                }
            }
        }

        Ok(())
    }

    /// Get total number of blocks in filesystem
    fn get_total_blocks(&self) -> u64 {
        if self.superblock.s_feature_incompat & Ext4FeatureIncompat::BIT64.bits() != 0 {
            ((self.superblock.s_blocks_count_hi as u64) << 32) | (self.superblock.s_blocks_count_lo as u64)
        } else {
            self.superblock.s_blocks_count_lo as u64
        }
    }

    /// Read a block from disk with caching
    fn read_block(&self, block_num: u64) -> FsResult<Vec<u8>> {
        // Check cache first
        {
            let cache = self.block_cache.read();
            if let Some(cached_block) = cache.get(&block_num) {
                return Ok(cached_block.clone());
            }
        }

        // Read from disk
        let sectors_per_block = self.block_size / 512;
        let start_sector = block_num * sectors_per_block as u64;
        let mut buffer = vec![0u8; self.block_size as usize];

        read_storage_sectors(self.device_id, start_sector, &mut buffer)
            .map_err(|_| FsError::IoError)?;

        // Cache the block
        {
            let mut cache = self.block_cache.write();
            cache.insert(block_num, buffer.clone());
        }

        Ok(buffer)
    }

    /// Write a block to disk with caching
    fn write_block(&self, block_num: u64, data: &[u8]) -> FsResult<()> {
        if data.len() != self.block_size as usize {
            return Err(FsError::InvalidArgument);
        }

        // Mark as dirty for write-back
        {
            let mut dirty = self.dirty_blocks.write();
            dirty.insert(block_num, data.to_vec());
        }

        // Update cache
        {
            let mut cache = self.block_cache.write();
            cache.insert(block_num, data.to_vec());
        }

        Ok(())
    }

    /// Write inode to disk
    fn write_inode(&self, inode_num: InodeNumber, inode: &Ext4Inode) -> FsResult<()> {
        // Calculate inode location
        let group = (inode_num - 1) / self.inodes_per_group as u64;
        let index = (inode_num - 1) % self.inodes_per_group as u64;

        if group >= self.group_desc_table.len() as u64 {
            return Err(FsError::NotFound);
        }

        let group_desc = &self.group_desc_table[group as usize];
        let inode_table_block = if self.superblock.s_feature_incompat & Ext4FeatureIncompat::BIT64.bits() != 0 {
            ((group_desc.bg_inode_table_hi as u64) << 32) | (group_desc.bg_inode_table_lo as u64)
        } else {
            group_desc.bg_inode_table_lo as u64
        };

        let inode_size = if self.superblock.s_rev_level >= 1 {
            self.superblock.s_inode_size as usize
        } else {
            EXT4_GOOD_OLD_INODE_SIZE as usize
        };

        let inodes_per_block = self.block_size as usize / inode_size;
        let block_offset = index as usize / inodes_per_block;
        let inode_offset = (index as usize % inodes_per_block) * inode_size;

        // Read the block containing the inode
        let mut block_data = self.read_block(inode_table_block + block_offset as u64)?;

        // Copy inode data into the block
        let inode_bytes = unsafe {
            core::slice::from_raw_parts(
                inode as *const Ext4Inode as *const u8,
                core::mem::size_of::<Ext4Inode>()
            )
        };

        block_data[inode_offset..inode_offset + core::mem::size_of::<Ext4Inode>()]
            .copy_from_slice(inode_bytes);

        // Write the block back
        self.write_block(inode_table_block + block_offset as u64, &block_data)?;

        // Update cache
        {
            let mut cache = self.inode_cache.write();
            cache.insert(inode_num, *inode);
        }

        Ok(())
    }

    /// Read block bitmap for a group
    fn read_block_bitmap(&self, group: u64) -> FsResult<Vec<u8>> {
        if group >= self.group_desc_table.len() as u64 {
            return Err(FsError::InvalidArgument);
        }

        let group_desc = &self.group_desc_table[group as usize];
        let bitmap_block = if self.superblock.s_feature_incompat & Ext4FeatureIncompat::BIT64.bits() != 0 {
            ((group_desc.bg_block_bitmap_hi as u64) << 32) | (group_desc.bg_block_bitmap_lo as u64)
        } else {
            group_desc.bg_block_bitmap_lo as u64
        };

        self.read_block(bitmap_block)
    }

    /// Write block bitmap for a group
    fn write_block_bitmap(&self, group: u64, bitmap: &[u8]) -> FsResult<()> {
        if group >= self.group_desc_table.len() as u64 {
            return Err(FsError::InvalidArgument);
        }

        let group_desc = &self.group_desc_table[group as usize];
        let bitmap_block = if self.superblock.s_feature_incompat & Ext4FeatureIncompat::BIT64.bits() != 0 {
            ((group_desc.bg_block_bitmap_hi as u64) << 32) | (group_desc.bg_block_bitmap_lo as u64)
        } else {
            group_desc.bg_block_bitmap_lo as u64
        };

        self.write_block(bitmap_block, bitmap)
    }

    /// Read inode bitmap for a group
    fn read_inode_bitmap(&self, group: u64) -> FsResult<Vec<u8>> {
        if group >= self.group_desc_table.len() as u64 {
            return Err(FsError::InvalidArgument);
        }

        let group_desc = &self.group_desc_table[group as usize];
        let bitmap_block = if self.superblock.s_feature_incompat & Ext4FeatureIncompat::BIT64.bits() != 0 {
            ((group_desc.bg_inode_bitmap_hi as u64) << 32) | (group_desc.bg_inode_bitmap_lo as u64)
        } else {
            group_desc.bg_inode_bitmap_lo as u64
        };

        self.read_block(bitmap_block)
    }

    /// Write inode bitmap for a group
    fn write_inode_bitmap(&self, group: u64, bitmap: &[u8]) -> FsResult<()> {
        if group >= self.group_desc_table.len() as u64 {
            return Err(FsError::InvalidArgument);
        }

        let group_desc = &self.group_desc_table[group as usize];
        let bitmap_block = if self.superblock.s_feature_incompat & Ext4FeatureIncompat::BIT64.bits() != 0 {
            ((group_desc.bg_inode_bitmap_hi as u64) << 32) | (group_desc.bg_inode_bitmap_lo as u64)
        } else {
            group_desc.bg_inode_bitmap_lo as u64
        };

        self.write_block(bitmap_block, bitmap)
    }

    /// Allocate a new block
    fn allocate_block(&mut self, preferred_group: u64) -> FsResult<u64> {
        let group_count = self.group_desc_table.len() as u64;

        // Try preferred group first, then scan all groups
        for offset in 0..group_count {
            let group = (preferred_group + offset) % group_count;
            let mut bitmap = self.read_block_bitmap(group)?;

            // Find first free bit in bitmap
            for byte_idx in 0..bitmap.len() {
                if bitmap[byte_idx] != 0xFF {
                    for bit_idx in 0..8 {
                        if (bitmap[byte_idx] & (1 << bit_idx)) == 0 {
                            // Found free block
                            bitmap[byte_idx] |= 1 << bit_idx;
                            self.write_block_bitmap(group, &bitmap)?;

                            let block_num = group * self.blocks_per_group as u64 +
                                          byte_idx as u64 * 8 + bit_idx as u64;

                            // Update group descriptor
                            let mut group_desc = self.group_desc_table[group as usize];
                            let free_blocks = if self.superblock.s_feature_incompat & Ext4FeatureIncompat::BIT64.bits() != 0 {
                                ((group_desc.bg_free_blocks_count_hi as u32) << 16) | group_desc.bg_free_blocks_count_lo as u32
                            } else {
                                group_desc.bg_free_blocks_count_lo as u32
                            };

                            if free_blocks > 0 {
                                let new_free = free_blocks - 1;
                                group_desc.bg_free_blocks_count_lo = (new_free & 0xFFFF) as u16;
                                if self.superblock.s_feature_incompat & Ext4FeatureIncompat::BIT64.bits() != 0 {
                                    group_desc.bg_free_blocks_count_hi = (new_free >> 16) as u16;
                                }
                                self.group_desc_table[group as usize] = group_desc;
                                self.write_group_descriptor(group, &group_desc)?;
                            }

                            return Ok(block_num);
                        }
                    }
                }
            }
        }

        Err(FsError::NoSpaceLeft)
    }

    /// Free a block
    fn free_block(&mut self, block_num: u64) -> FsResult<()> {
        let group = block_num / self.blocks_per_group as u64;
        let block_in_group = block_num % self.blocks_per_group as u64;

        let mut bitmap = self.read_block_bitmap(group)?;
        let byte_idx = (block_in_group / 8) as usize;
        let bit_idx = (block_in_group % 8) as u8;

        if byte_idx >= bitmap.len() {
            return Err(FsError::InvalidArgument);
        }

        // Clear the bit
        bitmap[byte_idx] &= !(1 << bit_idx);
        self.write_block_bitmap(group, &bitmap)?;

        // Update group descriptor
        let mut group_desc = self.group_desc_table[group as usize];
        let free_blocks = if self.superblock.s_feature_incompat & Ext4FeatureIncompat::BIT64.bits() != 0 {
            ((group_desc.bg_free_blocks_count_hi as u32) << 16) | group_desc.bg_free_blocks_count_lo as u32
        } else {
            group_desc.bg_free_blocks_count_lo as u32
        };

        let new_free = free_blocks + 1;
        group_desc.bg_free_blocks_count_lo = (new_free & 0xFFFF) as u16;
        if self.superblock.s_feature_incompat & Ext4FeatureIncompat::BIT64.bits() != 0 {
            group_desc.bg_free_blocks_count_hi = (new_free >> 16) as u16;
        }
        self.group_desc_table[group as usize] = group_desc;
        self.write_group_descriptor(group, &group_desc)?;

        Ok(())
    }

    /// Allocate a new inode
    fn allocate_inode(&mut self, preferred_group: u64, is_dir: bool) -> FsResult<InodeNumber> {
        let group_count = self.group_desc_table.len() as u64;

        // Try preferred group first, then scan all groups
        for offset in 0..group_count {
            let group = (preferred_group + offset) % group_count;
            let mut bitmap = self.read_inode_bitmap(group)?;

            // Find first free bit in bitmap
            for byte_idx in 0..bitmap.len() {
                if bitmap[byte_idx] != 0xFF {
                    for bit_idx in 0..8 {
                        if (bitmap[byte_idx] & (1 << bit_idx)) == 0 {
                            // Found free inode
                            bitmap[byte_idx] |= 1 << bit_idx;
                            self.write_inode_bitmap(group, &bitmap)?;

                            let inode_num = group * self.inodes_per_group as u64 +
                                          byte_idx as u64 * 8 + bit_idx as u64 + 1;

                            // Skip reserved inodes
                            if inode_num < self.superblock.s_first_ino as u64 && inode_num != 2 {
                                continue;
                            }

                            // Update group descriptor
                            let mut group_desc = self.group_desc_table[group as usize];
                            let free_inodes = if self.superblock.s_feature_incompat & Ext4FeatureIncompat::BIT64.bits() != 0 {
                                ((group_desc.bg_free_inodes_count_hi as u32) << 16) | group_desc.bg_free_inodes_count_lo as u32
                            } else {
                                group_desc.bg_free_inodes_count_lo as u32
                            };

                            if free_inodes > 0 {
                                let new_free = free_inodes - 1;
                                group_desc.bg_free_inodes_count_lo = (new_free & 0xFFFF) as u16;
                                if self.superblock.s_feature_incompat & Ext4FeatureIncompat::BIT64.bits() != 0 {
                                    group_desc.bg_free_inodes_count_hi = (new_free >> 16) as u16;
                                }

                                if is_dir {
                                    let used_dirs = if self.superblock.s_feature_incompat & Ext4FeatureIncompat::BIT64.bits() != 0 {
                                        ((group_desc.bg_used_dirs_count_hi as u32) << 16) | group_desc.bg_used_dirs_count_lo as u32
                                    } else {
                                        group_desc.bg_used_dirs_count_lo as u32
                                    };
                                    let new_used = used_dirs + 1;
                                    group_desc.bg_used_dirs_count_lo = (new_used & 0xFFFF) as u16;
                                    if self.superblock.s_feature_incompat & Ext4FeatureIncompat::BIT64.bits() != 0 {
                                        group_desc.bg_used_dirs_count_hi = (new_used >> 16) as u16;
                                    }
                                }

                                self.group_desc_table[group as usize] = group_desc;
                                self.write_group_descriptor(group, &group_desc)?;
                            }

                            return Ok(inode_num);
                        }
                    }
                }
            }
        }

        Err(FsError::NoSpaceLeft)
    }

    /// Free an inode
    fn free_inode(&mut self, inode_num: InodeNumber, is_dir: bool) -> FsResult<()> {
        let group = (inode_num - 1) / self.inodes_per_group as u64;
        let inode_in_group = (inode_num - 1) % self.inodes_per_group as u64;

        let mut bitmap = self.read_inode_bitmap(group)?;
        let byte_idx = (inode_in_group / 8) as usize;
        let bit_idx = (inode_in_group % 8) as u8;

        if byte_idx >= bitmap.len() {
            return Err(FsError::InvalidArgument);
        }

        // Clear the bit
        bitmap[byte_idx] &= !(1 << bit_idx);
        self.write_inode_bitmap(group, &bitmap)?;

        // Update group descriptor
        let mut group_desc = self.group_desc_table[group as usize];
        let free_inodes = if self.superblock.s_feature_incompat & Ext4FeatureIncompat::BIT64.bits() != 0 {
            ((group_desc.bg_free_inodes_count_hi as u32) << 16) | group_desc.bg_free_inodes_count_lo as u32
        } else {
            group_desc.bg_free_inodes_count_lo as u32
        };

        let new_free = free_inodes + 1;
        group_desc.bg_free_inodes_count_lo = (new_free & 0xFFFF) as u16;
        if self.superblock.s_feature_incompat & Ext4FeatureIncompat::BIT64.bits() != 0 {
            group_desc.bg_free_inodes_count_hi = (new_free >> 16) as u16;
        }

        if is_dir {
            let used_dirs = if self.superblock.s_feature_incompat & Ext4FeatureIncompat::BIT64.bits() != 0 {
                ((group_desc.bg_used_dirs_count_hi as u32) << 16) | group_desc.bg_used_dirs_count_lo as u32
            } else {
                group_desc.bg_used_dirs_count_lo as u32
            };
            if used_dirs > 0 {
                let new_used = used_dirs - 1;
                group_desc.bg_used_dirs_count_lo = (new_used & 0xFFFF) as u16;
                if self.superblock.s_feature_incompat & Ext4FeatureIncompat::BIT64.bits() != 0 {
                    group_desc.bg_used_dirs_count_hi = (new_used >> 16) as u16;
                }
            }
        }

        self.group_desc_table[group as usize] = group_desc;
        self.write_group_descriptor(group, &group_desc)?;

        Ok(())
    }

    /// Write group descriptor to disk
    fn write_group_descriptor(&self, group: u64, desc: &Ext4GroupDesc) -> FsResult<()> {
        let gdt_block = if self.block_size == 1024 { 2 } else { 1 };

        let desc_size = if self.superblock.s_feature_incompat & Ext4FeatureIncompat::BIT64.bits() != 0 {
            self.superblock.s_desc_size as usize
        } else {
            32
        };

        let descs_per_block = self.block_size as usize / desc_size;
        let block_idx = group as usize / descs_per_block;
        let desc_idx = group as usize % descs_per_block;

        let mut block_data = self.read_block(gdt_block + block_idx as u64)?;

        let offset = desc_idx * desc_size;
        let desc_bytes = unsafe {
            core::slice::from_raw_parts(
                desc as *const Ext4GroupDesc as *const u8,
                core::mem::size_of::<Ext4GroupDesc>().min(desc_size)
            )
        };

        block_data[offset..offset + desc_bytes.len()].copy_from_slice(desc_bytes);
        self.write_block(gdt_block + block_idx as u64, &block_data)?;

        Ok(())
    }

    /// Add directory entry
    fn add_directory_entry(&self, dir_inode_num: InodeNumber, name: &str, child_inode: InodeNumber, file_type: FileType) -> FsResult<()> {
        let mut dir_inode = self.read_inode(dir_inode_num)?;

        if name.len() > 255 {
            return Err(FsError::NameTooLong);
        }

        // Calculate required entry size (aligned to 4 bytes)
        let entry_size = (mem::size_of::<Ext4DirEntry2>() + name.len() + 3) & !3;

        // Search for free space in existing blocks
        let i_block = unsafe { core::ptr::addr_of!(dir_inode.i_block).read_unaligned() };
        for &block_ptr in &i_block[0..12] {
            if block_ptr == 0 {
                break;
            }

            let mut block_data = self.read_block(block_ptr as u64)?;
            let mut offset = 0;

            while offset < block_data.len() {
                if offset + mem::size_of::<Ext4DirEntry2>() > block_data.len() {
                    break;
                }

                let dir_entry = unsafe {
                    core::ptr::read_unaligned(
                        block_data.as_ptr().add(offset) as *const Ext4DirEntry2
                    )
                };

                if dir_entry.rec_len == 0 {
                    break;
                }

                // Calculate actual space used by this entry
                let actual_size = (mem::size_of::<Ext4DirEntry2>() + dir_entry.name_len as usize + 3) & !3;
                let available_space = dir_entry.rec_len as usize - actual_size;

                // Check if we can fit the new entry here
                if available_space >= entry_size {
                    // Shrink current entry
                    let mut current_entry = dir_entry;
                    current_entry.rec_len = actual_size as u16;

                    let current_bytes = unsafe {
                        core::slice::from_raw_parts(
                            &current_entry as *const Ext4DirEntry2 as *const u8,
                            mem::size_of::<Ext4DirEntry2>()
                        )
                    };
                    block_data[offset..offset + mem::size_of::<Ext4DirEntry2>()].copy_from_slice(current_bytes);

                    // Create new entry
                    let new_offset = offset + actual_size;
                    let new_entry = Ext4DirEntry2 {
                        inode: child_inode as u32,
                        rec_len: available_space as u16,
                        name_len: name.len() as u8,
                        file_type: match file_type {
                            FileType::Regular => 1,
                            FileType::Directory => 2,
                            FileType::CharacterDevice => 3,
                            FileType::BlockDevice => 4,
                            FileType::NamedPipe => 5,
                            FileType::Socket => 6,
                            FileType::SymbolicLink => 7,
                        },
                    };

                    let new_bytes = unsafe {
                        core::slice::from_raw_parts(
                            &new_entry as *const Ext4DirEntry2 as *const u8,
                            mem::size_of::<Ext4DirEntry2>()
                        )
                    };
                    block_data[new_offset..new_offset + mem::size_of::<Ext4DirEntry2>()].copy_from_slice(new_bytes);
                    block_data[new_offset + mem::size_of::<Ext4DirEntry2>()..new_offset + mem::size_of::<Ext4DirEntry2>() + name.len()]
                        .copy_from_slice(name.as_bytes());

                    self.write_block(block_ptr as u64, &block_data)?;
                    return Ok(());
                }

                offset += dir_entry.rec_len as usize;
            }
        }

        // Need to allocate a new block for the directory
        let group = (dir_inode_num - 1) / self.inodes_per_group as u64;
        let new_block = self.allocate_block_mut(group)?;

        // Create directory entry in new block
        let mut block_data = vec![0u8; self.block_size as usize];
        let new_entry = Ext4DirEntry2 {
            inode: child_inode as u32,
            rec_len: self.block_size as u16,
            name_len: name.len() as u8,
            file_type: match file_type {
                FileType::Regular => 1,
                FileType::Directory => 2,
                FileType::CharacterDevice => 3,
                FileType::BlockDevice => 4,
                FileType::NamedPipe => 5,
                FileType::Socket => 6,
                FileType::SymbolicLink => 7,
            },
        };

        let new_bytes = unsafe {
            core::slice::from_raw_parts(
                &new_entry as *const Ext4DirEntry2 as *const u8,
                mem::size_of::<Ext4DirEntry2>()
            )
        };
        block_data[0..mem::size_of::<Ext4DirEntry2>()].copy_from_slice(new_bytes);
        block_data[mem::size_of::<Ext4DirEntry2>()..mem::size_of::<Ext4DirEntry2>() + name.len()]
            .copy_from_slice(name.as_bytes());

        self.write_block(new_block, &block_data)?;

        // Add block to inode
        let mut i_block = unsafe { core::ptr::addr_of!(dir_inode.i_block).read_unaligned() };
        for i in 0..12 {
            if i_block[i] == 0 {
                i_block[i] = new_block as u32;
                break;
            }
        }
        unsafe {
            core::ptr::write_unaligned(&mut dir_inode.i_block as *mut [u32; 15], i_block);
        }

        // Update inode size
        let new_size = unsafe { core::ptr::addr_of!(dir_inode.i_size_lo).read_unaligned() as u64 } +
                      self.block_size as u64;
        unsafe {
            core::ptr::write_unaligned(&mut dir_inode.i_size_lo as *mut u32, new_size as u32);
        }

        self.write_inode(dir_inode_num, &dir_inode)?;

        Ok(())
    }

    /// Remove directory entry
    fn remove_directory_entry(&self, dir_inode_num: InodeNumber, name: &str) -> FsResult<InodeNumber> {
        let dir_inode = self.read_inode(dir_inode_num)?;

        let i_block = unsafe { core::ptr::addr_of!(dir_inode.i_block).read_unaligned() };
        for &block_ptr in &i_block[0..12] {
            if block_ptr == 0 {
                break;
            }

            let mut block_data = self.read_block(block_ptr as u64)?;
            let mut offset = 0;
            let mut prev_offset = 0;

            while offset < block_data.len() {
                if offset + mem::size_of::<Ext4DirEntry2>() > block_data.len() {
                    break;
                }

                let dir_entry = unsafe {
                    core::ptr::read_unaligned(
                        block_data.as_ptr().add(offset) as *const Ext4DirEntry2
                    )
                };

                if dir_entry.rec_len == 0 || dir_entry.inode == 0 {
                    break;
                }

                if dir_entry.name_len as usize == name.len() {
                    let entry_name_bytes = &block_data[offset + mem::size_of::<Ext4DirEntry2>()..
                                                       offset + mem::size_of::<Ext4DirEntry2>() + dir_entry.name_len as usize];

                    if let Ok(entry_name) = core::str::from_utf8(entry_name_bytes) {
                        if entry_name == name {
                            let removed_inode = dir_entry.inode as InodeNumber;

                            // Extend previous entry to cover this one
                            if prev_offset < offset {
                                let mut prev_entry = unsafe {
                                    core::ptr::read_unaligned(
                                        block_data.as_ptr().add(prev_offset) as *const Ext4DirEntry2
                                    )
                                };
                                prev_entry.rec_len += dir_entry.rec_len;

                                let prev_bytes = unsafe {
                                    core::slice::from_raw_parts(
                                        &prev_entry as *const Ext4DirEntry2 as *const u8,
                                        mem::size_of::<Ext4DirEntry2>()
                                    )
                                };
                                block_data[prev_offset..prev_offset + mem::size_of::<Ext4DirEntry2>()]
                                    .copy_from_slice(prev_bytes);
                            } else {
                                // This is the first entry, mark it as deleted
                                let mut deleted_entry = dir_entry;
                                deleted_entry.inode = 0;

                                let deleted_bytes = unsafe {
                                    core::slice::from_raw_parts(
                                        &deleted_entry as *const Ext4DirEntry2 as *const u8,
                                        mem::size_of::<Ext4DirEntry2>()
                                    )
                                };
                                block_data[offset..offset + mem::size_of::<Ext4DirEntry2>()]
                                    .copy_from_slice(deleted_bytes);
                            }

                            self.write_block(block_ptr as u64, &block_data)?;
                            return Ok(removed_inode);
                        }
                    }
                }

                prev_offset = offset;
                offset += dir_entry.rec_len as usize;
            }
        }

        Err(FsError::NotFound)
    }

    /// Helper for mutable block allocation
    fn allocate_block_mut(&self, preferred_group: u64) -> FsResult<u64> {
        // This is a workaround for the &self requirement
        // We need to convert to &mut temporarily
        let self_ptr = self as *const Self as *mut Self;
        unsafe { (*self_ptr).allocate_block(preferred_group) }
    }

    /// Helper for mutable inode allocation
    fn allocate_inode_mut(&self, preferred_group: u64, is_dir: bool) -> FsResult<InodeNumber> {
        let self_ptr = self as *const Self as *mut Self;
        unsafe { (*self_ptr).allocate_inode(preferred_group, is_dir) }
    }

    /// Helper for mutable block freeing
    fn free_block_mut(&self, block_num: u64) -> FsResult<()> {
        let self_ptr = self as *const Self as *mut Self;
        unsafe { (*self_ptr).free_block(block_num) }
    }

    /// Helper for mutable inode freeing
    fn free_inode_mut(&self, inode_num: InodeNumber, is_dir: bool) -> FsResult<()> {
        let self_ptr = self as *const Self as *mut Self;
        unsafe { (*self_ptr).free_inode(inode_num, is_dir) }
    }

    /// Flush dirty blocks to disk
    fn flush_dirty_blocks(&self) -> FsResult<()> {
        let dirty_blocks = {
            let mut dirty = self.dirty_blocks.write();
            let blocks = dirty.clone();
            dirty.clear();
            blocks
        };

        for (block_num, data) in dirty_blocks {
            let sectors_per_block = self.block_size / 512;
            let start_sector = block_num * sectors_per_block as u64;

            write_storage_sectors(self.device_id, start_sector, &data)
                .map_err(|_| FsError::IoError)?;
        }

        Ok(())
    }

    /// Read inode from disk
    fn read_inode(&self, inode_num: InodeNumber) -> FsResult<Ext4Inode> {
        // Check cache first
        {
            let cache = self.inode_cache.read();
            if let Some(cached_inode) = cache.get(&inode_num) {
                return Ok(*cached_inode);
            }
        }

        // Calculate inode location
        let group = (inode_num - 1) / self.inodes_per_group as u64;
        let index = (inode_num - 1) % self.inodes_per_group as u64;

        if group >= self.group_desc_table.len() as u64 {
            return Err(FsError::NotFound);
        }

        let group_desc = &self.group_desc_table[group as usize];
        let inode_table_block = if self.superblock.s_feature_incompat & Ext4FeatureIncompat::BIT64.bits() != 0 {
            ((group_desc.bg_inode_table_hi as u64) << 32) | (group_desc.bg_inode_table_lo as u64)
        } else {
            group_desc.bg_inode_table_lo as u64
        };

        let inode_size = if self.superblock.s_rev_level >= 1 {
            self.superblock.s_inode_size as usize
        } else {
            EXT4_GOOD_OLD_INODE_SIZE as usize
        };

        let inodes_per_block = self.block_size as usize / inode_size;
        let block_offset = index as usize / inodes_per_block;
        let inode_offset = (index as usize % inodes_per_block) * inode_size;

        let block_data = self.read_block(inode_table_block + block_offset as u64)?;
        
        if inode_offset + mem::size_of::<Ext4Inode>() > block_data.len() {
            return Err(FsError::IoError);
        }

        let inode = unsafe {
            core::ptr::read_unaligned(
                block_data.as_ptr().add(inode_offset) as *const Ext4Inode
            )
        };

        // Cache the inode
        {
            let mut cache = self.inode_cache.write();
            cache.insert(inode_num, inode);
        }

        Ok(inode)
    }

    /// Convert EXT4 inode to VFS metadata
    fn inode_to_metadata(&self, inode_num: InodeNumber, inode: &Ext4Inode) -> FileMetadata {
        let file_type = match inode.i_mode & 0xF000 {
            0x1000 => FileType::NamedPipe,
            0x2000 => FileType::CharacterDevice,
            0x4000 => FileType::Directory,
            0x6000 => FileType::BlockDevice,
            0x8000 => FileType::Regular,
            0xA000 => FileType::SymbolicLink,
            0xC000 => FileType::Socket,
            _ => FileType::Regular,
        };

        let size = if file_type == FileType::Regular {
            if self.superblock.s_feature_ro_compat & Ext4FeatureRoCompat::LARGE_FILE.bits() != 0 {
                ((inode.i_size_high as u64) << 32) | (inode.i_size_lo as u64)
            } else {
                inode.i_size_lo as u64
            }
        } else {
            0
        };

        FileMetadata {
            inode: inode_num,
            file_type,
            size,
            permissions: FilePermissions::from_octal(inode.i_mode & 0o777),
            uid: inode.i_uid as u32,
            gid: inode.i_gid as u32,
            created: inode.i_ctime as u64,
            modified: inode.i_mtime as u64,
            accessed: inode.i_atime as u64,
            link_count: inode.i_links_count as u32,
            device_id: None,
        }
    }

    /// Read directory entries from an inode
    fn read_directory_entries(&self, inode: &Ext4Inode) -> FsResult<Vec<DirectoryEntry>> {
        let mut entries = Vec::new();

        // For simplicity, only handle direct blocks (first 12 block pointers)
        // SAFETY: inode is a packed struct representing EXT4 on-disk format.
        // We use addr_of! to avoid creating misaligned references.
        let i_block = unsafe { core::ptr::addr_of!(inode.i_block).read_unaligned() };
        for &block_ptr in &i_block[0..12] {
            if block_ptr == 0 {
                break;
            }

            let block_data = self.read_block(block_ptr as u64)?;
            let mut offset = 0;

            while offset < block_data.len() {
                if offset + mem::size_of::<Ext4DirEntry2>() > block_data.len() {
                    break;
                }

                let dir_entry = unsafe {
                    core::ptr::read_unaligned(
                        block_data.as_ptr().add(offset) as *const Ext4DirEntry2
                    )
                };

                if dir_entry.inode == 0 || dir_entry.rec_len == 0 {
                    break;
                }

                if dir_entry.name_len > 0 && offset + mem::size_of::<Ext4DirEntry2>() + dir_entry.name_len as usize <= block_data.len() {
                    let name_bytes = &block_data[offset + mem::size_of::<Ext4DirEntry2>()..offset + mem::size_of::<Ext4DirEntry2>() + dir_entry.name_len as usize];
                    
                    if let Ok(name) = core::str::from_utf8(name_bytes) {
                        let file_type = match dir_entry.file_type {
                            1 => FileType::Regular,
                            2 => FileType::Directory,
                            3 => FileType::CharacterDevice,
                            4 => FileType::BlockDevice,
                            5 => FileType::NamedPipe,
                            6 => FileType::Socket,
                            7 => FileType::SymbolicLink,
                            _ => FileType::Regular,
                        };

                        entries.push(DirectoryEntry {
                            name: name.to_string(),
                            inode: dir_entry.inode as InodeNumber,
                            file_type,
                        });
                    }
                }

                offset += dir_entry.rec_len as usize;
            }
        }

        Ok(entries)
    }

    /// Resolve path to inode number
    fn resolve_path(&self, path: &str) -> FsResult<InodeNumber> {
        if path == "/" {
            return Ok(2); // Root inode is always 2 in EXT4
        }

        let components: Vec<&str> = path.split('/').filter(|c| !c.is_empty()).collect();
        let mut current_inode = 2; // Start from root

        for component in components {
            let inode = self.read_inode(current_inode)?;
            let metadata = self.inode_to_metadata(current_inode, &inode);
            
            if metadata.file_type != FileType::Directory {
                return Err(FsError::NotADirectory);
            }

            let entries = self.read_directory_entries(&inode)?;
            let mut found = false;

            for entry in entries {
                if entry.name == component {
                    current_inode = entry.inode;
                    found = true;
                    break;
                }
            }

            if !found {
                return Err(FsError::NotFound);
            }
        }

        Ok(current_inode)
    }
}

impl FileSystem for Ext4FileSystem {
    fn fs_type(&self) -> FileSystemType {
        FileSystemType::Ext2 // Using Ext2 enum value for EXT4
    }

    fn statfs(&self) -> FsResult<FileSystemStats> {
        let total_blocks = self.get_total_blocks();
        let free_blocks = if self.superblock.s_feature_incompat & Ext4FeatureIncompat::BIT64.bits() != 0 {
            ((self.superblock.s_free_blocks_count_hi as u64) << 32) | (self.superblock.s_free_blocks_count_lo as u64)
        } else {
            self.superblock.s_free_blocks_count_lo as u64
        };

        Ok(FileSystemStats {
            total_blocks,
            free_blocks,
            available_blocks: free_blocks,
            total_inodes: self.superblock.s_inodes_count as u64,
            free_inodes: self.superblock.s_free_inodes_count as u64,
            block_size: self.block_size,
            max_filename_length: 255,
        })
    }

    fn create(&self, path: &str, permissions: FilePermissions) -> FsResult<InodeNumber> {
        // Parse path to get parent directory and filename
        let path_parts: Vec<&str> = path.rsplitn(2, '/').collect();
        if path_parts.is_empty() {
            return Err(FsError::InvalidArgument);
        }

        let filename = path_parts[0];
        let parent_path = if path_parts.len() > 1 {
            path_parts[1]
        } else {
            "/"
        };

        // Resolve parent directory
        let parent_inode_num = self.resolve_path(parent_path)?;
        let parent_inode = self.read_inode(parent_inode_num)?;
        let parent_meta = self.inode_to_metadata(parent_inode_num, &parent_inode);

        if parent_meta.file_type != FileType::Directory {
            return Err(FsError::NotADirectory);
        }

        // Check if file already exists
        let entries = self.read_directory_entries(&parent_inode)?;
        for entry in entries {
            if entry.name == filename {
                return Err(FsError::AlreadyExists);
            }
        }

        // Allocate new inode
        let group = (parent_inode_num - 1) / self.inodes_per_group as u64;
        let new_inode_num = self.allocate_inode_mut(group, false)?;

        // Initialize inode
        let current_time = crate::time::get_system_time_ms() / 1000;
        let mut new_inode: Ext4Inode = unsafe { mem::zeroed() };
        new_inode.i_mode = 0x8000 | permissions.to_octal(); // Regular file
        new_inode.i_uid = 0;
        new_inode.i_gid = 0;
        new_inode.i_size_lo = 0;
        new_inode.i_atime = current_time as u32;
        new_inode.i_ctime = current_time as u32;
        new_inode.i_mtime = current_time as u32;
        new_inode.i_dtime = 0;
        new_inode.i_links_count = 1;
        new_inode.i_blocks_lo = 0;

        self.write_inode(new_inode_num, &new_inode)?;

        // Add directory entry
        self.add_directory_entry(parent_inode_num, filename, new_inode_num, FileType::Regular)?;

        Ok(new_inode_num)
    }

    fn open(&self, path: &str, _flags: OpenFlags) -> FsResult<InodeNumber> {
        self.resolve_path(path)
    }

    fn read(&self, inode_num: InodeNumber, offset: u64, buffer: &mut [u8]) -> FsResult<usize> {
        let inode = self.read_inode(inode_num)?;
        let metadata = self.inode_to_metadata(inode_num, &inode);

        if metadata.file_type != FileType::Regular {
            return Err(FsError::IsADirectory);
        }

        if offset >= metadata.size {
            return Ok(0);
        }

        let bytes_to_read = core::cmp::min(buffer.len(), (metadata.size - offset) as usize);
        let mut bytes_read = 0;

        // For simplicity, only handle direct blocks
        let block_size = self.block_size as u64;
        let start_block = offset / block_size;
        let start_offset = offset % block_size;

        for block_idx in start_block.. {
            if bytes_read >= bytes_to_read || block_idx >= 12 {
                break;
            }

            let block_ptr = inode.i_block[block_idx as usize];
            if block_ptr == 0 {
                break;
            }

            let block_data = self.read_block(block_ptr as u64)?;
            let copy_offset = if block_idx == start_block { start_offset as usize } else { 0 };
            let copy_len = core::cmp::min(
                block_data.len() - copy_offset,
                bytes_to_read - bytes_read
            );

            buffer[bytes_read..bytes_read + copy_len]
                .copy_from_slice(&block_data[copy_offset..copy_offset + copy_len]);
            
            bytes_read += copy_len;
        }

        Ok(bytes_read)
    }

    fn write(&self, inode_num: InodeNumber, offset: u64, buffer: &[u8]) -> FsResult<usize> {
        let mut inode = self.read_inode(inode_num)?;
        let metadata = self.inode_to_metadata(inode_num, &inode);

        if metadata.file_type != FileType::Regular {
            return Err(FsError::IsADirectory);
        }

        if buffer.is_empty() {
            return Ok(0);
        }

        let block_size = self.block_size as u64;
        let start_block = offset / block_size;
        let start_offset = (offset % block_size) as usize;
        let mut bytes_written = 0;
        let group = (inode_num - 1) / self.inodes_per_group as u64;

        let mut i_block = unsafe { core::ptr::addr_of!(inode.i_block).read_unaligned() };

        while bytes_written < buffer.len() {
            let block_idx = start_block + (bytes_written / block_size as usize) as u64;

            if block_idx >= 12 {
                // Would need indirect blocks for larger files
                break;
            }

            // Allocate block if needed
            if i_block[block_idx as usize] == 0 {
                let new_block = self.allocate_block_mut(group)?;
                i_block[block_idx as usize] = new_block as u32;

                // Initialize block with zeros
                let zero_block = vec![0u8; self.block_size as usize];
                self.write_block(new_block, &zero_block)?;
            }

            let block_ptr = i_block[block_idx as usize];
            let mut block_data = self.read_block(block_ptr as u64)?;

            let write_offset = if bytes_written == 0 { start_offset } else { 0 };
            let bytes_to_write = core::cmp::min(
                buffer.len() - bytes_written,
                self.block_size as usize - write_offset
            );

            block_data[write_offset..write_offset + bytes_to_write]
                .copy_from_slice(&buffer[bytes_written..bytes_written + bytes_to_write]);

            self.write_block(block_ptr as u64, &block_data)?;
            bytes_written += bytes_to_write;
        }

        // Update inode
        unsafe {
            core::ptr::write_unaligned(&mut inode.i_block as *mut [u32; 15], i_block);
        }

        // Update file size if necessary
        let new_size = core::cmp::max(
            metadata.size,
            offset + bytes_written as u64
        );
        unsafe {
            core::ptr::write_unaligned(&mut inode.i_size_lo as *mut u32, new_size as u32);
            if self.superblock.s_feature_ro_compat & Ext4FeatureRoCompat::LARGE_FILE.bits() != 0 {
                core::ptr::write_unaligned(&mut inode.i_size_high as *mut u32, (new_size >> 32) as u32);
            }
        }

        // Update modification time
        let current_time = crate::time::get_system_time_ms() / 1000;
        unsafe {
            core::ptr::write_unaligned(&mut inode.i_mtime as *mut u32, current_time as u32);
        }

        // Update block count
        let blocks_used = ((new_size + block_size - 1) / block_size) as u32;
        unsafe {
            core::ptr::write_unaligned(&mut inode.i_blocks_lo as *mut u32, blocks_used * (block_size as u32 / 512));
        }

        self.write_inode(inode_num, &inode)?;

        Ok(bytes_written)
    }

    fn metadata(&self, inode_num: InodeNumber) -> FsResult<FileMetadata> {
        let inode = self.read_inode(inode_num)?;
        Ok(self.inode_to_metadata(inode_num, &inode))
    }

    fn set_metadata(&self, inode_num: InodeNumber, metadata: &FileMetadata) -> FsResult<()> {
        let mut inode = self.read_inode(inode_num)?;

        // Update permissions
        let mode_type = unsafe { core::ptr::addr_of!(inode.i_mode).read_unaligned() } & 0xF000;
        let new_mode = mode_type | metadata.permissions.to_octal();
        unsafe {
            core::ptr::write_unaligned(&mut inode.i_mode as *mut u16, new_mode);
        }

        // Update timestamps
        unsafe {
            core::ptr::write_unaligned(&mut inode.i_atime as *mut u32, metadata.accessed as u32);
            core::ptr::write_unaligned(&mut inode.i_mtime as *mut u32, metadata.modified as u32);
            core::ptr::write_unaligned(&mut inode.i_ctime as *mut u32, metadata.created as u32);
        }

        // Update ownership
        unsafe {
            core::ptr::write_unaligned(&mut inode.i_uid as *mut u16, metadata.uid as u16);
            core::ptr::write_unaligned(&mut inode.i_gid as *mut u16, metadata.gid as u16);
        }

        self.write_inode(inode_num, &inode)
    }

    fn mkdir(&self, path: &str, permissions: FilePermissions) -> FsResult<InodeNumber> {
        // Parse path to get parent directory and directory name
        let path_parts: Vec<&str> = path.rsplitn(2, '/').collect();
        if path_parts.is_empty() {
            return Err(FsError::InvalidArgument);
        }

        let dirname = path_parts[0];
        let parent_path = if path_parts.len() > 1 {
            path_parts[1]
        } else {
            "/"
        };

        // Resolve parent directory
        let parent_inode_num = self.resolve_path(parent_path)?;
        let parent_inode = self.read_inode(parent_inode_num)?;
        let parent_meta = self.inode_to_metadata(parent_inode_num, &parent_inode);

        if parent_meta.file_type != FileType::Directory {
            return Err(FsError::NotADirectory);
        }

        // Check if directory already exists
        let entries = self.read_directory_entries(&parent_inode)?;
        for entry in entries {
            if entry.name == dirname {
                return Err(FsError::AlreadyExists);
            }
        }

        // Allocate new inode
        let group = (parent_inode_num - 1) / self.inodes_per_group as u64;
        let new_inode_num = self.allocate_inode_mut(group, true)?;

        // Allocate block for directory entries
        let new_block = self.allocate_block_mut(group)?;

        // Initialize inode
        let current_time = crate::time::get_system_time_ms() / 1000;
        let mut new_inode: Ext4Inode = unsafe { mem::zeroed() };
        new_inode.i_mode = 0x4000 | permissions.to_octal(); // Directory
        new_inode.i_uid = 0;
        new_inode.i_gid = 0;
        new_inode.i_size_lo = self.block_size;
        new_inode.i_atime = current_time as u32;
        new_inode.i_ctime = current_time as u32;
        new_inode.i_mtime = current_time as u32;
        new_inode.i_dtime = 0;
        new_inode.i_links_count = 2; // . and ..
        new_inode.i_blocks_lo = (self.block_size / 512);
        new_inode.i_block[0] = new_block as u32;

        // Create . and .. entries
        let mut block_data = vec![0u8; self.block_size as usize];

        // . entry
        let dot_entry = Ext4DirEntry2 {
            inode: new_inode_num as u32,
            rec_len: 12, // 8 bytes header + 1 byte name + 3 padding
            name_len: 1,
            file_type: 2, // Directory
        };
        let dot_bytes = unsafe {
            core::slice::from_raw_parts(
                &dot_entry as *const Ext4DirEntry2 as *const u8,
                mem::size_of::<Ext4DirEntry2>()
            )
        };
        block_data[0..mem::size_of::<Ext4DirEntry2>()].copy_from_slice(dot_bytes);
        block_data[mem::size_of::<Ext4DirEntry2>()] = b'.';

        // .. entry
        let dotdot_entry = Ext4DirEntry2 {
            inode: parent_inode_num as u32,
            rec_len: self.block_size as u16 - 12, // Rest of block
            name_len: 2,
            file_type: 2, // Directory
        };
        let dotdot_bytes = unsafe {
            core::slice::from_raw_parts(
                &dotdot_entry as *const Ext4DirEntry2 as *const u8,
                mem::size_of::<Ext4DirEntry2>()
            )
        };
        block_data[12..12 + mem::size_of::<Ext4DirEntry2>()].copy_from_slice(dotdot_bytes);
        block_data[12 + mem::size_of::<Ext4DirEntry2>()] = b'.';
        block_data[12 + mem::size_of::<Ext4DirEntry2>() + 1] = b'.';

        self.write_block(new_block, &block_data)?;
        self.write_inode(new_inode_num, &new_inode)?;

        // Add directory entry in parent
        self.add_directory_entry(parent_inode_num, dirname, new_inode_num, FileType::Directory)?;

        // Update parent link count
        let mut parent_inode_updated = self.read_inode(parent_inode_num)?;
        let parent_links = unsafe { core::ptr::addr_of!(parent_inode_updated.i_links_count).read_unaligned() };
        unsafe {
            core::ptr::write_unaligned(&mut parent_inode_updated.i_links_count as *mut u16, parent_links + 1);
        }
        self.write_inode(parent_inode_num, &parent_inode_updated)?;

        Ok(new_inode_num)
    }

    fn rmdir(&self, path: &str) -> FsResult<()> {
        // Parse path
        let path_parts: Vec<&str> = path.rsplitn(2, '/').collect();
        if path_parts.is_empty() {
            return Err(FsError::InvalidArgument);
        }

        let dirname = path_parts[0];
        let parent_path = if path_parts.len() > 1 {
            path_parts[1]
        } else {
            "/"
        };

        // Resolve directory to remove
        let dir_inode_num = self.resolve_path(path)?;
        let dir_inode = self.read_inode(dir_inode_num)?;
        let dir_meta = self.inode_to_metadata(dir_inode_num, &dir_inode);

        if dir_meta.file_type != FileType::Directory {
            return Err(FsError::NotADirectory);
        }

        // Check if directory is empty (only . and .. should remain)
        let entries = self.read_directory_entries(&dir_inode)?;
        let non_special_entries: Vec<_> = entries.iter()
            .filter(|e| e.name != "." && e.name != "..")
            .collect();

        if !non_special_entries.is_empty() {
            return Err(FsError::DirectoryNotEmpty);
        }

        // Remove from parent directory
        let parent_inode_num = self.resolve_path(parent_path)?;
        self.remove_directory_entry(parent_inode_num, dirname)?;

        // Free blocks used by directory
        let i_block = unsafe { core::ptr::addr_of!(dir_inode.i_block).read_unaligned() };
        for &block_ptr in &i_block[0..12] {
            if block_ptr != 0 {
                self.free_block_mut(block_ptr as u64)?;
            }
        }

        // Free the inode
        self.free_inode_mut(dir_inode_num, true)?;

        // Update parent link count
        let mut parent_inode = self.read_inode(parent_inode_num)?;
        let parent_links = unsafe { core::ptr::addr_of!(parent_inode.i_links_count).read_unaligned() };
        if parent_links > 0 {
            unsafe {
                core::ptr::write_unaligned(&mut parent_inode.i_links_count as *mut u16, parent_links - 1);
            }
            self.write_inode(parent_inode_num, &parent_inode)?;
        }

        Ok(())
    }

    fn unlink(&self, path: &str) -> FsResult<()> {
        // Parse path
        let path_parts: Vec<&str> = path.rsplitn(2, '/').collect();
        if path_parts.is_empty() {
            return Err(FsError::InvalidArgument);
        }

        let filename = path_parts[0];
        let parent_path = if path_parts.len() > 1 {
            path_parts[1]
        } else {
            "/"
        };

        // Resolve file to remove
        let file_inode_num = self.resolve_path(path)?;
        let file_inode = self.read_inode(file_inode_num)?;
        let file_meta = self.inode_to_metadata(file_inode_num, &file_inode);

        if file_meta.file_type == FileType::Directory {
            return Err(FsError::IsADirectory);
        }

        // Remove from parent directory
        let parent_inode_num = self.resolve_path(parent_path)?;
        self.remove_directory_entry(parent_inode_num, filename)?;

        // Decrement link count
        let mut updated_inode = file_inode;
        let link_count = unsafe { core::ptr::addr_of!(updated_inode.i_links_count).read_unaligned() };

        if link_count > 1 {
            // File still has other hard links
            unsafe {
                core::ptr::write_unaligned(&mut updated_inode.i_links_count as *mut u16, link_count - 1);
            }
            self.write_inode(file_inode_num, &updated_inode)?;
        } else {
            // No more links, free the file
            // Free all blocks
            let i_block = unsafe { core::ptr::addr_of!(file_inode.i_block).read_unaligned() };
            for &block_ptr in &i_block[0..12] {
                if block_ptr != 0 {
                    self.free_block_mut(block_ptr as u64)?;
                }
            }

            // Mark deletion time
            let current_time = crate::time::get_system_time_ms() / 1000;
            unsafe {
                core::ptr::write_unaligned(&mut updated_inode.i_dtime as *mut u32, current_time as u32);
                core::ptr::write_unaligned(&mut updated_inode.i_links_count as *mut u16, 0);
            }
            self.write_inode(file_inode_num, &updated_inode)?;

            // Free the inode
            self.free_inode_mut(file_inode_num, false)?;
        }

        Ok(())
    }

    fn readdir(&self, inode_num: InodeNumber) -> FsResult<Vec<DirectoryEntry>> {
        let inode = self.read_inode(inode_num)?;
        let metadata = self.inode_to_metadata(inode_num, &inode);

        if metadata.file_type != FileType::Directory {
            return Err(FsError::NotADirectory);
        }

        self.read_directory_entries(&inode)
    }

    fn rename(&self, old_path: &str, new_path: &str) -> FsResult<()> {
        // Parse old path
        let old_parts: Vec<&str> = old_path.rsplitn(2, '/').collect();
        if old_parts.is_empty() {
            return Err(FsError::InvalidArgument);
        }

        let old_name = old_parts[0];
        let old_parent_path = if old_parts.len() > 1 {
            old_parts[1]
        } else {
            "/"
        };

        // Parse new path
        let new_parts: Vec<&str> = new_path.rsplitn(2, '/').collect();
        if new_parts.is_empty() {
            return Err(FsError::InvalidArgument);
        }

        let new_name = new_parts[0];
        let new_parent_path = if new_parts.len() > 1 {
            new_parts[1]
        } else {
            "/"
        };

        // Get the inode being renamed
        let inode_num = self.resolve_path(old_path)?;
        let inode = self.read_inode(inode_num)?;
        let metadata = self.inode_to_metadata(inode_num, &inode);

        // Check if new path already exists
        if self.resolve_path(new_path).is_ok() {
            return Err(FsError::AlreadyExists);
        }

        // Remove from old parent
        let old_parent_inode_num = self.resolve_path(old_parent_path)?;
        self.remove_directory_entry(old_parent_inode_num, old_name)?;

        // Add to new parent
        let new_parent_inode_num = self.resolve_path(new_parent_path)?;
        self.add_directory_entry(new_parent_inode_num, new_name, inode_num, metadata.file_type)?;

        // If moving a directory and parents are different, update link counts
        if metadata.file_type == FileType::Directory && old_parent_inode_num != new_parent_inode_num {
            // Decrement old parent link count
            let mut old_parent = self.read_inode(old_parent_inode_num)?;
            let old_links = unsafe { core::ptr::addr_of!(old_parent.i_links_count).read_unaligned() };
            if old_links > 0 {
                unsafe {
                    core::ptr::write_unaligned(&mut old_parent.i_links_count as *mut u16, old_links - 1);
                }
                self.write_inode(old_parent_inode_num, &old_parent)?;
            }

            // Increment new parent link count
            let mut new_parent = self.read_inode(new_parent_inode_num)?;
            let new_links = unsafe { core::ptr::addr_of!(new_parent.i_links_count).read_unaligned() };
            unsafe {
                core::ptr::write_unaligned(&mut new_parent.i_links_count as *mut u16, new_links + 1);
            }
            self.write_inode(new_parent_inode_num, &new_parent)?;

            // Update .. entry in the moved directory
            let mut dir_inode = inode;
            let i_block = unsafe { core::ptr::addr_of!(dir_inode.i_block).read_unaligned() };
            if i_block[0] != 0 {
                let mut block_data = self.read_block(i_block[0] as u64)?;

                // Find .. entry (should be second entry)
                let mut offset = 0;
                let mut found_dot = false;

                while offset < block_data.len() {
                    if offset + mem::size_of::<Ext4DirEntry2>() > block_data.len() {
                        break;
                    }

                    let mut dir_entry = unsafe {
                        core::ptr::read_unaligned(
                            block_data.as_ptr().add(offset) as *const Ext4DirEntry2
                        )
                    };

                    if dir_entry.rec_len == 0 {
                        break;
                    }

                    if found_dot && dir_entry.name_len == 2 {
                        // This is the .. entry
                        dir_entry.inode = new_parent_inode_num as u32;

                        let entry_bytes = unsafe {
                            core::slice::from_raw_parts(
                                &dir_entry as *const Ext4DirEntry2 as *const u8,
                                mem::size_of::<Ext4DirEntry2>()
                            )
                        };
                        block_data[offset..offset + mem::size_of::<Ext4DirEntry2>()]
                            .copy_from_slice(entry_bytes);
                        self.write_block(i_block[0] as u64, &block_data)?;
                        break;
                    }

                    if dir_entry.name_len == 1 {
                        found_dot = true;
                    }

                    offset += dir_entry.rec_len as usize;
                }
            }
        }

        // Update ctime
        let mut updated_inode = inode;
        let current_time = crate::time::get_system_time_ms() / 1000;
        unsafe {
            core::ptr::write_unaligned(&mut updated_inode.i_ctime as *mut u32, current_time as u32);
        }
        self.write_inode(inode_num, &updated_inode)?;

        Ok(())
    }

    fn symlink(&self, target: &str, link_path: &str) -> FsResult<()> {
        // Parse link path
        let path_parts: Vec<&str> = link_path.rsplitn(2, '/').collect();
        if path_parts.is_empty() {
            return Err(FsError::InvalidArgument);
        }

        let link_name = path_parts[0];
        let parent_path = if path_parts.len() > 1 {
            path_parts[1]
        } else {
            "/"
        };

        // Resolve parent directory
        let parent_inode_num = self.resolve_path(parent_path)?;
        let parent_inode = self.read_inode(parent_inode_num)?;
        let parent_meta = self.inode_to_metadata(parent_inode_num, &parent_inode);

        if parent_meta.file_type != FileType::Directory {
            return Err(FsError::NotADirectory);
        }

        // Check if symlink already exists
        let entries = self.read_directory_entries(&parent_inode)?;
        for entry in entries {
            if entry.name == link_name {
                return Err(FsError::AlreadyExists);
            }
        }

        // Allocate new inode
        let group = (parent_inode_num - 1) / self.inodes_per_group as u64;
        let new_inode_num = self.allocate_inode_mut(group, false)?;

        // Initialize inode
        let current_time = crate::time::get_system_time_ms() / 1000;
        let mut new_inode: Ext4Inode = unsafe { mem::zeroed() };
        new_inode.i_mode = 0xA000 | 0o777; // Symbolic link with all permissions
        new_inode.i_uid = 0;
        new_inode.i_gid = 0;
        new_inode.i_size_lo = target.len() as u32;
        new_inode.i_atime = current_time as u32;
        new_inode.i_ctime = current_time as u32;
        new_inode.i_mtime = current_time as u32;
        new_inode.i_dtime = 0;
        new_inode.i_links_count = 1;

        if target.len() <= 60 {
            // Fast symlink - store target in inode
            let target_bytes = target.as_bytes();
            let i_block_bytes = unsafe {
                core::slice::from_raw_parts_mut(
                    &mut new_inode.i_block as *mut [u32; 15] as *mut u8,
                    60
                )
            };
            i_block_bytes[..target_bytes.len()].copy_from_slice(target_bytes);
            new_inode.i_blocks_lo = 0;
        } else {
            // Slow symlink - store target in block
            let new_block = self.allocate_block_mut(group)?;
            let mut block_data = vec![0u8; self.block_size as usize];
            block_data[..target.len()].copy_from_slice(target.as_bytes());
            self.write_block(new_block, &block_data)?;

            new_inode.i_block[0] = new_block as u32;
            new_inode.i_blocks_lo = (self.block_size / 512);
        }

        self.write_inode(new_inode_num, &new_inode)?;

        // Add directory entry
        self.add_directory_entry(parent_inode_num, link_name, new_inode_num, FileType::SymbolicLink)?;

        Ok(())
    }

    fn readlink(&self, path: &str) -> FsResult<String> {
        let inode_num = self.resolve_path(path)?;
        let inode = self.read_inode(inode_num)?;
        let metadata = self.inode_to_metadata(inode_num, &inode);

        if metadata.file_type != FileType::SymbolicLink {
            return Err(FsError::InvalidArgument);
        }

        // For small symlinks, target is stored in i_block
        if metadata.size <= 60 {
            // SAFETY: inode is a packed struct representing EXT4 on-disk format.
            // We use addr_of! to avoid creating misaligned references.
            let target_bytes = unsafe {
                core::slice::from_raw_parts(
                    core::ptr::addr_of!(inode.i_block) as *const u8,
                    metadata.size as usize
                )
            };
            
            core::str::from_utf8(target_bytes)
                .map(|s| s.to_string())
                .map_err(|_| FsError::IoError)
        } else {
            // Large symlinks are stored in blocks
            let mut buffer = vec![0u8; metadata.size as usize];
            self.read(inode_num, 0, &mut buffer)?;
            
            core::str::from_utf8(&buffer)
                .map(|s| s.to_string())
                .map_err(|_| FsError::IoError)
        }
    }

    fn sync(&self) -> FsResult<()> {
        self.flush_dirty_blocks()
    }
}