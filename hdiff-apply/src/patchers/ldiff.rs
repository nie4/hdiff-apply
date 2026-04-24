use std::collections::HashSet;
use std::fs;
use std::io::{Seek, SeekFrom};
use std::path::Path;
use std::{fs::File, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use common::sophon_proto::{SophonPatchAssetChunk, SophonPatchAssetProperty, SophonPatchProto};
use common::types::DiffEntry;
use indicatif::ProgressBar;
use prost::Message;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use ruzstd::decoding::StreamingDecoder;
use ruzstd::io::Read;
use walkdir::WalkDir;

use crate::patchers::Patcher;

pub struct Ldiff {
    manifest_path: PathBuf,
}

impl Ldiff {
    pub fn new(manifest_path: PathBuf) -> Self {
        Self { manifest_path }
    }

    fn load_manifest(manifest_path: &Path) -> Result<SophonPatchProto> {
        let mut manifest_file =
            File::open(manifest_path).context("Failed to open ldiff manifest file")?;
        let mut decoder = StreamingDecoder::new(&mut manifest_file)?;
        let mut manifest_decompressed = Vec::new();
        decoder.read_to_end(&mut manifest_decompressed)?;

        SophonPatchProto::decode(manifest_decompressed.as_slice())
            .context("Failed to decode ldiff manifest proto")
    }

    fn asset_pairs<'a>(
        manifest: &'a SophonPatchProto,
    ) -> impl Iterator<Item = (&'a SophonPatchAssetProperty, &'a SophonPatchAssetChunk)> + 'a {
        manifest.patch_assets.iter().flat_map(|asset_prop| {
            asset_prop
                .asset_infos
                .iter()
                .filter_map(move |info| info.chunk.as_ref().map(|chunk| (asset_prop, chunk)))
        })
    }

    fn get_patch_file_name(asset_name: &str, original_path: &str) -> String {
        if original_path.is_empty() {
            format!("{}.hdiff", asset_name)
        } else {
            format!("{}.hdiff", original_path)
        }
    }

    fn create_diff_entries(manifest: &SophonPatchProto) -> Result<Vec<DiffEntry>> {
        Self::asset_pairs(manifest)
            .map(|(asset_prop, chunk)| {
                Ok(DiffEntry {
                    source_file_name: chunk.original_file_name.clone(),
                    source_file_size: chunk.original_file_length as u64,
                    source_file_md5: chunk.original_file_md5.clone(),
                    target_file_name: asset_prop.asset_name.clone(),
                    target_file_md5: asset_prop.asset_hash_md5.clone(),
                    target_file_size: asset_prop.asset_size as u64,
                    patch_file_name: Self::get_patch_file_name(
                        &asset_prop.asset_name,
                        &chunk.original_file_name,
                    ),
                    ..Default::default()
                })
            })
            .collect()
    }

    fn extract_hdiff_files(manifest: &SophonPatchProto, patch_path: &Path) -> Result<()> {
        Self::asset_pairs(manifest)
            .collect::<Vec<_>>()
            .into_par_iter()
            .map(|(asset_prop, chunk)| {
                let patch_file_name =
                    Self::get_patch_file_name(&asset_prop.asset_name, &chunk.original_file_name);

                let chunk_path = patch_path.join("ldiff").join(&chunk.patch_name);
                let output_path = patch_path.join(&patch_file_name);

                if let Some(parent) = output_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                let mut chunk_file = File::open(&chunk_path)
                    .with_context(|| format!("Failed to open chunk: {}", chunk.patch_name))?;

                chunk_file.seek(SeekFrom::Start(chunk.patch_offset as u64))?;

                let mut hdiff_bytes = vec![0u8; chunk.patch_length as usize];
                chunk_file.read_exact(&mut hdiff_bytes)?;

                fs::write(&output_path, hdiff_bytes)
                    .with_context(|| format!("Failed to write: {}", patch_file_name))?;

                Ok(())
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(())
    }

    fn cleanup_generated_hdiff(patch_path: &Path, diff_entries: &[DiffEntry]) {
        diff_entries.par_iter().for_each(|entry| {
            let _ = fs::remove_file(patch_path.join(&entry.patch_file_name));
        });
    }

    fn cleanup_old_files(
        game_path: &Path,
        diff_entries: &[DiffEntry],
        manifest: &SophonPatchProto,
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
            .patch_assets
            .iter()
            .map(|asset_prop| PathBuf::from(&asset_prop.asset_name))
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
    fn start(&self, game_path: &Path, patch_path: &Path, progress: &ProgressBar) -> Result<()> {
        progress.unset_length();
        progress.set_message("Reading manifest");
        let manifest = Self::load_manifest(&self.manifest_path)?;

        progress.set_message("Extracting files");
        Self::extract_hdiff_files(&manifest, patch_path)
            .context("Failed to extract hdiff files from ldiff")?;

        let diff_entries =
            Self::create_diff_entries(&manifest).context("Failed to create diff entries")?;

        progress.set_length(diff_entries.len() as u64);
        progress.set_message("Patching files");

        match self.patch_files(game_path, patch_path, &diff_entries, progress) {
            Ok(_) => {
                Self::cleanup_generated_hdiff(patch_path, &diff_entries);
                Self::cleanup_old_files(game_path, &diff_entries, &manifest)?;
            }
            Err(e) => {
                Self::cleanup_generated_hdiff(patch_path, &diff_entries);
                return Err(anyhow!("{e}"));
            }
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "ldiff"
    }
}
