use errors::{BerylliumError, BerylliumResult};
use futures::{Future, Stream};
use futures::sync::mpsc as futures_mpsc;
use handlers::{BotHandler, Handler};
use hyper::Client;
use hyper::server::Http;
use hyper_rustls::HttpsConnector;
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls::internal::pemfile;
use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use tokio_core::reactor::Core;
use tokio_rustls::proto::Server;
use tokio_proto::TcpServer;
use types::EventLoopRequest;
use utils;

pub struct BotService {
    config: ServerConfig,
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

    /// Initialize the service with the given auth token (obtained from devbot),
    /// storage, private key and certificate paths.
    ///
    /// Note that the certificate and the (RSA) private key should be
    /// in PEM format.
    pub fn new<P>(auth: String, store_path: P, key_path: P, cert_path: P)
                  -> BerylliumResult<BotService>
        where P: AsRef<Path>
    {
        let certs = Self::load_certs(cert_path)?;
        let key = Self::load_private_key(key_path)?;
        let mut tls_config = ServerConfig::new();
        tls_config.set_single_cert(certs, key);
        utils::set_auth_token(auth);
        utils::set_store_path(store_path);

        Ok(BotService {
            config: tls_config,
        })
    }

    /// Start listening for incoming HTTPS requests (indefinitely), and forward
    /// events to the associated handler.
    ///
    /// This has two threads - one for serving HTTPS requests from
    /// the Wire server, and another for running an event loop
    /// which makes HTTPS requests to the Wire server.
    pub fn start_listening<H>(self, addr: &SocketAddr, handler: H)
        where H: Handler
    {
        let https_server = Server::new(Http::new(), Arc::new(self.config));
        let tcp_server = TcpServer::new(https_server, addr.clone());
        let (tx, rx) = futures_mpsc::channel(0);
        let handler = Arc::new(handler);

        let _ = thread::spawn(move || {
            let mut core = Core::new().expect("event loop creation");
            let handle = core.handle();
            let https = HttpsConnector::new(4, &handle);
            let client = Client::configure().connector(https).build(&handle);
            info!("Created listener queue for requests!");

            let listen_messages = rx.for_each(|call: EventLoopRequest<()>| {
                call(&client).map_err(|e| {
                    info!("Error resolving closure: {}", e);
                })
            });

            core.run(listen_messages).expect("running event loop");
        });

        tcp_server.serve(move || {
            Ok(BotHandler::new(handler.clone(), tx.clone()))
        });
    }
}
