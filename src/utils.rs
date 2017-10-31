use errors::BerylliumError;
use futures::{Future, Stream, future};
use hyper::{Body, Error as HyperError, Headers};
use hyper::header::ContentLength;
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use types::BerylliumFuture;

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
