extern crate cryptobox;
#[macro_use] extern crate log;
extern crate proteus;
extern crate protobuf;

mod errors;
mod otr_manager;
mod utils;

include!(concat!(env!("OUT_DIR"), "/messages.rs"));
