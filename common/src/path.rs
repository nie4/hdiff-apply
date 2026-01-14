use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result};

pub fn get_temp_dir() -> Result<PathBuf> {
    let path = env::temp_dir().join("hdiff-apply");
    fs::create_dir_all(&path).context("Failed to create temp directory")?;
    Ok(path)
}
