use {base64, utils};
use errors::{BerylliumError, BerylliumResult};
use futures::{Future, Sink, future};
use futures::sync::mpsc::Sender as FutureSender;
use hyper::{Body, Method, Request, StatusCode};
use hyper::header::{Authorization, Bearer, ContentLength, ContentType, Headers};
use image::ImageFormat;
use messages_proto::{Confirmation, GenericMessage, Text};
use messages_proto::Confirmation_Type as ConfirmationType;
use mime::Mime;
use parking_lot::Mutex;
use protobuf::Message;
use serde::Serialize;
use serde_json::{self, Value as SerdeValue};
use std::collections::HashMap;
use std::io::{BufRead, Seek};
use std::mem;
use std::sync::Arc;
use storage::StorageManager;
use types::{BerylliumFuture, BotCreationData, Devices, DevicePreKeys};
use types::{EventLoopRequest, HyperClient, MessageRequest, MessageStatus};
use types::AssetData;
use utils::MultipartWriter;
use uuid::Uuid;

const HOST_ADDRESS: &'static str = "https://prod-nginz-https.wire.com";
const MULTIPART_BOUNDARY: &'static str = "frontier";
lazy_static! {
    static ref MULTIPART_MIXED: Mime = {
        let mime_str = format!("multipart/mixed; boundary={}", MULTIPART_BOUNDARY);
        mime_str.parse().unwrap()
    };
}

header! {
    (ContentMd5, "Content-MD5") => [String]     // base64-encoded MD5 hash digest
}

/// Private client to isolate some methods.
#[derive(Clone)]
pub struct HttpsClient {
    client_id: String,
    auth_token: String,
}

impl HttpsClient {
    fn prepare_request_for_url(&self, method: Method, rel_url: &str) -> Request {
        let url = format!("{}{}", HOST_ADDRESS, rel_url);
        info!("{}: {}", method, url);
        let mut request = Request::new(method, url.parse().unwrap());
        request.headers_mut().set(Authorization(Bearer {
            token: self.auth_token.to_owned(),
        }));

        request
    }

    fn request_with_request(client: &HyperClient, request: Request)
                           -> BerylliumFuture<(StatusCode, Headers, Body)>
    {
        let f = client.request(request).and_then(|mut resp| {
            let code = resp.status();
            debug!("Got {} response", code);
            let hdrs = mem::replace(resp.headers_mut(), Headers::new());
            future::ok((code, hdrs, resp.body()))
        }).map_err(BerylliumError::from);

        Box::new(f)
    }

    /// Generic request builder for all API requests.
    fn request<T>(&self, client: &HyperClient, method: Method,
                  rel_url: &str, data: Option<T>)
                 -> BerylliumFuture<(StatusCode, Headers, Body)>
        where T: Serialize
    {
        let mut request = self.prepare_request_for_url(method, rel_url);
        request.headers_mut().set(ContentType::json());

        if let Some(object) = data {
            let res = serde_json::to_vec(&object).map(|bytes| {   // FIXME: Error?
                debug!("Setting JSON payload");
                request.set_body::<Vec<u8>>(bytes.into());
            });

            future_try!(res);
        }

        HttpsClient::request_with_request(client, request)
    }

    /// Send raw message. This is usually called by `send_encrypted_message`
    fn send_message<T>(&self, client: &HyperClient,
                       data: T, ignore_missing: bool)
                      -> BerylliumFuture<MessageStatus>
        where T: Serialize
    {
        info!("Sending message...");
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

    /// Get the device prekeys.
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

    /// Used to send all messages encrypted with the appropriate device prekeys.
    pub fn send_encrypted_message(&self, client: &HyperClient,
                                  data: &GenericMessage,
                                  storage: Arc<StorageManager>,
                                  devices: Arc<Mutex<Devices>>)
        -> BerylliumFuture<()>
    {
        let bytes = future_try!(data.write_to_bytes());
        let mut devices_clone = {
            let devs = devices.lock();
            devs.missing.clone()    // clone and release the lock
        };

        let f = {
            let encrypted = storage.encrypt_for_devices(&bytes, &devices_clone);
            let msg = MessageRequest {
                sender: &self.client_id,
                recipients: encrypted,
            };

            self.send_message(&client, msg, false)
        };

        let bot_client = self.clone();
        let hyper_client = client.clone();

        let f = f.and_then(move |stat| match stat {
            MessageStatus::Sent =>
                Box::new(future::ok(())) as BerylliumFuture<()>,
            MessageStatus::Failed(devs) => {
                info!("Getting prekeys for missing devices...");
                let f = bot_client.get_prekeys(&hyper_client, &devs.missing);
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
                        sender: &bot_client.client_id,
                        recipients: new_data,
                    };

                    let f = bot_client.send_message(&hyper_client, message, false);
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

    /// Send confirmation message that we've received a message from conversation.
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

    /// Upload a given asset to Wire servers and return the asset key and token.
    fn upload_asset(&self, client: &HyperClient, data: &[u8])
                   -> BerylliumFuture<AssetData> {
        let mut request = self.prepare_request_for_url(Method::Post, "/bots/assets");
        request.headers_mut().set(ContentType(MULTIPART_MIXED.clone()));
        request.headers_mut().set(ContentLength(data.len() as u64));

        let f = HttpsClient::request_with_request(client, request)
                            .and_then(|(code, headers, body)| {
            utils::acquire_body_with_err(&headers, body).and_then(move |vec| {
                if code.is_success() {
                    info!("Successfully uploaded asset.");
                    let res = serde_json::from_slice::<AssetData>(&vec)
                                         .map_err(BerylliumError::from);
                    future::result(res)
                } else {
                    let res = serde_json::from_slice::<SerdeValue>(&vec)
                                         .map_err(BerylliumError::from);
                    let msg = format!("Error uploading asset. Response: {:?}", res);
                    future::err(BerylliumError::Other(msg))
                }
            })
        });

        Box::new(f)
    }
}

impl<'a> From<&'a BotCreationData> for HttpsClient {
    fn from(data: &'a BotCreationData) -> HttpsClient {
        HttpsClient {
            auth_token: data.token.to_owned(),
            client_id: data.client.clone(),
        }
    }
}

pub struct BotData {
    pub storage: Arc<StorageManager>,
    pub data: BotCreationData,
    pub client: HttpsClient,
    /// `Arc<Mutex<T>>` because it's shared with the global bot data.
    /// Whenever we get new devices, we'll update this.
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

#[derive(Clone)]
/// User client to execute bot actions.
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
    /// Send a user text message to the conversation associated with the bot instance.
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

    /// Send an user image to the associated conversation.
    pub fn send_image<R>(&self, data: &[u8], fmt: ImageFormat)
        where R: BufRead + Seek
    {
        //
    }
}
