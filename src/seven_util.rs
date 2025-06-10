// I didnt find any good 7z crates so this will have to do for now

use std::{path::{Path, PathBuf}, process::Command, sync::OnceLock};

use thiserror::Error;
use crate::{utils, TEMP_DIR_NAME};

static INST: OnceLock<SevenUtil> = OnceLock::new();

#[derive(Error, Debug)]
pub enum SevenError {
    #[error("7-zip failed to run using Command")]
    CommandError(#[source] std::io::Error),
    #[error("7-zip extraction failed: '{0}'")]
    ExtractionFailed(String),
    #[error("Embedded 7z.exe extraction failed: {0}")]
    EmbeddedExtractionFailed(String),
}

#[derive(Default)]
pub struct SevenUtil {
    executable: PathBuf,
}

impl SevenUtil {
    pub fn new() -> Result<Self, SevenError> {
        let executable = Self::extract_embedded_sevenz()?;
        Ok(Self { executable })
    }

    /// Extract the embedded 7z.exe to the temp directory and return its path
    fn extract_embedded_sevenz() -> Result<PathBuf, SevenError> {
        // 7z.exe is embedded via include_bytes!
        const SEVENZ_BIN: &[u8] = include_bytes!("../bin/7z.exe");
        let temp_dir = std::env::temp_dir().join(TEMP_DIR_NAME);
        std::fs::create_dir_all(&temp_dir).map_err(|e| {
            SevenError::EmbeddedExtractionFailed(format!("Failed to create temp dir: {e}"))
        })?;
        let exe_path = temp_dir.join("7z.exe");
        // Overwrite if already exists
        std::fs::write(&exe_path, SEVENZ_BIN).map_err(|e| {
            SevenError::EmbeddedExtractionFailed(format!("Failed to write 7z.exe: {e}"))
        })?;
        Ok(exe_path)
    }

    pub fn inst() -> &'static SevenUtil {
        INST.get_or_init(|| match SevenUtil::new() {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("{}", e);
                utils::wait_for_input();
                std::process::exit(0);
            }
        })
    }

    pub fn extract_specific_files_to(
        &self,
        archive: &PathBuf,
        files_in_archive: &[&str],
        dst: &PathBuf,
    ) -> Result<(), SevenError> {
        let output = Command::new(&self.executable)
            .arg("e")
            .arg(archive)
            .args(files_in_archive)
            .arg(format!("-o{}", dst.display()))
            .arg("-aoa")
            .output()
            .map_err(SevenError::CommandError)?;

        if !output.status.success() {
            let stderr_msg = String::from_utf8_lossy(&output.stderr);
            return Err(SevenError::ExtractionFailed(stderr_msg.to_string()));
        }

        Ok(())
    }

    pub fn extract_hdiff_to(&self, archive: &Path, dst: &Path) -> Result<(), SevenError> {
        let output = Command::new(&self.executable)
            .arg("x")
            .arg(archive)
            .arg(format!("-o{}", dst.display()))
            .arg("-aoa")
            .args(["-x!hdiffmap.json", "-x!deletefiles.txt"])
            .output()
            .map_err(SevenError::CommandError)?;

        if !output.status.success() {
            let stderr_msg = String::from_utf8_lossy(&output.stderr);
            return Err(SevenError::ExtractionFailed(stderr_msg.to_string()));
        }

        Ok(())
    }
}
