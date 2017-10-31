use base64::DecodeError as B64DecodeError;
use cryptobox::CBoxError;
use cryptobox::store::file::FileStore;
use hyper::Error as HyperError;
use image::ImageError;
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
    Image(ImageError),
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
            BerylliumError::Image(ref e)    => write!(f, "Image error: {}", e),
            BerylliumError::Hyper(ref e)    => write!(f, "Hyper error: {}", e),
            BerylliumError::PemFileError    => f.write_str("PEM file error"),
            BerylliumError::Serde(ref e)    => write!(f, "Serde error: {}", e),
            BerylliumError::Base64(ref e)   => write!(f, "Base64 decode error: {}", e),
            BerylliumError::Protobuf(ref e) => write!(f, "Protobuf error: {}", e),
            BerylliumError::Uuid(ref e)     => write!(f, "UUID parse error: {}", e),
            BerylliumError::Other(ref e)    => write!(f, "Unknown error: {}", e),
            BerylliumError::Unreachable     => f.write_str("Entered unreachable code!"),
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
            BerylliumError::Image(ref e)    => Some(e),
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

macro_rules! impl_error {
    ($err:ty => $ident:ident) => {
        impl From<$err> for BerylliumError {
            fn from(e: $err) -> BerylliumError {
                BerylliumError::$ident(e)
            }
        }
    }
}

impl_error!(ImageError => Image);
impl_error!(io::Error => Io);
impl_error!(ErrorStack => Openssl);
impl_error!(DecodeError => Decode);
impl_error!(EncodeError => Encode);
impl_error!(HyperError => Hyper);
impl_error!(CBoxError<FileStore> => CBox);
impl_error!(SerdeError => Serde);
impl_error!(B64DecodeError => Base64);
impl_error!(ProtobufError => Protobuf);
impl_error!(UuidError => Uuid);
