use std::{
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
};

use anyhow::{Context, Result, bail};
use indicatif::ProgressBar;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{
    app::HaTemp,
    patchers::{self, Patcher},
    types::{DiffEntry, HDiffFiles, HDiffMap},
};

pub struct Hdiff;

impl Hdiff {
    fn parse_hdiff_map(patch_path: &Path) -> Result<HDiffMap> {
        if patch_path.join("hdifffiles.txt").exists() {
            Self::load_files_format(patch_path)
        } else if patch_path.join("hdiffmap.json").exists() {
            Self::load_map_format(patch_path)
        } else {
            bail!("No valid HDiff format found");
        }
    }

    fn load_files_format(patch_path: &Path) -> Result<HDiffMap> {
        let path = patch_path.join("hdifffiles.txt");
        let data = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        let diff_map = data
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|line| {
                serde_json::from_str::<HDiffFiles>(line.trim()).map(|entry| DiffEntry {
                    source_file_name: entry.remote_name.clone(),
                    patch_file_name: format!("{}.hdiff", entry.remote_name),
                    target_file_name: entry.remote_name,
                    ..Default::default()
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(HDiffMap { diff_map })
    }

    fn load_map_format(patch_path: &Path) -> Result<HDiffMap> {
        let path = patch_path.join("hdiffmap.json");
        let data = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let map: HDiffMap = serde_json::from_str(&data).context("Failed to parse hdiffmap.json")?;
        Ok(map)
    }

    fn cleanup(patch_path: &Path, diff_entries: &[DiffEntry]) {
        for entry in diff_entries {
            let _ = fs::remove_file(patch_path.join(&entry.patch_file_name));
        }
    }

    fn apply_delete_list(game_path: &Path, patch_path: &Path) -> Result<()> {
        let path = patch_path.join("deletefiles.txt");

        if !path.exists() {
            return Ok(());
        }

        let file = File::open(&path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            let _ = fs::remove_file(game_path.join(trimmed));
        }

        Ok(())
    }

    fn patch_files(
        game_path: &Path,
        patch_path: &Path,
        diff_entries: &[DiffEntry],
        progress: &ProgressBar,
    ) -> Result<()> {
        let staging_dir = HaTemp::new(game_path.join(".ha-staging"))?;

        progress.set_message("Patching files");
        progress.set_length(diff_entries.len() as _);
        progress.set_position(0);

        diff_entries
            .par_iter()
            .try_for_each(|entry| -> Result<()> {
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

        patchers::commit_files(diff_entries, progress, game_path, staging_dir)
    }
}

impl Patcher for Hdiff {
    fn name(&self) -> &'static str {
        "hdiff"
    }

    fn start(&self, game_path: &Path, patch_path: &Path, progress: &ProgressBar) -> Result<()> {
        let diff_entries = Self::parse_hdiff_map(patch_path)?.diff_map;

        Self::patch_files(game_path, patch_path, &diff_entries, progress)?;
        Self::apply_delete_list(game_path, patch_path)?;
        Self::cleanup(patch_path, &diff_entries);

        Ok(())
    }
}
