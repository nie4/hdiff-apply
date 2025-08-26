use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use rand::{distr::Alphanumeric, Rng};

use super::{deletefiles::DeleteFiles, hdiff::HDiff, ldiff::LDiff, verifier::Verifier};

use crate::{
    types::DiffEntry,
    utils::{self, binary_version::BinaryVersion, hpatchz::HPatchZ, seven_zip::SevenZip},
};

#[derive(Debug, PartialEq, Clone)]
pub enum PatchMethod {
    Hdiff,
    Ldiff,
    CustomMade,
}

#[derive(Debug, PartialEq, Clone)]
pub enum UpdateMode {
    Archives,
    Legacy(PatchMethod),
}

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    update_version: BinaryVersion,
    temp_path: PathBuf,
    archive_path: PathBuf,
    patch_method: PatchMethod,
}

pub struct UpdateMgr {
    update_archives_paths: Vec<PathBuf>,
    update_info: Vec<UpdateInfo>,
    temp_dir_path: PathBuf,
    client_version: BinaryVersion,
    game_path: PathBuf,
    update_mode: Option<UpdateMode>,
}

impl UpdateMgr {
    const BINARY_VERSION_PATH: &'static str = "StarRail_Data/StreamingAssets/BinaryVersion.bytes";

    pub fn new<T: AsRef<Path>>(game_path: T) -> Result<Self> {
        let game_path = game_path.as_ref().to_path_buf();

        let update_archives_paths =
            utils::get_update_archives(&game_path).context("Failed to get update archives")?;

        let temp_dir_path =
            utils::get_or_create_temp_dir().context("Failed to create temporary directory")?;

        let client_version = BinaryVersion::parse(game_path.join(Self::BINARY_VERSION_PATH))
            .context("Failed to parse client binary version")?;

        Ok(Self {
            update_archives_paths,
            update_info: Vec::new(),
            temp_dir_path,
            client_version,
            game_path,
            update_mode: None,
        })
    }

    fn detect_update_mode(&mut self) -> Result<UpdateMode> {
        if !self.update_archives_paths.is_empty() {
            let mode = UpdateMode::Archives;
            self.update_mode = Some(mode.clone());
            return Ok(mode);
        }

        let deletefiles_path = self.game_path.join("deletefiles.txt");
        let hdiffmap_path = self.game_path.join("hdiffmap.json");
        let hdifffiles_path = self.game_path.join("hdifffiles.txt");
        let manifest_path = self.game_path.join("manifest");

        let has_hdiff_files = hdiffmap_path.exists();
        let has_ldiff_files = manifest_path.exists();

        let has_custom_hdiff = deletefiles_path.exists() && hdifffiles_path.exists();

        if has_hdiff_files && has_ldiff_files {
            anyhow::bail!("Detected hdiff and ldiff files in the same place cannot proceed!")
        }

        let mode = if has_ldiff_files {
            utils::print_info("Running in legacy ldiff mode!");
            UpdateMode::Legacy(PatchMethod::Ldiff)
        } else if has_hdiff_files {
            utils::print_info("Running in legacy hdiff mode!");
            UpdateMode::Legacy(PatchMethod::Hdiff)
        } else if has_custom_hdiff {
            utils::print_info("Running in legacy custom hdiff mode!");
            UpdateMode::Legacy(PatchMethod::CustomMade)
        } else {
            anyhow::bail!("Update files/archives not found in the client directory!")
        };

        self.update_mode = Some(mode.clone());
        Ok(mode)
    }

    fn detect_archive_patch_method(&self, archive_path: &PathBuf) -> Result<PatchMethod> {
        let has_hdiffmap = SevenZip::check_if_contains_file(archive_path, "hdiffmap.json")?;

        if has_hdiffmap {
            Ok(PatchMethod::Hdiff)
        } else {
            let has_hdifffiles = SevenZip::check_if_contains_file(archive_path, "hdifffiles.txt")?;

            if has_hdifffiles {
                Ok(PatchMethod::CustomMade)
            } else {
                Ok(PatchMethod::Ldiff)
            }
        }
    }

