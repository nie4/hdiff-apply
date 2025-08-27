use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use anyhow::Result;
use prost::Message;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use ruzstd::decoding::StreamingDecoder;

use crate::{
    types::DiffEntry,
    update::manifest_proto::SophonManifestProto,
    utils::{hpatchz::HPatchZ, pb_helper::create_progress_bar},
};

pub struct LDiff<'a> {
    game_path: &'a Path,
    pub ldiff_path: PathBuf,
    pub manifest_proto: SophonManifestProto,
}

impl<'a> LDiff<'a> {
    pub fn new(game_path: &'a Path, temp_folder_path: Option<&'a Path>) -> Result<Self> {
        let manifest_path = if let Some(temp_path) = temp_folder_path {
            temp_path.join("manifest")
        } else {
            game_path.join("manifest")
        };

        anyhow::ensure!(manifest_path.exists(), "manifest file not found");

        let ldiff_path = if let Some(temp_path) = temp_folder_path {
            temp_path.join("ldiff")
        } else {
            game_path.join("ldiff")
        };

        anyhow::ensure!(ldiff_path.exists(), "ldiff folder not found");

        let mut manifest_file = File::open(manifest_path)?;
        let mut decoder = StreamingDecoder::new(&mut manifest_file)?;

        let mut manifest_decompressed = Vec::new();
        decoder.read_to_end(&mut manifest_decompressed)?;

        let manifest_proto = SophonManifestProto::decode(manifest_decompressed.as_slice())?;

        Ok(Self {
            game_path,
            ldiff_path,
            manifest_proto,
        })
    }

    pub fn create_diff_entries(&self) -> Result<Vec<DiffEntry>> {
        let diff_entries: Result<Vec<DiffEntry>> = self
            .manifest_proto
            .assets
            .par_iter()
            .filter_map(|asset| asset.asset_data.as_ref().map(|data| (asset, data)))
            .flat_map(|(asset, data)| {
                data.assets
                    .par_iter()
                    .map(move |update_asset| (asset, update_asset))
            })
            .map(|(asset, update_asset)| -> Result<DiffEntry> {
                let chunk_file_path = self.ldiff_path.join(&update_asset.chunk_file_name);
                anyhow::ensure!(
                    chunk_file_path.exists(),
                    "update asset: `{}` not found in ldiff folder",
                    update_asset.chunk_file_name
                );

                let patch_file_name = if update_asset.original_file_path.is_empty() {
                    format!("{}.hdiff", asset.asset_name)
                } else {
                    format!("{}.hdiff", update_asset.original_file_path)
                };

                Ok(DiffEntry {
                    source_file_name: update_asset.original_file_path.clone(),
                    source_file_md5: update_asset.original_file_md5.clone(),
                    source_file_size: update_asset.original_file_size as u64,

                    target_file_name: asset.asset_name.clone(),
                    target_file_md5: asset.asset_hash_md5.clone(),
                    target_file_size: asset.asset_size as u64,

                    patch_file_name,
                    ..Default::default()
                })
            })
            .collect();

        diff_entries
    }

    pub fn create_hdiff_files(&self) -> Result<()> {
        let results: Result<Vec<_>> = self
            .manifest_proto
            .assets
            .par_iter()
            .filter_map(|asset| asset.asset_data.as_ref().map(|data| (asset, data)))
            .flat_map(|(asset, data)| {
                data.assets
                    .par_iter()
                    .map(move |patch_asset| (asset, patch_asset))
            })
            .map(|(asset, patch_asset)| -> Result<()> {
                let chunk_path = self.ldiff_path.join(&patch_asset.chunk_file_name);
                let mut chunk_file = File::open(&chunk_path)?;

                chunk_file.seek(SeekFrom::Start(
                    patch_asset.hdiff_file_in_chunk_offset as u64,
                ))?;
                let mut hdiff_bytes = vec![0u8; patch_asset.hdiff_file_size as usize];
                chunk_file.read_exact(&mut hdiff_bytes)?;

                let patch_file_name = if patch_asset.original_file_path.is_empty() {
                    format!("{}.hdiff", asset.asset_name)
                } else {
                    format!("{}.hdiff", patch_asset.original_file_path)
                };

                let output_path = self.game_path.join(patch_file_name);
                let mut output_file = File::create(&output_path)?;
                output_file.write_all(&hdiff_bytes)?;
                Ok(())
            })
            .collect();

        results?;
        Ok(())
    }

    pub fn patch(&mut self, diff_entries: Vec<DiffEntry>) -> Result<()> {
        let pb = create_progress_bar(diff_entries.len());

        diff_entries
            .into_par_iter()
            .try_for_each(|entry| -> Result<()> {
                let source_file = if entry.source_file_name.is_empty() {
                    PathBuf::new()
                } else {
                    self.game_path.join(&entry.source_file_name)
                };
                let patch_file = self.game_path.join(&entry.patch_file_name);
                let target_file = self.game_path.join(&entry.target_file_name);

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

    pub fn handle_delete_files(&self) -> Result<()> {
        let ldiff_asset_set: HashSet<PathBuf> = self
            .manifest_proto
            .assets
            .par_iter()
            .map(|asset| PathBuf::from(&asset.asset_name))
            .collect();

        let star_rail_data_path = self.game_path.join("StarRail_Data");
        let all_game_files = self.walk_dir_excluding(&star_rail_data_path, "Persistent")?;

        let files_to_delete: Vec<_> = all_game_files
            .into_par_iter()
            .filter(|file_path| {
                file_path
                    .strip_prefix(self.game_path)
                    .map(|relative_path| !ldiff_asset_set.contains(&relative_path.to_path_buf()))
                    .unwrap_or(true)
            })
            .collect();

        files_to_delete.into_iter().try_for_each(fs::remove_file)?;

        Ok(())
    }

    fn walk_dir_excluding(
        &self,
        dir: &Path,
        exclude_dir: &str,
    ) -> Result<Vec<PathBuf>, std::io::Error> {
        let mut files = Vec::new();
        let mut stack = vec![dir.to_path_buf()];

        while let Some(current_dir) = stack.pop() {
            for entry in fs::read_dir(current_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    if path.file_name().and_then(|n| n.to_str()) != Some(exclude_dir) {
                        stack.push(path);
                    }
                } else {
                    files.push(path);
                }
            }
        }
        Ok(files)
    }
}
