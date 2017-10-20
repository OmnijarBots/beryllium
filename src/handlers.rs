use {protobuf, utils};
use client::{BotClient, BotData};
use errors::{BerylliumError, BerylliumResult};
use futures::{Future, Stream};
use futures::future;
use futures_cpupool::{Builder, CpuFuture, CpuPool};
use hyper::{Body, Error as HyperError, Headers, Method, StatusCode};
use hyper::header::{Authorization, Bearer, ContentLength};
use hyper::server::{Service, Request, Response};
use messages_proto::GenericMessage;
use parking_lot::Mutex;
use serde_json::{self, Value as SerdeValue};
use storage::StorageManager;
use std::collections::HashMap;
use std::sync::Arc;
use types::{BotCreationData, BotCreationResponse, Event, EventData};
use types::{ConversationData, ConversationEventType, MessageData, Member};
use uuid::Uuid;

// TODO:
// - Proper logging
// - Isolate events into their own functions.
// - Revisit `clone` usage on various types.
// - Revisit usage of Arc's and Mutex'es

pub trait Handler: Send + Sync + 'static {
    fn handle(&self, data: EventData, client: BotClient);
}

// FIXME: I *know* that Arc has an overhead, but I'm not entirely
// sure about the performance impact of this in our case (i.e., HTTP requests)
pub struct BotHandler<H> {
    handler: Arc<H>,
    pool: Arc<CpuPool>,
    bot_data: Arc<Mutex<HashMap<Uuid, Arc<Mutex<BotData>>>>>,
}

impl<H: Handler> BotHandler<H> {
    pub fn new(handler: Arc<H>) -> BotHandler<H> {
        BotHandler {
            handler: handler,
            pool: Arc::new(Builder::new().create()),
            bot_data: Arc::new(Mutex::new(HashMap::new())),
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
            debug!("Disallowed method: {}", method);
            resp.set_status(StatusCode::MethodNotAllowed);
            return Box::new(future::ok(resp))
        } else {        // all requests should have Bearer token auth
            match headers.get::<Authorization<Bearer>>() {
                Some(header) if utils::check_auth_token(&header.to_string()[7..]) => (),
                _ => {
                    resp.set_status(StatusCode::Unauthorized);
                    debug!("Unauthorized request!");
                    return Box::new(future::ok(resp))
                }
            }
        }

        macro_rules! parse_json_and {
            ($call:expr $( , $arg:expr )*) => {{
                let mut bytes = vec![];
                if let Some(len) = headers.get::<ContentLength>() {
                    bytes = Vec::with_capacity(**len as usize);
                }

                // FIXME: Prone to DDoS
                let f = body.fold(bytes, |mut acc, ref chunk| {
                    acc.extend_from_slice(chunk);
                    future::ok::<_, Self::Error>(acc)
                }).map(|vec| {
                    if let Ok(value) = serde_json::from_slice(&vec) {
                        if let Err(e) = $call($( $arg, )* value, &mut resp) {
                            error!("{}", e);
                            resp.set_status(StatusCode::InternalServerError);
                        }
                    } else {
                        debug!("Cannot parse JSON object!");
                        resp.set_status(StatusCode::BadRequest);
                    }

                    resp
                });

                Box::new(f)
            }};
        }

        let rel_url = uri.path();
        debug!("Incoming authorized request - Path: {}", rel_url);
        let mut split = rel_url.trim_matches('/').split('/');

        // FIXME: Better way to detect relative URL paths?
        match (split.next(), split.next(), split.next(), split.next()) {
            (Some("bots"), None, None, None) => parse_json_and!(create_bot),
            (Some("bots"), Some(id), Some("messages"), None) => {
                let pool = self.pool.clone();
                let handler = self.handler.clone();
                let bot_id = String::from(id);
                let bot_data = self.bot_data.clone();
                parse_json_and!(handle_events, pool, bot_data, bot_id, handler)
            },
            _ => parse_json_and!(empty_response, headers),
        }
    }
}

