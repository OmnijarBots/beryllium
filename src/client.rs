use {base64, utils};
use errors::{BerylliumError, BerylliumResult};
use hyper::Method;
use hyper::header::{Authorization, Bearer, ContentType, Headers};
use messages_proto::{Confirmation, GenericMessage, Text};
use messages_proto::Confirmation_Type as ConfirmationType;
use parking_lot::Mutex;
use protobuf::Message;
use reqwest::{Response, StatusCode};
use serde::Serialize;
use serde_json::Value as SerdeValue;
use std::collections::HashMap;
use std::sync::Arc;
use storage::StorageManager;
use types::{BotCreationData, Devices, DevicePreKeys, MessageRequest};
use utils::HYPER_CLIENT;
use uuid::Uuid;

const HOST_ADDRESS: &'static str = "https://prod-nginz-https.wire.com";

#[derive(Clone)]
pub struct HttpsClient {
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

    pub fn send_message<T>(&self, data: T, ignore_missing: bool)
                          -> BerylliumResult<Response>
        where T: Serialize
    {
        let url = format!("/bot/messages?ignore_missing={}", ignore_missing);
        let resp = self.request(Method::Post, &url, Some(data), None)?;
        Ok(resp)
    }

    pub fn get_prekeys<T>(&self, data: T) -> BerylliumResult<DevicePreKeys>
        where T: Serialize
    {
        let mut resp = self.request(Method::Post,
                                    "/bot/users/prekeys",
                                    Some(data), None)?;
        if resp.status() != StatusCode::Ok {
            let json = resp.json::<SerdeValue>()?;
            let msg = format!("Cannot obtain prekeys for missing devices! (Response: {:?})", json);
            Err(BerylliumError::Other(msg))
        } else {
            Ok(resp.json()?)
        }
    }

    pub fn send_encrypted_message(&self, data: &GenericMessage, storage: &StorageManager,
                                  devices: &Mutex<Devices>) -> BerylliumResult<()> {
        let bytes = data.write_to_bytes()?;
        let mut devices_clone = {
            let devs = devices.lock();
            devs.missing.clone()    // We clone and release the lock
        };

        let mut resp = {
            let encrypted = storage.encrypt_for_devices(&bytes, &devices_clone);
            let message = MessageRequest {
                sender: &self.client,
                recipients: encrypted,
            };

            self.send_message(&message, false)?
        };

        let status = resp.status();
        // This happens only when we haven't sent the encrypted message
        // for all the devices in the conversation (i.e., we don't have all the devices).
        if status == StatusCode::PreconditionFailed {
            let resp_devices: Devices = resp.json()?;
            let prekeys = self.get_prekeys(&resp_devices.missing)?;
            let mut new_data = HashMap::with_capacity(resp_devices.missing.len());

            for (user_id, clients) in &prekeys {
                for (client_id, prekey) in clients {
                    let prekey = base64::decode(&prekey.key)?;
                    let clients = new_data.entry(user_id.as_str()).or_insert(HashMap::new());
                    let encrypted = storage.encrypt(user_id.as_str(), client_id, &bytes, &prekey)?;
                    clients.entry(client_id.as_str()).or_insert(encrypted);

                    // We've successfully encrypted the message for a new device with a new prekey.
                    // Since we've already stored the session, we can safely update our devices.
                    let clients = devices_clone.entry(user_id.clone()).or_insert(vec![]);
                    clients.push(client_id.to_owned());
                }
            }

            let message = MessageRequest {
                sender: &self.client,
                recipients: new_data,
            };

            let mut resp = self.send_message(&message, false)?;
            if !resp.status().is_success() {
                let json = resp.json::<SerdeValue>();
                let msg = format!("Got failure response in device-check: {:?}", json);
                return Err(BerylliumError::Other(msg))
            }
        } else if !status.is_success() {
            let json = resp.json::<SerdeValue>();
            let msg = format!("Unexpected response: {:?}", json);
            return Err(BerylliumError::Other(msg))
        }

        Ok(())
    }

    pub fn send_confirmation(&self, message_id: &str, storage: &StorageManager,
                             devices: &Mutex<Devices>) -> BerylliumResult<()> {
        let mut message = GenericMessage::new();
        let uuid = utils::uuid_v1();
        message.set_message_id(uuid.to_string());
        let mut confirmation = Confirmation::new();
        confirmation.set_message_id(message_id.to_owned());
        confirmation.set_field_type(ConfirmationType::DELIVERED);
        message.set_confirmation(confirmation);
        self.send_encrypted_message(&message, storage, devices)
    }
}

impl<'a> From<&'a BotCreationData> for HttpsClient {
    fn from(data: &'a BotCreationData) -> HttpsClient {
        HttpsClient {
            auth_token: data.token.to_owned(),
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
    pub fn from_storage(bot_id: Uuid) -> BerylliumResult<BotData> {
        let storage = StorageManager::new(bot_id)?;
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
    devices: Arc<Mutex<Devices>>,
}

impl<'a> From<&'a BotData> for BotClient {
    fn from(data: &'a BotData) -> BotClient {
        BotClient {
            inner: data.client.clone(),
            storage: data.storage.clone(),
            sender: data.data.client.clone(),
            devices: data.devices.clone(),
        }
    }
}

impl BotClient {
    pub fn send_message(&self, text: &str) -> BerylliumResult<()> {
        let mut message = GenericMessage::new();
        let uuid = utils::uuid_v1();
        message.set_message_id(uuid.to_string());
        let mut txt = Text::new();
        txt.set_content(text.to_owned());
        message.set_text(txt);
        self.inner.send_encrypted_message(&message, &self.storage, &self.devices)
    }
}