    pub fn prepare_updates(&mut self) -> Result<()> {
        let mode = self.detect_update_mode()?;

        match mode {
            UpdateMode::Archives => self.prepare_archives()?,
            UpdateMode::Legacy(PatchMethod::Hdiff) => {}
            UpdateMode::Legacy(PatchMethod::Ldiff) => {}
            UpdateMode::Legacy(PatchMethod::CustomMade) => {}
        }

        Ok(())
    }

    fn prepare_archives(&mut self) -> Result<()> {
        let mut update_infos: Vec<UpdateInfo> = Vec::new();

        for (i, update_archive) in self.update_archives_paths.iter().enumerate() {
            let rnd_name: String = rand::rng()
                .sample_iter(&Alphanumeric)
                .take(5)
                .map(char::from)
                .collect();

            let patch_method = self.detect_archive_patch_method(update_archive)?;

            let temp_path = match patch_method {
                PatchMethod::Hdiff => self.temp_dir_path.join(format!("hdiff_{}", rnd_name)),
                PatchMethod::Ldiff => self.temp_dir_path.join(format!("ldiff_{}", rnd_name)),
                PatchMethod::CustomMade => self.temp_dir_path.join(format!("chdiff_{}", rnd_name)),
            };

            let previous_temp_path = if i > 0 {
                Some(&update_infos[i - 1].temp_path)
            } else {
                None
            };

            let update_version = match patch_method {
                PatchMethod::Hdiff => {
                    SevenZip::extract_specific_files_to(
                        update_archive,
                        &[
                            "StarRail_Data\\StreamingAssets\\BinaryVersion.bytes",
                            "StarRail_Data\\StreamingAssets\\BinaryVersion.bytes.hdiff",
                            "hdiffmap.json",
                            "deletefiles.txt",
                        ],
                        &temp_path,
                    )?;
                    self.get_hdiff_binary_version(&temp_path, previous_temp_path)?
                }
                PatchMethod::Ldiff => {
                    SevenZip::extract_to(update_archive, &temp_path)?;
                    self.get_ldiff_binary_version(&temp_path, previous_temp_path)?
                }
                PatchMethod::CustomMade => {
                    SevenZip::extract_specific_files_to(
                        update_archive,
                        &[
                            "StarRail_Data\\StreamingAssets\\BinaryVersion.bytes.hdiff",
                            "hdifffiles.txt",
                            "deletefiles.txt",
                        ],
                        &temp_path,
                    )?;
                    self.get_custom_hdiff_binary_version(&temp_path, previous_temp_path)?
                }
            };

            update_infos.push(UpdateInfo {
                update_version,
                temp_path,
                archive_path: update_archive.to_path_buf(),
                patch_method,
            });
        }

        self.update_info = update_infos;
        self.fix_update_sequence()?;

        Ok(())
    }

    fn get_hdiff_binary_version(
        &self,
        temp_path: &PathBuf,
        previous_temp_path: Option<&PathBuf>,
    ) -> Result<BinaryVersion> {
        if temp_path.join("BinaryVersion.bytes.hdiff").exists() {
            let source_file = if let Some(prev_path) = previous_temp_path {
                prev_path.join("BinaryVersion.bytes")
            } else {
                self.game_path.join(Self::BINARY_VERSION_PATH)
            };
            let patch_file = temp_path.join("BinaryVersion.bytes.hdiff");
            let output_file = temp_path.join("BinaryVersion.bytes");

            HPatchZ::patch_file_no_delete(&source_file, &patch_file, &output_file)?;

            BinaryVersion::parse(&temp_path.join("BinaryVersion.bytes"))
        } else {
            BinaryVersion::parse(&temp_path.join("BinaryVersion.bytes"))
        }
    }

