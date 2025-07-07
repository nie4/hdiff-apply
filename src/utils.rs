use std::{
    env::{current_dir, temp_dir},
    fs::{create_dir, read_dir, remove_dir_all, File},
    io::{stdin, stdout}
};

use ansi_term::Color;
use crossterm::{terminal::SetTitle, QueueableCommand};

use crate::*;

pub fn wait_for_input() {
    print!("Press enter to exit");
    stdout().flush().unwrap();

    stdin().read_line(&mut String::new()).unwrap();
}

pub fn get_hpatchz() -> Result<PathBuf, Error> {
    let temp_path = temp_dir().join(TEMP_DIR_NAME).join("hpatchz.exe");

    const HPATCHZ_BIN: &[u8] = include_bytes!("../bin/hpatchz.exe");

    let mut file = File::create(&temp_path)?;
    file.write_all(HPATCHZ_BIN)?;

    Ok(temp_path)
}

pub fn determine_game_path(game_path: &Option<String>) -> Result<PathBuf, Error> {
    match game_path {
        Some(path) => Ok(PathBuf::from(path)),
        None => {
            let cwd = current_dir()?;
            let sr_exe = cwd.join("StarRail.exe");

            if sr_exe.is_file() {
                Ok(cwd)
            } else {
                Err(Error::GameNotFound(cwd.display().to_string()))
            }
        }
    }
}

pub fn wait_for_confirmation(default_choice: bool) -> bool {
    stdout().flush().unwrap();

    let mut input = String::new();
    stdin().read_line(&mut input).unwrap();

    match input.trim().to_lowercase().as_str() {
        "y" | "yes" => return true,
        "n" | "no" => return false,
        _ => return default_choice,
    }
}

pub fn get_update_archives(game_path: &PathBuf) -> Result<Vec<PathBuf>, Error> {
    let mut paths = Vec::new();
    for entry in game_path.read_dir()? {
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

    if paths.is_empty() {
        return Err(Error::ArchiveNotFound());
    }

    Ok(paths)
}

pub fn get_and_create_temp_dir() -> Result<PathBuf, Error> {
    let path = temp_dir().join(TEMP_DIR_NAME);
    if !path.exists() {
        create_dir(&path)?;
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

pub fn print_err<E: std::fmt::Display + std::fmt::Debug>(err: E) {
    eprintln!("{} {}", Color::Red.paint("error:"), err)
}
