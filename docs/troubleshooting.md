# Troubleshooting

## `status` shows `installed: false`

Either install has not run, or the ESP path is wrong. Pass `--esp`.

## `ESP directory not found`

The ESP is not mounted. Mount it, normally at `/boot/efi`, or pass `--esp`.

## `root privileges required for --apply`

Run with `sudo`.

## `ESP is not writable`

Check the mount and permissions. A dry run does not need write access.

## The firmware logo still shows

This is expected. The firmware draws its own logo first. bootlogo-forge cannot
change that. See [Introduction](./introduction.md).

## The laptop does not boot after install

Use the firmware boot menu (F12 on Acer, Lenovo and Dell, F9 on HP) and select
the original entry. Then run uninstall.