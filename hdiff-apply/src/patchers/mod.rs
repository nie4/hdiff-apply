use std::{
    fs::{self, File},
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
    fn start(&self, game_path: &Path, patch_path: &Path, progress: &ProgressBar) -> Result<()>;
    fn name(&self) -> &'static str;

    fn patch_files(
        &self,
        game_path: &Path,
        patch_path: &Path,
        diff_entries: &[DiffEntry],
        progress: &ProgressBar,
    ) -> Result<()> {
        let staging_dir = HaTemp::new(game_path.join(".ha-staging"))?;

        progress.set_message("Patching files");
        progress.set_length(diff_entries.len() as _);
        progress.set_position(0);

        // A hack for ldiffs since i wanna be sure "normal" diffs patch correctly before creating dummy files in the game folder
        let (empty_source, normal): (Vec<&DiffEntry>, Vec<&DiffEntry>) = diff_entries
            .iter()
            .partition(|entry| entry.source_file_name.is_empty());

        normal.par_iter().try_for_each(|entry| -> Result<()> {
            let source_file = game_path.join(&entry.source_file_name);
            if !source_file.exists() {
                anyhow::bail!("Missing source file: {}", source_file.display());
            }

            let patch_file = patch_path.join(&entry.patch_file_name);
            if !patch_file.exists() {
                anyhow::bail!("Missing patch file: {}", patch_file.display());
            }

            let staged = staging_dir.join(&entry.target_file_name);
            if let Some(parent) = staged.parent() {
                fs::create_dir_all(parent)?;
            }

            hdiffpatch_rs::patch_hdiff(&source_file, &patch_file, &staged).map_err(|e| {
                anyhow::anyhow!(e.to_string())
                    .context(format!("Failed to patch '{}'", entry.target_file_name))
            })?;

            progress.inc(1);
            Ok(())
        })?;

        empty_source
            .par_iter()
            .try_for_each(|entry| -> Result<()> {
                let source_file = game_path.join(&entry.target_file_name);
                let parent = source_file.parent().ok_or_else(|| {
                    anyhow::anyhow!(
                        "entry.target_file_name has no parent: {}",
                        source_file.display()
                    )
                })?;
                fs::create_dir_all(parent)?;
                File::create(&source_file).with_context(|| {
                    format!(
                        "Failed to create dummy source file: {}",
                        source_file.display()
                    )
                })?;

                let patch_file = patch_path.join(&entry.patch_file_name);
                if !patch_file.exists() {
                    anyhow::bail!("Missing patch file: {}", patch_file.display());
                }

                let staged = staging_dir.join(&entry.target_file_name);
                if let Some(parent) = staged.parent() {
                    fs::create_dir_all(parent)?;
                }

                hdiffpatch_rs::patch_hdiff(&source_file, &patch_file, &staged).map_err(|e| {
                    anyhow::anyhow!(e.to_string())
                        .context(format!("Failed to patch '{}'", entry.target_file_name))
                })?;

                progress.inc(1);
                Ok(())
            })?;

        // Commit patched files into the game
        progress.set_message("Merging files");
        progress.set_position(0);
        progress.set_length(diff_entries.len() as _);

        // To be 100% sure everything went smoothly
        for entry in diff_entries.as_ref() {
            let staged_file = staging_dir.join(&entry.target_file_name);
            if !staged_file.exists() {
                bail!("Staged file missing: {}", entry.target_file_name);
            }
        }

        diff_entries
            .par_iter()
            .try_for_each(|entry| -> Result<()> {
                let staged_file = staging_dir.join(&entry.target_file_name);
                let target_file = game_path.join(&entry.target_file_name);

                if let Some(parent) = target_file.parent() {
                    fs::create_dir_all(parent).with_context(|| {
                        format!("Failed to create target directory: {}", parent.display())
                    })?;
                }

                fs::rename(&staged_file, &target_file)
                    .or_else(|_| fs::copy(&staged_file, &target_file).map(|_| ()))
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
