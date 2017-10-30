use base64::DecodeError as B64DecodeError;
use cryptobox::CBoxError;
use cryptobox::store::file::FileStore;
use hyper::Error as HyperError;
use openssl::error::ErrorStack;
use proteus::{DecodeError, EncodeError};
use protobuf::error::ProtobufError;
use serde_json::error::Error as SerdeError;
use std::error::Error;
use std::fmt::{self, Display};
use std::io;
use uuid::ParseError as UuidError;

pub type BerylliumResult<T> = Result<T, BerylliumError>;

/// Global error which encapsulates all related errors.
#[derive(Debug)]
pub enum BerylliumError {
    Io(io::Error),
    CBox(CBoxError<FileStore>),
    Openssl(ErrorStack),
    Encode(EncodeError),
    Decode(DecodeError),
    Hyper(HyperError),
    PemFileError,
    Serde(SerdeError),
    Base64(B64DecodeError),
    Protobuf(ProtobufError),
    Uuid(UuidError),
    Other(String),
    Unreachable,
}

impl Display for BerylliumError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            BerylliumError::CBox(ref e)     => write!(f, "Cryptobox error: {}", e),
            BerylliumError::Io(ref e)       => write!(f, "I/O error: {}", e),
            BerylliumError::Openssl(ref e)  => write!(f, "Openssl error: {}", e),
            BerylliumError::Encode(ref e)   => write!(f, "Encode error: {}", e),
            BerylliumError::Decode(ref e)   => write!(f, "Decode error: {}", e),
            BerylliumError::Hyper(ref e)    => write!(f, "Hyper error: {}", e),
            BerylliumError::PemFileError    => f.write_str("PEM file error"),
            BerylliumError::Serde(ref e)    => write!(f, "Serde error: {}", e),
            BerylliumError::Base64(ref e)   => write!(f, "Base64 decode error: {}", e),
            BerylliumError::Protobuf(ref e) => write!(f, "Protobuf error: {}", e),
            BerylliumError::Uuid(ref e)     => write!(f, "UUID parse error: {}", e),
            BerylliumError::Other(ref e)    => write!(f, "Unknown error: {}", e),
            BerylliumError::Unreachable     => write!(f, "Entered unreachable code!"),
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
            BerylliumError::Openssl(ref e)  => Some(e),
            BerylliumError::Io(ref e)       => Some(e),
            BerylliumError::Decode(ref e)   => Some(e),
            BerylliumError::Encode(ref e)   => Some(e),
            BerylliumError::Hyper(ref e)    => Some(e),
            BerylliumError::Base64(ref e)   => Some(e),
            BerylliumError::Protobuf(ref e) => Some(e),
            BerylliumError::Uuid(ref e)     => Some(e),
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

impl From<ErrorStack> for BerylliumError {
    fn from(e: ErrorStack) -> BerylliumError {
        BerylliumError::Openssl(e)
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

impl From<HyperError> for BerylliumError {
    fn from(e: HyperError) -> BerylliumError {
        BerylliumError::Hyper(e)
    }
}

impl From<CBoxError<FileStore>> for BerylliumError {
    fn from(e: CBoxError<FileStore>) -> BerylliumError {
        BerylliumError::CBox(e)
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

impl From<UuidError> for BerylliumError {
    fn from(e: UuidError) -> BerylliumError {
        BerylliumError::Uuid(e)
    }
}
