![bootlogo-forge hero](assets/hero.svg)

# bootlogo-forge

Replace the boot logo shown after firmware POST on Acer laptops (InsydeH2O firmware,
ACPI BGRT present) with your own image, installed from Linux.

## Hard limit

The logo drawn by the firmware itself, before the OS loader runs, lives inside the
firmware image. bootlogo-forge never reads or writes firmware. The first logo you
see during POST stays. The new logo appears once the EFI application runs and
publishes an ACPI BGRT table, and remains visible through Plymouth and Windows
through their BGRT handling. A firmware update may replace the firmware logo.

## How it works

1. An EFI application on the EFI system partition loads a BMP, builds or replaces
   the ACPI BGRT table, and chains to the original boot loader (shim or GRUB).
2. If anything fails in the EFI application, it chains to the original loader
   unchanged and the machine still boots.

## Requirements

- Vendor detection selects the BGRT backend; Lenovo native support is opt-in.
- Secure Boot off, or enrollment of the binary hash via MOK (not automated).
- ACPI BGRT present.

## Usage

Detection:

    bootlogo-forge detect

Install (dry run; add --apply to change anything):

    sudo bootlogo-forge install --image ./logo.png --plymouth --apply

The laptop model is not required. The command saves the previous boot order,
installs the EFI application, converts the image to BMP, and (with --plymouth)
sets the Plymouth bgrt theme.

Uninstall:

    sudo bootlogo-forge uninstall --apply

Status:

    bootlogo-forge status

## Recovery

If the boot entry fails, open the firmware boot menu (F12 on Acer) and select the
original boot entry. Uninstall restores the saved boot order.

## Secure Boot

The binary is not signed automatically. Use a Machine Owner Key (MOK) to enroll
its hash with mokutil, or disable Secure Boot. Signing automation is not provided.

## Supported vendors

| Vendor | Path | Status |
| --- | --- | --- |
| Acer | BGRT | tested in QEMU only |
| Lenovo | BGRT (native ESP logo documented, opt-in) | tested in QEMU only |
| Dell | BGRT | tested in QEMU only |
| HP | BGRT | tested in QEMU only |

No others are supported.