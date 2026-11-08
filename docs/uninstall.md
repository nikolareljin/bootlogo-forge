# Uninstall and recovery

## Uninstall

    sudo bootlogo-forge uninstall --apply

This restores the saved boot order, deletes only the entry bootlogo-forge
created and removes `/EFI/bootlogo-forge` from the ESP.

If Plymouth was enabled, uninstall restores the saved theme and rebuilds the
initramfs.

## Dry run

    bootlogo-forge uninstall

## Recovery

If the laptop does not boot through the new entry, open the firmware boot menu
and select the original entry:

- Acer: F12
- Lenovo: F12
- Dell: F12
- HP: F9

Then run uninstall as above. The firmware logo is never changed.