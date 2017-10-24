use {base64, utils};
use errors::{BerylliumError, BerylliumResult};
use futures::{Future, Sink, future};
use futures::sync::mpsc::Sender as FutureSender;
use hyper::{Body, Method, Request, StatusCode};
use hyper::header::{Authorization, Bearer, ContentType, Headers};
use messages_proto::{Confirmation, GenericMessage, Text};
use messages_proto::Confirmation_Type as ConfirmationType;
use parking_lot::Mutex;
use protobuf::Message;
use serde::Serialize;
use serde_json::{self, Value as SerdeValue};
use std::collections::HashMap;
use std::mem;
use std::sync::Arc;
use storage::StorageManager;
use types::{BerylliumFuture, BotCreationData, Devices, DevicePreKeys};
use types::{EventLoopRequest, HyperClient, MessageRequest, MessageStatus};
use uuid::Uuid;

const HOST_ADDRESS: &'static str = "https://prod-nginz-https.wire.com";

#[derive(Clone)]
pub struct HttpsClient {
    client: String,
    auth_token: String,
}

// FIXME: Figure out how we can eliminate/improve the usage of tokio-core event loop
// (which is here only for the sake of establishing TLS connection)

impl HttpsClient {
    fn request<T>(&self, client: &HyperClient, method: Method, rel_url: &str, data: Option<T>)
                 -> BerylliumFuture<(StatusCode, Headers, Body)>
        where T: Serialize
    {
        let url = format!("{}{}", HOST_ADDRESS, rel_url);
        info!("{}: {}", method, url);
        let mut request = Request::new(method, url.parse().unwrap());
        request.headers_mut().set(ContentType::json());
        request.headers_mut().set(Authorization(Bearer {
            token: self.auth_token.to_owned(),
        }));

        if let Some(object) = data {
            let res = serde_json::to_vec(&object).map(|bytes| {   // FIXME: Error?
                debug!("Setting JSON payload");
                request.set_body::<Vec<u8>>(bytes.into());
            });

            future_try!(res);
        }

        let f = client.request(request).and_then(|mut resp| {
            let code = resp.status();
            let hdrs = mem::replace(resp.headers_mut(), Headers::new());
            future::ok((code, hdrs, resp.body()))
        }).map_err(BerylliumError::from);

        Box::new(f)
    }

    fn send_message<T>(&self, client: &HyperClient,
                       data: T, ignore_missing: bool)
                      -> BerylliumFuture<MessageStatus>
        where T: Serialize
    {
        let url = format!("/bot/messages?ignore_missing={}", ignore_missing);
        let f = self.request(client, Method::Post, &url, Some(data));
        let f = f.and_then(|(code, headers, body)| {
            utils::acquire_body_with_err(&headers, body).and_then(move |vec| {
                // This happens only when we haven't sent the encrypted message
                // for all the devices in the conversation (i.e., we don't have all the devices).
                if code == StatusCode::PreconditionFailed {
                    info!("It looks like we're missing devices.");
                    let res = serde_json::from_slice::<Devices>(&vec)
                                         .map(MessageStatus::Failed)
                                         .map_err(BerylliumError::from);
                    future::result(res)
                } else if code.is_success() {
                    info!("Successfully sent the message.");
                    future::ok(MessageStatus::Sent)
                } else {
                    let res = serde_json::from_slice::<SerdeValue>(&vec)
                                         .map_err(BerylliumError::from);
                    let msg = format!("Error sending message. Response: {:?}", res);
                    future::err(BerylliumError::Other(msg))
                }
            })
        });

        Box::new(f)
    }

    fn get_prekeys<T>(&self, client: &HyperClient, data: T)
                     -> BerylliumFuture<DevicePreKeys>
        where T: Serialize
    {
        let f = self.request(client, Method::Post, "/bot/users/prekeys", Some(data));
        let f = f.and_then(|(code, headers, body)| {
            utils::acquire_body_with_err(&headers, body).and_then(move |vec| {
                if code == StatusCode::Ok {
                    info!("Successfully obtained prekeys for missing devices");
                    let res = serde_json::from_slice::<DevicePreKeys>(&vec)
                                         .map_err(BerylliumError::from);
                    future::result(res)
                } else {
                    let res = serde_json::from_slice::<SerdeValue>(&vec)
                                         .map_err(BerylliumError::from);
                    let msg = format!("Cannot obtain prekeys for missing devices. Response: {:?}", res);
                    future::err(BerylliumError::Other(msg))
                }
            })
        });

        Box::new(f)
    }