fn empty_response(headers: Headers, data: SerdeValue,
                  resp: &mut Response) -> BerylliumResult<()> {
    debug!("[Headers] \n{}\nData: {}\n", headers, data);
    resp.set_status(StatusCode::NotFound);
    Ok(())
}

fn create_bot(data: BotCreationData, resp: &mut Response) -> BerylliumResult<()> {
    info!("Creating new bot...");
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

fn handle_events<H>(pool: Arc<CpuPool>,
                    bot_data: Arc<Mutex<HashMap<Uuid, Arc<Mutex<BotData>>>>>,
                    bot_id: String, handler: Arc<H>,
                    data: MessageData, resp: &mut Response)
                   -> BerylliumResult<()>
    where H: Handler
{
    let bot_id = bot_id.parse::<Uuid>()?;
    debug!("Preparing to handle conversation event...");
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

    let event = match (data.type_, &data.data) {
        (ConversationEventType::MessageAdd,
         &ConversationData::MessageAdd { ref sender, recipient: _, ref text }) => {
            let (storage, client, devices, user_client) = {
                let lock = this_bot_data.lock();
                (lock.storage.clone(), lock.client.clone(),
                 lock.devices.clone(), BotClient::from(&*lock))
            };

            let plain_bytes = storage.decrypt(&data.from, sender, text)?;
            let mut message: GenericMessage = protobuf::parse_from_bytes(&plain_bytes)?;
            debug!("Successfully decrypted message!");

            // We can decrypt and decode the message - 200 OK
            {
                let msg_id = message.get_message_id();
                client.send_confirmation(msg_id, &storage, &devices)?;
            }

            if message.has_text() {
                let mut text = message.take_text();
                let content = text.take_content();

                Some((EventData {
                    bot_id,
                    conversation: this_bot_data.lock().data.conversation.clone(),
                    event: Event::Message {
                        from: data.from.clone(),
                        text: content,
                    }
                }, user_client))
            } else {    // FIXME: Handle images
                None
            }
        },

        (ConversationEventType::MemberJoin,
         &ConversationData::LeavingOrJoiningMembers { ref user_ids }) => {
            // FIXME: What if we don't have the devices of these members?
            let (conversation, client) = {
                let mut old_data = this_bot_data.lock();
                // Add users to existing data
                for id in user_ids {
                    old_data.data.conversation.members.insert(Member {
                        id: *id,
                        status: 0,
                    });
                }

                (old_data.data.conversation.clone(), BotClient::from(&*old_data))
            };

            let members_joined = user_ids.clone();
            Some((EventData {
                bot_id,
                conversation,
                event: Event::ConversationMemberJoin { members_joined },
            }, client))
        },

        (ConversationEventType::MemberLeave,
         &ConversationData::LeavingOrJoiningMembers { ref user_ids }) => {
            let (conversation, client) = {
                let mut old_data = this_bot_data.lock();
                // Remove users from existing data
                for id in user_ids {
                    old_data.data.conversation.members.remove(id);
                }

                (old_data.data.conversation.clone(), BotClient::from(&*old_data))
            };

            // If our bot has left, then remove the entire data.
            if user_ids.iter().find(|&id| id == &bot_id).is_some() {
                bot_data.lock().remove(&bot_id).unwrap();
            }

            let members_left = user_ids.clone();
            Some((EventData {
                bot_id,
                conversation,
                event: Event::ConversationMemberLeave { members_left },
            }, client))
        },

        (ConversationEventType::Rename,
         &ConversationData::Rename { ref name }) => {
            let (conversation, client) = {
                let mut old_data = this_bot_data.lock();
                old_data.data.conversation.name = name.clone();
                (old_data.data.conversation.clone(), BotClient::from(&*old_data))
            };

            Some((EventData {
                bot_id,
                conversation,
                event: Event::ConversationRename,
            }, client))
        },

        _ => {
            debug!("Unknown type {:?} and data {:?}", data.type_, data.data);
            return Err(BerylliumError::Unreachable)
        },
    };

    if let Some((event_data, client)) = event {
        let handle: CpuFuture<(), ()> = pool.spawn_fn(move || {
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
