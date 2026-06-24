use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct MessageParam {
    pub content: String,
    pub role: String,
}

impl MessageParam {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct MessageRequest {
    pub max_tokens: u32,
    pub model: String,
    pub messages: Vec<MessageParam>,
    pub system: Option<String>,
    pub stream: bool,
}

#[derive(Serialize, Deserialize)]
pub struct MessageResponse {
    pub role: String,
    pub content: Vec<ContentBlock>,
    pub stop_reason: Option<String>,
}

#[derive(Deserialize)]
pub struct MessageDelta {
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentDelta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
    ThinkingDelta { thinking: String },
    SignatureDelta { signature: String },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    MessageStart {/* ignored for now */},
    ContentBlockDelta { index: u32, delta: ContentDelta },
    MessageDelta { delta: MessageDelta },
    MessageStop {/* ignored for now */},
}

impl MessageResponse {
    /// concatenate all text blocks into one mstring

    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|b| b.text.as_deref()) // skip non-text blocks
            .collect()
    }
}

#[derive(Serialize, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub kind: String,
    pub text: Option<String>,
}

#[test]
fn test_event_stream() {}
