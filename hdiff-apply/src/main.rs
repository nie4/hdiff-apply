#![feature(try_blocks)]

use std::{
    env,
    io::{self, Write},
    path::PathBuf,
    process,
};

use anyhow::{Context, Result, anyhow};
use app::{RED, RESET};
use seven_zip::SevenZip;

mod app;
mod byte_convert;
mod patchers;
mod sophon_proto;
mod types;
mod update_package;

const USAGE: &'static str = r"Usage:
    hdiff-apply.exe [options]

Options:
    -g, --game-path <DIR>       Game installation directory (default: current working directory)
    -a, --archives-path <DIR>   Directory containing patch archives (default: --game-path)
    -h, --help                  Show this help message
";

#[derive(Debug)]
struct Args {
    game_path: Option<PathBuf>,
    archives_path: Option<PathBuf>,
}

impl Args {
    fn parse() -> Self {
        let mut game_path = Option::default();
        let mut archives_path = Option::default();

        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-g" | "--game-path" => {
                    game_path = Some(PathBuf::from(
                        args.next().expect("Missing value for --game-path"),
                    ));
                }
                "-a" | "--archives-path" => {
                    archives_path = Some(PathBuf::from(
                        args.next().expect("Missing value for --archives-path"),
                    ));
                }
                "-h" | "--help" => {
                    println!("{}", USAGE);
                    process::exit(0);
                }
                _ => {}
            }
        }

        Self {
            game_path,
            archives_path,
        }
    }
}

fn main() {
    app::print_banner();

    #[cfg(target_os = "windows")]
    crossterm::ansi_support::supports_ansi();

    let args = Args::parse();
    let should_pause = env::args().len() == 1;

    let result: Result<()> = try {
        // Throw any error early if they occur
        SevenZip::instance().map_err(|e| anyhow!(e))?;

        // If args.game_path is None, default to env::current_dir()
        let game_path = args
            .game_path
            .unwrap_or(env::current_dir().context("Failed to get the current directory")?);

        // If args.archives_path is None, default to game_path
        let archives_path = args.archives_path.as_deref().unwrap_or(game_path.as_path());

        app::run(&game_path, archives_path)?;
    };

    if let Err(e) = result {
        eprintln!("{RED}error{RESET}: {:?}", e);
    }

    if should_pause {
        print!("Press enter to exit");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut String::new()).unwrap();
    }
}
