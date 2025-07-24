use std::{fs::File, io::Read, path::Path};

use regex::Regex;

use crate::{error::IOError, AppError};

#[derive(Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct BinaryVersion {
    pub major_version: u32,
    pub minor_version: u32,
    pub patch_version: u32,
}

impl BinaryVersion {
    pub fn parse<T: AsRef<Path>>(binary_version_path: T) -> Result<Self, AppError> {
        let mut file = File::open(&binary_version_path)
            .map_err(|e| IOError::open(binary_version_path.as_ref(), e))?;

        let mut buf = Vec::new();
        let n = file
            .read_to_end(&mut buf)
            .map_err(|e| IOError::read_to_end(binary_version_path.as_ref(), e))?;

        let content = String::from_utf8_lossy(&buf[..n]);

        let re = Regex::new(r"(\d+)\.(\d+)\.(\d{1,2})").unwrap();

        if let Some(caps) = re.captures(&content) {
            Ok(Self {
                major_version: caps[1].parse::<u32>().unwrap_or(0),
                minor_version: caps[2].parse::<u32>().unwrap_or(0),
                patch_version: caps[3].parse::<u32>().unwrap_or(0),
            })
        } else {
            Ok(BinaryVersion::default())
        }
    }
}

impl ToString for BinaryVersion {
    fn to_string(&self) -> String {
        format!(
            "{}.{}.{}",
            self.major_version, self.minor_version, self.patch_version
        )
    }
}
