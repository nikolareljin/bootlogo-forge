#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::{Command, ExitCode};

use bootlogo_forge::install::{
    install, status, uninstall, InstallError, InstallOptions, StatusOptions, UninstallOptions,
};
use bootlogo_forge_core::detect::detect;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "bootlogo-forge",
    version,
    about = "Replace the boot logo via ACPI BGRT"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Detect {
        #[arg(long = "dmi-dir")]
        dmi_dir: Option<PathBuf>,
    },
    Status {
        #[arg(long)]
        esp: Option<PathBuf>,
        #[arg(long, default_value = "/usr/bin/efibootmgr")]
        efibootmgr: PathBuf,
        #[arg(long, default_value = "/sys")]
        sysfs_root: PathBuf,
    },
    Install {
        #[arg(long)]
        image: PathBuf,
        #[arg(long)]
        esp: Option<PathBuf>,
        #[arg(long, default_value = "/usr/bin/efibootmgr")]
        efibootmgr: PathBuf,
        #[arg(long)]
        efi_app: Option<PathBuf>,
        #[arg(long, default_value = "/usr/bin/plymouth-set-default-theme")]
        plymouth_set_default_theme: PathBuf,
        #[arg(long, default_value = "/usr/sbin/update-initramfs")]
        update_initramfs: PathBuf,
        #[arg(long, value_enum, default_value = "bgrt")]
        backend: BackendChoice,
        #[arg(long)]
        plymouth: bool,
        #[arg(long)]
        apply: bool,
    },
    Uninstall {
        #[arg(long)]
        esp: Option<PathBuf>,
        #[arg(long, default_value = "/usr/bin/efibootmgr")]
        efibootmgr: PathBuf,
        #[arg(long, default_value = "/usr/bin/plymouth-set-default-theme")]
        plymouth_set_default_theme: PathBuf,
        #[arg(long, default_value = "/usr/sbin/update-initramfs")]
        update_initramfs: PathBuf,
        #[arg(long)]
        apply: bool,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum BackendChoice {
    Bgrt,
    LenovoNative,
}

fn is_root() -> bool {
    Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false)
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Commands::Detect { dmi_dir } => {
            let dir = dmi_dir.unwrap_or_else(|| PathBuf::from("/sys/class/dmi/id"));
            let d = detect(&dir);
            println!("vendor: {}", d.vendor);
            println!("model: {}", d.model);
            println!("backend: {}", d.backend);
            ExitCode::SUCCESS
        }
        Commands::Status {
            esp,
            efibootmgr,
            sysfs_root,
        } => {
            let opts = StatusOptions {
                esp,
                efibootmgr,
                sysfs_root,
            };
            match status(opts) {
                Ok(text) => {
                    print!("{}", text);
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{}", e);
                    ExitCode::FAILURE
                }
            }
        }
        Commands::Install {
            image,
            esp,
            efibootmgr,
            efi_app,
            plymouth_set_default_theme,
            update_initramfs,
            backend,
            plymouth,
            apply,
        } => {
            if matches!(backend, BackendChoice::LenovoNative) {
                println!(
                    "backend: lenovo-native (dry-run; vendor native ESP logo is not automated)"
                );
                println!("would copy a logo BMP to the ESP root for a ThinkPad");
                return ExitCode::SUCCESS;
            }
            let esp_path = esp.unwrap_or_else(|| PathBuf::from("/boot/efi"));
            let opts = InstallOptions {
                image,
                esp: esp_path.clone(),
                efibootmgr,
                efi_app,
                dry_run: !apply,
                apply,
                is_root: is_root(),
            };
            match install(opts) {
                Ok(actions) => {
                    for a in actions {
                        println!("{}", a);
                    }
                    if plymouth {
                        let popts = bootlogo_forge::PlymouthInstallOptions {
                            esp: esp_path,
                            set_theme_bin: plymouth_set_default_theme,
                            update_initramfs_bin: update_initramfs,
                            dry_run: !apply,
                            apply,
                            is_root: is_root(),
                        };
                        match bootlogo_forge::plymouth_install(popts) {
                            Ok(pa) => {
                                for a in pa {
                                    println!("{}", a);
                                }
                            }
                            Err(e) => {
                                eprintln!("{}", e);
                                return ExitCode::FAILURE;
                            }
                        }
                    }
                    ExitCode::SUCCESS
                }
                Err(InstallError::NotRoot) => {
                    eprintln!("root privileges required for --apply");
                    ExitCode::FAILURE
                }
                Err(e) => {
                    eprintln!("{}", e);
                    ExitCode::FAILURE
                }
            }
        }
        Commands::Uninstall {
            esp,
            efibootmgr,
            plymouth_set_default_theme,
            update_initramfs,
            apply,
        } => {
            let esp_path = esp.unwrap_or_else(|| PathBuf::from("/boot/efi"));
            let opts = UninstallOptions {
                esp: esp_path.clone(),
                efibootmgr,
                dry_run: !apply,
                apply,
                is_root: is_root(),
            };
            match uninstall(opts) {
                Ok(actions) => {
                    for a in actions {
                        println!("{}", a);
                    }
                    if esp_path
                        .join("EFI")
                        .join("bootlogo-forge")
                        .join("plymouth-theme")
                        .exists()
                    {
                        let popts = bootlogo_forge::PlymouthUninstallOptions {
                            esp: esp_path,
                            set_theme_bin: plymouth_set_default_theme,
                            update_initramfs_bin: update_initramfs,
                            dry_run: !apply,
                            apply,
                            is_root: is_root(),
                        };
                        match bootlogo_forge::plymouth_uninstall(popts) {
                            Ok(pa) => {
                                for a in pa {
                                    println!("{}", a);
                                }
                            }
                            Err(e) => {
                                eprintln!("{}", e);
                                return ExitCode::FAILURE;
                            }
                        }
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{}", e);
                    ExitCode::FAILURE
                }
            }
        }
    }
}
