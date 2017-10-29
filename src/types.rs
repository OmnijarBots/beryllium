use errors::BerylliumError;
use futures::Future;
use hyper::Client;
use hyper_rustls::HttpsConnector;
use serde::de::{Deserialize, Deserializer, Error as DecodeError};
use serde_json::Value;
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use uuid::Uuid;

// FIXME: Check the types (for example, id should be Uuid instead of String),

/// HTTPS client (courtesy of rustls)
pub type HyperClient = Client<HttpsConnector>;
/// The `Future` type used throughout the lib.
pub type BerylliumFuture<I> = Box<Future<Item=I, Error=BerylliumError>>;
/// A closure which takes a HTTPS client and returns a `Future`. This is
/// how HTTPS client requests are queued in the event loop.
pub type EventLoopRequest<I> = Box<Fn(&HyperClient) -> BerylliumFuture<I> + Send + 'static>;

/// Represents the type of event handed over to the user.
pub enum Event {
    ConversationMemberJoin {
        members_joined: Vec<Uuid>,
    },
    ConversationMemberLeave {
        members_left: Vec<Uuid>,
    },
    ConversationRename,
    Message {
        text: String,
        from: String,       // FIXME: Should be `Uuid`
    },
    Image,
}

/// Event data passed to the type implementing the `Handler` trait.
pub struct EventData {
    /// ID of this bot instance.
    pub bot_id: Uuid,
    /// Conversation data
    pub conversation: Conversation,
    /// Event-type and related data (if any)
    pub event: Event,
}

/// Represents a conversation member.
#[derive(Clone, Deserialize, Serialize)]
pub struct Member {
    pub id: Uuid,
    pub status: i8,
}

// Custom implementations for HashSet addressing.
// We don't care about anything other than the member's UUID.
impl Borrow<Uuid> for Member {
    fn borrow(&self) -> &Uuid {
        &self.id
    }
}

impl Hash for Member {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for Member {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Member {}

#[derive(Clone, Deserialize, Serialize)]
pub struct Conversation {
    pub id: Uuid,
    pub name: String,
    pub members: HashSet<Member>,
}

#[derive(Deserialize, Serialize)]
pub struct Origin {
    pub id: Uuid,
    pub name: String,
    pub handle: String,
    pub accent_id: i8,
}

#[derive(Deserialize, Serialize)]
pub struct BotCreationData {
    pub id: Uuid,
    pub client: String,
    pub origin: Origin,
    pub conversation: Conversation,
    pub token: String,
    pub locale: String,
}

#[derive(Default, Deserialize)]
pub struct Devices {
    // UserID -> [ClientID]
    pub missing: HashMap<String, Vec<String>>,
}

#[derive(Clone, Copy, Debug)]
pub enum ConversationEventType {
    MessageAdd,
    MemberJoin,
    MemberLeave,
    Rename,
}

fn deserialize_conv_event_type<'de, D>(de: D) -> Result<ConversationEventType, D::Error>
    where D: Deserializer<'de>
{
    let deser_result: Value = Deserialize::deserialize(de)?;
    match deser_result {
        Value::String(ref s) if s == "conversation.otr-message-add"
            => Ok(ConversationEventType::MessageAdd),
        Value::String(ref s) if s == "conversation.member-join"
            => Ok(ConversationEventType::MemberJoin),
        Value::String(ref s) if s == "conversation.member-leave"
            => Ok(ConversationEventType::MemberLeave),
        Value::String(ref s) if s == "conversation.rename"
            => Ok(ConversationEventType::Rename),
        _ => Err(DecodeError::custom("Unexpected value for ConversationEventType")),
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ConversationData {
    MessageAdd {
        sender: String,
        recipient: String,
        text: String,
    },
    LeavingOrJoiningMembers {
        user_ids: Vec<Uuid>,
    },
    Rename {
        name: String,
    }
}

#[derive(Deserialize)]
pub struct MessageData {
    #[serde(rename = "type")]
    #[serde(deserialize_with = "deserialize_conv_event_type")]
    pub type_: ConversationEventType,
    pub conversation: String,
    pub from: String,
    pub data: ConversationData,
    pub time: String,
}

#[derive(Deserialize, Serialize)]
pub struct EncodedPreKey {
    pub id: u16,
    pub key: String,
}

pub type DevicePreKeys = HashMap<String, HashMap<String, EncodedPreKey>>;

#[derive(Serialize)]
pub struct BotCreationResponse {
    pub prekeys: Vec<EncodedPreKey>,
    pub last_prekey: EncodedPreKey,
}

#[derive(Serialize)]
pub struct MessageRequest<'a, 'b> {
    pub sender: &'a str,
    pub recipients: HashMap<&'b str, HashMap<&'b str, String>>,
}

pub enum MessageStatus {
    Sent,
    Failed(Devices),
}
