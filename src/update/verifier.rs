use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use anyhow::{Context, Result};
use indicatif::ProgressBar;
use md5::{Digest, Md5};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::types::DiffEntry;
use crate::utils::pb_helper::create_progress_bar;

pub struct Verifier<'a> {
    game_path: &'a Path,
    diff_entries: &'a Vec<DiffEntry>,
}

impl<'a> Verifier<'a> {
    pub fn new(game_path: &'a Path, diff_entries: &'a Vec<DiffEntry>) -> Self {
        Self {
            game_path,
            diff_entries,
        }
    }

    fn verify_file(&self, entry: &DiffEntry, pb: ProgressBar) -> Result<()> {
        if entry.source_file_md5.is_empty() && entry.source_file_size == 0 {
            pb.inc(1);
            return Ok(());
        }

        let source_file_path = self.game_path.join(&entry.source_file_name);

        let mut file = File::open(&source_file_path)
            .with_context(|| format!("Failed to open file '{}'", source_file_path.display()))?;

        let file_size = file.seek(SeekFrom::End(0))?;

        if file_size != entry.source_file_size {
            anyhow::bail!(
                "File size mismatch: expected {} bytes, got {} bytes in '{}'",
                entry.source_file_size,
                file_size,
                source_file_path.display()
            );
        }

        file.seek(SeekFrom::Start(0))?;

        let mut hasher = Md5::new();
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = file.read(&mut buffer).with_context(|| {
                format!("Failed to read from file '{}'", source_file_path.display())
            })?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        let md5_hash = format!("{:x}", hasher.finalize());
        let expected_hash = &entry.source_file_md5;

        if md5_hash != *expected_hash {
            anyhow::bail!(
                "MD5 mismatch: expected {}, got {} in '{}'",
                entry.source_file_md5,
                md5_hash,
                source_file_path.display()
            );
        }

        pb.inc(1);
        Ok(())
    }

    pub fn verify_all(&self) -> Result<()> {
        let pb = create_progress_bar(self.diff_entries.len());

        self.diff_entries
            .par_iter()
            .try_for_each(|entry| self.verify_file(entry, pb.clone()))?;

        pb.finish();

        Ok(())
    }
}
