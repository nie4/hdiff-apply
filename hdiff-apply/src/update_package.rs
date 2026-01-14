use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use seven_zip::SevenZip;

use crate::byte_convert::ByteConvert;

#[derive(Debug, Clone)]
pub struct UpdatePackage {
    pub name: String,
    pub path: PathBuf,
    pub size: ByteConvert,
}

impl UpdatePackage {
    pub fn find(scan_path: &Path) -> Result<Vec<Self>> {
        let mut archives = Vec::new();

        for entry in scan_path
            .read_dir()
            .with_context(|| format!("Failed to read directory: {}", scan_path.display()))?
        {
            let path = entry?.path();

            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if matches!(
                    ext.to_ascii_lowercase().as_str(),
                    "zip" | "7z" | "rar" | "tar"
                ) {
                    let size = path.metadata()?.len().into();
                    archives.push(UpdatePackage {
                        name: path
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("archive")
                            .to_string(),
                        path,
                        size,
                    });
                }
            }
        }

        Ok(archives)
    }

    pub fn extract(&self, game_path: &PathBuf) -> Result<()> {
        SevenZip::extract(&self.path, &game_path)?;
        Ok(())
    }
}

impl std::fmt::Display for UpdatePackage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
