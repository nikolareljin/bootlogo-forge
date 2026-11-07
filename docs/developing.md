# Developing

## Build

    cargo build
    cargo build -p bootlogo-forge-efi --target x86_64-unknown-uefi

## Test

    cargo fmt --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    bash scripts/e2e_qemu.sh

## QEMU

The end-to-end script builds the EFI app, makes a FAT ESP in a temporary
directory and boots it under OVMF. It checks the serial output.

## Layout

- `crates/core` - pure logic
- `crates/cli` - the Linux binary
- `crates/efi` - the UEFI application
- `crates/e2e-payload` - the test payload for QEMU
- `docs` - this book
- `scripts` - helper scripts