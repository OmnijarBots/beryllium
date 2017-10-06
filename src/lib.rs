extern crate cryptobox;
extern crate futures;
extern crate hyper;
#[macro_use] extern crate log;
extern crate proteus;
extern crate protobuf;
extern crate rustls;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate tokio_rustls;
extern crate tokio_proto;

mod errors;
mod handlers;
mod otr_manager;
mod service;

include!(concat!(env!("OUT_DIR"), "/messages.rs"));

pub use service::BotService;
