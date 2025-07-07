use std::{
    fs::{remove_file, File},
    io::{BufRead, BufReader},
    path::Path,
};

use thiserror::Error;

use crate::*;

#[derive(Debug, Error)]
pub enum DeleteFileError {
    #[error("{0} doesn't exist, skipping")]
    NotFound(String),
    #[error("{0}")]
    Io(#[from] io::Error),
}

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

    pub fn remove(&mut self) -> Result<(), DeleteFileError> {
        let deletefiles_path = self.deletefiles_path;

        if !deletefiles_path.exists() {
            return Err(DeleteFileError::NotFound(format!(
                "{}",
                deletefiles_path.display()
            )));
        }

        let file = File::open(&deletefiles_path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;

            let path = Path::new(&line);
            let full_path = &self.game_path.join(path);

            if let Err(e) = remove_file(&full_path) {
                utils::print_err(format!(
                    "Failed to remove {}: {}",
                    full_path.display().to_string(),
                    e
                ));
            }
        }

        Ok(())
    }
}
