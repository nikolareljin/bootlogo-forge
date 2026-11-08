# How it works

1. The firmware draws its own logo first. That logo is inside the firmware image.
   bootlogo-forge never changes it.
2. The EFI application on the ESP loads `logo.bmp`, builds or replaces the ACPI
   BGRT table and chains to the original boot loader.
3. Plymouth's `bgrt` theme and Windows show the BGRT image, so your image stays
   on screen.

## Files written

On the ESP, under `EFI/bootlogo-forge`:

- `bootlogo-forge-efi.efi` - the EFI application
- `logo.bmp` - the converted 24-bit bitmap
- `config` - the chain target loader path
- `backup.json` - the previous boot order and created entry

## What is never touched

Firmware, flashrom, BIOS settings, existing boot entries, the original loader.