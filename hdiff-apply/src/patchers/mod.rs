use std::{
    fs::{self},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use indicatif::ProgressBar;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{
    app::HaTemp,
    patchers::{hdiff::Hdiff, ldiff::Ldiff},
    types::DiffEntry,
};

mod hdiff;
mod ldiff;

pub trait Patcher {
    fn name(&self) -> &'static str;
    fn start(&self, game_path: &Path, patch_path: &Path, progress: &ProgressBar) -> Result<()>;
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

pub trait DiffTarget {
    fn target_file_name(&self) -> &str;
}

impl DiffTarget for DiffEntry {
    fn target_file_name(&self) -> &str {
        &self.target_file_name
    }
}

fn commit_files<T: DiffTarget + Sync>(
    diff_entries: &[T],
    progress: &ProgressBar,
    game_path: &Path,
    staging_dir: HaTemp,
) -> Result<()> {
    progress.set_message("Merging files");
    progress.set_position(0);
    progress.set_length(diff_entries.len() as _);

    for entry in diff_entries.as_ref() {
        let staged_file = staging_dir.join(&entry.target_file_name());
        if !staged_file.exists() {
            bail!("Staged file missing: {}", entry.target_file_name());
        }
    }

    diff_entries.par_iter().try_for_each(|entry| -> Result<()> {
        let staged_file = staging_dir.join(&entry.target_file_name());
        let target_file = game_path.join(&entry.target_file_name());

        if let Some(parent) = target_file.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create target directory: {}", parent.display())
            })?;
        }

        fs::rename(&staged_file, &target_file)
            .or_else(|_| fs::copy(&staged_file, &target_file).map(|_| ()))
            .with_context(|| format!("Failed to move into place: {}", entry.target_file_name()))?;

        progress.inc(1);

        Ok(())
    })
}
