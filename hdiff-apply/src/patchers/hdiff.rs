use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use common::types::{DiffEntry, HDiffMap};
use hpatchz::HPatchZ;
use indicatif::ProgressBar;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::patchers::Patcher;

// Old hdiff format if something fails its probably gg
pub struct Hdiff;

impl Hdiff {
    fn load_diff_entries(game_path: &Path) -> Result<Vec<DiffEntry>> {
        let hdiffmap_path = game_path.join("hdiffmap.json");

        let data = fs::read_to_string(hdiffmap_path)?;
        let hdiff_map: HDiffMap = serde_json::from_str(&data)?;

        Ok(hdiff_map.diff_map)
    }

    fn patch_files(
        game_path: &Path,
        diff_entries: &[DiffEntry],
        progress: Option<&ProgressBar>,
    ) -> Result<()> {
        diff_entries
            .par_iter()
            .try_for_each(|entry| -> Result<()> {
                let source_file = if entry.source_file_name.is_empty() {
                    PathBuf::new()
                } else {
                    game_path.join(&entry.source_file_name)
                };

                let patch_file = game_path.join(&entry.patch_file_name);
                let target_file = game_path.join(&entry.target_file_name);

                HPatchZ::patch_file(&source_file, &patch_file, &target_file).with_context(
                    || {
                        format!(
                            "Failed to patch: {} + {} -> {}",
                            source_file.display(),
                            patch_file.display(),
                            entry.target_file_name
                        )
                    },
                )?;

                if let Some(pb) = progress {
                    pb.inc(1);
                }

                Ok(())
            })?;

        Ok(())
    }
}

impl Patcher for Hdiff {
    fn patch(&self, game_path: &Path, progress: Option<&ProgressBar>) -> Result<()> {
        let diff_entries = Self::load_diff_entries(&game_path)?;

        if let Some(pb) = progress {
            pb.set_length(diff_entries.len() as u64);
            pb.set_message("Patching files");
        }

        Self::patch_files(game_path, &diff_entries, progress)?;

        Ok(())
    }

    fn name(&self) -> &'static str {
        "HDiff"
    }
}
