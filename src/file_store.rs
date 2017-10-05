use proteus::keys::IdentityKeyPair;
use std::fs;
use std::path::PathBuf;
use utils;

pub struct FileStore {
    path: PathBuf,
    bot_id: String,
    identity: IdentityKeyPair,
}

impl FileStore {
    pub fn new(path: &str, id: String) -> Result<FileStore, String> {
        let mut path = PathBuf::from(path);
        path.push(&id);

        if !path.is_dir() {
            fs::create_dir_all(&path)
               .map_err(|e| format!("Cannot create {} ({})", path.display(), e))?;
        }

        let mut identity_path = path.clone();
        identity_path.push("identity.id");
        let identity = if identity_path.is_file() {
            let content = utils::read_file_contents(&identity_path)?;
            IdentityKeyPair::deserialise(&content)
                            .map_err(|e| format!("Cannot decode identity ({})", e))?
        } else {
            let identity = IdentityKeyPair::new();
            let data: Vec<u8> = identity.serialise()
                .map_err(|e| format!("Cannot encode identity ({})", e))?;
            utils::write_to_file(&identity_path, &data)?;
            identity
        };

        Ok(FileStore {
            path: path,
            bot_id: id,
            identity: identity,
        })
    }
}
