use std::fs;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
    sync::Arc,
};

use indicatif::ProgressBar;
use md5::{Digest, Md5};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::Deserialize;

use crate::{
    error::{IOError, VerifyError},
    utils::pb_helper::create_progress_bar,
};

#[derive(Deserialize)]
struct DiffEntry {
    source_file_name: String,
    source_file_size: u64,
    source_file_md5: String,
}

#[derive(Deserialize)]
struct HDiffMap {
    diff_map: Vec<DiffEntry>,
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

    fn load_diff_map(&self) -> Result<Vec<DiffEntry>, VerifyError> {
        let data = fs::read_to_string(&self.hdiff_map_path)
            .map_err(|e| IOError::read_to_string(self.hdiff_map_path, e))?;
        let hdiff_map: HDiffMap = serde_json::from_str(&data).map_err(|_| VerifyError::Json())?;

        Ok(hdiff_map.diff_map)
    }

    fn verify_file(&self, entry: &DiffEntry, pb: Arc<ProgressBar>) -> Result<(), VerifyError> {
        let source_file_path = self.game_path.join(&entry.source_file_name);
        let mut file =
            File::open(&source_file_path).map_err(|e| IOError::open(&source_file_path, e))?;
        let file_size = file
            .seek(SeekFrom::End(0))
            .map_err(|e| IOError::seek(&source_file_path, e))?;

        if file_size != entry.source_file_size {
            return Err(VerifyError::FileSizeMismatchError {
                expected: entry.source_file_size,
                got: file_size,
                file_name: source_file_path.display().to_string(),
            });
        }

        file.seek(SeekFrom::Start(0))
            .map_err(|e| IOError::seek(&source_file_path, e))?;
        let mut hasher = Md5::new();
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = file
                .read(&mut buffer)
                .map_err(|e| IOError::read_buffer(&source_file_path, e))?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        let md5_hash = format!("{:x}", hasher.finalize());
        let expected_hash = &entry.source_file_md5;

        if md5_hash != *expected_hash {
            return Err(VerifyError::Md5MismatchError {
                expected: expected_hash.to_string(),
                got: md5_hash,
                file_name: source_file_path.display().to_string(),
            });
        }

        pb.inc(1);
        Ok(())
    }

    pub fn verify_all(&self) -> Result<(), VerifyError> {
        let hdiff_map = self.load_diff_map()?;

        let pb = Arc::new(create_progress_bar(hdiff_map.len()));

        hdiff_map
            .par_iter()
            .try_for_each(|entry| self.verify_file(entry, pb.clone()))?;

        pb.finish();

        Ok(())
    }
}
