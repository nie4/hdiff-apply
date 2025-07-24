// I didnt find any good 7z crates so this will have to do for now

use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

use crate::{error::SevenZipError, TEMP_DIR_NAME};

static INST: OnceLock<SevenZip> = OnceLock::new();

#[derive(Default)]
pub struct SevenZip {
    executable: PathBuf,
}

impl SevenZip {
    pub fn new() -> Result<Self, SevenZipError> {
        let executable = Self::extract_embedded_sevenz()?;
        Ok(Self { executable })
    }

    /// Extract the embedded 7z.exe to the temp directory and return its path
    fn extract_embedded_sevenz() -> Result<PathBuf, SevenZipError> {
        // 7z.exe is embedded via include_bytes!
        const SEVENZ_BIN: &[u8] = include_bytes!("../../bin/7z.exe");
        let temp_dir = std::env::temp_dir().join(TEMP_DIR_NAME);
        std::fs::create_dir_all(&temp_dir).map_err(|e| {
            SevenZipError::EmbeddedExtractionFailed(format!("Failed to create temp dir: {e}"))
        })?;
        let exe_path = temp_dir.join("7z.exe");
        // Overwrite if already exists
        std::fs::write(&exe_path, SEVENZ_BIN).map_err(|e| {
            SevenZipError::EmbeddedExtractionFailed(format!("Failed to write 7z.exe: {e}"))
        })?;
        Ok(exe_path)
    }

    pub fn inst() -> Result<&'static SevenZip, SevenZipError> {
        INST.get_or_try_init(Self::new)
    }

    pub fn extract_specific_files_to(
        &self,
        archive: &PathBuf,
        files_in_archive: &[&str],
        dst: &PathBuf,
    ) -> Result<(), SevenZipError> {
        let output = Command::new(&self.executable)
            .arg("e")
            .arg(archive)
            .args(files_in_archive)
            .arg(format!("-o{}", dst.display()))
            .arg("-aoa")
            .output()
            .map_err(SevenZipError::CommandError)?;

        if !output.status.success() {
            let stderr_msg = String::from_utf8_lossy(&output.stderr);
            return Err(SevenZipError::ExtractionFailed(stderr_msg.to_string()));
        }

        Ok(())
    }

    pub fn extract_hdiff_to(&self, archive: &Path, dst: &Path) -> Result<(), SevenZipError> {
        let output = Command::new(&self.executable)
            .arg("x")
            .arg(archive)
            .arg(format!("-o{}", dst.display()))
            .arg("-aoa")
            .args(["-x!hdiffmap.json", "-x!deletefiles.txt"])
            .output()
            .map_err(SevenZipError::CommandError)?;

        if !output.status.success() {
            let stderr_msg = String::from_utf8_lossy(&output.stderr);
            return Err(SevenZipError::ExtractionFailed(stderr_msg.to_string()));
        }

        Ok(())
    }
}
