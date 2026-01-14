#![feature(once_cell_try)] // Stable when

use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::OnceLock,
};

use crate::error::{Result, SevenZipError};

pub mod error;

static SEVENZ_INSTANCE: OnceLock<SevenZip> = OnceLock::new();

pub struct SevenZip(PathBuf);

impl SevenZip {
    pub fn instance() -> Result<&'static Self> {
        SEVENZ_INSTANCE.get_or_try_init(Self::new)
    }

    fn new() -> Result<Self> {
        let temp_dir = common::path::get_temp_dir()?;

        const SEVENZ_EXE: &[u8] = include_bytes!("../bin/7z.exe");
        const SEVENZ_DLL: &[u8] = include_bytes!("../bin/7z.dll");

        let exe_path = temp_dir.join("7z.exe");
        let dll_path = temp_dir.join("7z.dll");

        if !exe_path.exists() {
            fs::write(&exe_path, SEVENZ_EXE).map_err(|e| {
                SevenZipError::Initialization(format!("Failed to write 7z.exe: {}", e))
            })?;
        }

        if !dll_path.exists() {
            fs::write(&dll_path, SEVENZ_DLL).map_err(|e| {
                SevenZipError::Initialization(format!("Failed to write 7z.dll: {}", e))
            })?;
        }

        Ok(Self(exe_path))
    }

    fn execute(&self, args: &[impl AsRef<OsStr>]) -> Result<Output> {
        Command::new(&self.0)
            .args(args)
            .output()
            .map_err(|e| SevenZipError::Execute(format!("Command failed: {}", e)))
    }

    pub fn extract<P: AsRef<Path>>(archive_path: P, output_dir: P) -> Result<()> {
        let archive = archive_path.as_ref();
        let output_dir = output_dir.as_ref();

        if !archive.exists() {
            return Err(SevenZipError::ArchiveNotFound(
                archive.display().to_string(),
            ));
        }

        let inst = Self::instance()?;

        let args = [
            "x",
            &archive.display().to_string(),
            &format!("-o{}", &output_dir.display()),
            "-aoa",
            "-bsp0",
        ];

        let output = inst.execute(&args)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

            return Err(SevenZipError::ExtractionFailed {
                archive: archive.display().to_string(),
                exit_code: output.status.code().unwrap_or(-1),
                message: stderr,
            });
        }

        Ok(())
    }
}
