use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub enum PlymouthError {
    MissingPlymouth,
    MissingUpdateInitramfs,
    NotRoot,
    CommandFailed(String),
    Io(String),
}

impl std::fmt::Display for PlymouthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPlymouth => f.write_str("plymouth-set-default-theme not found"),
            Self::MissingUpdateInitramfs => f.write_str("update-initramfs not found"),
            Self::NotRoot => f.write_str("root privileges required for --apply"),
            Self::CommandFailed(m) => write!(f, "plymouth command failed: {}", m),
            Self::Io(m) => write!(f, "io error: {}", m),
        }
    }
}

impl std::error::Error for PlymouthError {}

#[derive(Clone)]
pub struct PlymouthInstallOptions {
    pub esp: PathBuf,
    pub set_theme_bin: PathBuf,
    pub update_initramfs_bin: PathBuf,
    pub dry_run: bool,
    pub apply: bool,
    pub is_root: bool,
}

#[derive(Clone)]
pub struct PlymouthUninstallOptions {
    pub esp: PathBuf,
    pub set_theme_bin: PathBuf,
    pub update_initramfs_bin: PathBuf,
    pub dry_run: bool,
    pub apply: bool,
    pub is_root: bool,
}

fn target_dir(esp: &Path) -> PathBuf {
    esp.join("EFI").join("bootlogo-forge")
}

fn theme_file(esp: &Path) -> PathBuf {
    target_dir(esp).join("plymouth-theme")
}

fn run_cmd(bin: &Path, args: &[&str], apply: bool) -> Result<String, PlymouthError> {
    if !apply {
        return Ok(String::new());
    }
    for _ in 0..20 {
        let output = Command::new(bin)
            .args(args)
            .output()
            .map_err(|e| PlymouthError::CommandFailed(e.to_string()))?;
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if err.contains("Text file busy") {
            std::thread::sleep(std::time::Duration::from_millis(50));
            continue;
        }
        return Err(PlymouthError::CommandFailed(err));
    }
    Err(PlymouthError::CommandFailed(String::from("Text file busy")))
}

fn current_theme(bin: &Path) -> Result<String, PlymouthError> {
    for _ in 0..20 {
        let output = Command::new(bin)
            .output()
            .map_err(|e| PlymouthError::CommandFailed(e.to_string()))?;
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if err.contains("Text file busy") {
            std::thread::sleep(std::time::Duration::from_millis(50));
            continue;
        }
        return Err(PlymouthError::CommandFailed(err));
    }
    Err(PlymouthError::CommandFailed(String::from("Text file busy")))
}

pub fn plymouth_install(opts: PlymouthInstallOptions) -> Result<Vec<String>, PlymouthError> {
    if !opts.set_theme_bin.is_file() {
        return Err(PlymouthError::MissingPlymouth);
    }
    if !opts.update_initramfs_bin.is_file() && opts.apply {
        return Err(PlymouthError::MissingUpdateInitramfs);
    }
    let mut actions = Vec::new();
    if opts.apply && !opts.is_root {
        return Err(PlymouthError::NotRoot);
    }
    if opts.dry_run {
        actions.push(String::from("save current Plymouth theme"));
        actions.push(String::from("set Plymouth theme to bgrt"));
        actions.push(String::from("rebuild initramfs"));
        return Ok(actions);
    }
    if !opts.apply {
        return Ok(actions);
    }

    let saved = current_theme(&opts.set_theme_bin)?;
    fs::create_dir_all(target_dir(&opts.esp)).map_err(|e| PlymouthError::Io(e.to_string()))?;
    fs::write(theme_file(&opts.esp), format!("{}\n", saved))
        .map_err(|e| PlymouthError::Io(e.to_string()))?;
    run_cmd(&opts.set_theme_bin, &["bgrt"], true)?;
    run_cmd(&opts.update_initramfs_bin, &["-u"], true)?;
    actions.push(format!("saved Plymouth theme {}", saved));
    actions.push(String::from("set Plymouth theme to bgrt"));
    actions.push(String::from("rebuilt initramfs"));
    Ok(actions)
}

