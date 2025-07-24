use std::path::Path;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error[transparent]]
    DeleteFileError(#[from] DeleteFileError),
    #[error[transparent]]
    PatchError(#[from] PatchError),
    #[error[transparent]]
    SevenError(#[from] SevenZipError),
    #[error[transparent]]
    VerifyError(#[from] VerifyError),

    #[error("{0}")]
    Io(#[from] IOError),
    #[error("StarRail.exe not found in the current directory: {0}\nTip: Pass the game path as the first argument if it's not in the current directory or move this .exe")]
    GameNotFound(String),
    #[error("Hdiff archive was not found in the client directory!")]
    ArchiveNotFound(),
    #[error("Incompatible hdiff version: cannot update client from {0} to {1}")]
    InvalidHdiffVersion(String, String),
}

#[derive(Debug, Error)]
pub enum DeleteFileError {
    #[error("{0} doesn't exist, skipping")]
    NotFound(String),
    #[error("{0}")]
    Io(#[from] IOError),
}

#[derive(Debug, Error)]
pub enum PatchError {
    #[error("hdiffmap.json structure changed!")]
    Json(),
    #[error("{0} doesn't exist, skipping")]
    NotFound(String),
    #[error("{0}")]
    Io(#[from] IOError),
}

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
    #[error("{0}")]
    Io(#[from] IOError),
    #[error("hdiffmap.json structure changed!")]
    Json(),
}

#[derive(Error, Debug)]
pub enum SevenZipError {
    #[error("7-zip failed to run using Command")]
    CommandError(#[source] std::io::Error),
    #[error("7-zip extraction failed: '{0}'")]
    ExtractionFailed(String),
    #[error("Embedded 7z.exe extraction failed: {0}")]
    EmbeddedExtractionFailed(String),
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
