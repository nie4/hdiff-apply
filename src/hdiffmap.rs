use std::{
    fs::{read_to_string, remove_file},
    path::Path,
    process::Command,
};

use indicatif::{ProgressBar, ProgressStyle};
use rayon::iter::IntoParallelIterator;
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

use crate::{error::IOError, *};

pub struct HDiffMap<'a> {
    game_path: &'a Path,
    hpatchz_path: &'a Path,
    hdiffmap_path: &'a Path,
}

#[derive(Debug, Error)]
pub enum PatchError {
    #[error("hdiffmap.json structure changed!")]
    Json(),
    #[error("{0} doesn't exist, skipping")]
    NotFound(String),
    #[error("{0}")]
    Io(#[from] IOError),
}

#[derive(Deserialize)]
struct DiffMap {
    source_file_name: String,
    target_file_name: String,
    patch_file_name: String,
}

impl<'a> HDiffMap<'a> {
    pub fn new(game_path: &'a Path, hpatchz_path: &'a Path, hdiffmap_path: &'a Path) -> Self {
        Self {
            game_path,
            hpatchz_path,
            hdiffmap_path,
        }
    }

    fn load_diff_map(&self) -> Result<Vec<DiffMap>, PatchError> {
        let hdiffmap_path = self.hdiffmap_path;

        if !hdiffmap_path.exists() {
            return Err(PatchError::NotFound(format!("{}", hdiffmap_path.display())));
        }

        let data = read_to_string(&hdiffmap_path)
            .map_err(|e| IOError::read_to_string(hdiffmap_path, e))?;
        let deserialized: Value = serde_json::from_str(&data).unwrap();

        let diff_map = deserialized.get("diff_map").ok_or(PatchError::Json())?;

        Ok(serde_json::from_value(diff_map.clone()).unwrap())
    }

    pub fn patch(&mut self) -> Result<(), PatchError> {
        let game_path = self.game_path;
        let hpatchz_path = self.hpatchz_path;

        let diff_map = self.load_diff_map()?;

        let spinner_style = ProgressStyle::with_template(
            "{spinner:.green} [{elapsed}] [{bar:35.cyan/blue}] {pos}/{len}",
        )
        .unwrap()
        .progress_chars("#>-");

        let pb = ProgressBar::new(diff_map.len() as u64);
        pb.set_style(spinner_style);

        diff_map.into_par_iter().for_each(|entry| {
            let source_file_name = game_path.join(&entry.source_file_name);
            let patch_file_name = game_path.join(&entry.patch_file_name);
            let target_file_name = game_path.join(&entry.target_file_name);

            let output = Command::new(&hpatchz_path)
                .arg(&source_file_name)
                .arg(&patch_file_name)
                .arg(&target_file_name)
                .arg("-f")
                .output();

            match output {
                Ok(out) => {
                    if out.status.success() {
                        let _ = remove_file(patch_file_name);
                        if source_file_name != target_file_name {
                            let _ = remove_file(source_file_name);
                        }

                        pb.inc(1);
                    } else {
                        if !out.stderr.is_empty() {
                            utils::print_err(String::from_utf8_lossy(&out.stderr).trim());
                        }
                    }
                }
                Err(e) => {
                    utils::print_err(format!("Failed to execute patch command: {}", e));
                }
            }
        });
        pb.finish();

        Ok(())
    }
}
