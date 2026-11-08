# Changelog

## Unreleased

- Scaffold Rust workspace with core, CLI and EFI crates.
- Vendor and model detection from a DMI directory.
- Image pipeline: PNG/JPEG to 24-bit uncompressed BMP, scaled to fit and centered.
- BGRT table builder and parser; EFI application builds and installs the table and chains to the original loader.
- Linux CLI: detect, status, install and uninstall with injectable paths and efibootmgr.
- Optional Plymouth bgrt theme install and restore.
- QEMU end-to-end test with ACPI BGRT verification and documentation updates.
- More PRs land in later PRs.
