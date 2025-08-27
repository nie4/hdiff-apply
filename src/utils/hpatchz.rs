use std::{
    env,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

use anyhow::{Context, Result};

use crate::TEMP_DIR_NAME;

static HPATCHZ_INSTANCE: OnceLock<HPatchZ> = OnceLock::new();

pub struct HPatchZ {
    executable: PathBuf,
}

impl HPatchZ {
    pub fn instance() -> Result<&'static HPatchZ> {
        HPATCHZ_INSTANCE.get_or_try_init(|| HPatchZ::new())
    }

    fn new() -> Result<Self> {
        let executable = Self::extract_embedded_hpatchz()?;
        Ok(Self { executable })
    }

    fn extract_embedded_hpatchz() -> Result<PathBuf> {
        let temp_path = env::temp_dir().join(TEMP_DIR_NAME).join("hpatchz.exe");
        const HPATCHZ_BIN: &[u8] = include_bytes!("../../bin/hpatchz.exe");

        let mut file = File::create(&temp_path).with_context(|| {
            format!("Failed to create hpatchz.exe at '{}'", temp_path.display())
        })?;

        file.write_all(HPATCHZ_BIN)
            .with_context(|| format!("Failed to write hpatchz.exe to '{}'", temp_path.display()))?;

        Ok(temp_path)
    }

    /// Patch one file with the result if patch was successfull
    ///
    /// Only throw error when command fails to execute
    pub fn patch_file<P: AsRef<Path>>(
        source_file: P,
        patch_file: P,
        target_file: P,
    ) -> Result<bool> {
        let instance = Self::instance()?;

        if let Ok(output) = Command::new(&instance.executable)
            .args([
                source_file.as_ref().as_os_str(),
                patch_file.as_ref().as_os_str(),
                target_file.as_ref().as_os_str(),
                "-f".as_ref(),
            ])
            .output()
        {
            if output.status.success() {
                let _ = fs::remove_file(&patch_file);
                if source_file.as_ref() != target_file.as_ref() {
                    let _ = fs::remove_file(&source_file);
                }
                return Ok(true);
            } else if !output.stderr.is_empty() {
                return Ok(false);
            }
        } else {
            anyhow::bail!(
                "Failed to execute patch command for: {}",
                source_file.as_ref().display()
            )
        }

        Ok(true)
    }

    /// Patch file and log which file failed
    ///
    /// Doesnt delete source_file when source_file != target_file
    ///
    /// Only throw error when command fails to execute
    pub fn patch_file_no_delete<P: AsRef<Path>>(
        source_file: P,
        patch_file: P,
        target_file: P,
    ) -> Result<()> {
        let instance = Self::instance()?;

        if let Ok(output) = Command::new(&instance.executable)
            .args([
                source_file.as_ref().as_os_str(),
                patch_file.as_ref().as_os_str(),
                target_file.as_ref().as_os_str(),
                "-f".as_ref(),
            ])
            .output()
        {
            if output.status.success() {
                let _ = fs::remove_file(&patch_file);
            } else if !output.stderr.is_empty() {
                println!("Failed to patch: {}", source_file.as_ref().display());
            }
        } else {
            anyhow::bail!(
                "Failed to execute patch command for: {}",
                source_file.as_ref().display()
            )
        }

        Ok(())
    }
}
