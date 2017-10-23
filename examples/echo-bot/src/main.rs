extern crate beryllium;
extern crate chrono;
extern crate env_logger;
extern crate log;

use beryllium::{BotClient, BotService, Handler, EventData, Event};
use chrono::offset::Utc;
use env_logger::LogBuilder;
use log::{LogRecord, LogLevelFilter};
use std::env;
use std::fs;
use std::path::PathBuf;

pub struct EchoServer;

impl Handler for EchoServer {
    fn handle(&self, data: EventData, client: BotClient) {
        match data.event {
            Event::Message { ref text, ref from } => {
                println!("{} received message from {}", data.bot_id, from);
                client.send_message(&format!("{} said: {}", from, text));
            },
            Event::ConversationMemberJoin { ref members_joined } => {
                println!("Members joined: {:?}", members_joined);
            },
            Event::ConversationMemberLeave { ref members_left } => {
                println!("Members left: {:?}", members_left);
            },
            Event::ConversationRename => {
                println!("Conversation has been renamed to {}",
                         data.conversation.name);
            },
            _ => (),
        }
    }
}

macro_rules! get_env {
    ($var:expr, $default:expr) => {
        match env::var($var) {
            Ok(val) => {
                println!("Found {}={} in env", $var, val);
                val
            },
            Err(_) => {
                println!("Cannot find {}, using default {}", $var, $default);
                String::from($default)
            },
        }
    };
}

fn main() {
    let mut builder = LogBuilder::new();
    builder.format(|record: &LogRecord| format!("{:?}: {}: {}", Utc::now(), record.level(), record.args()))
           .filter(None, LogLevelFilter::Info);
    if let Ok(v) = env::var("RUST_LOG") {
       builder.parse(&v);
    }

    builder.init().unwrap();

    let data_path = get_env!("DATA_DIR", "./bot_data");
    let addr = get_env!("ADDRESS", "0.0.0.0:6000").parse().unwrap();;
    let key = get_env!("KEY_PATH", "key.pem");
    let cert = get_env!("CERT_PATH", "cert.pem");
    let auth = get_env!("AUTH", "0xdeadbeef");

    if !PathBuf::from(&data_path).exists() {
        fs::create_dir(&data_path).unwrap();
    }

    let service = BotService::new(auth, &data_path, &key, &cert).unwrap();
    service.start_listening(&addr, EchoServer);
}
