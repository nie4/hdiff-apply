use std::fmt::{self};

#[derive(Debug, Clone)]
pub struct ByteConvert(f64);

impl ByteConvert {
    const UNITS: [&'static str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
}

impl From<u64> for ByteConvert {
    fn from(value: u64) -> Self {
        ByteConvert(value as f64)
    }
}

impl fmt::Display for ByteConvert {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut value = self.0;
        let mut unit_index = 0;

        while value >= 1024.0 && unit_index < Self::UNITS.len() - 1 {
            value = value / 1024.0;
            unit_index += 1;
        }

        write!(f, "{:.2} {}", value, Self::UNITS[unit_index])
    }
}
