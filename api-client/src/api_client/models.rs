use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct MessageParam {
    pub content: String,
    pub role: String,
}

#[derive(Serialize, Deserialize)]
pub struct MessageRequest {
    pub max_tokens: u32,
    pub model: String,
    pub messages: Vec<MessageParam>,
}
