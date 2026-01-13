# RustOS Package Management System

## Quick Start

The RustOS package management system provides experimental adapters for working with Linux packages (.deb, .rpm, .apk), package repository APIs, and app stores.

### What's Implemented

✅ **Package Format Support**
- .deb (Debian/Ubuntu) - **Full extraction and parsing** ⭐
- .rpm (Fedora/RHEL) - Magic number validation
- .apk (Alpine Linux) - Format detection
- .rustos (Native) - Custom format specification

✅ **Archive & Compression** ⭐ NEW
- AR archive parsing (complete)
- TAR archive extraction (POSIX ustar format)
- GZIP/DEFLATE decompression (via miniz_oxide)
- XZ/LZMA/Zstd/Bzip2 detection (decompression pending)

✅ **Debian Package (.deb) Support** ⭐ COMPLETE
- Full package extraction (control + data archives)
- Metadata parsing from control file
- Control script extraction (postinst, prerm, etc.)
- All file extraction with paths
- Dependency information

✅ **System Integration** ⭐ NEW
- 7 syscalls for package operations (200-206)
- Kernel initialization in kernel_main
- Process syscall integration
- Package database & cache

✅ **Package Database**
- Track installed packages
- Search and query functionality
- Package status management
- Cache for downloaded packages

✅ **Test Suite** ⭐ NEW
- Database operation tests
- Archive parsing tests
- Compression detection tests
- Package manager operation tests

### What's Not Yet Implemented

❌ Network stack integration for downloads
❌ Filesystem operations for installation
❌ Dependency resolution engine
❌ Script execution (postinst/prerm)
❌ XZ/LZMA decompression (detection only)
❌ Dynamic linking support

## Usage

### Basic Package Operations

```rust
use rustos::package::{PackageManager, PackageManagerType, PackageOperation};

// Create package manager for Debian packages
let mut pm = PackageManager::new(PackageManagerType::Apt);

// List installed packages
pm.execute_operation(PackageOperation::List, "")?;

// Search for packages
pm.execute_operation(PackageOperation::Search, "htop")?;

// Get package info
pm.execute_operation(PackageOperation::Info, "htop")?;
```

### Validate Package Format

```rust
use rustos::package::adapters::{PackageAdapter, DebAdapter};

let adapter = DebAdapter::new();

// Check if file is a valid .deb package
if adapter.validate(&package_data)? {
    println!("Valid .deb package");
    
    // Extract metadata
    let metadata = adapter.parse_metadata(&package_data)?;
    println!("Package: {} v{}", metadata.name, metadata.version);
}
```

### Parse AR Archives

```rust
use rustos::package::archive::ArArchive;

// Parse .deb package (which uses AR format)
let ar = ArArchive::parse(&deb_data)?;

// Find control archive
if let Some(control_data) = ar.find_member("control.tar.gz") {
    println!("Found control.tar.gz");
}

// List all archive members
for member in ar.members() {
    println!("{}: {} bytes", member.name, member.size);
}
```

### Manage Package Database

```rust
use rustos::package::{PackageDatabase, PackageInfo, PackageMetadata};

let mut db = PackageDatabase::new();

// Add package
let metadata = PackageMetadata::new(
    "htop".into(),
    "3.0.5".into(),
    "amd64".into()
);
let info = PackageInfo { metadata, /* ... */ };
db.add_package(info)?;

// Search packages
let results = db.search("http");

// Check if installed
if db.is_installed("htop") {
    println!("htop is installed");
}
```

## Documentation

- **[EXPERIMENTAL_PACKAGE_ADAPTERS.md](../docs/EXPERIMENTAL_PACKAGE_ADAPTERS.md)** - Complete adapter documentation
- **[LINUX_APP_SUPPORT.md](../docs/LINUX_APP_SUPPORT.md)** - Implementation roadmap and requirements
- **[package_manager_integration.md](../docs/package_manager_integration.md)** - Future vision

## Examples

See `examples/package_manager_demo.rs` for comprehensive usage examples.

## Architecture

```
src/package/
├── mod.rs              - Core types, errors, operations
├── types.rs            - Package metadata structures
├── adapters/           - Format-specific adapters
│   ├── deb.rs         - Debian .deb packages
│   ├── rpm.rs         - RPM packages
│   ├── apk.rs         - Alpine APK packages
│   └── native.rs      - Native RustOS packages
├── archive/            - Archive format support
│   └── ar.rs          - AR archive parser
├── database.rs         - Package database
├── api.rs              - Repository/app store APIs
└── manager.rs          - Package manager orchestrator
```

## Roadmap

To achieve full Linux package support, the following must be implemented:

**Phase 1: Dynamic Linker (3-4 months)**
- Parse PT_DYNAMIC segment
- Load shared libraries
- Symbol resolution
- Relocation processing

**Phase 2: POSIX Support (3-4 months)**
- Extended syscalls
- Filesystem support (ext4)
- POSIX threads
- IPC mechanisms

**Phase 3: Userspace Tools (4-5 months)**
- Shell (bash/sh)
- Core utilities
- Archive tools (tar, gzip)

**Phase 4: Package Management (2-3 months)**
- Complete archive extraction
- Dependency resolution
- Script execution
- Package operations

**Total: 15-20 months**

## Contributing

To contribute to package management:

1. **Start with the dynamic linker** - Highest impact
2. **Implement archive extraction** - TAR, GZIP support
3. **Add filesystem operations** - Install files to disk
4. **Build dependency resolver** - Package dependencies
5. **Test with real packages** - Validate against .deb files

## Current Status

**Maturity**: Experimental (Core Infrastructure Complete)
**Functionality**: ~75% complete ⭐
**Production Ready**: No - requires network/filesystem integration

**What works today**:
- ✅ Full .deb package extraction
- ✅ AR/TAR/GZIP archive handling
- ✅ Package metadata parsing
- ✅ Database & cache management
- ✅ Syscall interface (7 operations)
- ✅ Kernel integration
- ✅ Test suite

**What's needed for full functionality**:
- Network stack integration (for downloads)
- Filesystem support (for installation)
- Script execution engine
- Dependency resolver
- Repository synchronization

**What's needed for Linux app execution**:
- Dynamic linker (PT_DYNAMIC, symbol resolution)
- C library port (glibc/musl)
- Extended POSIX syscalls
- IPC mechanisms

## Testing

```bash
# Check compilation
cargo +nightly check --bin rustos -Zbuild-std=core,compiler_builtins --target x86_64-rustos.json

# Run package tests (in kernel context)
use rustos::package::tests;
let (passed, total) = tests::run_all_tests();

# Run specific tests
tests::test_package_database();
tests::test_deb_validation();
tests::test_ar_archive_parsing();
tests::test_tar_archive_parsing();
tests::test_compression_detection();
tests::test_package_manager_operations();
```

### Test Coverage

| Component | Test Status |
|-----------|-------------|
| Package Database | ✅ Tested |
| .deb Validation | ✅ Tested |
| AR Archive Parsing | ✅ Tested |
| TAR Extraction | ✅ Tested |
| GZIP Detection | ✅ Tested |
| Package Manager Ops | ✅ Tested |

## License

Part of RustOS - See main repository license.