pub fn plymouth_uninstall(opts: PlymouthUninstallOptions) -> Result<Vec<String>, PlymouthError> {
    let mut actions = Vec::new();
    if !opts.set_theme_bin.is_file() {
        return Err(PlymouthError::MissingPlymouth);
    }
    if opts.apply && !opts.is_root {
        return Err(PlymouthError::NotRoot);
    }
    let saved_path = theme_file(&opts.esp);
    if !saved_path.exists() {
        actions.push(String::from("no saved Plymouth theme"));
        return Ok(actions);
    }
    let saved = fs::read_to_string(&saved_path)
        .map_err(|e| PlymouthError::Io(e.to_string()))?
        .trim()
        .to_string();
    if opts.dry_run {
        actions.push(format!("restore Plymouth theme {}", saved));
        actions.push(String::from("rebuild initramfs"));
        return Ok(actions);
    }
    if !opts.apply {
        return Ok(actions);
    }
    run_cmd(&opts.set_theme_bin, &[&saved], true)?;
    if opts.update_initramfs_bin.is_file() {
        let _ = run_cmd(&opts.update_initramfs_bin, &["-u"], true);
    }
    let _ = fs::remove_file(&saved_path);
    actions.push(format!("restored Plymouth theme {}", saved));
    Ok(actions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    fn temp_dir(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("blf-ply-{}-{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn make_stub(dir: &Path, record: &Path) -> PathBuf {
        let path = dir.join("plymouth-set-default-theme");
        let body = format!(
            "#!/bin/sh\nif [ \"$#\" -eq 0 ]; then echo 'ubuntu-logo'; exit 0; fi\necho \"$@\" >> '{}'\nexit 0\n",
            record.display()
        );
        fs::write(&path, body).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        path
    }

    fn make_initramfs(dir: &Path) -> PathBuf {
        let path = dir.join("update-initramfs");
        fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[test]
    fn saved_theme_restored() {
        let base = temp_dir("restore");
        let esp = base.join("esp");
        fs::create_dir_all(esp.join("EFI").join("bootlogo-forge")).unwrap();
        let record = base.join("record");
        let stub = make_stub(&base, &record);
        let init = make_initramfs(&base);

        let opts = PlymouthInstallOptions {
            esp: esp.clone(),
            set_theme_bin: stub.clone(),
            update_initramfs_bin: init,
            dry_run: false,
            apply: true,
            is_root: true,
        };
        plymouth_install(opts).unwrap();
        let saved = fs::read_to_string(
            esp.join("EFI")
                .join("bootlogo-forge")
                .join("plymouth-theme"),
        )
        .unwrap();
        assert_eq!(saved.trim(), "ubuntu-logo");

        // reset record
        let _ = fs::remove_file(&record);
        let unopts = PlymouthUninstallOptions {
            esp: esp.clone(),
            set_theme_bin: stub.clone(),
            update_initramfs_bin: base.join("update-initramfs"),
            dry_run: false,
            apply: true,
            is_root: true,
        };
        plymouth_uninstall(unopts).unwrap();
        let restored = fs::read_to_string(&record).unwrap();
        assert!(restored.contains("ubuntu-logo"));
    }

    #[test]
    fn missing_plymouth() {
        let base = temp_dir("missing");
        let esp = base.join("esp");
        fs::create_dir_all(&esp).unwrap();
        let opts = PlymouthInstallOptions {
            esp: esp.clone(),
            set_theme_bin: base.join("no-such"),
            update_initramfs_bin: base.join("nope"),
            dry_run: true,
            apply: false,
            is_root: false,
        };
        assert!(matches!(
            plymouth_install(opts),
            Err(PlymouthError::MissingPlymouth)
        ));
        // ESP untouched
        assert!(!esp.join("EFI").exists());
    }
}
