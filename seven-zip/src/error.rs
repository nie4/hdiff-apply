use thiserror::Error;

pub type Result<T> = std::result::Result<T, SevenZipError>;

#[derive(Error, Debug)]
pub enum SevenZipError {
    #[error("Failed to initialize 7-Zip: {0}")]
    Initialization(String),
    #[error("Failed to execute 7-Zip: {0}")]
    Execute(String),
    #[error("Archive not found: {0}")]
    ArchiveNotFound(String),
    #[error("Extraction of '{archive}' failed (exit code {exit_code}): {message}")]
    ExtractionFailed {
        archive: String,
        exit_code: i32,
        message: String,
    },

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}