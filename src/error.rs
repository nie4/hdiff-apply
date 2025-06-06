use thiserror::Error;

use crate::{deletefiles, hdiffmap, seven_util};

#[derive(Error, Debug)]
pub enum Error {
    #[error[transparent]]
    DeleteFileError(#[from] deletefiles::DeleteFileError),
    #[error[transparent]]
    PatchError(#[from] hdiffmap::PatchError),
    #[error[transparent]]
    SevenError(#[from] seven_util::SevenError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("StarRail.exe not found in the current directory: {0}\nTip: Pass the game path as the first argument if it's not in the current directory or move this .exe")]
    PathNotFound(String),
    #[error("Hdiff archive was not found in the client directory!")]
    ArchiveNotFound(),
    #[error("Failed to parse BinaryVersion.bytes: could not extract version string!")]
    VersionParse(),
    #[error("Incompatible hdiff version: cannot update client from {0} to {1} if you believe this is a mistake use the --skip-version-check flag to override")]
    InvalidHdiffVersion(String, String),
}
