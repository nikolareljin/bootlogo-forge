# Introduction

bootlogo-forge shows your own image during boot on a UEFI system. It builds and
publishes an ACPI BGRT table that the firmware, Plymouth and Windows all read,
so the image stays on screen from the first boot loader to the desktop.

It runs from Linux. It does not touch firmware.

## The hard limit

The logo drawn by the firmware itself, before the operating system loader runs,
lives inside the firmware image. bootlogo-forge never reads or writes firmware.
The firmware logo still appears first, during POST. Your image appears once the
EFI application runs and publishes the BGRT table, and from there on. A firmware
update may replace the firmware logo.