    pub fn send_encrypted_message(&self, client: &HyperClient,
                                  data: &GenericMessage,
                                  storage: Arc<StorageManager>,
                                  devices: Arc<Mutex<Devices>>)
        -> BerylliumFuture<()>
    {
        let bytes = future_try!(data.write_to_bytes());
        let devices_clone = {
            let devs = devices.lock();
            devs.missing.clone()    // clone and release the lock
        };

        let message = {
            let encrypted = storage.encrypt_for_devices(&bytes, &devices_clone);
            MessageRequest {
                sender: &self.client,
                recipients: encrypted,
            }
        };

        let client_clone = self.clone();
        let client = client.clone();
        let mut devices_clone = devices_clone.clone();      // BAAAHHH!!!

        info!("Sending encrypted message...");
        let f = self.send_message(&client, message, false);
        let f = f.and_then(move |stat| match stat {
            MessageStatus::Sent =>
                Box::new(future::ok(())) as BerylliumFuture<()>,
            MessageStatus::Failed(devs) => {
                info!("Getting prekeys for missing devices...");
                let f = client_clone.get_prekeys(&client, &devs.missing);
                let f = f.and_then(move |keys| {
                    let mut new_data = HashMap::with_capacity(keys.len());
                    for (user_id, clients) in &keys {
                        for (client_id, prekey) in clients {
                            let prekey = future_try_box!(base64::decode(&prekey.key));
                            let clients = new_data.entry(user_id.as_str())
                                                  .or_insert(HashMap::new());
                            let res = storage.encrypt(user_id.as_str(), client_id,
                                                      &bytes, &prekey);
                            let encrypted = future_try_box!(res);
                            clients.entry(client_id.as_str()).or_insert(encrypted);

                            // We've successfully encrypted the message for a new device
                            // with a new prekey. Since we've already stored the session,
                            // we can safely update our devices.
                            let clients = devices_clone.entry(user_id.clone())
                                                       .or_insert(vec![]);
                            clients.push(client_id.to_owned());
                        }
                    }

                    devices.lock().missing = devices_clone;
                    let message = MessageRequest {
                        sender: &client_clone.client,
                        recipients: new_data,
                    };

                    let f = client_clone.send_message(&client, message, false);
                    let f = f.and_then(move |stat| {
                        match stat {
                            MessageStatus::Sent => future::ok(()),
                            MessageStatus::Failed(_) => {
                                let msg = "Cannot send message! Failed after device check";
                                future::err(BerylliumError::Other(String::from(msg)))
                            },
                        }
                    });

                    Box::new(f) as BerylliumFuture<()>
                });

                Box::new(f)
            },
        });

        Box::new(f)
    }

    pub fn send_confirmation(&self, client: &HyperClient,
                             message_id: &str,
                             storage: Arc<StorageManager>,
                             devices: Arc<Mutex<Devices>>)
        -> BerylliumFuture<()>
    {
        info!("Sending confirmation message...");
        let mut message = GenericMessage::new();
        let uuid = utils::uuid_v1();
        message.set_message_id(uuid.to_string());
        let mut confirmation = Confirmation::new();
        confirmation.set_message_id(message_id.to_owned());
        confirmation.set_field_type(ConfirmationType::DELIVERED);
        message.set_confirmation(confirmation);
        self.send_encrypted_message(client, &message, storage, devices)
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
    // Arc'd stuff will be shared with the `BotClient` for sending encrypted messages.
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
    event_loop_sender: FutureSender<EventLoopRequest<()>>,
}

impl<'a> From<(&'a BotData, &'a FutureSender<EventLoopRequest<()>>)> for BotClient {
    fn from(data: (&'a BotData, &'a FutureSender<EventLoopRequest<()>>)) -> BotClient {
        BotClient {
            inner: data.0.client.clone(),
            storage: data.0.storage.clone(),
            sender: data.0.data.client.clone(),
            devices: data.0.devices.clone(),
            event_loop_sender: data.1.clone(),
        }
    }
}

impl BotClient {
    pub fn send_message(&self, text: &str) {
        let text = text.to_owned();
        let (client, storage, devices) =
            (self.inner.clone(), self.storage.clone(), self.devices.clone());

        let call_closure = Box::new(move |c: &HyperClient| {
            let mut message = GenericMessage::new();
            let uuid = utils::uuid_v1();
            message.set_message_id(uuid.to_string());
            let mut txt = Text::new();
            txt.set_content(text.clone());
            message.set_text(txt);
            client.send_encrypted_message(c, &message, storage.clone(), devices.clone())
        });

        self.event_loop_sender.clone().send(call_closure).wait().map_err(|e| {
            error!("Cannot queue user message in event loop: {}", e);
        }).ok();
    }
}
