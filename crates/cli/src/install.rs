use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const ENTRY_LABEL: &str = "bootlogo-forge";
pub const EFI_APP_NAME: &str = "bootlogo-forge-efi.efi";
pub const LOGO_NAME: &str = "logo.bmp";
pub const CONFIG_NAME: &str = "config";
pub const BACKUP_NAME: &str = "backup.json";

#[derive(Debug)]
pub enum InstallError {
    MissingEsp,
    ReadOnlyEsp,
    NotRoot,
    Image(String),
    Io(String),
    Efibootmgr(String),
    Parse(String),
    MissingEfiApp,
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEsp => f.write_str("ESP directory not found"),
            Self::ReadOnlyEsp => f.write_str("ESP is not writable"),
            Self::NotRoot => f.write_str("root privileges required for --apply"),
            Self::Image(m) => write!(f, "image error: {}", m),
            Self::Io(m) => write!(f, "io error: {}", m),
            Self::Efibootmgr(m) => write!(f, "efibootmgr failed: {}", m),
            Self::Parse(m) => write!(f, "parse error: {}", m),
            Self::MissingEfiApp => f.write_str("EFI application binary not found"),
        }
    }
}

impl std::error::Error for InstallError {}

#[derive(Clone)]
pub struct InstallOptions {
    pub image: PathBuf,
    pub esp: PathBuf,
    pub efibootmgr: PathBuf,
    pub efi_app: Option<PathBuf>,
    pub dry_run: bool,
    pub apply: bool,
    pub is_root: bool,
}

#[derive(Clone)]
pub struct UninstallOptions {
    pub esp: PathBuf,
    pub efibootmgr: PathBuf,
    pub dry_run: bool,
    pub apply: bool,
    pub is_root: bool,
}

#[derive(Clone)]
pub struct StatusOptions {
    pub esp: Option<PathBuf>,
    pub efibootmgr: PathBuf,
    pub sysfs_root: PathBuf,
}

fn io_err(e: std::io::Error) -> InstallError {
    InstallError::Io(e.to_string())
}

fn mutate(bin: &Path, args: &[String], apply: bool) -> Result<String, InstallError> {
    if !apply {
        return Ok(String::new());
    }
    let output = Command::new(bin)
        .args(args)
        .output()
        .map_err(|e| InstallError::Efibootmgr(e.to_string()))?;
    if !output.status.success() {
        return Err(InstallError::Efibootmgr(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn get_boot_order(bin: &Path) -> Result<Vec<String>, InstallError> {
    let output = Command::new(bin)
        .output()
        .map_err(|e| InstallError::Efibootmgr(e.to_string()))?;
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        if let Some(rest) = line.trim().strip_prefix("BootOrder:") {
            return Ok(rest
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect());
        }
    }
    Ok(Vec::new())
}

fn find_entry_number(bin: &Path, label: &str, loader: &str) -> Option<String> {
    let output = Command::new(bin).output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let loader_norm = loader.replace('\\', "/").to_lowercase();
    for line in text.lines() {
        let t = line.trim();
        if !t.starts_with("Boot") {
            continue;
        }
        let Some(num) = t.get(4..8) else { continue };
        if !num.chars().all(|c| c.is_ascii_hexdigit()) {
            continue;
        }
        // check arrow or star
        if t.contains(label) {
            // ensure loader path also present
            let line_norm = t.replace('\\', "/").to_lowercase();
            if line_norm.contains(&loader_norm.to_lowercase()) {
                return Some(num.to_string());
            }
        }
    }
    None
}

fn detect_loader(esp: &Path) -> String {
    let mut shim: Option<PathBuf> = None;
    let mut grub: Option<PathBuf> = None;
    fn walk(dir: &Path, shim: &mut Option<PathBuf>, grub: &mut Option<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, shim, grub);
                } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    let lower = name.to_lowercase();
                    if lower == "shimx64.efi" && shim.is_none() {
                        *shim = Some(path.clone());
                    } else if lower == "grubx64.efi" && grub.is_none() {
                        *grub = Some(path.clone());
                    }
                }
            }
        }
    }
    let efi_dir = esp.join("EFI");
    walk(&efi_dir, &mut shim, &mut grub);
    let chosen = shim.or(grub);
    if let Some(p) = chosen {
        if let Ok(rel) = p.strip_prefix(esp) {
            return format!("\\{}", rel.to_string_lossy().replace('/', "\\"));
        }
    }
    String::from("\\EFI\\BOOT\\BOOTX64.EFI")
}

