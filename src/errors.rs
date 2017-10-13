use base64::DecodeError as B64DecodeError;
use cryptobox::CBoxError;
use cryptobox::store::file::FileStore;
use proteus::{DecodeError, EncodeError};
use protobuf::error::ProtobufError;
use reqwest::Error as ReqwestError;
use serde_json::error::Error as SerdeError;
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
    Reqwest(ReqwestError),
    Serde(SerdeError),
    Base64(B64DecodeError),
    Protobuf(ProtobufError),
    Unreachable,
}

impl Display for BerylliumError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            BerylliumError::CBox(ref e)   => write!(f, "Cryptobox error: {}", e),
            BerylliumError::Io(ref e)     => write!(f, "I/O error: {}", e),
            BerylliumError::Encode(ref e) => write!(f, "Encode error: {}", e),
            BerylliumError::Decode(ref e) => write!(f, "Decode error: {}", e),
            BerylliumError::PemFileError  => f.write_str("PEM file error"),
            BerylliumError::Reqwest(ref e) => write!(f, "Reqwest error: {}", e),
            BerylliumError::Serde(ref e) => write!(f, "Serde error: {}", e),
            BerylliumError::Base64(ref e) => write!(f, "Base64 decode error: {}", e),
            BerylliumError::Protobuf(ref e) => write!(f, "Protobuf error: {}", e),
            BerylliumError::Unreachable => write!(f, "Entered unreachable code!"),
        }
    }
}

impl Error for BerylliumError {
    fn description(&self) -> &str {
        "BerylliumError"
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            BerylliumError::CBox(ref e)     => Some(e),
            BerylliumError::Io(ref e)       => Some(e),
            BerylliumError::Decode(ref e)   => Some(e),
            BerylliumError::Encode(ref e)   => Some(e),
            BerylliumError::Reqwest(ref e)  => Some(e),
            BerylliumError::Base64(ref e)   => Some(e),
            BerylliumError::Protobuf(ref e) => Some(e),
            BerylliumError::Serde(ref e)    => Some(e),
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

impl From<ReqwestError> for BerylliumError {
    fn from(e: ReqwestError) -> BerylliumError {
        BerylliumError::Reqwest(e)
    }
}

impl From<SerdeError> for BerylliumError {
    fn from(e: SerdeError) -> BerylliumError {
        BerylliumError::Serde(e)
    }
}

impl From<B64DecodeError> for BerylliumError {
    fn from(e: B64DecodeError) -> BerylliumError {
        BerylliumError::Base64(e)
    }
}

impl From<ProtobufError> for BerylliumError {
    fn from(e: ProtobufError) -> BerylliumError {
        BerylliumError::Protobuf(e)
    }
}
