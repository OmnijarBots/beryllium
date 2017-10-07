// FIXME: Check the types for id (String), i32, etc.
// which are too generic

#[derive(Deserialize)]
pub struct Member {
    pub id: String,
    pub status: i32,
}

#[derive(Deserialize)]
pub struct Conversation {
    pub id: String,
    pub members: Vec<Member>
}

#[derive(Deserialize)]
pub struct Origin {
    pub id: String,
    pub name: String,
    pub accent_id: i32,
}

#[derive(Deserialize)]
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
