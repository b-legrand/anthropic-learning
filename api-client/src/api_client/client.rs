use crate::api_client::error::ApiError;
use crate::api_client::models::{MessageParam, MessageRequest};

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

    pub async fn message(&self, model: &str, prompt: &str) -> Result<String, ApiError> {
        let request_body = MessageRequest {
            model: model.to_owned(),
            max_tokens: 1024,
            messages: vec![MessageParam {
                role: "user".to_owned(),
                content: prompt.to_owned(),
            }],
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
        Ok(response.text().await?)
    }
}
