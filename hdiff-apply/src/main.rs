#![feature(try_blocks)]

use std::{
    env,
    io::{self, Write},
    path::PathBuf,
};

use anyhow::{Result, anyhow};
use clap::Parser;
use hpatchz::HPatchZ;
use seven_zip::SevenZip;

mod byte_convert;
mod cli;
mod patchers;
mod update_package;

#[derive(Parser, Debug)]
#[command(
    name = "hdiff-apply",
    version,
    about = "Patching utility for SR",
    after_help = "EXAMPLES:\n  \
    # Apply patches from current directory\n  \
    hdiff-apply\n\n  \
    # Specify game installation path\n  \
    hdiff-apply -g \"C:\\Games\\GameName\"\n\n  \
    # Patch archives in different directory\n  \
    hdiff-apply -g \"C:\\Games\\GameName\" -a \"D:\\Downloads\\patches\"\n\n"
)]
struct Args {
    #[arg(
        short,
        long,
        default_value = ".",
        value_name = "PATH",
        help = "Game installation directory"
    )]
    game_path: PathBuf,
    #[arg(
        short,
        long,
        value_name = "PATH",
        help = "Directory containing patch archives (defaults to game_path)"
    )]
    archives_path: Option<PathBuf>,
    #[arg(short, long)]
    legacy: bool,
}

fn main() {
    crossterm::ansi_support::supports_ansi();
    let args = Args::parse();
    let should_pause = env::args().len() == 1;

    let result: Result<()> = try {
        // Throw any error early if they occur
        SevenZip::instance().map_err(|e| anyhow!(e))?;
        HPatchZ::instance().map_err(|e| anyhow!(e))?;

        cli::run(&args)?;
    };

    if let Err(e) = result {
        eprintln!("\x1b[41merror:\x1b[m {:?}", e);
    }

    if should_pause {
        print!("Press enter to exit");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut String::new()).unwrap();
    }
}
