use std::{
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
};

use anyhow::{Context, Result};
use common::types::{CustomDiffMap, DiffEntry, HDiffMap};
use indicatif::ProgressBar;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::patchers::Patcher;

enum CustomHdiffType {
    /// Created by https://github.com/YYHEggEgg/HappyGenyuanImsactUpdate
    Normal,
    /// Custom hdiff that mimics the original hdiffmap.json structure
    Mimic,
}

pub struct CustomHdiff;

impl CustomHdiff {
    fn detect_hdiff_type(game_path: &Path) -> CustomHdiffType {
        if game_path.join("hdiffmap.json").exists() {
            CustomHdiffType::Mimic
        } else {
            CustomHdiffType::Normal
        }
    }

    fn load_diff_entries(game_path: &Path, hdiff_type: CustomHdiffType) -> Result<Vec<DiffEntry>> {
        match hdiff_type {
            CustomHdiffType::Normal => Self::load_normal_format(game_path),
            CustomHdiffType::Mimic => Self::load_mimic_format(game_path),
        }
    }

    fn load_normal_format(game_path: &Path) -> Result<Vec<DiffEntry>> {
        let hdiffmap_path = game_path.join("hdifffiles.txt");
        let data = fs::read_to_string(&hdiffmap_path)
            .with_context(|| format!("Failed to read {}", hdiffmap_path.display()))?;

        data.lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                serde_json::from_str::<CustomDiffMap>(line.trim())
                    .with_context(|| format!("Failed to parse line: {}", line))
                    .map(|entry| DiffEntry {
                        source_file_name: entry.remote_name.to_string(),
                        patch_file_name: format!("{}.hdiff", entry.remote_name),
                        target_file_name: entry.remote_name,
                        ..Default::default()
                    })
            })
            .collect()
    }

    fn load_mimic_format(game_path: &Path) -> Result<Vec<DiffEntry>> {
        let hdiffmap_path = game_path.join("hdiffmap.json");
        let data = fs::read_to_string(&hdiffmap_path)
            .with_context(|| format!("Failed to read {}", hdiffmap_path.display()))?;

        let hdiff_map: HDiffMap =
            serde_json::from_str(&data).context("Failed to parse hdiffmap.json")?;

        Ok(hdiff_map.diff_map)
    }

    fn cleanup_hdiff_files(game_path: &Path, diff_entries: &[DiffEntry]) {
        diff_entries.par_iter().for_each(|entry| {
            let _ = fs::remove_file(game_path.join(&entry.patch_file_name));
        });

        let _ = fs::remove_file(game_path.join("deletefiles.txt"));
        let _ = fs::remove_file(game_path.join("hdiffmap.json"));
        let _ = fs::remove_file(game_path.join("hdifffiles.txt"));
    }

    fn cleanup_old_files(game_path: &Path, diff_entries: &[DiffEntry]) -> Result<()> {
        diff_entries.par_iter().for_each(|entry| {
            if !entry.source_file_name.is_empty() {
                let source_file = game_path.join(&entry.source_file_name);
                let target_file = game_path.join(&entry.target_file_name);
                if source_file != target_file {
                    let _ = fs::remove_file(source_file);
                }
            }
        });

        let deletefiles_path = game_path.join("deletefiles.txt");
        let file = File::open(deletefiles_path)?;

        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
            let line = line.trim();

            if line.is_empty() {
                continue;
            }

            let file_path = game_path.join(line);
            let _ = fs::remove_file(&file_path);
        }

        Ok(())
    }
}

impl Patcher for CustomHdiff {
    fn start(&self, game_path: &Path, progress: &ProgressBar) -> Result<()> {
        let custom_hdiff_type = Self::detect_hdiff_type(&game_path);
        let diff_entries = Self::load_diff_entries(&game_path, custom_hdiff_type)?;

        progress.set_length(diff_entries.len() as u64);
        progress.set_message("Patching files");

        match self.patch_files(game_path, &diff_entries, progress) {
            Ok(_) => {
                Self::cleanup_old_files(game_path, &diff_entries)?;
                Self::cleanup_hdiff_files(game_path, &diff_entries);
            }
            Err(e) => {
                Self::cleanup_hdiff_files(game_path, &diff_entries);
                return Err(e.context("Patch failed - game files remain unchanged"));
            }
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "CustomHDiff"
    }
}
