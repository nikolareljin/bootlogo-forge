#![cfg_attr(target_os = "uefi", no_main)]
#![cfg_attr(target_os = "uefi", no_std)]

#[cfg(target_os = "uefi")]
use uefi::prelude::*;

#[cfg(target_os = "uefi")]
#[uefi::entry]
fn main() -> Status {
    Status::SUCCESS
}

#[cfg(not(target_os = "uefi"))]
fn main() {}
