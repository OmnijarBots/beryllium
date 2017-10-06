use errors::{BerylliumError, BerylliumResult};
use hyper::server::Http;
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls::internal::pemfile;
use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use tokio_rustls::proto::Server;
use tokio_proto::TcpServer;

pub struct BotHandler {
    path: Rc<PathBuf>,
    auth: String,
}

impl BotHandler {
    pub fn path(&self) -> Rc<PathBuf> {
        self.path.clone()
    }

    pub fn check_auth(&self, auth: &str) -> bool {
        self.auth == auth
    }
}

pub struct BotService {
    path: PathBuf,
    auth: String,
    config: Arc<ServerConfig>,
}

impl BotService {
    fn load_certs<P>(path: P) -> BerylliumResult<Vec<Certificate>>
        where P: AsRef<Path>
    {
        info!("Loading certificate from {}", path.as_ref().display());
        let cert = File::open(path)?;
        let mut reader = BufReader::new(cert);
        pemfile::certs(&mut reader).map_err(|_| BerylliumError::PemFileError)
    }

    fn load_private_key<P>(path: P) -> BerylliumResult<PrivateKey>
        where P: AsRef<Path>
    {
        info!("Loading private key from {}", path.as_ref().display());
        let key = File::open(path)?;
        let mut reader = BufReader::new(key);
        let mut keys = pemfile::rsa_private_keys(&mut reader)
                               .map_err(|_| BerylliumError::PemFileError)?;
        keys.truncate(1);
        if keys.is_empty() {
            return Err(BerylliumError::PemFileError)
        }

        Ok(keys.pop().unwrap())
    }

    pub fn new<P>(auth: String, store_path: P, key_path: P, cert_path: P)
                  -> BerylliumResult<BotService>
        where P: AsRef<Path>
    {
        let certs = Self::load_certs(cert_path)?;
        let key = Self::load_private_key(key_path)?;
        let mut tls_config = ServerConfig::new();
        tls_config.set_single_cert(certs, key);

        Ok(BotService {
            path: PathBuf::from(store_path.as_ref()),
            auth: auth,
            config: Arc::new(tls_config),
        })
    }

    pub fn start_listening(self, addr: &SocketAddr) {
        let https_server = Server::new(Http::new(), self.config.clone());
        let tcp_server = TcpServer::new(https_server, addr.clone());
        tcp_server.serve(move || Ok(BotHandler {
            path: Rc::new(self.path.clone()),
            auth: self.auth.clone(),
        }))
    }
}