    fn get_custom_hdiff_binary_version(
        &self,
        temp_path: &PathBuf,
        previous_temp_path: Option<&PathBuf>,
    ) -> Result<BinaryVersion> {
        let source_file = if let Some(prev_path) = previous_temp_path {
            prev_path.join("BinaryVersion.bytes")
        } else {
            self.game_path.join(Self::BINARY_VERSION_PATH)
        };
        let patch_file = temp_path.join("BinaryVersion.bytes.hdiff");
        let output_file = temp_path.join("BinaryVersion.bytes");

        HPatchZ::patch_file_no_delete(&source_file, &patch_file, &output_file)?;

        BinaryVersion::parse(output_file)
    }

    fn get_ldiff_binary_version(
        &self,
        temp_path: &PathBuf,
        previous_temp_path: Option<&PathBuf>,
    ) -> Result<BinaryVersion> {
        let client_binary_version = if let Some(prev_path) = previous_temp_path {
            prev_path.join("BinaryVersion.bytes")
        } else {
            self.game_path.join(Self::BINARY_VERSION_PATH)
        };

        let ldiff = LDiff::new(&self.game_path, Some(&temp_path))?;
        let binary_version_asset = ldiff
            .manifest_proto
            .assets
            .iter()
            .find(|a| a.asset_name.ends_with("/BinaryVersion.bytes"))
            .ok_or_else(|| {
                anyhow::anyhow!("Failed to get BinaryVersion.bytes from the manifest file")
            })?;

        if let Some(asset_chunk) = &binary_version_asset.asset_data {
            let patch_asset = &asset_chunk.assets.first();

            if let Some(patch_asset) = patch_asset {
                let chunk_path = ldiff.ldiff_path.join(&patch_asset.chunk_file_name);
                let mut chunk_file = File::open(&chunk_path)?;

                chunk_file.seek(SeekFrom::Start(
                    patch_asset.hdiff_file_in_chunk_offset as u64,
                ))?;
                let mut hdiff_bytes = vec![0u8; patch_asset.hdiff_file_size as usize];
                chunk_file.read_exact(&mut hdiff_bytes)?;

                let output_path = &temp_path.join("BinaryVersion.bytes.hdiff");
                let mut output_file = File::create(&output_path)?;
                output_file.write_all(&hdiff_bytes)?;
            }
        }

        let patch_file = &temp_path.join("BinaryVersion.bytes.hdiff");
        let target_file = &temp_path.join("BinaryVersion.bytes");
        HPatchZ::patch_file_no_delete(&client_binary_version, patch_file, target_file)?;

        BinaryVersion::parse(target_file)
    }

