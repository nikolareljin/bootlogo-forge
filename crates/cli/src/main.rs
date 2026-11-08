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
        #[arg(long)]
        apply: bool,
    },
    Uninstall {
        #[arg(long)]
        esp: Option<PathBuf>,
        #[arg(long, default_value = "/usr/bin/efibootmgr")]
        efibootmgr: PathBuf,
        #[arg(long)]
        apply: bool,
    },
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
            apply,
        } => {
            let opts = InstallOptions {
                image,
                esp: esp.unwrap_or_else(|| PathBuf::from("/boot/efi")),
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
            apply,
        } => {
            let opts = UninstallOptions {
                esp: esp.unwrap_or_else(|| PathBuf::from("/boot/efi")),
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
