use std::{
    fs::File,
    io::{Seek, SeekFrom},
    path::Path,
};

use md5::{Digest, Md5};
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VerifyError {
    #[error("File size mismatch expected `{expected}` bytes got `{got} bytes in `{file_name}`. Client might be corrupted or used incompatible hdiff")]
    FileSizeMismatchError {
        expected: u64,
        got: u64,
        file_name: String,
    },
    #[error("MD5 mismatch expected `{expected}` got `{got}` in `{file_name}`. Client might be corrupted or used incompatible hdiff")]
    Md5MismatchError {
        expected: String,
        got: String,
        file_name: String,
    },
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Deserialize)]
struct DiffMap {
    source_file_name: String,
    source_file_size: u64,
    source_file_md5: String,
}

pub struct Verifier<'a> {
    game_path: &'a Path,
    hdiff_map_path: &'a Path,
}

impl<'a> Verifier<'a> {
    pub fn new(game_path: &'a Path, hdiff_map_path: &'a Path) -> Self {
        Self {
            game_path,
            hdiff_map_path,
        }
    }

    fn load_diff_map(&self) -> Result<Vec<DiffMap>, VerifyError> {
        let data = std::fs::read_to_string(&self.hdiff_map_path)?;
        let deserialized: Value = serde_json::from_str(&data).unwrap();

        let diff_map = deserialized.get("diff_map").unwrap();

        Ok(serde_json::from_value(diff_map.clone()).unwrap())
    }

    pub fn by_file_size(&self) -> Result<(), VerifyError> {
        let hdiff_map = self.load_diff_map()?;

        for diff_map in &hdiff_map {
            let expected_size = diff_map.source_file_size;
            let source_file_path = self.game_path.join(&diff_map.source_file_name);

            let mut source_file = File::open(&source_file_path)?;
            let source_file_size = source_file.seek(SeekFrom::End(0))?;

            if source_file_size != expected_size {
                return Err(VerifyError::FileSizeMismatchError {
                    expected: expected_size,
                    got: source_file_size,
                    file_name: source_file_path.display().to_string(),
                });
            }
        }

        Ok(())
    }

    pub fn by_md5(&self) -> Result<(), VerifyError> {
        let hdiff_map = self.load_diff_map()?;

        hdiff_map
            .into_iter()
            .map(|entry| {
                let source_file_path = self.game_path.join(&entry.source_file_name);
                let expected_md5 = &entry.source_file_md5;

                let md5_hash = self.file_md5(&source_file_path)?;

                if md5_hash != *expected_md5 {
                    return Err(VerifyError::Md5MismatchError {
                        expected: expected_md5.to_string(),
                        got: md5_hash,
                        file_name: source_file_path.display().to_string(),
                    });
                } else {
                    Ok(())
                }
            })
            .collect::<Result<Vec<()>, VerifyError>>()?;

        Ok(())
    }

    fn file_md5<P: AsRef<Path>>(&self, path: P) -> Result<String, VerifyError> {
        let buffer = std::fs::read(path)?;
        let mut hasher = Md5::new();
        hasher.update(buffer);
        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }
}