    fn fix_update_sequence(&mut self) -> Result<()> {
        let mut cur_version = &self.client_version;
        let mut valid_start_idx = None;
        let mut valid_count = 0;

        for (i, update) in self.update_info.iter().enumerate() {
            if utils::verify_version(cur_version, &update.update_version) {
                if valid_start_idx.is_none() {
                    valid_start_idx = Some(i);
                }
                cur_version = &update.update_version;
                valid_count += 1;
            } else if valid_start_idx.is_some() {
                break;
            }
        }

        if valid_count == 0 {
            let last = self
                .update_info
                .last()
                .map(|update| update.update_version.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            return Err(anyhow::anyhow!(
                "Incompatible hdiff version: cannot update client from {} to {}",
                self.client_version.to_string(),
                last
            ));
        }

        if let Some(start_idx) = valid_start_idx {
            self.update_info.drain(0..start_idx);
            self.update_info.truncate(valid_count);
        }

        Ok(())
    }

    pub fn update_sequence(&self) -> String {
        let mut sequence = String::with_capacity(64);

        match &self.update_mode {
            Some(UpdateMode::Legacy(PatchMethod::Hdiff)) => {
                sequence.push_str("update to next version");
            }
            Some(UpdateMode::Legacy(PatchMethod::Ldiff)) => {
                sequence.push_str("update to next version");
            }
            Some(UpdateMode::Legacy(PatchMethod::CustomMade)) => {
                sequence.push_str("update to next version");
            }
            Some(UpdateMode::Archives) => {
                sequence.push_str(&self.client_version.to_string());
                for update in &self.update_info {
                    sequence.push_str(&format!(" → {}", &update.update_version.patch_version));
                }
            }
            None => {}
        }

        sequence
    }

    fn get_legacy_update_file_paths(&self) -> (PathBuf, PathBuf) {
        (
            self.game_path.join("hdiffmap.json"),
            self.game_path.join("deletefiles.txt"),
        )
    }

    fn get_update_file_paths(&self, update: &UpdateInfo) -> (PathBuf, PathBuf) {
        (
            update.temp_path.join("hdiffmap.json"),
            update.temp_path.join("deletefiles.txt"),
        )
    }

    fn run_integrity_check(&self, diff_entries: &Vec<DiffEntry>) -> Result<()> {
        let verify_client = Verifier::new(&self.game_path, diff_entries);
        verify_client.verify_all()
    }

    fn start_legacy_hdiff_updater(&self, do_integrity_check: bool) -> Result<()> {
        let (hdiffmap_path, deletefiles_path) = self.get_legacy_update_file_paths();

        let mut hdiff = HDiff::new(&self.game_path, &hdiffmap_path);
        let delete_files = DeleteFiles::new(&self.game_path, &deletefiles_path);
        let diff_entries = &hdiff.load_diff_entries()?;

        if do_integrity_check {
            println!("Verifying client integrity");
            self.run_integrity_check(diff_entries)?;
        }

        println!("Patching files");
        hdiff.patch(diff_entries)?;

        println!("Removing unused files");
        delete_files.remove()?;

        println!("Updated to {}", self.client_version.to_string());
        Ok(())
    }

    fn start_hdiff_updater(
        &self,
        update: &UpdateInfo,
        index: usize,
        do_integrity_check: bool,
    ) -> Result<()> {
        let (hdiffmap_path, deletefiles_path) = self.get_update_file_paths(update);

        let mut hdiff = HDiff::new(&self.game_path, &hdiffmap_path);
        let delete_files = DeleteFiles::new(&self.game_path, &deletefiles_path);
        let diff_entries = &hdiff.load_diff_entries()?;

        println!(
            "\n-- HDiff Update {} of {}",
            index + 1,
            self.update_info.len()
        );

        let archive_name = update
            .archive_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("hdiff");

        println!("Extracting {}", archive_name);
        SevenZip::extract_hdiff_to(&update.archive_path, &self.game_path)?;

        if do_integrity_check {
            println!("Verifying client integrity");
            self.run_integrity_check(diff_entries)?;
        }

        println!("Patching files");
        hdiff.patch(diff_entries)?;

        println!("Removing unused files");
        delete_files.remove()?;

        println!("Updated to {}", update.update_version.to_string());
        Ok(())
    }

    fn start_legacy_ldiff_updater(&self, do_integrity_check: bool) -> Result<()> {
        let mut ldiff = LDiff::new(&self.game_path, None)?;

        let deletefiles_path = self.game_path.join("deletefiles.txt");
        let delete_files = DeleteFiles::new(&self.game_path, &deletefiles_path);

        let diff_entries = ldiff.create_diff_entries()?;

        if do_integrity_check {
            println!("Verifying client integrity");
            self.run_integrity_check(&diff_entries)?;
        }

        println!("Patching files");
        ldiff.create_hdiff_files()?;
        ldiff.patch(diff_entries)?;

        println!("Removing unused files");
        if !delete_files.remove()? {
            ldiff.handle_delete_files()?;
        }

        println!("Updated");

        Ok(())
    }

    fn start_ldiff_updater(
        &self,
        update: &UpdateInfo,
        index: usize,
        do_integrity_check: bool,
    ) -> Result<()> {
        println!(
            "\n-- LDiff Update {} of {}",
            index + 1,
            self.update_info.len()
        );

        let archive_name = update
            .archive_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("ldiff");
        println!("Processing {}", archive_name);

        let mut ldiff = LDiff::new(&self.game_path, Some(&update.temp_path))?;

        let deletefiles_path = update.temp_path.join("deletefiles.txt");
        let delete_files = DeleteFiles::new(&self.game_path, &deletefiles_path);

        let diff_entries = ldiff.create_diff_entries()?;

        if do_integrity_check {
            println!("Verifying client integrity");
            self.run_integrity_check(&diff_entries)?;
        }

        println!("Patching files");
        ldiff.create_hdiff_files()?;
        ldiff.patch(diff_entries)?;

        println!("Removing unused files");
        if !delete_files.remove()? {
            ldiff.handle_delete_files()?;
        }

        println!("Updated to {}", update.update_version.to_string());
        Ok(())
    }

    fn start_legacy_custom_hdiff_updater(&self) -> Result<()> {
        let hdifffiles_path = self.game_path.join("hdifffiles.txt");
        let deletefiles_path = self.game_path.join("deletefiles.txt");

        let hdiff = HDiff::new(&self.game_path, &hdifffiles_path);
        let delete_files = DeleteFiles::new(&self.game_path, &deletefiles_path);

        let custom_entries = hdiff.load_custom_map()?;

        println!("Patching files");
        hdiff.patch_custom(custom_entries)?;

        println!("Removing unused files");
        delete_files.remove()?;

        println!("Updated");

        Ok(())
    }

    fn start_custom_hdiff_updater(&self, update: &UpdateInfo, index: usize) -> Result<()> {
        println!(
            "\n-- Custom HDiff Update {} of {}",
            index + 1,
            self.update_info.len()
        );

        let archive_name = update
            .archive_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("hdiff");

        println!("Extracting {}", archive_name);
        SevenZip::extract_custom_hdiff_to(&update.archive_path, &self.game_path)?;

        let hdifffiles_path = update.temp_path.join("hdifffiles.txt");
        let deletefiles_path = update.temp_path.join("deletefiles.txt");

        let hdiff = HDiff::new(&self.game_path, &hdifffiles_path);
        let delete_files = DeleteFiles::new(&self.game_path, &deletefiles_path);

        let custom_entries = hdiff.load_custom_map()?;

        println!("Patching files");
        hdiff.patch_custom(custom_entries)?;

        println!("Removing unused files");
        delete_files.remove()?;

        println!("Updated to {}", update.update_version.to_string());

        Ok(())
    }

    pub fn update(&self, do_integrity_check: bool) -> Result<()> {
        match &self.update_mode {
            Some(UpdateMode::Legacy(PatchMethod::Hdiff)) => {
                self.start_legacy_hdiff_updater(do_integrity_check)?;
            }
            Some(UpdateMode::Legacy(PatchMethod::Ldiff)) => {
                self.start_legacy_ldiff_updater(do_integrity_check)?;
            }
            Some(UpdateMode::Legacy(PatchMethod::CustomMade)) => {
                self.start_legacy_custom_hdiff_updater()?;
            }
            Some(UpdateMode::Archives) => {
                for (i, update) in self.update_info.iter().enumerate() {
                    match update.patch_method {
                        PatchMethod::Hdiff => {
                            self.start_hdiff_updater(update, i, do_integrity_check)?;
                        }
                        PatchMethod::Ldiff => {
                            self.start_ldiff_updater(update, i, do_integrity_check)?;
                        }
                        PatchMethod::CustomMade => {
                            self.start_custom_hdiff_updater(update, i)?;
                        }
                    }
                }
            }
            None => {}
        }

        Ok(())
    }
}
