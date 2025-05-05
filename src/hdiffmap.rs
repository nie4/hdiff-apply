use rayon::prelude::*;
use serde::Deserialize;
use serde_json::Value;
use std::{
    path::Path,
    process::Command,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex,
    },
};
use thiserror::Error;

pub struct HDiffMap<'a, 'b> {
    game_path: &'a Path,
    hpatchz_path: &'b Path,
    pub items: Arc<Mutex<u32>>,
}

#[derive(Debug, Error)]
pub enum PatchError {
    #[error("hdiffmap.json structure changed!")]
    JsonError(),
    #[error("{0} doesn't exist, skipping")]
    NotFound(String),
    #[error("IO error occurred: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Deserialize)]
struct DiffMap {
    source_file_name: String,
    target_file_name: String,
    patch_file_name: String,
}

impl<'a, 'b> HDiffMap<'a, 'b> {
    pub fn new(game_path: &'a Path, hpatchz_path: &'b Path) -> Self {
        Self {
            game_path,
            hpatchz_path,
            items: Arc::new(Mutex::new(0)),
        }
    }

    fn load_diff_map(&self) -> Result<Vec<DiffMap>, PatchError> {
        let path = self.game_path.join("hdiffmap.json");

        if !path.exists() {
            return Err(PatchError::NotFound(format!("{}", path.display())));
        }

        let data = std::fs::read_to_string(&path)?;
        let deserialized: Value = serde_json::from_str(&data).unwrap();

        let diff_map = deserialized
            .get("diff_map")
            .ok_or(PatchError::JsonError())?;

        Ok(serde_json::from_value(diff_map.clone()).unwrap())
    }

    fn remove_file<P: AsRef<Path>>(&self, path: P) {
        match std::fs::remove_file(&path) {
            Ok(_) => tracing::info!("Removed old hdiff file: {}", path.as_ref().display()),
            Err(e) => tracing::error!("Failed to remove {}: {}", path.as_ref().display(), e),
        }
    }

    pub fn patch(&mut self) -> Result<(), PatchError> {
        let path = self.game_path;
        let hpatchz_path = self.hpatchz_path;

        let diff_map = self.load_diff_map()?;
        let counter = AtomicU32::new(0);

        diff_map.into_par_iter().for_each(|entry| {
            let source_file_name = path.join(&entry.source_file_name);
            let patch_file_name = path.join(&entry.patch_file_name);
            let target_file_name = path.join(&entry.target_file_name);

            let output = Command::new(&hpatchz_path)
                .arg(&source_file_name)
                .arg(&patch_file_name)
                .arg(&target_file_name)
                .arg("-f")
                .output();

            match output {
                Ok(out) => {
                    if out.status.success() {
                        counter.fetch_add(1, Ordering::Relaxed);

                        if !out.stdout.is_empty() {
                            tracing::info!("{}", String::from_utf8_lossy(&out.stdout).trim());
                        }

                        self.remove_file(patch_file_name);
                        if source_file_name != target_file_name {
                            self.remove_file(source_file_name);
                        }
                    } else {
                        if !out.stderr.is_empty() {
                            tracing::error!("{}", String::from_utf8_lossy(&out.stderr).trim());
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to execute patch command: {}", e);
                }
            }
        });

        *self.items.lock().unwrap() = counter.load(Ordering::Relaxed);
        Ok(())
    }
}
