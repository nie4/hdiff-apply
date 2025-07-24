use std::fs;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use crate::{
    error::{DeleteFileError, IOError},
    utils,
};

// Handle deletefiles.txt
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

    pub fn execute(&self) -> Result<(), DeleteFileError> {
        if !self.deletefiles_path.exists() {
            return Err(DeleteFileError::NotFound(
                self.deletefiles_path.display().to_string(),
            ));
        }

        let file = File::open(&self.deletefiles_path)
            .map_err(|e| IOError::open(self.deletefiles_path, e))?;

        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line.map_err(|e| IOError::read_line(self.deletefiles_path, e))?;
            let line = line.trim();

            if line.is_empty() {
                continue;
            }

            let file_path = self.game_path.join(line);

            if let Err(e) = fs::remove_file(&file_path) {
                utils::print_err(format!("Failed to remove {}: {}", file_path.display(), e));
            }
        }

        Ok(())
    }
}
