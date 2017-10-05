use cryptobox::store::Store;
use cryptobox::Identity;
use proteus::keys::{IdentityKeyPair, PreKey, PreKeyId};
use proteus::session::Session;
use std::borrow::{Borrow, Cow};
use std::fs;
use std::path::{Path, PathBuf};
use utils::{self, FileStoreError, FileStoreResult};

pub struct FileStore {
    path: PathBuf,
    bot_id: String,
    // TODO(wafflespeanut): figure out whether we should store
    // the identity in the struct (probably `RefCell<IdentityKeyPair>`?).
}

impl FileStore {
    fn load_identity_from_file<P>(store_path: P) -> FileStoreResult<IdentityKeyPair>
        where P: AsRef<Path>
    {
        let path = store_path.as_ref().join("identity.id");
        let content = utils::read_file_contents(&path)?;
        IdentityKeyPair::deserialise(&content).map_err(FileStoreError::from)
    }

    fn save_identity_to_file<P>(store_path: P, identity: &IdentityKeyPair)
                               -> FileStoreResult<()>
        where P: AsRef<Path>
    {
        let path = store_path.as_ref().join("identity.id");
        let data: Vec<u8> = identity.serialise()?;
        utils::write_to_file(&path, &data)
    }

    pub fn new(path: &str, id: String) -> FileStoreResult<Self> {
        let mut path = PathBuf::from(path);
        path.push(&id);
        info!("Initializing file store for {}", path.display());

        if !path.is_dir() {
            fs::create_dir_all(&path)?;
        }

        if path.join("identity.id").is_file() {
            Self::load_identity_from_file(&path)?;
        } else {
            let identity = IdentityKeyPair::new();
            Self::save_identity_to_file(&path, &identity)?;
        }

        Ok(FileStore {
            path: path,
            bot_id: id,
        })
    }
}

impl Store for FileStore {
    type Error = FileStoreError;

    fn load_session<I>(&self, identity: I, id: &str)
                      -> Result<Option<Session<I>>, Self::Error>
        where I: Borrow<IdentityKeyPair>
    {
        info!("Loading session (id: {})", id);
        let path = self.path.join(&id);
        if !path.is_file() {
            return Ok(None)
        }

        let bytes = utils::read_file_contents(&path)?;
        Session::deserialise(identity, &bytes)
                .map(Some).map_err(FileStoreError::from)
    }

    fn save_session<I>(&self, id: &str, session: &Session<I>) -> Result<(), Self::Error>
        where I: Borrow<IdentityKeyPair>
    {
        info!("Saving session (id: {})", id);
        let path = self.path.join(&id);
        let data = session.serialise()?;
        utils::write_to_file(path, &data)
    }

    fn delete_session(&self, id: &str) -> Result<(), Self::Error> {
        info!("Deleting session (id: {})", id);
        let path = self.path.join(&id);
        fs::remove_file(&path).map_err(FileStoreError::from)
    }

    fn load_identity<'a>(&self) -> Result<Option<Identity<'a>>, Self::Error> {
        info!("Loading identity...");
        let i = Self::load_identity_from_file(&self.path)?;
        Ok(Some(Identity::Sec(Cow::Owned(i))))
    }

    fn save_identity(&self, id: &Identity) -> Result<(), Self::Error> {
        info!("Saving identity...");
        match *id {
            Identity::Sec(ref i) => Self::save_identity_to_file(&self.path, i),
            Identity::Pub(_) => Err(FileStoreError::IdentityError),
        }
    }

    fn load_prekey(&self, id: PreKeyId) -> Result<Option<PreKey>, Self::Error> {
        info!("Loading prekey (id: {}", id);
        let path = self.path.join(format!("{}.pkid", id));
        if !path.is_file() {
            return Ok(None)
        }

        let bytes = utils::read_file_contents(&path)?;
        PreKey::deserialise(&bytes).map(Some).map_err(FileStoreError::from)
    }

    fn add_prekey(&self, key: &PreKey) -> Result<(), Self::Error> {
        info!("Adding prekey (id: {})", key.key_id);
        let path = self.path.join(format!("{}.pkid", key.key_id));
        let data = key.serialise()?;
        utils::write_to_file(path, &data)
    }

    fn delete_prekey(&self, id: PreKeyId) -> Result<(), Self::Error> {
        info!("Removing prekey (id: {})", id);
        let path = self.path.join(format!("{}.pkid", id));
        fs::remove_file(&path).map_err(FileStoreError::from)
    }
}
