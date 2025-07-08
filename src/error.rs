use std::path::Path;

use thiserror::Error;

use crate::*;

#[derive(Error, Debug)]
pub enum Error {
    #[error[transparent]]
    DeleteFileError(#[from] deletefiles::DeleteFileError),
    #[error[transparent]]
    PatchError(#[from] hdiffmap::PatchError),
    #[error[transparent]]
    SevenError(#[from] seven_util::SevenError),
    #[error[transparent]]
    VerifyError(#[from] verifier::VerifyError),

    #[error("{0}")]
    Io(#[from] IOError),
    #[error("StarRail.exe not found in the current directory: {0}\nTip: Pass the game path as the first argument if it's not in the current directory or move this .exe")]
    GameNotFound(String),
    #[error("Hdiff archive was not found in the client directory!")]
    ArchiveNotFound(),
    #[error("Failed to parse BinaryVersion.bytes: could not extract version string!")]
    VersionParse(),
    #[error("Incompatible hdiff version: cannot update client from {0} to {1}")]
    InvalidHdiffVersion(String, String),
}

#[derive(Error, Debug)]
pub enum IOError {
    #[error("Failed to open file '{path}': {source}")]
    Open {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to seek in file '{path}': {source}")]
    Seek {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to create directory '{path}': {source}")]
    CreateDir {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to read entire file '{path}': {source}")]
    ReadToEnd {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to read line from file '{path}': {source}")]
    ReadLine {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to read file '{path}' to string: {source}")]
    ReadToString {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to write to file '{path}': {source}")]
    WriteAll {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to get current directory: {source}")]
    CurrentDir {
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to read directory '{path}': {source}")]
    ReadDir {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to read directory entry in '{path}': {source}")]
    ReadDirEntry {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to read buffer from file '{path}': {source}")]
    ReadBuffer {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to create file '{path}': {source}")]
    CreateFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

impl IOError {
    pub fn open(path: &Path, source: std::io::Error) -> Self {
        Self::Open {
            path: path.display().to_string(),
            source,
        }
    }
    pub fn seek(path: &Path, source: std::io::Error) -> Self {
        Self::Seek {
            path: path.display().to_string(),
            source,
        }
    }
    pub fn create_dir(path: &Path, source: std::io::Error) -> Self {
        Self::CreateDir {
            path: path.display().to_string(),
            source,
        }
    }
    pub fn read_to_end(path: &Path, source: std::io::Error) -> Self {
        Self::ReadToEnd {
            path: path.display().to_string(),
            source,
        }
    }
    pub fn read_line(path: &Path, source: std::io::Error) -> Self {
        Self::ReadLine {
            path: path.display().to_string(),
            source,
        }
    }
    pub fn read_to_string(path: &Path, source: std::io::Error) -> Self {
        Self::ReadToString {
            path: path.display().to_string(),
            source,
        }
    }
    pub fn write_all(path: &Path, source: std::io::Error) -> Self {
        Self::WriteAll {
            path: path.display().to_string(),
            source,
        }
    }
    pub fn read_dir(path: &Path, source: std::io::Error) -> Self {
        Self::ReadDir {
            path: path.display().to_string(),
            source,
        }
    }
    pub fn read_dir_entry(path: &Path, source: std::io::Error) -> Self {
        Self::ReadDirEntry {
            path: path.display().to_string(),
            source,
        }
    }
    pub fn read_buffer(path: &Path, source: std::io::Error) -> Self {
        Self::ReadBuffer {
            path: path.display().to_string(),
            source,
        }
    }
    pub fn current_dir(source: std::io::Error) -> Self {
        Self::CurrentDir { source }
    }
    pub fn create_file(path: &Path, source: std::io::Error) -> Self {
        Self::CreateFile {
            path: path.display().to_string(),
            source,
        }
    }
}
