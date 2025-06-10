use std::{
    fs::{remove_file, File},
    io::{BufRead, BufReader},
    path::Path,
};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeleteFileError {
    #[error("{0} doesn't exist, skipping")]
    NotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct DeleteFiles<'a> {
    game_path: &'a Path,
    deletefiles_path: &'a Path,
    count: u32,
}

impl<'a> DeleteFiles<'a> {
    pub fn new(game_path: &'a Path, deletefiles_path: &'a Path) -> Self {
        Self {
            game_path,
            deletefiles_path,
            count: 0,
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

            match remove_file(&full_path) {
                Ok(_) => {
                    tracing::info!("Deleted {}", full_path.display());
                    self.count += 1;
                }
                Err(e) => tracing::error!("Failed to delete {}: {}", full_path.display(), e),
            }
        }

        Ok(())
    }

    pub fn count(&self) -> u32 {
        self.count
    }
}
