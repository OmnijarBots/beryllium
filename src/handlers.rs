use {protobuf, utils};
use client::{BotClient, BotData};
use errors::{BerylliumError, BerylliumResult};
use futures::{Future, Sink, future};
use futures::sync::mpsc::Sender as FutureSender;
use futures_cpupool::{Builder, CpuFuture, CpuPool};
use hyper::{Body, Error as HyperError, Headers, Method, StatusCode};
use hyper::header::{Authorization, Bearer};
use hyper::server::{Service, Request, Response};
use messages_proto::GenericMessage;
use parking_lot::Mutex;
use serde_json::{self, Value as SerdeValue};
use storage::StorageManager;
use std::collections::HashMap;
use std::sync::Arc;
use types::{BotCreationData, BotCreationResponse, Event, EventData};
use types::{ConversationData, ConversationEventType, MessageData, Member};
use types::{HyperClient, EventLoopRequest};
use uuid::Uuid;

// TODO:
// - Isolate events into their own functions.
// - Revisit `clone` usage on various types.
// - Revisit usage of Arc's, Mutex'es and event loop handles.

pub trait Handler: Send + Sync + 'static {
    fn handle(&self, data: EventData, client: BotClient);
}

// FIXME: I *know* that Arc has an overhead, but I'm not entirely
// sure about the performance impact of this in our case (i.e., HTTP requests)
pub struct BotHandler<H> {
    handler: Arc<H>,
    pool: Arc<CpuPool>,
    bot_data: Arc<Mutex<HashMap<Uuid, Arc<Mutex<BotData>>>>>,
    event_loop_sender: FutureSender<EventLoopRequest<()>>,
}

impl<H: Handler> BotHandler<H> {
    pub fn new(handler: Arc<H>, sender: FutureSender<EventLoopRequest<()>>) -> BotHandler<H> {
        BotHandler {
            handler: handler,
            pool: Arc::new(Builder::new().create()),
            bot_data: Arc::new(Mutex::new(HashMap::new())),
            event_loop_sender: sender,
        }
    }
}

impl<H: Handler> Service for BotHandler<H> {
    type Request = Request;
    type Response = Response;
    type Error = HyperError;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let mut resp = Response::new();
        let (method, uri, _version, headers, body) = req.deconstruct();

        if method != Method::Post {     // only allow POST
            info!("Disallowed method: {}", method);
            resp.set_status(StatusCode::MethodNotAllowed);
            return Box::new(future::ok(resp))
        } else {        // all requests should have Bearer token auth
            match headers.get::<Authorization<Bearer>>() {
                Some(header) if utils::check_auth_token(&header.to_string()[7..]) => (),
                _ => {
                    resp.set_status(StatusCode::Unauthorized);
                    info!("Unauthorized request!");
                    return Box::new(future::ok(resp))
                }
            }
        }

        macro_rules! parse_json_and {
            ($call:expr $( , $arg:expr )*) => {{
                let f = utils::acquire_body(&headers, body).map(|vec| {
                    if let Ok(value) = serde_json::from_slice(&vec) {
                        if let Err(e) = $call($( $arg, )* value, &mut resp) {
                            error!("{}", e);
                            resp.set_status(StatusCode::InternalServerError);
                        }
                    } else {
                        info!("Cannot parse JSON object!");
                        resp.set_status(StatusCode::BadRequest);
                    }

                    info!("Responded with {}", resp.status());
                    resp
                });

                Box::new(f)
            }};
        }

        let rel_url = uri.path();
        info!("Incoming authorized request (path: {})", rel_url);
        let mut split = rel_url.trim_matches('/').split('/');

        // FIXME: Better way to detect relative URL paths?
        match (split.next(), split.next(), split.next(), split.next()) {
            (Some("bots"), None, None, None) => parse_json_and!(create_bot),
            (Some("bots"), Some(id), Some("messages"), None) => {
                let pool = self.pool.clone();
                let handler = self.handler.clone();
                let bot_id = String::from(id);
                let bot_data = self.bot_data.clone();
                let sender = self.event_loop_sender.clone();
                parse_json_and!(handle_events, pool, sender,
                                bot_data, bot_id, handler)
            },
            _ => parse_json_and!(empty_response, headers),
        }
    }
}

fn empty_response(headers: Headers, data: SerdeValue,
                  resp: &mut Response) -> BerylliumResult<()> {
    info!("Unknown endpoint.\n[Headers]\n{}\nData: {}\n", headers, data);
    resp.set_status(StatusCode::NotFound);
    Ok(())
}

fn create_bot(data: BotCreationData, resp: &mut Response) -> BerylliumResult<()> {
    info!("Creating new bot instance...");
    let storage = StorageManager::new(data.id)?;
    let mut prekeys = storage.initialize_prekeys(data.conversation.members.len())?;
    // There will always be a final prekey corresponding to u16::MAX
    let final_key = prekeys.pop().unwrap();
    storage.save_state(&data)?;

    let data = BotCreationResponse {
        prekeys: prekeys,
        last_prekey: final_key,
    };

    let bytes = serde_json::to_vec(&data)?;
    resp.set_body(Body::from(bytes));
    resp.set_status(StatusCode::Created);
    Ok(())
}

