use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use anyhow::Result;
use binary_version::BinaryVersion;
use crossterm::style::Stylize;

use crate::TEMP_DIR_NAME;

pub mod binary_version;
pub mod hpatchz;
pub mod pb_helper;
pub mod seven_zip;

pub fn wait_for_input() {
    print!("Press enter to exit");
    io::stdout().flush().unwrap();

    io::stdin().read_line(&mut String::new()).unwrap();
}

pub fn determine_game_path(game_path: Option<String>) -> Result<PathBuf> {
    let path = match game_path {
        Some(path) => PathBuf::from(path),
        None => env::current_dir()?,
    };

    if path.join("StarRail.exe").is_file() {
        Ok(path)
    } else {
        anyhow::bail!("StarRail.exe not found in: {}\n\tTip: Pass the game path as the first argument if it's not in the current directory or move this .exe", path.display());
    }
}

pub fn confirm(message: &str, default_choice: bool) -> bool {
    if default_choice {
        print!("{message} (Y/n): ")
    } else {
        print!("{message} (y/N): ")
    }
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    match input.trim().to_lowercase().as_str() {
        "y" | "yes" => return true,
        "n" | "no" => return false,
        _ => return default_choice,
    }
}

pub fn get_update_archives<T: AsRef<Path>>(game_path: T) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in game_path.as_ref().read_dir()? {
        let path = entry?.path();

        if let Some(ext) = path.extension() {
            if ext.eq_ignore_ascii_case("7z")
                || ext.eq_ignore_ascii_case("zip")
                || ext.eq_ignore_ascii_case("rar")
                || ext.eq_ignore_ascii_case("tar")
            {
                paths.push(path);
            }
        }
    }

    Ok(paths)
}

pub fn get_or_create_temp_dir() -> Result<PathBuf> {
    let path = env::temp_dir().join(TEMP_DIR_NAME);
    if !path.exists() {
        fs::create_dir(&path)?;
    }
    Ok(path)
}

pub fn verify_version(first_version: &BinaryVersion, next_version: &BinaryVersion) -> bool {
    first_version.major_version == next_version.major_version
        && first_version.minor_version == next_version.minor_version
        && next_version.patch_version == first_version.patch_version + 1
}

pub fn clean_temp_hdiff_data() {
    let temp_path = env::temp_dir().join(TEMP_DIR_NAME);

    if let Ok(entries) = fs::read_dir(temp_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    let _ = fs::remove_dir_all(path);
                }
            }
        }
    }
}

pub fn print_err<T: std::fmt::Display + std::fmt::Debug>(msg: T) {
    eprintln!("{} {:?}", "error:".red(), msg)
}

pub fn print_info<T: std::fmt::Display + std::fmt::Debug>(msg: T) {
    eprintln!("{} {}", "info:".green(), msg)
}
