use std::{
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
};

use anyhow::{Context, Result, bail};
use common::types::{CustomDiffMap, DiffEntry, HDiffMap};
use indicatif::ProgressBar;

use crate::patchers::Patcher;

#[derive(Clone, Copy)]
enum HdiffFormat {
    Files,
    Map,
}

pub struct Hdiff;

impl Hdiff {
    fn detect_format(patch_path: &Path) -> Result<HdiffFormat> {
        if patch_path.join("hdifffiles.txt").exists() {
            Ok(HdiffFormat::Files)
        } else if patch_path.join("hdiffmap.json").exists() {
            Ok(HdiffFormat::Map)
        } else {
            bail!("No valid HDiff format found");
        }
    }

    fn load_diff_entries(patch_path: &Path, format: HdiffFormat) -> Result<Vec<DiffEntry>> {
        match format {
            HdiffFormat::Files => {
                let path = patch_path.join("hdifffiles.txt");
                let data = fs::read_to_string(&path)?;

                data.lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(|line| {
                        serde_json::from_str::<CustomDiffMap>(line.trim())
                            .with_context(|| format!("Failed to parse line: {}", line))
                            .map(|entry| DiffEntry {
                                source_file_name: entry.remote_name.clone(),
                                patch_file_name: format!("{}.hdiff", entry.remote_name),
                                target_file_name: entry.remote_name,
                                ..Default::default()
                            })
                    })
                    .collect()
            }

            HdiffFormat::Map => {
                let path = patch_path.join("hdiffmap.json");
                let data = fs::read_to_string(&path)?;
                let map: HDiffMap = serde_json::from_str(&data)?;
                Ok(map.diff_map)
            }
        }
    }

    fn cleanup(&self, patch_path: &Path, diff_entries: &[DiffEntry]) {
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
}

impl Patcher for Hdiff {
    fn start(&self, game_path: &Path, patch_path: &Path, progress: &ProgressBar) -> Result<()> {
        let format = Self::detect_format(patch_path)?;
        let diff_entries = Self::load_diff_entries(patch_path, format)?;

        progress.set_length(diff_entries.len() as u64);
        progress.set_message("Patching files");

        match self.patch_files(game_path, patch_path, &diff_entries, progress) {
            Ok(_) => {
                Self::apply_delete_list(game_path, patch_path)?;
                self.cleanup(patch_path, &diff_entries);
                Ok(())
            }
            Err(e) => {
                self.cleanup(patch_path, &diff_entries);
                Err(e)
            }
        }
    }

    fn name(&self) -> &'static str {
        "hdiff"
    }
}
