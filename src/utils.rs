use parking_lot::RwLock;
use std::path::{Path, PathBuf};

lazy_static! {
    pub static ref STORE_PATH: RwLock<PathBuf> = RwLock::new(PathBuf::from("."));
    pub static ref AUTH_TOKEN: RwLock<String> = RwLock::new(String::new());
}

// NOTE: Should be called only once
pub fn set_store_path<P>(path: P) where P: AsRef<Path> {
    *STORE_PATH.write() = PathBuf::from(path.as_ref());
}

#[inline]
pub fn get_store_path() -> PathBuf {
    STORE_PATH.read().clone()
}

// NOTE: Should be called only once
pub fn set_auth_token(token: String) {
    *AUTH_TOKEN.write() = token;
}

#[inline]
pub fn check_auth_token(token: &str) -> bool {
    *AUTH_TOKEN.read() == token
}
