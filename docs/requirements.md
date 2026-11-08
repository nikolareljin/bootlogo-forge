# Requirements

UEFI boot. Secure Boot may be off, or you enroll the binary hash with a MOK
(see [Secure Boot](./secure-boot.md)). An ACPI BGRT table must be present.

Supported vendors:

| Vendor | Path | Status |
| --- | --- | --- |
| Acer | BGRT | tested in QEMU only |
| Lenovo | BGRT (native ESP logo documented, opt-in) | tested in QEMU only |
| Dell | BGRT | tested in QEMU only |
| HP | BGRT | tested in QEMU only |

No other vendors are supported.

## Distros

Tested build and run on Xubuntu 24.04 and later. Any recent Linux with UEFI
boot entries should work.

## Check Secure Boot

    mokutil --sb-state

## Check ACPI BGRT

    ls /sys/firmware/acpi/bgrt

If that directory exists, BGRT is available.