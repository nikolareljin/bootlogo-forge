#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

export RUSTC="$(rustup which --toolchain stable rustc)"

echo "Building EFI app and payload..."
rustup run stable cargo build -p bootlogo-forge-efi --target x86_64-unknown-uefi --manifest-path "$REPO_ROOT/Cargo.toml"
rustup run stable cargo build -p bootlogo-forge-e2e-payload --target x86_64-unknown-uefi --manifest-path "$REPO_ROOT/Cargo.toml"

EFI_APP="$REPO_ROOT/target/x86_64-unknown-uefi/debug/bootlogo-forge-efi.efi"
PAYLOAD="$REPO_ROOT/target/x86_64-unknown-uefi/debug/payload.efi"
if [[ ! -f "$EFI_APP" ]]; then
    EFI_APP="$REPO_ROOT/target/x86_64-unknown-uefi/debug/deps/bootlogo_forge_efi.efi"
fi
if [[ ! -f "$PAYLOAD" ]]; then
    PAYLOAD="$REPO_ROOT/target/x86_64-unknown-uefi/debug/deps/payload.efi"
fi

ESP="$WORK/esp"
mkdir -p "$ESP/EFI/BOOT" "$ESP/EFI/bootlogo-forge"
cp "$EFI_APP" "$ESP/EFI/BOOT/BOOTX64.EFI"
cp "$PAYLOAD" "$ESP/EFI/BOOT/payload.efi"

cp /usr/share/OVMF/OVMF_VARS_4M.fd "$WORK/OVMF_VARS.fd"

# Case 1: logo present
python3 - "$ESP/EFI/bootlogo-forge/logo.bmp" <<'PY'
import struct, sys
# 4x4 24-bit red BMP
w,h = 4,4
row = bytes([0x00,0x00,0xFF]*w) + bytes((-3*w)%3)
pixels = row*h
header = struct.pack('<2sIHHI', b'BM', 54+len(pixels), 0, 0, 54)
dib = struct.pack('<IiiHHIIIIII', 40, w, h, 1, 24, 0, len(pixels), 2835, 2835, 0, 0)
with open(sys.argv[1], 'wb') as f:
    f.write(header+dib+pixels)
PY
printf 'loader=\\EFI\\BOOT\\payload.efi\n' > "$ESP/EFI/bootlogo-forge/config"

run_case() {
    local case_name="$1"
    local logo_present="$2"
    local serial="$WORK/serial-$case_name.log"
    rm -f "$serial"
    if [[ "$logo_present" == "no" ]]; then
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
    if [[ "$logo_present" == "yes" ]]; then
        grep -q "E2E_PAYLOAD_START" "$serial"
        grep -q "BGRT_FOUND" "$serial"
        grep -q "IMAGE_MAGIC=BM" "$serial"
        grep -q "E2E_PAYLOAD_DONE" "$serial"
    else
        grep -q "E2E_PAYLOAD_START" "$serial"
        grep -q "E2E_PAYLOAD_DONE" "$serial"
    fi
    echo "$case_name: PASS"
    # restore logo for next case
    if [[ "$logo_present" == "no" ]]; then
        python3 - "$ESP/EFI/bootlogo-forge/logo.bmp" <<'PY'
import struct, sys
w,h=4,4
row=bytes([0x00,0x00,0xFF]*w)+bytes((-3*w)%3)
pixels=row*h
header=struct.pack('<2sIHHI',b'BM',54+len(pixels),0,0,54)
dib=struct.pack('<IiiHHIIIIII',40,w,h,1,24,0,len(pixels),2835,2835,0,0)
with open(sys.argv[1],'wb') as f: f.write(header+dib+pixels)
PY
        printf 'loader=\\EFI\\BOOT\\payload.efi\n' > "$ESP/EFI/bootlogo-forge/config"
    fi
}

run_case "with-logo" "yes"
run_case "no-logo" "no"

echo "All e2e cases passed."
