use {base64, utils};
use cryptobox::CBox;
use cryptobox::store::file::FileStore;
use errors::BerylliumResult;
use proteus::keys::PreKeyId;
use std::{fs, iter, u16};
use types::EncodedPreKey;

pub struct OtrManager {
    cbox: CBox<FileStore>,
}

impl OtrManager {
    pub fn new(id: &str) -> BerylliumResult<Self> {
        let mut path = utils::get_store_path();
        path.push(id);
        if !path.is_dir() {
            info!("Creating {}", path.display());
            fs::create_dir_all(&path)?;
        }

        Ok(OtrManager {
            cbox: CBox::file_open(&path)?,
        })
    }

    pub fn initialize(&self, keys: usize) -> BerylliumResult<Vec<EncodedPreKey>> {
        let mut vec = Vec::with_capacity(8 * keys + 1);
        for i in (0..8 * (keys as u16)).chain(iter::once(u16::MAX)) {
            let key = self.cbox.new_prekey(PreKeyId::new(i))?;
            let encoded = EncodedPreKey {
                id: i,
                key: base64::encode(&key.serialise()?)
            };

            vec.push(encoded);
        }

        Ok(vec)
    }
}
