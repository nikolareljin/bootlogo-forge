#![forbid(unsafe_code)]

use std::path::PathBuf;

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
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Detect { dmi_dir } => {
            let dir = dmi_dir.unwrap_or_else(|| PathBuf::from("/sys/class/dmi/id"));
            let d = detect(&dir);
            println!("vendor: {}", d.vendor);
            println!("model: {}", d.model);
            println!("backend: {}", d.backend);
        }
    }
}
