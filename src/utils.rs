use errors::{BerylliumError, BerylliumResult};
use futures::{Future, Stream, future};
use hyper::{Body, Error as HyperError, Headers};
use hyper::header::{ContentLength, Header};
use md5::Md5;
use openssl::rand;
use openssl::symm::{self, Cipher};
use parking_lot::RwLock;
use sha2::{Sha256, Digest};
use std::fmt::Display;
use std::path::{Path, PathBuf};
use types::{BerylliumFuture, EncryptData};

pub use uuid_v1::new_v1 as uuid_v1;

lazy_static! {
    static ref STORE_PATH: RwLock<PathBuf> = RwLock::new(PathBuf::from("."));
    static ref AUTH_TOKEN: RwLock<String> = RwLock::new(String::new());
}

// NOTE: Setting methods are meant to be called only once (during init)
pub fn set_store_path<P>(path: P) where P: AsRef<Path> {
    *STORE_PATH.write() = PathBuf::from(path.as_ref());
}

pub fn set_auth_token(token: String) {
    *AUTH_TOKEN.write() = token;
}

#[inline]
pub fn get_store_path() -> PathBuf {
    STORE_PATH.read().clone()
}

#[inline]
pub fn check_auth_token(token: &str) -> bool {
    *AUTH_TOKEN.read() == token
}

/// Return a `Future` that acquires the accumulated request body.
/// FIXME: Prone to DDoS attack! Restrict content length?
pub fn acquire_body(headers: &Headers, body: Body)
                   -> Box<Future<Item=Vec<u8>, Error=HyperError>> {
    let mut bytes = vec![];
    if let Some(l) = headers.get::<ContentLength>() {
        bytes.reserve(**l as usize);
    }

    let f = body.fold(bytes, |mut acc, ref chunk| {
        acc.extend_from_slice(chunk);
        future::ok::<_, HyperError>(acc)
    });

    Box::new(f)
}

/// ... only to map the `HyperError` with `BerylliumError`
#[inline]
pub fn acquire_body_with_err(headers: &Headers, body: Body)
                            -> BerylliumFuture<Vec<u8>> {
    let b = acquire_body(headers, body);
    Box::new(b.map_err(BerylliumError::from))
}

macro_rules! future_try {
    ($expr:expr) => {
        match $expr {
            Ok(v) => v,
            Err(e) => return Box::new(future::err(e.into()))
        }
    };
}

macro_rules! future_try_box {
    ($expr:expr) => {
        match $expr {
            Ok(v) => v,
            Err(e) => return Box::new(future::err(e.into())) as BerylliumFuture<_>
        }
    };
}

/// Helper struct for writing multipart data.
pub struct MultipartWriter {
    boundary: String,
    inner: Vec<u8>,
}

impl MultipartWriter {
    pub fn new(boundary: &str) -> MultipartWriter {
        MultipartWriter {
            boundary: boundary.to_owned(),
            inner: vec![],
        }
    }

    #[inline]
    pub fn add_line(&mut self) {
        self.inner.extend_from_slice("\r\n".as_bytes());
    }

    #[inline]
    pub fn add_boundary(&mut self) {
        self.inner.extend_from_slice("--".as_bytes());
        self.inner.extend_from_slice(self.boundary.as_bytes());
        self.add_line();
    }

    #[inline]
    pub fn add_header<H: Header + Display>(&mut self, header: H) {
        self.inner.extend_from_slice(header.to_string().as_bytes());
        self.add_line();
    }

    #[inline]
    pub fn add_body(&mut self, data: &[u8]) {
        self.inner.extend_from_slice(data);
        self.add_line();
    }

    #[inline]
    pub fn finish(self) -> Vec<u8> {
        self.inner
    }
}

/// Even though MD5 is broken, the Content-Md5 header is used by
/// Wire in multipart request for some reason.
pub fn md5_hash(data: &[u8]) -> Vec<u8> {
    let digest = Md5::digest(data);
    Vec::from(digest.as_slice())
}

/// Encrypt the given data with AES cipher (256 bits) in CBC mode
/// (with the initialization vector at the start). Also compute the
/// SHA-256 hash of the ciphertext.
pub fn encrypt(data: &[u8]) -> BerylliumResult<EncryptData> {
    let cipher = Cipher::aes_256_cbc();
    let mut iv = vec![0; cipher.iv_len().unwrap()];     // 16 bytes
    rand::rand_bytes(&mut iv)?;
    let mut key = vec![0; cipher.key_len()];    // 32 bytes
    rand::rand_bytes(&mut key)?;
    let mut bytes = symm::encrypt(cipher, &key, Some(&iv), data)?;
    let hash = Sha256::digest(&bytes);
    // First block is IV
    let mut out = iv.clone();
    out.append(&mut bytes);

    Ok(EncryptData {
        key: key,
        data: out,
        hash: Vec::from(hash.as_slice()),
    })
}
