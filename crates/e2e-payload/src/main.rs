#![cfg_attr(target_os = "uefi", no_main)]
#![cfg_attr(target_os = "uefi", no_std)]

#[cfg(target_os = "uefi")]
extern crate alloc;

#[cfg(target_os = "uefi")]
mod imp {
    use core::arch::asm;
    use uefi::Status;

    #[repr(C)]
    struct RawCfg {
        guid: [u8; 16],
        vendor_table: *const u8,
    }

    const ACPI2_GUID: [u8; 16] = [
        0x71, 0xe8, 0x68, 0x88, 0xf1, 0xe4, 0xd3, 0x11, 0xbc, 0x22, 0x00, 0x80, 0xc7, 0x3c, 0x88,
        0x81,
    ];

    fn dbg_byte(b: u8) {
        unsafe {
            asm!("out dx, al", in("dx") 0xE9u16, in("al") b);
            asm!("out dx, al", in("dx") 0x3F8u16, in("al") b);
        }
    }

    fn dbg(s: &str) {
        for &b in s.as_bytes() {
            dbg_byte(b);
        }
    }

    fn exit_qemu() {
        unsafe {
            asm!("out dx, al", in("dx") 0xF4u16, in("al") 0u8);
        }
    }

    pub fn run() -> Status {
        dbg("E2E_PAYLOAD_START\n");
        let mut found = false;
        if let Some(st) = uefi::table::system_table_raw() {
            let st = unsafe { &*st.as_ptr() };
            for i in 0..st.number_of_configuration_table_entries {
                let entry = unsafe { &*(st.configuration_table as *const RawCfg).add(i) };
                if entry.guid == ACPI2_GUID {
                    let rsdp = entry.vendor_table as usize;
                    if rsdp != 0 {
                        let xsdt_addr = unsafe {
                            u64::from_le_bytes(
                                core::slice::from_raw_parts((rsdp + 24) as *const u8, 8)
                                    .try_into()
                                    .unwrap(),
                            )
                        } as usize;
                        if xsdt_addr != 0 {
                            let len = unsafe {
                                u32::from_le_bytes(
                                    core::slice::from_raw_parts(xsdt_addr as *const u8, 8)[4..8]
                                        .try_into()
                                        .unwrap(),
                                )
                            } as usize;
                            let _ = len;
                            let entry_count = if xsdt_addr != 0 {
                                let hdr = unsafe {
                                    u32::from_le_bytes(
                                        core::slice::from_raw_parts(xsdt_addr as *const u8, 8)
                                            [4..8]
                                            .try_into()
                                            .unwrap(),
                                    )
                                } as usize;
                                (hdr.saturating_sub(36)) / 8
                            } else {
                                0
                            };
                            for j in 0..entry_count {
                                let addr = unsafe {
                                    u64::from_le_bytes(
                                        core::slice::from_raw_parts(
                                            (xsdt_addr + 36 + j * 8) as *const u8,
                                            8,
                                        )
                                        .try_into()
                                        .unwrap(),
                                    )
                                } as usize;
                                if addr == 0 {
                                    continue;
                                }
                                let mut sig = [0u8; 4];
                                unsafe {
                                    core::ptr::copy_nonoverlapping(
                                        addr as *const u8,
                                        sig.as_mut_ptr(),
                                        4,
                                    );
                                }
                                if sig == *b"BGRT" {
                                    found = true;
                                    let img_addr = unsafe {
                                        u64::from_le_bytes(
                                            core::slice::from_raw_parts(
                                                (addr + 40) as *const u8,
                                                8,
                                            )
                                            .try_into()
                                            .unwrap(),
                                        )
                                    };
                                    dbg(&alloc::format!("BGRT_FOUND addr=0x{:X}\n", img_addr));
                                    let magic0 = unsafe { *((img_addr as usize) as *const u8) };
                                    let magic1 = unsafe { *((img_addr as usize + 1) as *const u8) };
                                    if magic0 == b'B' && magic1 == b'M' {
                                        dbg("IMAGE_MAGIC=BM\n");
                                    } else {
                                        dbg(&alloc::format!(
                                            "IMAGE_MAGIC=0x{:02X}{:02X}\n",
                                            magic0,
                                            magic1
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    break;
                }
            }
        }
        if !found {
            dbg("NO_BGRT\n");
        }
        dbg("E2E_PAYLOAD_DONE\n");
        exit_qemu();
        Status::SUCCESS
    }
}

#[cfg(target_os = "uefi")]
#[uefi::entry]
fn main() -> uefi::Status {
    imp::run()
}

#[cfg(not(target_os = "uefi"))]
fn main() {}
