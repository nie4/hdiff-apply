use thiserror::Error;

pub type Result<T> = std::result::Result<T, HPatchZError>;

#[derive(Error, Debug)]
pub enum HPatchZError {
    #[error("Failed to initialize HPatchZ: {0}")]
    Initialization(String),
    #[error("Failed to execute HPatchZ: {0}")]
    Execute(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}