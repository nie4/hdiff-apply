#![feature(once_cell_try)] // Stable when

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::OnceLock,
};

use crate::error::{HPatchZError, Result};

pub mod error;

static HPATCHZ_INSTANCE: OnceLock<HPatchZ> = OnceLock::new();

pub struct HPatchZ(PathBuf);

impl HPatchZ {
    pub fn instance() -> Result<&'static Self> {
        HPATCHZ_INSTANCE.get_or_try_init(Self::new)
    }

    fn new() -> Result<Self> {
        let temp_dir = common::path::get_temp_dir()?;

        const HPATCHZ_EXE: &[u8] = include_bytes!("../bin/hpatchz.exe");

        let exe_path = temp_dir.join("hpatchz.exe");

        if !exe_path.exists() {
            fs::write(&exe_path, HPATCHZ_EXE).map_err(|e| {
                HPatchZError::Initialization(format!("Failed to write hpatchz.exe: {}", e))
            })?;
        }

        Ok(Self(exe_path))
    }

    fn execute(&self, args: &[impl AsRef<std::ffi::OsStr>]) -> Result<Output> {
        Command::new(&self.0)
            .args(args)
            .output()
            .map_err(|e| HPatchZError::Execute(format!("Command failed: {}", e)))
    }

    pub fn patch_file<P: AsRef<Path>>(
        source_file: P,
        patch_file: P,
        target_file: P,
    ) -> Result<bool> {
        let inst = Self::instance()?;

        let args = [
            source_file.as_ref().as_os_str(),
            patch_file.as_ref().as_os_str(),
            target_file.as_ref().as_os_str(),
            "-f".as_ref(),
        ];

        let output = inst.execute(&args)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(HPatchZError::Execute(stderr.to_string()));
        }

        Ok(true)
    }
}
