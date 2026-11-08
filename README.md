# bootlogo-forge

Replace the boot logo shown after firmware POST on Acer laptops (InsydeH2O firmware,
ACPI BGRT present) with your own image, installed from Linux.

## Hard limit

The logo drawn by the firmware itself, before the OS loader runs, lives inside the
firmware image. bootlogo-forge never reads or writes firmware. The first logo you
see during POST stays. The new logo appears once the EFI application runs and
publishes an ACPI BGRT table, and remains visible through Plymouth and Windows
through their BGRT handling. firmware update may replace the firmware logo.

## How it works

1. An EFI application on the EFI system partition loads a BMP, builds or replaces
   the ACPI BGRT table, and chains to the original boot loader (shim or GRUB).
2. If anything fails in the EFI application, it chains to the original loader
   unchanged and the machine still boots.

## Status

Acer first. Lenovo, Dell and HP get the BGRT path.

## Usage

See the installed help:

    bootlogo-forge --help

Detection:

    bootlogo-forge detect

Supported vendors: acer, lenovo, dell, hp, unknown.
