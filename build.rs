fn main() {
    // Note: boot.s assembly is not used for bootimage builds
    // The bootloader crate handles boot setup automatically
    // Custom boot assembly is only needed for manual multiboot builds

    // Rerun if these files change
    println!("cargo:rerun-if-changed=src/boot.s");
    println!("cargo:rerun-if-changed=link.ld");
}
