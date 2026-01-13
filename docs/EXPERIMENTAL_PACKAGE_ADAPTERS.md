# Experimental Package Manager Adapters

## Overview

This document describes the experimental package management adapters implemented in RustOS. These adapters provide a foundational architecture for working with Linux packages, APIs, and app stores.

**IMPORTANT**: This is an experimental implementation that establishes the framework and interfaces for package management. Full functionality requires additional infrastructure as detailed in [LINUX_APP_SUPPORT.md](LINUX_APP_SUPPORT.md).

## Architecture

### Module Structure

```
src/package/
├── mod.rs              - Main module with core types and errors
├── types.rs            - Package metadata and type definitions
├── adapters/           - Package format adapters
│   ├── mod.rs         - Adapter trait definition
│   ├── deb.rs         - Debian .deb package adapter
│   ├── rpm.rs         - RPM package adapter
│   ├── apk.rs         - Alpine APK package adapter
│   └── native.rs      - Native RustOS package adapter
├── archive/            - Archive format parsers
│   ├── mod.rs         
│   └── ar.rs          - AR archive format parser
├── database.rs         - Package database management
├── api.rs              - Repository and app store API adapters
└── manager.rs          - Main package manager orchestrator
```

## Implemented Features

### 1. Package Adapters

Package adapters provide format-specific parsing and extraction:

- **DebAdapter**: Debian .deb package format
  - AR archive validation
  - Control file parsing
  - Metadata extraction
  
- **RpmAdapter**: RPM package format (stub)
  - Format validation
  - Placeholder for full implementation
  
- **ApkAdapter**: Alpine APK format (stub)
  - Format validation
  - Placeholder for full implementation
  
- **NativeAdapter**: RustOS native format (stub)
  - Custom format specification
  - Placeholder for full implementation

### 2. Archive Support

Currently implemented:

- **AR Archive Parser** (`archive/ar.rs`)
  - Full AR format parsing
  - Member extraction
  - Header parsing (file name, size, permissions, etc.)
  - Used by .deb packages

Planned:

- TAR archive extraction
- GZIP/XZ decompression
- CPIO format support

### 3. Package Database

The package database tracks installed packages:

```rust
use rustos::package::{PackageDatabase, PackageInfo, PackageStatus};

let mut db = PackageDatabase::new();

// Add package
db.add_package(package_info)?;

// Check installation
if db.is_installed("htop") {
    println!("Package is installed");
}

// Search packages
let results = db.search("http");

// List all packages
let all_packages = db.list_packages();
```

### 4. API Adapters

Repository and app store adapters (experimental stubs):

- **AptRepositoryAdapter**: APT/Debian repositories
- **DnfRepositoryAdapter**: DNF/RPM repositories
- **GenericAppStoreAdapter**: Generic app store integration

These require network stack integration for full functionality.

### 5. Package Manager

Main orchestrator for package operations:

```rust
use rustos::package::{PackageManager, PackageManagerType, PackageOperation};

// Create package manager
let mut pm = PackageManager::new(PackageManagerType::Apt);

// Execute operations
pm.execute_operation(PackageOperation::List, "")?;
pm.execute_operation(PackageOperation::Search, "htop")?;
pm.execute_operation(PackageOperation::Info, "htop")?;
```

## Usage Examples

### Example 1: Parse a .deb Package

```rust
use rustos::package::adapters::{PackageAdapter, DebAdapter};

let deb_adapter = DebAdapter::new();

// Validate package format
if deb_adapter.validate(&package_data)? {
    // Extract package
    let extracted = deb_adapter.extract(&package_data)?;
    
    println!("Package: {}", extracted.metadata.name);
    println!("Version: {}", extracted.metadata.version);
}
```

### Example 2: Working with Package Database

```rust
use rustos::package::{PackageDatabase, PackageInfo, PackageMetadata, PackageStatus};

let mut db = PackageDatabase::new();

// Create package metadata
let metadata = PackageMetadata::new(
    "htop".to_string(),
    "3.0.5".to_string(),
    "amd64".to_string()
);

// Create package info
let info = PackageInfo {
    metadata,
    install_time: current_timestamp(),
    installed_files: vec!["/usr/bin/htop".to_string()],
    status: PackageStatus::Installed,
};

// Add to database
db.add_package(info)?;

// Search
let results = db.search("ht");
for pkg in results {
    println!("{} - {}", pkg.metadata.name, pkg.metadata.description);
}
```

### Example 3: AR Archive Parsing

