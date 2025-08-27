use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{
    types::CustomDiffMap,
    utils::{self, pb_helper::create_progress_bar},
};
use crate::{
    types::{DiffEntry, HDiffMap},
    utils::hpatchz::HPatchZ,
};

pub struct HDiff<'a> {
    game_path: &'a Path,
    hdiffmap_path: &'a Path,
}

impl<'a> HDiff<'a> {
    pub fn new(game_path: &'a Path, hdiffmap_path: &'a Path) -> Self {
        Self {
            game_path,
            hdiffmap_path,
        }
    }

    pub fn load_diff_entries(&self) -> Result<Vec<DiffEntry>> {
        if !self.hdiffmap_path.exists() {
            anyhow::bail!("{} doesn't exist", self.hdiffmap_path.display());
        }

        let data = fs::read_to_string(self.hdiffmap_path).with_context(|| {
            format!(
                "Failed to read '{}' to string",
                self.hdiffmap_path.display()
            )
        })?;

        let hdiff_map: HDiffMap =
            serde_json::from_str(&data).context("hdiffmap.json structure changed!")?;

        Ok(hdiff_map.diff_map)
    }

    pub fn load_custom_map(&self) -> Result<Vec<CustomDiffMap>> {
        if !self.hdiffmap_path.exists() {
            anyhow::bail!("{} doesn't exist", self.hdiffmap_path.display());
        }

        let data = fs::read_to_string(self.hdiffmap_path).with_context(|| {
            format!(
                "Failed to read '{}' to string",
                self.hdiffmap_path.display()
            )
        })?;

        let custom_map: Result<Vec<CustomDiffMap>, _> = data
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str::<CustomDiffMap>(line.trim()))
            .collect();

        custom_map.with_context(|| {
            format!(
                "Failed to parse JSON from '{}'",
                self.hdiffmap_path.display()
            )
        })
    }

    pub fn patch(&mut self, diff_entries: &'a Vec<DiffEntry>) -> Result<()> {
        let pb = create_progress_bar(diff_entries.len());

        diff_entries
            .into_par_iter()
            .try_for_each(|entry| -> Result<()> {
                let mut source_file = self.game_path.join(&entry.source_file_name);
                let patch_file = self.game_path.join(&entry.patch_file_name);
                let target_file = self.game_path.join(&entry.target_file_name);

                if entry.source_file_name.is_empty() {
                    source_file = PathBuf::new();
                }

                let result = HPatchZ::patch_file(&source_file, &patch_file, &target_file)?;
                if !result {
                    pb.suspend(|| {
                        println!("Failed to patch: {}", source_file.display());
                    });
                }
                pb.inc(1);

                Ok(())
            })?;

        pb.finish();

        Ok(())
    }

    pub fn patch_custom(&self, custom_entries: Vec<CustomDiffMap>) -> Result<()> {
        let pb = create_progress_bar(custom_entries.len());

        custom_entries
            .into_par_iter()
            .try_for_each(|entry| -> Result<()> {
                let source_file = self.game_path.join(&entry.remote_name);
                let patch_file = self.game_path.join(format!("{}.hdiff", &entry.remote_name));
                let target_file = self.game_path.join(&entry.remote_name);

                let result = HPatchZ::patch_file(&source_file, &patch_file, &target_file)?;
                if !result {
                    pb.suspend(|| {
                        println!("Failed to patch: {}", source_file.display());
                    });
                }
                pb.inc(1);

                Ok(())
            })?;

        pb.finish();

        Ok(())
    }
}
