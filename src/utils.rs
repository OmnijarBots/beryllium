use proteus::{DecodeError, EncodeError};
use std::error::Error;
use std::fmt::{self, Display};
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;

pub type FileStoreResult<T> = Result<T, FileStoreError>;

#[derive(Debug)]
pub enum FileStoreError {
    Io(io::Error),
    Encode(EncodeError),
    Decode(DecodeError),
    IdentityError,
}

impl Display for FileStoreError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            FileStoreError::Io(ref e)     => write!(f, "FileStoreError: I/O error: {}", e),
            FileStoreError::Encode(ref e) => write!(f, "FileStoreError: Encode error: {}", e),
            FileStoreError::Decode(ref e) => write!(f, "FileStoreError: Decode error: {}", e),
            FileStoreError::IdentityError => f.write_str("FileStoreError: IdentityError"),
        }
    }
}

impl Error for FileStoreError {
    fn description(&self) -> &str {
        "FileStoreError"
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            FileStoreError::Io(ref e)     => Some(e),
            FileStoreError::Decode(ref e) => Some(e),
            FileStoreError::Encode(ref e) => Some(e),
            _ => None,
        }
    }
}


impl From<io::Error> for FileStoreError {
    fn from(e: io::Error) -> FileStoreError {
        FileStoreError::Io(e)
    }
}

impl From<DecodeError> for FileStoreError {
    fn from(e: DecodeError) -> FileStoreError {
        FileStoreError::Decode(e)
    }
}

impl From<EncodeError> for FileStoreError {
    fn from(e: EncodeError) -> FileStoreError {
        FileStoreError::Encode(e)
    }
}

pub fn read_file_contents<P: AsRef<Path>>(path: P) -> FileStoreResult<Vec<u8>> {
    let mut buf = vec![];
    let mut fd = File::open(&path).map(BufReader::new).map_err(FileStoreError::from)?;
    fd.read_to_end(&mut buf).map_err(FileStoreError::from)?;
    Ok(buf)
}

pub fn write_to_file<P: AsRef<Path>>(path: P, buf: &[u8]) -> FileStoreResult<()> {
    let mut fd = File::create(&path).map(BufWriter::new).map_err(FileStoreError::from)?;
    fd.write_all(buf).map_err(FileStoreError::from)
}
