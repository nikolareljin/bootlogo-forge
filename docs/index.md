---
layout: default
title: bootlogo-forge
---

![bootlogo-forge hero](../assets/hero.svg)

# bootlogo-forge

Replace the boot logo after firmware POST with your own image on EFI systems, using ACPI BGRT.

![logo](../assets/logo.svg)

## Quick start

```bash
bootlogo-forge detect
sudo bootlogo-forge install --image ./logo.png --plymouth --apply
sudo bootlogo-forge uninstall --apply
```

## Limits

The firmware logo shown during POST is not changed. Recovery uses the firmware boot menu.

See the [repository](https://github.com/nikolareljin/bootlogo-forge/) for full details.