use std::{
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
};

use anyhow::{Context, Result};

use crate::utils;

pub struct DeleteFiles<'a> {
    game_path: &'a Path,
    deletefiles_path: &'a Path,
}

impl<'a> DeleteFiles<'a> {
    pub fn new(game_path: &'a Path, deletefiles_path: &'a Path) -> Self {
        Self {
            game_path,
            deletefiles_path,
        }
    }

    pub fn remove(&self) -> Result<bool> {
        if !self.deletefiles_path.exists() {
            return Ok(false);
        }

        let file = File::open(&self.deletefiles_path)
            .with_context(|| format!("Failed to open '{}'", self.deletefiles_path.display()))?;

        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();

            if line.is_empty() {
                continue;
            }

            let file_path = self.game_path.join(line);

            if let Err(e) = fs::remove_file(&file_path) {
                utils::print_err(format!("Failed to remove {}: {}", file_path.display(), e));
            }
        }

        Ok(true)
    }
}
