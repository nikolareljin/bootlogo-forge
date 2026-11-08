# Usage

Every command defaults to dry run for anything that writes. Pass `--apply` to
make changes. Pass `--apply` without root exits non-zero.

## detect

    $ bootlogo-forge detect
    vendor: acer
    model: (your model)
    backend: bgrt

Vendor detection reads the DMI directory. You can point it at a copy for testing:

    $ bootlogo-forge detect --dmi-dir ./fixtures/dmi/acer

## status

    $ bootlogo-forge status

Prints the BGRT fields from sysfs, the ESP mount, the efibootmgr output and
whether bootlogo-forge is installed. Sample:

    sysfs root: /sys
    bgrt version: 1
    bgrt status: 0
    bgrt type: 0
    bgrt xoffset: 320
    bgrt yoffset: 240
    bgrt image size: 123456
    ESP: /boot/efi
    ...efibootmgr output...
    installed: false

## install

Dry run:

    $ bootlogo-forge install --image ./logo.png
    convert image to /boot/efi/EFI/bootlogo-forge/logo.bmp
    copy EFI app to /boot/efi/EFI/bootlogo-forge/bootlogo-forge-efi.efi
    write config targeting \EFI\ubuntu\shimx64.efi
    create or reuse boot entry labeled bootlogo-forge
    DRY RUN: would set BootOrder with bootlogo-forge first

Apply:

    $ sudo bootlogo-forge install --image ./logo.png --apply

With Plymouth:

    $ sudo bootlogo-forge install --image ./logo.png --plymouth --apply

Flags:

    --image PATH              input PNG or JPEG
    --esp PATH                override the ESP mount
    --efi-app PATH            override the EFI application binary
    --plymouth                also set the Plymouth bgrt theme
    --backend bgrt            default backend

Lenovo native backend is opt-in and dry-run only:

    $ bootlogo-forge install --backend lenovo-native --image ./logo.png
    backend: lenovo-native (dry-run; vendor native ESP logo is not automated)
    would copy a logo BMP to the ESP root for a ThinkPad

## uninstall

Dry run:

    $ bootlogo-forge uninstall
    restore boot order
    remove entry labeled bootlogo-forge
    remove /boot/efi/EFI/bootlogo-forge

Apply:

    $ sudo bootlogo-forge uninstall --apply

Uninstall restores the saved boot order, removes only its own entry and
removes its directory from the ESP.

## help

    $ bootlogo-forge --help
    Replace the boot logo via ACPI BGRT

    Usage: bootlogo-forge <COMMAND>

    Commands:
      detect
      status
      install
      uninstall
      help       Print help

    Options:
      -h, --help     Print help
      -V, --version  Print version