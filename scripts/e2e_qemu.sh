#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORK="$(mktemp -d)"
trap 'if [ "$(id -u)" -ne 0 ]; then sudo rm -rf "$WORK" 2>/dev/null || true; else rm -rf "$WORK"; fi' EXIT

echo "Building EFI app, payload and CLI..."
cargo build -p bootlogo-forge-efi --target x86_64-unknown-uefi --manifest-path "$REPO_ROOT/Cargo.toml"
cargo build -p bootlogo-forge-e2e-payload --target x86_64-unknown-uefi --manifest-path "$REPO_ROOT/Cargo.toml"
cargo build -p bootlogo-forge --manifest-path "$REPO_ROOT/Cargo.toml"

EFI_APP="$REPO_ROOT/target/x86_64-unknown-uefi/debug/bootlogo-forge-efi.efi"
PAYLOAD="$REPO_ROOT/target/x86_64-unknown-uefi/debug/payload.efi"
CLI="$REPO_ROOT/target/debug/bootlogo-forge"
if [ ! -f "$EFI_APP" ]; then
    EFI_APP="$REPO_ROOT/target/x86_64-unknown-uefi/debug/deps/bootlogo_forge_efi.efi"
fi
if [ ! -f "$PAYLOAD" ]; then
    PAYLOAD="$REPO_ROOT/target/x86_64-unknown-uefi/debug/deps/payload.efi"
fi

ESP="$WORK/esp"
mkdir -p "$ESP/EFI/BOOT"
cp "$PAYLOAD" "$ESP/EFI/BOOT/payload.efi"

mkdir -p "$WORK/bin"
STUB="$WORK/bin/efibootmgr"
cat > "$STUB" <<'INNER_STUB'
#!/bin/sh
if [ "$1" = "--create" ]; then
    echo "BootCurrent: 0000"
    echo "Timeout: 2 seconds"
    echo "BootOrder: 0000"
    echo "Boot000F* bootlogo-forge /EFI/bootlogo-forge/bootlogo-forge-efi.efi"
    echo "Boot0000* ubuntu /EFI/BOOT/payload.efi"
    exit 0
fi
cat <<'INNER_EOF'
BootCurrent: 0000
Timeout: 2 seconds
BootOrder: 000F,0000
Boot000F* bootlogo-forge /EFI/bootlogo-forge/bootlogo-forge-efi.efi
Boot0000* ubuntu /EFI/BOOT/payload.efi
INNER_EOF
INNER_STUB
chmod +x "$STUB"

printf 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGPgEpEDAABoAD1UCKP3AAAAAElFTkSuQmCC' | base64 -d > "$WORK/logo.png"

cp /usr/share/OVMF/OVMF_VARS_4M.fd "$WORK/OVMF_VARS.fd"

INSTALL=( "$CLI" install --image "$WORK/logo.png" --esp "$ESP" --efi-app "$EFI_APP" --efibootmgr "$STUB" --apply )
if [ "$(id -u)" -ne 0 ]; then
    INSTALL=( sudo "${INSTALL[@]}" )
fi
"${INSTALL[@]}"

if [ ! -f "$ESP/EFI/bootlogo-forge/logo.bmp" ]; then
    echo "install did not write logo.bmp" >&2
    exit 1
fi
cp "$ESP/EFI/bootlogo-forge/bootlogo-forge-efi.efi" "$ESP/EFI/BOOT/BOOTX64.EFI"

run_case() {
    local case_name="$1"
    local logo_present="$2"
    local serial="$WORK/serial-$case_name.log"
    rm -f "$serial"
    if [ "$logo_present" = "no" ]; then
        rm -f "$ESP/EFI/bootlogo-forge/logo.bmp"
    fi
    timeout 120 qemu-system-x86_64 \
        -machine q35 -m 256 -display none -nographic \
        -drive if=pflash,format=raw,unit=0,file=/usr/share/OVMF/OVMF_CODE_4M.fd,readonly=on \
        -drive if=pflash,format=raw,unit=1,file="$WORK/OVMF_VARS.fd",format=raw \
        -drive file=fat:rw:"$ESP",format=raw \
        -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
        -debugcon "file:$serial" \
        -serial none -monitor none || true
    if [ "$logo_present" = "yes" ]; then
        grep -q "E2E_PAYLOAD_START" "$serial"
        grep -q "BGRT_FOUND" "$serial"
        grep -q "IMAGE_MAGIC=BM" "$serial"
        grep -q "E2E_PAYLOAD_DONE" "$serial"
    else
        grep -q "E2E_PAYLOAD_START" "$serial"
        grep -q "E2E_PAYLOAD_DONE" "$serial"
    fi
    echo "$case_name: PASS"
}

run_case "with-logo" "yes"
run_case "no-logo" "no"

echo "All e2e cases passed."
