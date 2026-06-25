use bytes::Bytes;

use futures_util::{Stream, StreamExt};
use reqwest::Response;

use crate::error::ApiError;
use crate::models::{MessageParam, MessageRequest, MessageResponse};

const BASE_URL: &str = "https://api.anthropic.com/v1/messages";

pub struct Client {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl Client {
    pub fn new(api_key: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key,
            base_url: BASE_URL.to_string(),
        }
    }
    pub async fn send(
        &self,
        model: &str,
        messages: &[MessageParam],
    ) -> Result<MessageResponse, ApiError> {
        let request_body = MessageRequest {
            model: model.to_owned(),
            max_tokens: 1024,
            messages: messages.to_vec(),
            system: Some("".to_string()),
            stream: false,
        };
        let response = self
            .http
            .post(&self.base_url)
            .json(&request_body)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await?;
            return Err(ApiError::Status {
                status: status.as_u16(),
                body,
            });
        }
        let parsed = response.json::<MessageResponse>().await?;
        Ok(parsed)
    }

    /// streaming method
    pub async fn stream(
        &self,
        model: &str,
        messages: &[MessageParam],
    ) -> Result<impl Stream<Item = Result<Bytes, ApiError>> + use<>, ApiError> {
        let request_body = MessageRequest {
            model: model.to_owned(),
            max_tokens: 1024,
            messages: messages.to_vec(),
            system: Some("".to_string()),
            stream: true,
        };
        let response = self
            .http
            .post(&self.base_url)
            .json(&request_body)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await?;
            return Err(ApiError::Status {
                status: status.as_u16(),
                body,
            });
        }
        let parsed = Response::bytes_stream(response).map(|chunk| chunk.map_err(ApiError::from));
        Ok(parsed)
    }
}
