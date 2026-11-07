# Secure Boot

The produced binary is not signed. Either disable Secure Boot or enroll its hash
with a Machine Owner Key.

1. Disable Secure Boot in firmware, or keep it on and enroll the hash.
2. After install, enroll the hash:

    mokutil --import /boot/efi/EFI/bootlogo-forge/bootlogo-forge-efi.efi

3. Reboot and confirm the MOK enrollment prompt.

Signing automation is not provided. The user performs every step manually.