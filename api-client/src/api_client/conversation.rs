use crate::api_client::client::Client;
use crate::api_client::error::ApiError;
use crate::api_client::models::MessageParam;

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

    pub fn history(&self) -> &[MessageParam] {
        &self.messages
    }
}
