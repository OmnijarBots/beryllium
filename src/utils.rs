use parking_lot::RwLock;
use reqwest::Client;
use std::path::{Path, PathBuf};

pub use uuid_v1::new_v1 as uuid_v1;

lazy_static! {
    static ref STORE_PATH: RwLock<PathBuf> = RwLock::new(PathBuf::from("."));
    static ref AUTH_TOKEN: RwLock<String> = RwLock::new(String::new());
    pub static ref HYPER_CLIENT: Client = Client::new();
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