fn read_sysfs_value(root: &Path, name: &str) -> String {
    fs::read_to_string(root.join("bgrt").join(name))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| String::from("unknown"))
}

pub fn install(opts: InstallOptions) -> Result<Vec<String>, InstallError> {
    let mut actions = Vec::new();
    if !opts.esp.is_dir() {
        return Err(InstallError::MissingEsp);
    }
    if opts.apply && !opts.is_root {
        return Err(InstallError::NotRoot);
    }
    let target_dir = opts.esp.join("EFI").join("bootlogo-forge");
    let target_existed_before = target_dir.exists();

    // writability check
    fs::create_dir_all(opts.esp.join("EFI")).map_err(io_err)?;
    // try creating target
    if !target_dir.exists() {
        fs::create_dir_all(&target_dir).map_err(|_| InstallError::ReadOnlyEsp)?;
    } else {
        // test writability with temp file
        let test_path = target_dir.join(".write_test");
        fs::write(&test_path, b"").map_err(|_| InstallError::ReadOnlyEsp)?;
        let _ = fs::remove_file(&test_path);
    }

    let efi_app = match &opts.efi_app {
        Some(p) => p.clone(),
        None => {
            let candidate = PathBuf::from("/usr/share/bootlogo-forge").join(EFI_APP_NAME);
            if candidate.exists() {
                candidate
            } else {
                // fall back to current working dir
                std::env::current_dir()
                    .unwrap_or_default()
                    .join(EFI_APP_NAME)
            }
        }
    };
    if !efi_app.exists() {
        return Err(InstallError::MissingEfiApp);
    }

    // convert image
    let input = fs::read(&opts.image).map_err(|e| InstallError::Image(e.to_string()))?;
    let decoded_dims = bootlogo_forge_core::parse_dimensions(&input)
        .map_err(|e| InstallError::Image(e.to_string()))?;
    let bmp = bootlogo_forge_core::render_bmp(&input, decoded_dims.0, decoded_dims.1)
        .map_err(|e| InstallError::Image(e.to_string()))?;

    let loader = detect_loader(&opts.esp);

    actions.push(format!(
        "convert image to {}",
        target_dir.join(LOGO_NAME).display()
    ));
    actions.push(format!(
        "copy EFI app to {}",
        target_dir.join(EFI_APP_NAME).display()
    ));
    actions.push(format!("write config targeting {}", loader));
    actions.push(format!(
        "create or reuse boot entry labeled {}",
        ENTRY_LABEL
    ));

    if opts.dry_run {
        actions.push(format!(
            "DRY RUN: would set BootOrder with {} first",
            ENTRY_LABEL
        ));
        return Ok(actions);
    }
    if opts.apply && !opts.is_root {
        return Err(InstallError::NotRoot);
    }
    if !opts.apply {
        return Ok(actions);
    }

    // actual apply
    let created_dir = !target_existed_before;
    let apply_result = (|| -> Result<(), InstallError> {
        fs::create_dir_all(&target_dir).map_err(io_err)?;
        // backup previous boot order before changes
        let original_order = get_boot_order(&opts.efibootmgr)?;
        let existing = find_entry_number(&opts.efibootmgr, ENTRY_LABEL, &loader);
        let created = if let Some(e) = existing {
            e
        } else {
            mutate(
                &opts.efibootmgr,
                &[
                    String::from("--create"),
                    String::from("--label"),
                    String::from(ENTRY_LABEL),
                    String::from("--loader"),
                    loader.clone(),
                ],
                true,
            )?;
            find_entry_number(&opts.efibootmgr, ENTRY_LABEL, &loader)
                .ok_or_else(|| InstallError::Parse(String::from("created entry not found")))?
        };
        let backup = format!(
            r#"{{"boot_order":[{}],"created_entry":"{}","loader":"{}"}}"#,
            original_order
                .iter()
                .map(|s| format!(r#""{}""#, s))
                .collect::<Vec<_>>()
                .join(","),
            created,
            loader.replace('\\', "\\\\"),
        );
        fs::write(target_dir.join(BACKUP_NAME), backup).map_err(io_err)?;

        // copy files
        fs::copy(&efi_app, target_dir.join(EFI_APP_NAME)).map_err(io_err)?;
        fs::write(target_dir.join(LOGO_NAME), &bmp).map_err(io_err)?;
        fs::write(target_dir.join(CONFIG_NAME), format!("loader={}\n", loader)).map_err(io_err)?;

        // set boot order
        let mut new_order = vec![created.clone()];
        for b in &original_order {
            if *b != created {
                new_order.push(b.clone());
            }
        }
        mutate(
            &opts.efibootmgr,
            &[String::from("-o"), new_order.join(",")],
            true,
        )?;

        Ok(())
    })();

    if let Err(e) = apply_result {
        if created_dir {
            let _ = fs::remove_dir_all(&target_dir);
        }
        return Err(e);
    }

    actions.push(String::from("install complete"));
    Ok(actions)
}

pub fn uninstall(opts: UninstallOptions) -> Result<Vec<String>, InstallError> {
    let mut actions = Vec::new();
    if !opts.esp.is_dir() {
        return Err(InstallError::MissingEsp);
    }
    if opts.apply && !opts.is_root {
        return Err(InstallError::NotRoot);
    }
    let target_dir = opts.esp.join("EFI").join("bootlogo-forge");
    if !target_dir.exists() {
        actions.push(String::from("nothing to uninstall"));
        return Ok(actions);
    }

    // read backup
    let backup_path = target_dir.join(BACKUP_NAME);
    let backup = fs::read_to_string(&backup_path).ok();
    let (original_order, created_entry, loader): (Vec<String>, Option<String>, Option<String>) =
        if let Some(text) = backup {
            let order = parse_json_array(&text, "boot_order");
            let entry = parse_json_string(&text, "created_entry");
            let loader = parse_json_string(&text, "loader");
            (order, entry, loader)
        } else {
            (Vec::new(), None, None)
        };

    let loader = loader.unwrap_or_else(|| detect_loader(&opts.esp));

    actions.push(String::from("restore boot order"));
    actions.push(format!("remove entry labeled {}", ENTRY_LABEL));
    actions.push(format!("remove {}", target_dir.display()));

    if opts.dry_run {
        return Ok(actions);
    }
    if opts.apply && !opts.is_root {
        return Err(InstallError::NotRoot);
    }
    if !opts.apply {
        return Ok(actions);
    }

    // delete matching entry
    let entry_to_delete =
        created_entry.or_else(|| find_entry_number(&opts.efibootmgr, ENTRY_LABEL, &loader));
    if let Some(entry) = &entry_to_delete {
        let _ = mutate(
            &opts.efibootmgr,
            &[String::from("-B"), String::from("-b"), entry.clone()],
            true,
        );
    }
    if !original_order.is_empty() {
        let _ = mutate(
            &opts.efibootmgr,
            &[String::from("-o"), original_order.join(",")],
            true,
        );
    }
    let _ = fs::remove_dir_all(&target_dir);
    actions.push(String::from("uninstall complete"));
    Ok(actions)
}

pub fn status(opts: StatusOptions) -> Result<String, InstallError> {
    let mut out = String::new();
    let sys_root = opts.sysfs_root.clone();
    out.push_str(&format!("sysfs root: {}\n", sys_root.display()));
    for name in ["version", "status", "type", "xoffset", "yoffset"] {
        let val = read_sysfs_value(&sys_root, name);
        out.push_str(&format!("bgrt {}: {}\n", name, val));
    }
    let image_path = sys_root.join("bgrt").join("image");
    if let Ok(meta) = fs::metadata(&image_path) {
        out.push_str(&format!("bgrt image size: {}\n", meta.len()));
    } else {
        out.push_str("bgrt image: unavailable\n");
    }

    let esp = opts
        .esp
        .clone()
        .unwrap_or_else(|| PathBuf::from("/boot/efi"));
    out.push_str(&format!("ESP: {}\n", esp.display()));

    let entries = Command::new(&opts.efibootmgr).output().ok();
    if let Some(o) = entries {
        out.push_str(&String::from_utf8_lossy(&o.stdout));
    } else {
        out.push_str("efibootmgr: unavailable\n");
    }

    let installed = esp
        .join("EFI")
        .join("bootlogo-forge")
        .join(BACKUP_NAME)
        .exists();
    out.push_str(&format!("installed: {}\n", installed));
    Ok(out)
}

pub fn findmnt_esp() -> Option<PathBuf> {
    let output = Command::new("findmnt")
        .args(["-rn", "-o", "TARGET", "--target", "/boot/efi"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().next()?.trim();
    if line.is_empty() {
        None
    } else {
        Some(PathBuf::from(line))
    }
}

fn parse_json_array(text: &str, key: &str) -> Vec<String> {
    let n1 = format!("\"{}\": [", key);
    let n2 = format!("\"{}\": [", key);
    for needle in [n1, n2] {
        if let Some(pos) = text.find(&needle) {
            let rest = &text[pos + needle.len()..];
            if let Some(end) = rest.find(']') {
                let body = &rest[..end];
                return body
                    .split(',')
                    .map(|s| s.trim().trim_matches('"').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
    }
    Vec::new()
}

fn parse_json_string(text: &str, key: &str) -> Option<String> {
    let needle = format!(r#""{}":""#, key);
    let pos = text.find(&needle)?;
    let rest = &text[pos + needle.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].replace("\\\\", "\\"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{codecs::png::PngEncoder, DynamicImage, ImageBuffer, ImageEncoder, Rgb};
    use std::io::Cursor;
    use std::os::unix::fs::PermissionsExt;

    fn temp_dir(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("blf-install-{}-{}", std::process::id(), name));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn make_png() -> Vec<u8> {
        let buf = ImageBuffer::from_pixel(4, 4, Rgb([10u8, 20, 30]));
        let img = DynamicImage::ImageRgb8(buf);
        let mut out = Vec::new();
        let mut cur = Cursor::new(&mut out);
        let enc = PngEncoder::new(&mut cur);
        enc.write_image(img.as_bytes(), 4, 4, image::ExtendedColorType::Rgb8)
            .unwrap();
        out
    }

    fn make_stub(dir: &Path, record: &Path, fail: bool, entry: &str) -> PathBuf {
        let path = dir.join("efibootmgr");
        let body = if fail {
            format!("#!/bin/sh\necho \"$@\" >> '{}'\nexit 1\n", record.display())
        } else {
            format!(
                "#!/bin/sh\necho \"$@\" >> '{}'\ncat <<'EOF'\nBootCurrent: 0000\nTimeout: 2 seconds\nBootOrder: 0000\nBoot0000* ubuntu\nBoot{}* {} /EFI/ubuntu/shimx64.efi\nEOF\nexit 0\n",
                record.display(),
                entry,
                ENTRY_LABEL
            )
        };
        std::fs::write(&path, body).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[test]
    fn install_uninstall_roundtrip() {
        let base = temp_dir("roundtrip");
        let esp = base.join("esp");
        std::fs::create_dir_all(esp.join("EFI").join("ubuntu")).unwrap();
        std::fs::write(esp.join("EFI").join("ubuntu").join("shimx64.efi"), b"x").unwrap();

        let image_path = base.join("image.png");
        std::fs::write(&image_path, make_png()).unwrap();

        let efi_app = base.join(EFI_APP_NAME);
        std::fs::write(&efi_app, b"EFIAPP").unwrap();

        let record = base.join("record");
        let stub = make_stub(&base, &record, false, "000F");

        let opts = InstallOptions {
            image: image_path.clone(),
            esp: esp.clone(),
            efibootmgr: stub.clone(),
            efi_app: Some(efi_app),
            dry_run: false,
            apply: true,
            is_root: true,
        };
        let actions = install(opts).unwrap();
        assert!(!actions.is_empty());
        let target = esp.join("EFI").join("bootlogo-forge");
        assert!(target.join(LOGO_NAME).exists());
        assert!(target.join(BACKUP_NAME).exists());
        assert!(target.join(CONFIG_NAME).exists());

        // install twice
        let opts2 = InstallOptions {
            image: image_path.clone(),
            esp: esp.clone(),
            efibootmgr: stub.clone(),
            efi_app: Some(base.join(EFI_APP_NAME)),
            dry_run: false,
            apply: true,
            is_root: true,
        };
        install(opts2).unwrap();

        let unopts = UninstallOptions {
            esp: esp.clone(),
            efibootmgr: stub.clone(),
            dry_run: false,
            apply: true,
            is_root: true,
        };
        uninstall(unopts).unwrap();
        assert!(!target.exists());
    }

    #[test]
    fn missing_esp() {
        let base = temp_dir("missing");
        let opts = InstallOptions {
            image: base.join("x.png"),
            esp: base.join("nope"),
            efibootmgr: base.join("efi"),
            efi_app: None,
            dry_run: false,
            apply: true,
            is_root: true,
        };
        assert!(matches!(install(opts), Err(InstallError::MissingEsp)));
    }

    #[test]
    fn readonly_esp() {
        let base = temp_dir("ro");
        let esp = base.join("esp");
        std::fs::create_dir_all(esp.join("EFI")).unwrap();
        std::fs::set_permissions(esp.join("EFI"), std::fs::Permissions::from_mode(0o555)).unwrap();
        let opts = InstallOptions {
            image: base.join("x.png"),
            esp: esp.clone(),
            efibootmgr: base.join("efi"),
            efi_app: None,
            dry_run: false,
            apply: true,
            is_root: true,
        };
        let res = install(opts);
        std::fs::set_permissions(esp.join("EFI"), std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(res, Err(InstallError::ReadOnlyEsp)));
        // no partial target
        assert!(!esp.join("EFI").join("bootlogo-forge").exists());
    }

    #[test]
    fn failing_efibootmgr() {
        let base = temp_dir("fail");
        let esp = base.join("esp");
        std::fs::create_dir_all(esp.join("EFI")).unwrap();
        let image_path = base.join("image.png");
        std::fs::write(&image_path, make_png()).unwrap();
        let efi_app = base.join(EFI_APP_NAME);
        std::fs::write(&efi_app, b"EFIAPP").unwrap();
        let record = base.join("record");
        let stub = make_stub(&base, &record, true, "000F");

        let opts = InstallOptions {
            image: image_path,
            esp: esp.clone(),
            efibootmgr: stub,
            efi_app: Some(efi_app),
            dry_run: false,
            apply: true,
            is_root: true,
        };
        let res = install(opts);
        assert!(matches!(res, Err(InstallError::Efibootmgr(_))));
        // no partial state
        assert!(!esp.join("EFI").join("bootlogo-forge").exists());
    }

    #[test]
    fn non_root_apply() {
        let base = temp_dir("nonroot");
        let esp = base.join("esp");
        std::fs::create_dir_all(esp.join("EFI")).unwrap();
        let opts = InstallOptions {
            image: base.join("x.png"),
            esp,
            efibootmgr: base.join("efi"),
            efi_app: None,
            dry_run: false,
            apply: true,
            is_root: false,
        };
        assert!(matches!(install(opts), Err(InstallError::NotRoot)));
    }
}