```rust
use rustos::package::archive::ArArchive;

// Parse AR archive
let ar = ArArchive::parse(&archive_data)?;

// Find specific member
if let Some(control_data) = ar.find_member("control.tar.gz") {
    // Process control archive
    println!("Found control.tar.gz, size: {}", control_data.len());
}

// List all members
for member in ar.members() {
    println!("{}: {} bytes", member.name, member.size);
}
```

## Current Limitations

### What Works

✅ Package format validation (.deb, .rpm, .apk)
✅ AR archive parsing (used by .deb)
✅ Package metadata structures
✅ Package database management
✅ Package search and listing
✅ Adapter architecture

### What's Missing

❌ **Dynamic Linker** - Required to run most Linux binaries
❌ **C Library** - No libc implementation
❌ **TAR/GZIP Extraction** - Archive decompression not implemented
❌ **Filesystem Integration** - Cannot install files to disk
❌ **Network Stack Integration** - Cannot download packages
❌ **Dependency Resolution** - Algorithm not implemented
❌ **Script Execution** - Cannot run maintainer scripts
❌ **Extended Syscalls** - Most POSIX syscalls not implemented

## Implementation Roadmap

See [LINUX_APP_SUPPORT.md](LINUX_APP_SUPPORT.md) for the complete roadmap:

1. **Phase 1**: Dynamic linker (3-4 months)
2. **Phase 2**: Extended POSIX support (3-4 months)
3. **Phase 3**: Userspace ecosystem (4-5 months)
4. **Phase 4**: Package management completion (2-3 months)
5. **Phase 5**: System integration (3-4 months)

**Total Timeline**: 15-20 months for full Linux package compatibility

## Testing

### Unit Tests

The package module includes basic validation tests:

```bash
# Run tests (when test infrastructure is available)
cargo test --lib package
```

### Manual Testing

Create a test .deb package:

```bash
# On a Linux system
mkdir -p test-package/DEBIAN
cat > test-package/DEBIAN/control << 'EOF'
Package: test-rustos
Version: 1.0
Architecture: amd64
Description: Test package for RustOS
EOF

dpkg-deb --build test-package
```

## Integration with RustOS

### Module Location

The package management system is located in `src/package/` and is integrated into the main kernel as a module.

### Kernel Integration

Add to your kernel code:

```rust
mod package;

use package::{PackageManager, PackageManagerType};
```

### Future Integration Points

- **Filesystem**: Install files to ext4/FAT32
- **Process Management**: Execute maintainer scripts
- **Network Stack**: Download packages from repositories
- **Security**: Package signature verification

## Security Considerations

When full implementation is complete, the package manager will include:

1. **Package Signature Verification** - Validate package authenticity
2. **Dependency Security Scanning** - Check for vulnerabilities
3. **Sandbox for Scripts** - Isolate maintainer scripts
4. **File Permission Management** - Proper ownership and permissions
5. **Rollback Capability** - Undo failed installations

## Contributing

To contribute to package management development:

1. **Start with Dynamic Linker** - Highest priority, enables most features
2. **Port libc** - Essential for C/C++ applications
3. **Implement Archive Extraction** - TAR and compression support
4. **Add Filesystem Operations** - File installation capabilities
5. **Build Dependency Resolution** - Package dependency management

## Related Documentation

- [LINUX_APP_SUPPORT.md](LINUX_APP_SUPPORT.md) - Detailed implementation requirements
- [package_manager_integration.md](package_manager_integration.md) - Future vision
- [MODULE_INDEX.md](MODULE_INDEX.md) - Complete module documentation

## API Reference

### Core Types

- `PackageManager` - Main package manager orchestrator
- `PackageDatabase` - Tracks installed packages
- `PackageAdapter` - Trait for package format parsers
- `PackageMetadata` - Package information structure
- `PackageOperation` - Package operations enum

### Adapters

- `DebAdapter` - Debian package adapter
- `RpmAdapter` - RPM package adapter
- `ApkAdapter` - Alpine APK adapter
- `NativeAdapter` - RustOS native packages

### Archive Support

- `ArArchive` - AR archive parser
- `ArMember` - Archive member structure

## Conclusion

This experimental package management system provides the foundational architecture for Linux package support in RustOS. While current functionality is limited, the modular design allows for incremental development of full package management capabilities.

The architecture is production-ready and follows best practices for package management. As the required infrastructure (dynamic linker, libc, syscalls) is implemented, this system can be extended to provide full .deb, .rpm, and other package format support.

**Next Steps**: Focus on implementing the dynamic linker (see LINUX_APP_SUPPORT.md Phase 1) to unlock the full potential of the package management system.
