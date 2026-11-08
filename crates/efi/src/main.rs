#![cfg_attr(target_os = "uefi", no_main)]
#![cfg_attr(target_os = "uefi", no_std)]

#[cfg(target_os = "uefi")]
extern crate alloc;

#[cfg(target_os = "uefi")]
mod imp {
    use alloc::string::String;
    use alloc::vec::Vec;
    use core::ptr;
    use uefi::boot::{self, AllocateType, ScopedProtocol};
    use uefi::fs::FileSystem;
    use uefi::proto::console::gop::GraphicsOutput;
    use uefi::proto::media::fs::SimpleFileSystem;
    use uefi::{CString16, Handle, Status};
    use uefi_raw::table::boot::MemoryType;

    use bootlogo_forge_core::bgrt::{build_bgrt, BGRT_LEN};

    const CONFIG_PATH: &str = "\\EFI\\bootlogo-forge\\config";

    pub fn run() -> Status {
        let image_handle = boot::image_handle();

        let loader_path =
            read_config(image_handle).unwrap_or_else(|| String::from("\\EFI\\BOOT\\BOOTX64.EFI"));

        let _ = try_install(image_handle);

        chain_loader(image_handle, &loader_path)
    }

    fn try_install(image_handle: Handle) -> Result<(), ()> {
        let bmp = read_file(image_handle, "\\EFI\\bootlogo-forge\\logo.bmp").ok_or(())?;

        let mut screen_w = 0u32;
        let mut screen_h = 0u32;
        if let Ok(handle) = boot::get_handle_for_protocol::<GraphicsOutput>() {
            if let Ok(gop) = boot::open_protocol_exclusive::<GraphicsOutput>(handle) {
                let (w, h) = gop.current_mode_info().resolution();
                screen_w = w as u32;
                screen_h = h as u32;
            }
        }

        let (image_w, image_h) = bmp_dimensions(&bmp).ok_or(())?;

        let pages = bmp.len().div_ceil(4096);
        let image_addr: u64 = boot::allocate_pages(
            AllocateType::AnyPages,
            MemoryType::RUNTIME_SERVICES_DATA,
            pages,
        )
        .map_err(|_| ())?
        .as_ptr() as u64;
        // SAFETY: memory was just allocated for the full BMP.
        unsafe {
            ptr::write_bytes(image_addr as *mut u8, 0, pages * 4096);
            ptr::copy_nonoverlapping(bmp.as_ptr(), image_addr as *mut u8, bmp.len());
        }

        let table_bytes = build_bgrt(image_addr, image_w, image_h, screen_w, screen_h);
        let table_pages = table_bytes.len().div_ceil(4096);
        let table_addr: u64 = boot::allocate_pages(
            AllocateType::AnyPages,
            MemoryType::RUNTIME_SERVICES_DATA,
            table_pages,
        )
        .map_err(|_| ())?
        .as_ptr() as u64;
        // SAFETY: memory was just allocated for the BGRT table.
        unsafe {
            ptr::write_bytes(table_addr as *mut u8, 0, table_pages * 4096);
            ptr::copy_nonoverlapping(
                table_bytes.as_ptr(),
                table_addr as *mut u8,
                table_bytes.len(),
            );
        }

        update_xsdt(table_addr).map_err(|_| ())?;
        let _ = BGRT_LEN;
        Ok(())
    }

