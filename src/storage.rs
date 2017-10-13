use {base64, utils};
use cryptobox::CBox;
use cryptobox::store::file::FileStore;
use errors::BerylliumResult;
use proteus::keys::PreKeyId;
use serde::{Deserialize, Serialize};
use serde_json;
use std::{iter, u16};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use types::EncodedPreKey;

pub struct StorageManager {
    path: PathBuf,
    cbox: CBox<FileStore>,
}

impl StorageManager {
    pub fn new(id: &str) -> BerylliumResult<Self> {
        let mut path = utils::get_store_path();
        path.push(id);
        if !path.is_dir() {
            info!("Creating {}", path.display());
            fs::create_dir_all(&path)?;
        }

        Ok(StorageManager {
            cbox: CBox::file_open(&path)?,
            path: path,
        })
    }

    pub fn initialize_prekeys(&self, keys: usize) -> BerylliumResult<Vec<EncodedPreKey>> {
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

    pub fn save_state<T>(&self, data: &T) -> BerylliumResult<()>
        where T: Serialize
    {
        let mut fd = File::create(self.path.join("bot_data.json"))
                          .map(BufWriter::new)?;
        serde_json::to_writer(&mut fd, data)?;
        Ok(())
    }

    pub fn load_state<T>(&self) -> BerylliumResult<T>
        where for<'de> T: Deserialize<'de>
    {
        let mut fd = File::open(self.path.join("bot_data.json"))
                          .map(BufReader::new)?;
        let data = serde_json::from_reader(&mut fd)?;
        Ok(data)
    }

    pub fn decrypt(&self, user_id: &str, client_id: &str,
                   data: &str) -> BerylliumResult<Vec<u8>>
    {
        let id = format!("{}_{}", user_id, client_id);
        let bytes = base64::decode(&data)?;
        let plain_data = match self.cbox.session_load(id.clone())? {
            Some(mut session) => {
                let data = session.decrypt(&bytes)?;
                self.cbox.session_save(&mut session)?;
                data
            },
            None => {
                info!("Couldn't find session for id: {}", id);
                let (mut session, data) =
                    self.cbox.session_from_message(id.clone(), &bytes)?;
                self.cbox.session_save(&mut session)?;
                data
            },
        };

        Ok(plain_data)
    }
}
