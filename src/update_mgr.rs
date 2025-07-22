use rayon::iter::IntoParallelRefIterator;

use crate::verifier::VerifyError;

use super::*;

pub struct UpdateInfo {
    hdiff_version: BinaryVersion,
    temp_path: PathBuf,
    archive_path: PathBuf,
}

pub struct UpdateMgr {
    update_archives_paths: Vec<PathBuf>,
    update_info: Vec<UpdateInfo>,
    temp_dir_path: PathBuf,
    client_version: BinaryVersion,
    game_path: PathBuf,
    hpatchz_path: PathBuf,
    legacy_mode: bool,
}

impl UpdateMgr {
    pub fn new(
        update_archives_paths: Vec<PathBuf>,
        temp_dir_path: PathBuf,
        client_version: BinaryVersion,
        game_path: PathBuf,
        hpatchz_path: PathBuf,
    ) -> Self {
        Self {
            update_archives_paths,
            update_info: Vec::new(),
            temp_dir_path,
            client_version,
            game_path,
            hpatchz_path,
            legacy_mode: false,
        }
    }

    pub fn detect_legacy_mode(&mut self) -> Result<bool, Error> {
        if !self.update_archives_paths.is_empty() {
            return Ok(false);
        }

        let deletefiles_path = self.game_path.join("deletefiles.txt");
        let hdiffmap_path = self.game_path.join("hdiffmap.json");

        let is_legacy = deletefiles_path.exists() && hdiffmap_path.exists();

        if is_legacy {
            self.legacy_mode = true;
            utils::print_warn("Running in legacy mode!");
            Ok(true)
        } else {
            Err(Error::ArchiveNotFound())
        }
    }

    pub fn prepare_update_info(&mut self) -> Result<(), Error> {
        if self.detect_legacy_mode()? {
            return Ok(());
        }

        let update_infos: Result<Vec<UpdateInfo>, Error> = self
            .update_archives_paths
            .par_iter()
            .map(|update_archive| {
                let rnd_name: String = rand::rng()
                    .sample_iter(&Alphanumeric)
                    .take(5)
                    .map(char::from)
                    .collect();

                let temp_path = self.temp_dir_path.join(format!("hdiff_{}", rnd_name));

                SevenUtil::inst()?.extract_specific_files_to(
                    update_archive,
                    &[
                        "StarRail_Data\\StreamingAssets\\BinaryVersion.bytes",
                        "hdiffmap.json",
                        "deletefiles.txt",
                    ],
                    &temp_path,
                )?;

                let hdiff_version = BinaryVersion::parse(&temp_path.join("BinaryVersion.bytes"))?;

                Ok(UpdateInfo {
                    hdiff_version,
                    temp_path,
                    archive_path: update_archive.to_path_buf(),
                })
            })
            .collect();

        self.update_info = update_infos?;
        self.fix_update_sequence()?;

        Ok(())
    }

    fn fix_update_sequence(&mut self) -> Result<(), Error> {
        let mut cur_version = &self.client_version;
        let mut valid_start_idx = None;
        let mut valid_count = 0;

        for (i, update) in self.update_info.iter().enumerate() {
            if utils::verify_version(cur_version, &update.hdiff_version) {
                if valid_start_idx.is_none() {
                    valid_start_idx = Some(i);
                }
                cur_version = &update.hdiff_version;
                valid_count += 1;
            } else if valid_start_idx.is_some() {
                break;
            }
        }

        if valid_count == 0 {
            let last = self
                .update_info
                .last()
                .map(|update| update.hdiff_version.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            return Err(Error::InvalidHdiffVersion(
                self.client_version.to_string(),
                last,
            ));
        }

        if let Some(start_idx) = valid_start_idx {
            self.update_info.drain(0..start_idx);
            self.update_info.truncate(valid_count);
        }

        Ok(())
    }

    pub fn show_update_sequence(&self) -> String {
        let mut sequence = String::with_capacity(34);
        if self.legacy_mode {
            sequence.push_str(&format!("update to {}", &self.client_version.to_string()));
        } else {
            sequence.push_str(&self.client_version.to_string());
        }

        for update in &self.update_info {
            sequence.push_str(&format!(" → {}", update.hdiff_version.patch_version));
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

    fn run_patcher(&self, hdiffmap_path: &PathBuf, deletefiles_path: &PathBuf) {
        let mut delete_files = DeleteFiles::new(&self.game_path, &deletefiles_path);
        if let Err(err) = delete_files.remove() {
            utils::print_err(err);
        }

        let mut hdiff_map = HDiffMap::new(&self.game_path, &self.hpatchz_path, &hdiffmap_path);
        if let Err(err) = hdiff_map.patch() {
            utils::print_err(err);
        }
    }

    fn run_integrity_check(&self, hdiffmap_path: &PathBuf) -> Result<(), VerifyError> {
        println!("Verifying client integrity");
        let verify_client = Verifier::new(&self.game_path, hdiffmap_path);
        verify_client.verify_all()
    }

    fn start_legacy_updater(&self, run_integrity_check: bool) -> Result<(), Error> {
        let (hdiffmap_path, deletefiles_path) = self.get_legacy_update_file_paths();

        if run_integrity_check {
            self.run_integrity_check(&hdiffmap_path)?;
        }

        println!("Patching files");
        self.run_patcher(&hdiffmap_path, &deletefiles_path);

        println!("Updated to {}", self.client_version.to_string());
        Ok(())
    }

    fn start_updater(
        &self,
        update: &UpdateInfo,
        index: usize,
        do_integrity_check: bool,
    ) -> Result<(), Error> {
        let (hdiffmap_path, deletefiles_path) = self.get_update_file_paths(update);

        println!("\n-- Update {} of {}", index + 1, self.update_info.len());

        let archive_name = update
            .archive_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("hdiff");

        println!("Extracting {}", archive_name);
        SevenUtil::inst()?.extract_hdiff_to(&update.archive_path, &self.game_path)?;

        if do_integrity_check {
            self.run_integrity_check(&hdiffmap_path)?;
        }

        println!("Patching files");
        self.run_patcher(&hdiffmap_path, &deletefiles_path);

        println!("Updated to {}", update.hdiff_version.to_string());
        Ok(())
    }

    pub fn update(&self, do_integrity_check: bool) -> Result<(), Error> {
        if self.legacy_mode {
            self.start_legacy_updater(do_integrity_check)?;
        }

        for (i, update) in self.update_info.iter().enumerate() {
            self.start_updater(update, i, do_integrity_check)?;
        }

        Ok(())
    }
}
