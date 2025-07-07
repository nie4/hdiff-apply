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
    Io(#[from] std::io::Error),
    #[error("StarRail.exe not found in the current directory: {0}\nTip: Pass the game path as the first argument if it's not in the current directory or move this .exe")]
    GameNotFound(String),
    #[error("Hdiff archive was not found in the client directory!")]
    ArchiveNotFound(),
    #[error("Failed to parse BinaryVersion.bytes: could not extract version string!")]
    VersionParse(),
    #[error("Incompatible hdiff version: cannot update client from {0} to {1}")]
    InvalidHdiffVersion(String, String),
}
