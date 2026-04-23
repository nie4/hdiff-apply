use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use common::types::DiffEntry;
use hpatchz::HPatchZ;
use indicatif::ProgressBar;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tempfile::TempDir;

use crate::patchers::{hdiff::Hdiff, ldiff::Ldiff};

mod hdiff;
mod ldiff;

pub trait Patcher {
    fn start(&self, game_path: &Path, patch_path: &Path, progress: &ProgressBar) -> Result<()>;
    fn name(&self) -> &'static str;

    fn patch_files(
        &self,
        game_path: &Path,
        patch_path: &Path,
        diff_entries: &[DiffEntry],
        progress: &ProgressBar,
    ) -> Result<()> {
        let staging_dir =
            TempDir::new_in(game_path).context("Failed to create staging directory")?;

        progress.set_message("Patching files");
        progress.set_length(diff_entries.len() as u64);
        progress.set_position(0);

        diff_entries
            .par_iter()
            .try_for_each(|entry| -> Result<()> {
                // Patch
                let source_file = if entry.source_file_name.is_empty() {
                    PathBuf::new()
                } else {
                    game_path.join(&entry.source_file_name)
                };

                let patch_file = patch_path.join(&entry.patch_file_name);
                if !patch_file.exists() {
                    return Err(anyhow::anyhow!(
                        "Missing patch file: {}",
                        patch_file.display()
                    ));
                }

                let staged = staging_dir.path().join(&entry.target_file_name);

                if let Some(parent) = staged.parent() {
                    fs::create_dir_all(parent)?;
                }

                HPatchZ::patch_file(&source_file, &patch_file, &staged).with_context(|| {
                    format!(
                        "Failed to patch: {} + {} -> {}",
                        source_file.display(),
                        patch_file.display(),
                        staged.display()
                    )
                })?;

                // Commit
                let target = game_path.join(&entry.target_file_name);

                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }

                fs::rename(&staged, &target)
                    .or_else(|_| fs::copy(&staged, &target).map(|_| ()))
                    .with_context(|| {
                        format!("Failed to move into place: {}", entry.target_file_name)
                    })?;

                progress.inc(1);
                Ok(())
            })?;

        Ok(())
    }
}

pub struct PatchManager {
    game_path: PathBuf,
    patch_path: PathBuf,
    patcher: Box<dyn Patcher>,
}

impl PatchManager {
    pub fn new(game_path: &Path, patch_path: &Path) -> Result<Self> {
        let patcher = Self::create_patcher(&patch_path)?;
        Ok(Self {
            game_path: game_path.to_path_buf(),
            patch_path: patch_path.to_path_buf(),
            patcher,
        })
    }

    pub fn create_patcher(patch_path: &Path) -> Result<Box<dyn Patcher>> {
        if let Some(manifest_path) = Self::find_manifest(patch_path) {
            Ok(Box::new(Ldiff::new(manifest_path)))
        } else if patch_path.join("hdifffiles.txt").exists()
            || patch_path.join("hdiffmap.json").exists()
        {
            Ok(Box::new(Hdiff))
        } else {
            bail!("Could not detect patch format in: {}", patch_path.display())
        }
    }

    fn find_manifest(patch_path: &Path) -> Option<PathBuf> {
        fs::read_dir(patch_path)
            .ok()?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .find(|path| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("manifest"))
                    .unwrap_or(false)
            })
    }

    pub fn patch(&self, progress: &ProgressBar) -> Result<()> {
        self.patcher
            .start(&self.game_path, &self.patch_path, progress)
    }

    pub fn patcher_name(&self) -> &'static str {
        self.patcher.name()
    }
}
