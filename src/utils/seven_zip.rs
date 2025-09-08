// I didnt find any good 7z crates so this will have to do for now

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

use anyhow::{Context, Result};

use crate::TEMP_DIR_NAME;

static SEVENZ_INSTANCE: OnceLock<SevenZip> = OnceLock::new();

pub struct SevenZip {
    executable: PathBuf,
}

impl SevenZip {
    pub fn instance() -> Result<&'static SevenZip> {
        SEVENZ_INSTANCE.get_or_try_init(Self::new)
    }

    fn new() -> Result<Self> {
        let executable = Self::extract_embedded_sevenz()?;
        Ok(Self { executable })
    }

    /// Extract the embedded 7z.exe to the temp directory and return its path
    fn extract_embedded_sevenz() -> Result<PathBuf> {
        // 7z.exe is embedded via include_bytes!
        const SEVENZ_BIN: &[u8] = include_bytes!("../../bin/7z.exe");
        const SEVENZ_DLL_BIN: &[u8] = include_bytes!("../../bin/7z.dll");

        let temp_dir = std::env::temp_dir().join(TEMP_DIR_NAME);

        fs::create_dir_all(&temp_dir)
            .with_context(|| format!("Failed to create temp directory '{}'", temp_dir.display()))?;

        let exe_path = temp_dir.join("7z.exe");
        let dll_path = temp_dir.join("7z.dll");

        // Overwrite if already exists
        fs::write(&exe_path, SEVENZ_BIN)
            .with_context(|| format!("Failed to write 7z.exe to '{}'", exe_path.display()))?;
        fs::write(&dll_path, SEVENZ_DLL_BIN)
            .with_context(|| format!("Failed to write 7z.dll to '{}'", exe_path.display()))?;

        Ok(exe_path)
    }

    /// Checks if file exists in the root directory of the archive
    pub fn check_if_contains_file<P: AsRef<Path>>(archive: P, file: &str) -> Result<bool> {
        let instance = Self::instance()?;

        let output = Command::new(&instance.executable)
            .arg("l")
            .arg("-ba")
            .arg(archive.as_ref())
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run 7z: {}", e))?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("7z exited with error: {:?}", output.status));
        }

        let stdout = str::from_utf8(&output.stdout).context("Invalid UTF-8 in 7z output")?;

        let contains = stdout
            .lines()
            .filter_map(|line| line.split_whitespace().last())
            .any(|name| name == file && !name.contains('/') && !name.contains('\\'));

        Ok(contains)
    }

    // Extract defined files without preserving the archive folder structure
    pub fn extract_specific_files_to<P: AsRef<Path>>(
        archive: P,
        files_in_archive: &[&str],
        dst: P,
    ) -> Result<()> {
        let instance = Self::instance()?;

        let output = Command::new(&instance.executable)
            .arg("e")
            .arg(archive.as_ref())
            .args(files_in_archive)
            .arg(format!("-o{}", dst.as_ref().display()))
            .arg("-aoa")
            .output()
            .context("7-zip failed to run using Command")?;

        if !output.status.success() {
            let stderr_msg = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("7-zip extraction failed: '{}'", stderr_msg.to_string());
        }

        Ok(())
    }

    /// Extract all files with preserved folder structure excluding hdiffmap.json and deletefiles.txt
    pub fn extract_hdiff_to<P: AsRef<Path>>(archive: P, dst: P) -> Result<()> {
        let instance = Self::instance()?;

        let output = Command::new(&instance.executable)
            .arg("x")
            .arg(archive.as_ref())
            .arg(format!("-o{}", dst.as_ref().display()))
            .arg("-aoa")
            .args(["-x!hdiffmap.json", "-x!deletefiles.txt"])
            .output()
            .context("7-zip failed to run using Command")?;

        if !output.status.success() {
            let stderr_msg = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("7-zip extraction failed: '{}'", stderr_msg.to_string());
        }

        Ok(())
    }

    /// Extract all files with preserved folder structure excluding hdifffiles.txt and deletefiles.txt
    pub fn extract_custom_hdiff_to<P: AsRef<Path>>(archive: P, dst: P) -> Result<()> {
        let instance = Self::instance()?;

        let output = Command::new(&instance.executable)
            .arg("x")
            .arg(archive.as_ref())
            .arg(format!("-o{}", dst.as_ref().display()))
            .arg("-aoa")
            .args(["-x!hdifffiles.txt", "-x!deletefiles.txt"])
            .output()
            .context("7-zip failed to run using Command")?;

        if !output.status.success() {
            let stderr_msg = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("7-zip extraction failed: '{}'", stderr_msg.to_string());
        }

        Ok(())
    }

    // Extract all files with preserved folder structure
    pub fn extract_to<P: AsRef<Path>>(archive: P, dst: P) -> Result<()> {
        let instance = Self::instance()?;

        let output = Command::new(&instance.executable)
            .arg("x")
            .arg(archive.as_ref())
            .arg(format!("-o{}", dst.as_ref().display()))
            .arg("-aoa")
            .output()
            .context("7-zip failed to run using Command")?;

        if !output.status.success() {
            let stderr_msg = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("7-zip extraction failed: '{}'", stderr_msg.to_string());
        }

        Ok(())
    }
}
