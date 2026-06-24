use crate::api_client::client::Client;
use crate::api_client::error::ApiError;
use crate::api_client::models::{ContentDelta, MessageParam, StreamEvent};

use futures_util::StreamExt;
use memchr::memmem::find;

pub struct Conversation {
    model: String,
    messages: Vec<MessageParam>,
}

impl Conversation {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            messages: Vec::new(),
        }
    }

    /// Send one user turn, store both sides, return the assistant's reply text.
    pub async fn say(
        &mut self,
        client: &Client,
        text: impl Into<String>,
    ) -> Result<&str, ApiError> {
        // 1. record user message
        self.messages.push(MessageParam::user(text));
        // 2. send full history
        let response = client.send(&&self.model, &self.messages).await?;
        // 3. record assistant response
        self.messages.push(MessageParam::assistant(response.text()));
        // 4. hand back a borrow of what we just stored
        Ok(&self.messages.last().unwrap().content)
    }

    pub async fn say_stream(
        &mut self,
        client: &Client,
        text: impl Into<String>,
        mut on_text: impl FnMut(&str),
    ) -> Result<&str, ApiError> {
        // 1. record user message
        self.messages.push(MessageParam::user(text));
        // 2. send full history, receive stream
        let stream = client.stream(&&self.model, &self.messages).await?;
        futures_util::pin_mut!(stream);
        // 3. record assistant response
        let mut buf: Vec<u8> = Vec::new();
        let mut reply = String::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buf.extend_from_slice(&chunk);
            loop {
                match find(&buf, b"\n\n") {
                    Some(pos) => {
                        let event_bytes = buf[..pos].to_vec();
                        buf.drain(..pos + 2);
                        // parse eventybytes into a StreamEvent
                        let text = match std::str::from_utf8(&event_bytes) {
                            Ok(t) => t,
                            Err(_) => continue, // skip malformed, keep streaming
                        };
                        for line in text.lines() {
                            let json = match line.strip_prefix("data: ") {
                                Some(j) => j,
                                None => continue, // skip "event:" lines
                            };
                            if json.trim() == "[DONE]" {
                                continue;
                            }
                            let event: StreamEvent = match serde_json::from_str(json) {
                                Ok(ev) => ev,
                                Err(_) => continue, // unmodeled event type, skip
                            };

                            match event {
                                StreamEvent::ContentBlockDelta {
                                    delta: ContentDelta::TextDelta { text },
                                    ..
                                } => {
                                    on_text(&text);
                                    reply.push_str(&text);
                                }
                                _ => {}
                            }
                        }
                    }
                    None => break,
                }
            }
        }
        self.messages.push(MessageParam::assistant(reply));
        // 4. hand back a borrow of what we just stored
        Ok(&self.messages.last().unwrap().content)
    }

    pub fn history(&self) -> &[MessageParam] {
        &self.messages
    }
}
