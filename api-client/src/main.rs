mod chat;

use api_client::client::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let streaming: bool = std::env::args().any(|arg| arg == "--streaming");
    let api_key = dotenvy::var("ANTHROPIC_API_KEY")?;
    let client = Client::new(api_key);
    chat::run(&client, streaming).await
}
