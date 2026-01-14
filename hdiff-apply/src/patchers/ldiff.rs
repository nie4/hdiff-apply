use std::collections::HashSet;
use std::fs;
use std::io::{Seek, SeekFrom};
use std::path::Path;
use std::{fs::File, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use common::manifest_proto::{Asset, AssetProperty, SophonManifestProto};
use common::types::DiffEntry;
use indicatif::ProgressBar;
use prost::Message;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use ruzstd::decoding::StreamingDecoder;
use ruzstd::io::Read;
use walkdir::WalkDir;

use crate::patchers::Patcher;

pub struct Ldiff;

impl Ldiff {
    fn load_manifest(game_path: &Path) -> Result<SophonManifestProto> {
        let mut manifest_file =
            File::open(game_path.join("manifest")).context("Ldiff manifest file not found!")?;

        let mut decoder = StreamingDecoder::new(&mut manifest_file)?;
        let mut manifest_decompressed = Vec::new();
        decoder.read_to_end(&mut manifest_decompressed)?;

        SophonManifestProto::decode(manifest_decompressed.as_slice())
            .context("Failed to decode manifest proto")
    }

    fn asset_pairs<'a>(
        manifest: &'a SophonManifestProto,
    ) -> impl Iterator<Item = (&'a AssetProperty, &'a Asset)> + 'a {
        manifest
            .assets
            .iter()
            .filter_map(|asset| asset.asset_data.as_ref().map(|data| (asset, data)))
            .flat_map(|(asset, data)| {
                data.assets
                    .iter()
                    .map(move |patch_asset| (asset, patch_asset))
            })
    }

    fn get_patch_file_name(asset_name: &str, original_path: &str) -> String {
        if original_path.is_empty() {
            format!("{}.hdiff", asset_name)
        } else {
            format!("{}.hdiff", original_path)
        }
    }

    fn create_diff_entries(manifest: &SophonManifestProto) -> Result<Vec<DiffEntry>> {
        Self::asset_pairs(manifest)
            .map(|(asset, update_asset)| {
                Ok(DiffEntry {
                    source_file_name: update_asset.original_file_path.clone(),
                    source_file_md5: update_asset.original_file_md5.clone(),
                    source_file_size: update_asset.original_file_size as u64,
                    target_file_name: asset.asset_name.clone(),
                    target_file_md5: asset.asset_hash_md5.clone(),
                    target_file_size: asset.asset_size as u64,
                    patch_file_name: Self::get_patch_file_name(
                        &asset.asset_name,
                        &update_asset.original_file_path,
                    ),
                    ..Default::default()
                })
            })
            .collect()
    }

    fn extract_hdiff_files(manifest: &SophonManifestProto, game_path: &Path) -> Result<()> {
        Self::asset_pairs(manifest)
            .collect::<Vec<_>>()
            .into_par_iter()
            .map(|(asset, patch_asset)| {
                let patch_file_name =
                    Self::get_patch_file_name(&asset.asset_name, &patch_asset.original_file_path);

                let chunk_path = game_path.join("ldiff").join(&patch_asset.chunk_file_name);
                let output_path = game_path.join(&patch_file_name);

                if let Some(parent) = output_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                let mut chunk_file = fs::File::open(&chunk_path).with_context(|| {
                    format!("Failed to open chunk: {}", patch_asset.chunk_file_name)
                })?;

                chunk_file.seek(SeekFrom::Start(
                    patch_asset.hdiff_file_in_chunk_offset as u64,
                ))?;

                let mut hdiff_bytes = vec![0u8; patch_asset.hdiff_file_size as usize];
                chunk_file.read_exact(&mut hdiff_bytes)?;

                fs::write(&output_path, hdiff_bytes)
                    .with_context(|| format!("Failed to write: {}", patch_file_name))?;

                Ok(())
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(())
    }

    fn cleanup_ldiff_files(game_path: &Path, diff_entries: &[DiffEntry]) {
        diff_entries.par_iter().for_each(|entry| {
            let _ = fs::remove_file(game_path.join(&entry.patch_file_name));
        });

        let _ = fs::remove_dir_all(game_path.join("ldiff"));
        let _ = fs::remove_file(game_path.join("manifest"));
    }

    fn cleanup_old_files(
        game_path: &Path,
        diff_entries: &[DiffEntry],
        manifest: &SophonManifestProto,
    ) -> Result<()> {
        diff_entries.par_iter().for_each(|entry| {
            if !entry.source_file_name.is_empty() {
                let source_file = game_path.join(&entry.source_file_name);
                let target_file = game_path.join(&entry.target_file_name);
                if source_file != target_file {
                    let _ = fs::remove_file(source_file);
                }
            }
        });

        let asset_set: HashSet<_> = manifest
            .assets
            .iter()
            .map(|asset| PathBuf::from(&asset.asset_name))
            .collect();

        let data_path = game_path.join("StarRail_Data");
        if !data_path.exists() {
            return Ok(());
        }
        let files_to_delete: Vec<_> = WalkDir::new(&data_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| !e.path().components().any(|c| c.as_os_str() == "Persistent"))
            .map(|e| e.path().to_path_buf())
            .collect::<Vec<_>>()
            .into_par_iter()
            .filter(|path| {
                path.strip_prefix(game_path)
                    .map(|rel| !asset_set.contains(rel))
                    .unwrap_or(false)
            })
            .collect();

        files_to_delete.into_iter().try_for_each(fs::remove_file)?;

        Ok(())
    }
}

impl Patcher for Ldiff {
    fn patch(&self, game_path: &Path, progress: Option<&ProgressBar>) -> Result<()> {
        let manifest = Self::load_manifest(game_path).context("Ldiff manifest not found")?;
        Self::extract_hdiff_files(&manifest, game_path).context("Failed to extract hdiff files")?;
        let diff_entries =
            Self::create_diff_entries(&manifest).context("Failed to create diff entries")?;

        if let Some(pb) = progress {
            pb.set_length(diff_entries.len() as u64);
            pb.set_message("Patching files");
        }

        match self.patch_files(game_path, &diff_entries, progress) {
            Ok(_) => {
                Self::cleanup_ldiff_files(game_path, &diff_entries);
                Self::cleanup_old_files(game_path, &diff_entries, &manifest)?;
            }
            Err(e) => {
                Self::cleanup_ldiff_files(game_path, &diff_entries);
                return Err(anyhow!("{e}"));
            }
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "LDiff"
    }
}
