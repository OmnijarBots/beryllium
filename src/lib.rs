extern crate base64;
extern crate cryptobox;
extern crate futures;
extern crate futures_cpupool;
extern crate hyper;
extern crate hyper_rustls;
extern crate image;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
extern crate openssl;
extern crate parking_lot;
extern crate proteus;
extern crate protobuf;
extern crate rustls;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate sha2;
extern crate tokio_core;
extern crate tokio_rustls;
extern crate tokio_proto;
extern crate uuid;
extern crate uuid_v1;

#[macro_use] mod utils;
mod client;
mod handlers;
mod service;
mod storage;
mod types;

include!(concat!(env!("OUT_DIR"), "/messages.rs"));

pub mod errors;

pub use client::BotClient;
pub use handlers::Handler;
pub use service::BotService;
pub use types::{Event, EventData};