    fn read_config(image_handle: Handle) -> Option<String> {
        let bytes = read_file(image_handle, CONFIG_PATH)?;
        let text = core::str::from_utf8(&bytes).ok()?;
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("loader=") {
                let v = rest.trim();
                if !v.is_empty() {
                    return Some(String::from(v));
                }
            }
        }
        None
    }

    fn read_file(image_handle: Handle, path: &str) -> Option<Vec<u8>> {
        let fs_proto: ScopedProtocol<SimpleFileSystem> =
            boot::get_image_file_system(image_handle).ok()?;
        let mut fs = FileSystem::new(fs_proto);
        let cpath = CString16::try_from(path).ok()?;
        fs.read(cpath.as_ref()).ok()
    }

    fn bmp_dimensions(bmp: &[u8]) -> Option<(u32, u32)> {
        if bmp.len() < 26 {
            return None;
        }
        if &bmp[0..2] != b"BM" {
            return None;
        }
        let w = u32::from_le_bytes(bmp[18..22].try_into().ok()?);
        let h = u32::from_le_bytes(bmp[22..26].try_into().ok()?);
        if w == 0 || h == 0 {
            return None;
        }
        Some((w, h))
    }

    fn table_signature(addr: u64) -> Option<[u8; 4]> {
        if addr == 0 {
            return None;
        }
        let mut sig = [0u8; 4];
        // SAFETY: addr validated non-zero.
        unsafe {
            core::ptr::copy_nonoverlapping(addr as *const u8, sig.as_mut_ptr(), 4);
        }
        Some(sig)
    }

    fn update_xsdt(bgrt_addr: u64) -> Result<(), ()> {
        let st_raw = uefi::table::system_table_raw().ok_or(())?;
        // SAFETY: system table pointer provided by UEFI firmware.
        let st = unsafe { &*st_raw.as_ptr() };

        let mut rsdp_addr: u64 = 0;
        for i in 0..st.number_of_configuration_table_entries {
            // SAFETY: configuration table entries from firmware.
            let entry = unsafe { &*st.configuration_table.add(i) };
            if entry.vendor_guid == uefi::table::cfg::ConfigTableEntry::ACPI2_GUID {
                rsdp_addr = entry.vendor_table as u64;
                break;
            }
        }
        if rsdp_addr == 0 {
            return Err(());
        }

        // SAFETY: RSDP physical address from firmware table.
        let xsdt_addr = unsafe {
            u64::from_le_bytes(
                core::slice::from_raw_parts((rsdp_addr + 24) as *const u8, 8)
                    .try_into()
                    .unwrap(),
            )
        };
        if xsdt_addr == 0 {
            return Err(());
        }

        unsafe {
            let hdr = xsdt_addr as *const u8;
            let length = u32::from_le_bytes(
                core::slice::from_raw_parts(hdr.add(4), 4)
                    .try_into()
                    .unwrap(),
            ) as usize;
            if length < 36 {
                return Err(());
            }
            let entry_count = (length - 36) / 8;

            let mut entries: Vec<u64> = Vec::with_capacity(entry_count + 1);
            for i in 0..entry_count {
                let addr = xsdt_addr as usize + 36 + i * 8;
                let val = u64::from_le_bytes(
                    core::slice::from_raw_parts(addr as *const u8, 8)
                        .try_into()
                        .unwrap(),
                );
                entries.push(val);
            }

            let mut new_entries: Vec<u64> = Vec::new();
            let mut replaced = false;
            for &e in &entries {
                if table_signature(e) == Some(*b"BGRT") {
                    if !replaced {
                        new_entries.push(bgrt_addr);
                        replaced = true;
                    }
                } else {
                    new_entries.push(e);
                }
            }
            if !replaced {
                new_entries.push(bgrt_addr);
            }

            let new_len = 36 + new_entries.len() * 8;
            let pages = new_len.div_ceil(4096);
            let new_xsdt: u64 = boot::allocate_pages(
                AllocateType::AnyPages,
                MemoryType::RUNTIME_SERVICES_DATA,
                pages,
            )
            .map_err(|_| ())?
            .as_ptr() as u64;
            ptr::write_bytes(new_xsdt as *mut u8, 0, pages * 4096);

            ptr::copy_nonoverlapping(xsdt_addr as *const u8, new_xsdt as *mut u8, 36);
            let len_bytes = (new_len as u32).to_le_bytes();
            ptr::copy_nonoverlapping(len_bytes.as_ptr(), (new_xsdt as *mut u8).add(4), 4);

            for (i, &e) in new_entries.iter().enumerate() {
                let dst = (new_xsdt as usize + 36 + i * 8) as *mut u8;
                let bytes = e.to_le_bytes();
                core::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, 8);
            }

            {
                let bytes = core::slice::from_raw_parts_mut(new_xsdt as *mut u8, new_len);
                bytes[9] = 0;
                let mut sum: u8 = 0;
                for &b in bytes.iter() {
                    sum = sum.wrapping_add(b);
                }
                bytes[9] = sum.wrapping_neg();
            }

            let xsdt_le = new_xsdt.to_le_bytes();
            ptr::copy_nonoverlapping(xsdt_le.as_ptr(), (rsdp_addr + 24) as *mut u8, 8);

            {
                let rsdp_bytes = core::slice::from_raw_parts_mut(rsdp_addr as *mut u8, 20);
                rsdp_bytes[9] = 0;
                let mut sum: u8 = 0;
                for &b in rsdp_bytes.iter() {
                    sum = sum.wrapping_add(b);
                }
                rsdp_bytes[9] = sum.wrapping_neg();
            }
            {
                let rsdp_bytes = core::slice::from_raw_parts_mut(rsdp_addr as *mut u8, 36);
                rsdp_bytes[32] = 0;
                let mut sum: u8 = 0;
                for &b in rsdp_bytes.iter() {
                    sum = sum.wrapping_add(b);
                }
                rsdp_bytes[32] = sum.wrapping_neg();
            }
        }

        Ok(())
    }

    fn chain_loader(image_handle: Handle, path: &str) -> Status {
        if let Some(data) = read_file(image_handle, path) {
            let source = uefi::boot::LoadImageSource::FromBuffer {
                buffer: data.as_slice(),
                file_path: None,
            };
            if let Ok(handle) = boot::load_image(image_handle, source) {
                let _ = boot::start_image(handle);
            }
        }
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
