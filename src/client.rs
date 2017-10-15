use errors::{BerylliumError, BerylliumResult};
use hyper::Method;
use hyper::header::{Authorization, Bearer, ContentType, Headers};
use messages_proto::{Confirmation, GenericMessage};
use messages_proto::Confirmation_Type as ConfirmationType;
use parking_lot::Mutex;
use protobuf::Message;
use reqwest::{Response, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::Arc;
use storage::StorageManager;
use types::{BotCreationData, Devices, MessageRequest};
use utils::HYPER_CLIENT;

const HOST_ADDRESS: &'static str = "https://prod-nginz-https.wire.com";

#[derive(Clone)]
pub struct HttpsClient {
    bot_id: String,
    client: String,
    auth_token: String,
}

impl HttpsClient {
    fn request<T>(&self, method: Method, rel_url: &str,
                  data: Option<T>,
                  additional_headers: Option<Headers>)
                  -> BerylliumResult<Response>
        where T: Serialize
    {
        let url = format!("{}{}", HOST_ADDRESS, rel_url);
        let mut request = HYPER_CLIENT.request(method, &url);
        let mut headers = Headers::new();
        headers.set(ContentType::json());
        headers.set(Authorization(Bearer {
            token: self.auth_token.to_owned(),
        }));

        request.headers(headers);
        if let Some(h) = additional_headers {
            request.headers(h);
        }

        if let Some(object) = data {
            request.json(&object);
        }

        request.send().map_err(BerylliumError::from)
    }

    pub fn send_message<T, R>(&self, data: T, ignore_missing: bool)
                             -> BerylliumResult<(R, StatusCode)>
        where T: Serialize, R: DeserializeOwned
    {
        let url = format!("/bot/messages?ignore_missing={}", ignore_missing);
        let mut resp = self.request(Method::Post, &url, Some(data), None)?;
        let json = resp.json()?;
        Ok((json, resp.status()))
    }

    pub fn send_encrypted_message(&self, data: &GenericMessage,
                                  devices: &Mutex<Devices>)
                                 -> BerylliumResult<StatusCode>
    {
        let bytes = data.write_to_bytes()?;
        let mut message = MessageRequest {
            sender: &self.client,
            recipients: HashMap::new(),
        };

        unimplemented!();
    }
}

impl<'a> From<&'a BotCreationData> for HttpsClient {
    fn from(data: &'a BotCreationData) -> HttpsClient {
        HttpsClient {
            auth_token: data.token.to_owned(),
            bot_id: data.id.clone(),
            client: data.client.clone(),
        }
    }
}

pub struct BotData {
    // Arc'd stuff will be shared with the HttpsClient for sending encrypted messages.
    // Mutex'ed stuff will be shared with the global handler itself.
    pub storage: Arc<StorageManager>,
    pub data: BotCreationData,
    pub client: HttpsClient,
    pub devices: Arc<Mutex<Devices>>,
}

impl BotData {
    pub fn from_storage(bot_id: &str) -> BerylliumResult<BotData> {
        let storage = StorageManager::new(&bot_id)?;
        let store_data: BotCreationData = storage.load_state()?;
        Ok(BotData {
            storage: Arc::new(storage),
            client: HttpsClient::from(&store_data),
            data: store_data,
            devices: Arc::new(Mutex::new(Devices::default())),
        })
    }
}

// Another client for isolating internal methods from user methods.
#[derive(Clone)]
pub struct BotClient {
    inner: HttpsClient,
    sender: String,
    storage: Arc<StorageManager>,
}

impl<'a> From<&'a BotData> for BotClient {
    fn from(data: &'a BotData) -> BotClient {
        BotClient {
            inner: data.client.clone(),
            storage: data.storage.clone(),
            sender: data.data.client.clone(),
        }
    }
}
