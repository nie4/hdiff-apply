use std::fs;
use std::{path::Path, process::Command};

use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::Deserialize;

use crate::{
    error::{IOError, PatchError},
    utils::{self, pb_helper::create_progress_bar},
};

#[derive(Deserialize)]
struct DiffEntry {
    source_file_name: String,
    target_file_name: String,
    patch_file_name: String,
}

#[derive(Deserialize)]
struct HDiffMap {
    diff_map: Vec<DiffEntry>,
}

// Handle hdiffmap.json
pub struct HDiff<'a> {
    game_path: &'a Path,
    hpatchz_path: &'a Path,
    hdiffmap_path: &'a Path,
}

impl<'a> HDiff<'a> {
    pub fn new(game_path: &'a Path, hpatchz_path: &'a Path, hdiffmap_path: &'a Path) -> Self {
        Self {
            game_path,
            hpatchz_path,
            hdiffmap_path,
        }
    }

    fn load_diff_map(&self) -> Result<Vec<DiffEntry>, PatchError> {
        if !self.hdiffmap_path.exists() {
            return Err(PatchError::NotFound(
                self.hdiffmap_path.display().to_string(),
            ));
        }

        let data = fs::read_to_string(self.hdiffmap_path)
            .map_err(|e| IOError::read_to_string(self.hdiffmap_path, e))?;
        let hdiff_map: HDiffMap = serde_json::from_str(&data).map_err(|_| PatchError::Json())?;

        Ok(hdiff_map.diff_map)
    }

    pub fn execute(&mut self) -> Result<(), PatchError> {
        let diff_map = self.load_diff_map()?;

        let pb = create_progress_bar(diff_map.len());

        diff_map.into_par_iter().for_each(|entry| {
            let source_file = self.game_path.join(&entry.source_file_name);
            let patch_file = self.game_path.join(&entry.patch_file_name);
            let target_file = self.game_path.join(&entry.target_file_name);

            let output = Command::new(self.hpatchz_path)
                .args([
                    source_file.as_os_str(),
                    patch_file.as_os_str(),
                    target_file.as_os_str(),
                    "-f".as_ref(),
                ])
                .output();

            match output {
                Ok(out) => {
                    if out.status.success() {
                        let _ = fs::remove_file(patch_file);

                        if source_file != target_file {
                            let _ = fs::remove_file(source_file);
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
