use std::io::Error as IoError;

use err_derive::Error;

#[derive(Debug, Error)]
pub enum MpqError {
    #[error(display = "No header found")]
    NoHeader,
    #[error(display = "IO Error: {}", cause)]
    IoError { cause: IoError },
    #[error(display = "Unsupported MPQ version")]
    UnsupportedVersion,
    #[error(display = "Corrupted archive")]
    Corrupted,
    #[error(display = "File not found")]
    FileNotFound,
    #[error(display = "Compression type unsupported: {}", kind)]
    UnsupportedCompression { kind: String },
}

impl From<IoError> for MpqError {
    fn from(other: IoError) -> Self {
        MpqError::IoError { cause: other }
    }
}
