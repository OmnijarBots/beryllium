use client::BotClient;
use serde::de::{Deserialize, Deserializer, Error as DecodeError};
use serde_json::Value;
use storage::StorageManager;
// FIXME: Check the types for id (String), i32, etc.
// which are too generic

pub enum Event {
    ConversationMemberJoin,
    ConversationMemberLeave,
    ConversationRename {
        old: String,
        new: String,
    },
    Message,
    Image,
}

pub struct EventData {
    pub bot_id: String,
    pub event: Event,
}

#[derive(Deserialize, Serialize)]
pub struct Member {
    pub id: String,
    pub status: i32,
}

#[derive(Deserialize, Serialize)]
pub struct Conversation {
    pub id: String,
    pub name: String,
    pub members: Vec<Member>,
}

#[derive(Deserialize, Serialize)]
pub struct Origin {
    pub id: String,
    pub name: String,
    pub accent_id: i32,
}

#[derive(Deserialize, Serialize)]
pub struct BotCreationData {
    pub id: String,
    pub client: String,
    pub origin: Origin,
    pub conversation: Conversation,
    pub token: String,
    pub locale: String,
}

pub struct BotData {
    pub storage: StorageManager,
    pub data: BotCreationData,
    pub client: BotClient,
}

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

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ConversationData {
    MessageAdd {
        sender: String,
        recipient: String,
        text: String,
    },
    LeavingOrJoiningMembers {
        user_ids: Vec<String>,
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

#[derive(Serialize)]
pub struct EncodedPreKey {
    pub id: u16,
    pub key: String,
}

#[derive(Serialize)]
pub struct BotCreationResponse {
    pub prekeys: Vec<EncodedPreKey>,
    pub last_prekey: EncodedPreKey,
}
