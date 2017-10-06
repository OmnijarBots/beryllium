use cryptobox::CBox;
use cryptobox::store::file::FileStore;
use errors::BerylliumResult;
use proteus::keys::{PreKeyBundle, PreKeyId};
use std::fs;
use std::path::{Path, PathBuf};
use std::u16;

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

    pub fn initialize(&self, keys: usize) -> BerylliumResult<Vec<PreKeyBundle>> {
        let mut vec = Vec::with_capacity(8 * keys + 1);
        for i in 0..8 * keys {
            let key = self.cbox.new_prekey(PreKeyId::new(i as u16))?;
            vec.push(key);
        }

        vec.push(self.cbox.new_prekey(PreKeyId::new(u16::MAX))?);
        Ok(vec)
    }
}
