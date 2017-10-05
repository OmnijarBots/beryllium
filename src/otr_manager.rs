use cryptobox::CBox;
use cryptobox::store::file::FileStore;
use errors::{BerylliumError, BerylliumResult};
use std::fs;
use std::path::{Path, PathBuf};

pub struct OtrManager {
    path: PathBuf,
    bot_id: String,
    cbox: CBox<FileStore>,
}

impl OtrManager {
    pub fn new<P: AsRef<Path>>(path: P, id: &str) -> BerylliumResult<Self> {
        let mut path = PathBuf::from(path.as_ref());
        path.push(id);
        if !path.is_dir() {
            info!("Creating {}", path.display());
            fs::create_dir_all(&path)?;
        }

        Ok(OtrManager {
            cbox: CBox::file_open(&path)?,
            path: path,
            bot_id: id.to_owned(),
        })
    }
}
