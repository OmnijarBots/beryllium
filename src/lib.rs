extern crate base64;
extern crate cryptobox;
extern crate futures;
extern crate futures_cpupool;
extern crate hyper;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
extern crate parking_lot;
extern crate proteus;
extern crate protobuf;
extern crate reqwest;
extern crate rustls;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate tokio_rustls;
extern crate tokio_proto;
// TODO: Remove this once this has been merged into nursery
extern crate uuid_v1;

mod client;
mod errors;
mod handlers;
mod service;
mod storage;
mod types;
mod utils;

include!(concat!(env!("OUT_DIR"), "/messages.rs"));

pub use client::BotClient;
pub use handlers::Handler;
pub use service::BotService;
pub use types::{Event, EventData};
