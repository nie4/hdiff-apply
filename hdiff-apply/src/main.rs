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
struct Args {
    #[arg(short, long, default_value = ".")]
    game_path: PathBuf,
    #[arg(short, long)]
    archives_path: Option<PathBuf>,
    #[arg(short, long)]
    legacy_mode: bool,
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
