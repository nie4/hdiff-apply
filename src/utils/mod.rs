use std::{
    env::{current_dir, temp_dir},
    fs::{create_dir, read_dir, remove_dir_all, File},
    io::{stdin, stdout, Write},
    path::{Path, PathBuf},
};

use binary_version::BinaryVersion;
use crossterm::{style::Stylize, terminal::SetTitle, QueueableCommand};

use crate::{error::IOError, AppError, TEMP_DIR_NAME};

pub mod binary_version;
pub mod pb_helper;
pub mod seven_zip;

pub fn wait_for_input() {
    print!("Press enter to exit");
    stdout().flush().unwrap();

    stdin().read_line(&mut String::new()).unwrap();
}

pub fn get_hpatchz() -> Result<PathBuf, AppError> {
    let temp_path = temp_dir().join(TEMP_DIR_NAME).join("hpatchz.exe");

    const HPATCHZ_BIN: &[u8] = include_bytes!("../../bin/hpatchz.exe");

    let mut file = File::create(&temp_path).map_err(|e| IOError::create_file(&temp_path, e))?;
    file.write_all(HPATCHZ_BIN)
        .map_err(|e| IOError::write_all(&temp_path, e))?;

    Ok(temp_path)
}

pub fn determine_game_path(game_path: Option<String>) -> Result<PathBuf, AppError> {
    let path = match game_path {
        Some(path) => PathBuf::from(path),
        None => current_dir().map_err(|e| IOError::current_dir(e))?,
    };

    if path.join("StarRail.exe").is_file() {
        Ok(path)
    } else {
        Err(AppError::GameNotFound(path.display().to_string()))
    }
}

pub fn confirm(message: &str, default_choice: bool) -> bool {
    if default_choice {
        print!("{message} (Y/n): ")
    } else {
        print!("{message} (y/N): ")
    }
    stdout().flush().unwrap();

    let mut input = String::new();
    stdin().read_line(&mut input).unwrap();

    match input.trim().to_lowercase().as_str() {
        "y" | "yes" => return true,
        "n" | "no" => return false,
        _ => return default_choice,
    }
}

pub fn get_update_archives<T: AsRef<Path>>(game_path: T) -> Result<Vec<PathBuf>, AppError> {
    let mut paths = Vec::new();
    for entry in game_path
        .as_ref()
        .read_dir()
        .map_err(|e| IOError::read_dir(game_path.as_ref(), e))?
    {
        let path = entry
            .map_err(|e| IOError::read_dir_entry(game_path.as_ref(), e))?
            .path();

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

pub fn get_and_create_temp_dir() -> Result<PathBuf, AppError> {
    let path = temp_dir().join(TEMP_DIR_NAME);
    if !path.exists() {
        create_dir(&path).map_err(|e| IOError::create_dir(&path, e))?;
    }
    Ok(path)
}

pub fn verify_version(first_version: &BinaryVersion, next_version: &BinaryVersion) -> bool {
    first_version.major_version == next_version.major_version
        && first_version.minor_version == next_version.minor_version
        && next_version.patch_version == first_version.patch_version + 1
}

pub fn set_console_title() {
    stdout()
        .queue(SetTitle(format!(
            "{} v{} | Made by nie",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        )))
        .unwrap();
}

pub fn clean_temp_hdiff_data() {
    let temp_path = temp_dir().join(TEMP_DIR_NAME);

    if let Ok(entries) = read_dir(temp_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    let _ = remove_dir_all(path);
                }
            }
        }
    }
}

pub fn print_err<T: std::fmt::Display + std::fmt::Debug>(msg: T) {
    eprintln!("{} {}", "error:".red(), msg)
}

pub fn print_info<T: std::fmt::Display + std::fmt::Debug>(msg: T) {
    eprintln!("{} {}", "info:".green(), msg)
}
