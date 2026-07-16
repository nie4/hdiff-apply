#![feature(once_cell_try)] // Stable when

use std::{
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::OnceLock,
};

use anyhow::Context;

use crate::error::{Result, SevenZipError};

pub mod error;

static SEVENZ_INSTANCE: OnceLock<SevenZip> = OnceLock::new();

pub struct SevenZip(PathBuf);

impl SevenZip {
    pub fn instance() -> Result<&'static Self> {
        SEVENZ_INSTANCE.get_or_try_init(Self::new)
    }

    fn new() -> Result<Self> {
        let temp_dir = env::temp_dir().join("hdiff-apply");
        fs::create_dir_all(&temp_dir).context("Failed to create temp directory")?;

        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::PermissionsExt;

            const SEVENZ_BINARY: &[u8] = include_bytes!("../bin/linux-x64/7zzs");
            let binary_path = temp_dir.join("7zzs");

            fs::write(&binary_path, SEVENZ_BINARY).map_err(|e| {
                SevenZipError::Initialization(format!("Failed to write 7zzs: {}", e))
            })?;

            fs::set_permissions(&binary_path, fs::Permissions::from_mode(0o700)).map_err(|e| {
                SevenZipError::Initialization(format!("Failed to set permissions: {}", e))
            })?;

            Ok(Self(binary_path))
        }

        #[cfg(target_os = "windows")]
        {
            const SEVENZ_BINARY: &[u8] = include_bytes!("../bin/windows-x64/7z.exe");
            const SEVENZ_DLL: &[u8] = include_bytes!("../bin/windows-x64/7z.dll");
            let binary_path = temp_dir.join("7z.exe");
            let dll_path = temp_dir.join("7z.dll");

            fs::write(&binary_path, SEVENZ_BINARY).map_err(|e| {
                SevenZipError::Initialization(format!("Failed to write 7z.exe: {}", e))
            })?;

            fs::write(&dll_path, SEVENZ_DLL).map_err(|e| {
                SevenZipError::Initialization(format!("Failed to write 7z.dll: {}", e))
            })?;

            Ok(Self(binary_path))
        }
    }

    fn execute(&self, args: &[impl AsRef<OsStr>]) -> Result<Output> {
        Command::new(&self.0)
            .args(args)
            .output()
            .map_err(|e| SevenZipError::Execute(format!("Command failed: {}", e)))
    }

    pub fn extract(archive_path: &Path, output_dir: &Path) -> Result<()> {
        if !archive_path.exists() {
            return Err(SevenZipError::ArchiveNotFound(
                archive_path.display().to_string(),
            ));
        }

        let inst = Self::instance()?;

        let args = [
            "x",
            &archive_path.display().to_string(),
            &format!("-o{}", &output_dir.display()),
            "-aoa",
            "-bsp0",
        ];

        let output = inst.execute(&args)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

            return Err(SevenZipError::ExtractionFailed {
                archive: archive_path.display().to_string(),
                exit_code: output.status.code().unwrap_or(-1),
                message: stderr,
            });
        }

        Ok(())
    }
}
