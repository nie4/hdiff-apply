use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use common::types::DiffEntry;
use hpatchz::HPatchZ;
use indicatif::ProgressBar;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tempfile::TempDir;

use crate::patchers::{custom_hdiff::CustomHdiff, hdiff::Hdiff, ldiff::Ldiff};

mod custom_hdiff;
mod hdiff;
mod ldiff;

pub trait Patcher {
    fn patch(&self, game_path: &Path, progress: &ProgressBar) -> Result<()>;
    fn name(&self) -> &'static str;

    fn patch_files(
        &self,
        game_path: &Path,
        diff_entries: &[DiffEntry],
        progress: &ProgressBar,
    ) -> Result<()> {
        let staging_dir =
            TempDir::new_in(game_path).context("Failed to create staging directory")?;

        diff_entries
            .par_iter()
            .try_for_each(|entry| -> Result<()> {
                let source_file = if entry.source_file_name.is_empty() {
                    PathBuf::new()
                } else {
                    game_path.join(&entry.source_file_name)
                };

                let patch_file = game_path.join(&entry.patch_file_name);
                let target_file = staging_dir.path().join(&entry.target_file_name);

                if let Some(parent) = target_file.parent() {
                    fs::create_dir_all(parent)?;
                }

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

                progress.inc(1);

                Ok(())
            })?;

        progress.set_message("Merging files");
        progress.set_position(0);
        progress.set_length(diff_entries.len() as u64);

        diff_entries
            .par_iter()
            .try_for_each(|entry| -> Result<()> {
                let staged_file = staging_dir.path().join(&entry.target_file_name);
                let target_file = game_path.join(&entry.target_file_name);

                fs::rename(&staged_file, &target_file)
                    .or_else(|_| fs::copy(&staged_file, &target_file).map(|_| ()))
                    .with_context(|| format!("Failed to move: {}", entry.target_file_name))?;

                progress.inc(1);

                Ok(())
            })?;

        Ok(())
    }
}

pub struct PatchManager {
    game_path: PathBuf,
    patcher: Box<dyn Patcher>,
}

impl PatchManager {
    pub fn new(game_path: &Path) -> Self {
        let patcher = Self::create_patcher(&game_path);
        Self {
            game_path: game_path.to_path_buf(),
            patcher,
        }
    }

    pub fn create_patcher(game_path: &Path) -> Box<dyn Patcher> {
        if game_path.join("manifest").exists() {
            Box::new(Ldiff)
        } else if game_path.join("GameAssembly.dll.hdiff").exists() {
            Box::new(CustomHdiff)
        } else {
            Box::new(Hdiff)
        }
    }

    pub fn patch(&self, progress: &ProgressBar) -> Result<()> {
        self.patcher.patch(&self.game_path, progress)
    }

    pub fn patcher_name(&self) -> &'static str {
        self.patcher.name()
    }
}
