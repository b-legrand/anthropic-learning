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

#[derive(Debug, PartialEq, Deserialize)]
pub struct MessageDelta {
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentDelta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
    ThinkingDelta { thinking: String },
    SignatureDelta { signature: String },
}

#[derive(Debug, PartialEq, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_text_delta_event() {
        let json = r#"{"type":"content_block_delta","index":0,
                       "delta":{"type":"text_delta","text":"Hi"}}"#;

        let event: StreamEvent = serde_json::from_str(json).unwrap();

        assert_eq!(
            event,
            StreamEvent::ContentBlockDelta {
                index: 0,
                delta: ContentDelta::TextDelta { text: "Hi".into() },
            }
        );
    }

    #[test]
    fn deserializes_lifecycle_events() {
        // The `type` tag alone selects the variant; empty-bodied events carry no fields.
        let start: StreamEvent = serde_json::from_str(r#"{"type":"message_start"}"#).unwrap();
        let stop: StreamEvent = serde_json::from_str(r#"{"type":"message_stop"}"#).unwrap();

        assert_eq!(start, StreamEvent::MessageStart {});
        assert_eq!(stop, StreamEvent::MessageStop {});
    }

    #[test]
    fn unmodeled_event_type_is_an_error() {
        // An unknown tag has no matching variant, so deserialization fails.
        assert!(serde_json::from_str::<StreamEvent>(r#"{"type":"ping"}"#).is_err());
    }
}
