use api_client::client::Client;
use api_client::error::ApiError;
use api_client::models::{ContentDelta, MessageParam, StreamEvent};

use futures_util::StreamExt;

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
        let response = client.send(&self.model, &self.messages).await?;
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
        // 2. send full history, receive a stream of modeled events
        let stream = client.stream(&self.model, &self.messages).await?;
        futures_util::pin_mut!(stream);
        // 3. accumulate the assistant response from the text deltas
        let mut reply = String::new();
        while let Some(event) = stream.next().await {
            if let StreamEvent::ContentBlockDelta {
                delta: ContentDelta::TextDelta { text },
                ..
            } = event?
            {
                on_text(&text);
                reply.push_str(&text);
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
