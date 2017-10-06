use cryptobox::CBoxError;
use cryptobox::store::file::FileStore;
use proteus::{DecodeError, EncodeError};
use std::error::Error;
use std::fmt::{self, Display};
use std::io;

pub type BerylliumResult<T> = Result<T, BerylliumError>;

#[derive(Debug)]
pub enum BerylliumError {
    Io(io::Error),
    CBox(CBoxError<FileStore>),
    Encode(EncodeError),
    Decode(DecodeError),
    PemFileError,
}

impl Display for BerylliumError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            BerylliumError::CBox(ref e)   => write!(f, "Cryptobox error: {}", e),
            BerylliumError::Io(ref e)     => write!(f, "I/O error: {}", e),
            BerylliumError::Encode(ref e) => write!(f, "Encode error: {}", e),
            BerylliumError::Decode(ref e) => write!(f, "Decode error: {}", e),
            BerylliumError::PemFileError  => f.write_str("PEM file error"),
        }
    }
}

impl Error for BerylliumError {
    fn description(&self) -> &str {
        "BerylliumError"
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            BerylliumError::CBox(ref e)   => Some(e),
            BerylliumError::Io(ref e)     => Some(e),
            BerylliumError::Decode(ref e) => Some(e),
            BerylliumError::Encode(ref e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for BerylliumError {
    fn from(e: io::Error) -> BerylliumError {
        BerylliumError::Io(e)
    }
}

impl From<DecodeError> for BerylliumError {
    fn from(e: DecodeError) -> BerylliumError {
        BerylliumError::Decode(e)
    }
}

impl From<EncodeError> for BerylliumError {
    fn from(e: EncodeError) -> BerylliumError {
        BerylliumError::Encode(e)
    }
}

impl From<CBoxError<FileStore>> for BerylliumError {
    fn from(e: CBoxError<FileStore>) -> BerylliumError {
        BerylliumError::CBox(e)
    }
}