fn handle_events<H>(pool: Arc<CpuPool>, job_sender: FutureSender<EventLoopRequest<()>>,
                    bot_data: Arc<Mutex<HashMap<Uuid, Arc<Mutex<BotData>>>>>,
                    bot_id: String, handler: Arc<H>,
                    data: MessageData, resp: &mut Response)
                   -> BerylliumResult<()>
    where H: Handler
{
    let bot_id = bot_id.parse::<Uuid>()?;
    info!("Preparing to handle conversation event...");
    // NOTE: parking_lot's Mutex is suitable for fine-grained locks, so we
    // acquire and release the lock quite a lot of times below.

    // Maybe we've rebooted our bot and we don't have the creation data in memory.
    let this_bot_data = if bot_data.lock().get(&bot_id).is_none() {
        let this_bot_data = BotData::from_storage(bot_id)?;
        let this_bot_data = Arc::new(Mutex::new(this_bot_data));
        bot_data.lock().insert(bot_id, this_bot_data.clone());
        this_bot_data
    } else {
        bot_data.lock().get(&bot_id).unwrap().clone()
    };

    // NOTE: Since we have `Arc<Mutex<BotData>>`, we won't block the
    // requests related to other conversations.
    let mut event_occurred = None;

    match (data.type_, &data.data) {
        (ConversationEventType::MessageAdd,
         &ConversationData::MessageAdd { ref sender, recipient: _, ref text }) => {
            let (storage, client, devices) = {
                let lock = this_bot_data.lock();
                (lock.storage.clone(), lock.client.clone(), lock.devices.clone())
            };

            let plain_bytes = storage.decrypt(&data.from, sender, text)?;
            let mut message: GenericMessage = protobuf::parse_from_bytes(&plain_bytes)?;
            info!("Successfully decrypted message!");

            // We can decrypt and decode the message - 200 OK
            let msg_id = message.get_message_id().to_owned();
            job_sender.clone().send(Box::new(move |c: &HyperClient| {
                client.send_confirmation(c, &msg_id, storage.clone(), devices.clone())
            })).wait().map_err(|e| {
                error!("Cannot queue confirmation message in event loop: {}", e);
            }).ok();

            if message.has_text() {     // FIXME: Handle images
                info!("Got text message.");
                let mut text = message.take_text();
                let content = text.take_content();

                event_occurred = Some(EventData {
                    bot_id,
                    conversation: this_bot_data.lock().data.conversation.clone(),
                    event: Event::Message {
                        from: data.from.clone(),
                        text: content,
                    }
                });
            }
        },

        (ConversationEventType::MemberJoin,
         &ConversationData::LeavingOrJoiningMembers { ref user_ids }) => {
            // FIXME: What if we don't have the devices of these members?
            let conversation = {
                let mut old_data = this_bot_data.lock();
                // Add users to existing data
                for id in user_ids {
                    old_data.data.conversation.members.insert(Member {
                        id: *id,
                        status: 0,
                    });
                }

                old_data.data.conversation.clone()
            };

            info!("{} member(s) have joined the conversation {}",
                  user_ids.len(), conversation.id);
            let members_joined = user_ids.clone();
            event_occurred = Some(EventData {
                bot_id,
                conversation,
                event: Event::ConversationMemberJoin { members_joined },
            });
        },

        (ConversationEventType::MemberLeave,
         &ConversationData::LeavingOrJoiningMembers { ref user_ids }) => {
            let conversation = {
                let mut old_data = this_bot_data.lock();
                // Remove users from existing data
                for id in user_ids {
                    old_data.data.conversation.members.remove(id);
                }

                old_data.data.conversation.clone()
            };

            // If our bot has left, then remove the entire data.
            if user_ids.iter().find(|&id| id == &bot_id).is_some() {
                bot_data.lock().remove(&bot_id).unwrap();
            }

            info!("{} member(s) have left the conversation {}",
                  user_ids.len(), conversation.id);
            let members_left = user_ids.clone();
            event_occurred = Some(EventData {
                bot_id,
                conversation,
                event: Event::ConversationMemberLeave { members_left },
            });
        },

        (ConversationEventType::Rename,
         &ConversationData::Rename { ref name }) => {
            let conversation = {
                let mut old_data = this_bot_data.lock();
                info!("conversation {} has been renamed from {} to {}",
                      old_data.data.conversation.id, old_data.data.conversation.name, name);
                old_data.data.conversation.name = name.clone();
                old_data.data.conversation.clone()
            };

            event_occurred = Some(EventData {
                bot_id,
                conversation,
                event: Event::ConversationRename,
            });
        },

        _ => {
            info!("Unknown type {:?} and data {:?}", data.type_, data.data);
            return Err(BerylliumError::Unreachable)
        },
    };

    if let Some(event_data) = event_occurred {
        let client = {
            let lock = this_bot_data.lock();
            BotClient::from((&*lock, &job_sender))
        };

        let handle: CpuFuture<(), ()> = pool.spawn_fn(move || {
            info!("Handling user event...");
            handler.handle(event_data, client);
            Ok(())
        });

        // NOTE: This prevents the pool from canceling the computations
        // once the handle is dropped.
        handle.forget();
    }

    resp.set_status(StatusCode::Ok);
    Ok(())
}
