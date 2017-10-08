// FIXME: Check the types for id (String), i32, etc.
// which are too generic

pub enum Event {
    ConversationMemberJoin,
    ConversationMemberLeave,
    ConversationRename,
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
