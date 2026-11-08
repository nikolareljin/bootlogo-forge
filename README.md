⚠️ THIS PROJECT IS STILL UNDER DEVELOPMENT. DO NOT INSTALL OR USE YET ⚠️ 

![bootlogo-forge hero](assets/hero.svg)

# bootlogo-forge

Replace the boot logo shown after firmware POST on UEFI systems, installed from
Linux. The EFI application loads a BMP, publishes an ACPI BGRT table and chains
to the original loader.

Full documentation: https://nikolareljin.github.io/bootlogo-forge/

## Hard limit

The firmware logo appears first, during POST. bootlogo-forge never reads or
writes firmware. Your image appears once the EFI application runs.

## Quick start

    bootlogo-forge detect
    sudo bootlogo-forge install --image ./logo.png --plymouth --apply
    sudo bootlogo-forge uninstall --apply

Dry run is the default. Add `--apply` to change anything.

## Recovery

If boot fails, use the firmware boot menu (F12 on Acer, Lenovo and Dell, F9 on
HP) to pick the original entry, then run uninstall.
