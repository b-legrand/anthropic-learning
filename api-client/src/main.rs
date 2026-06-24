mod api_client;
use api_client::client::Client;

const MODEL: &str = "claude-opus-4-8";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = dotenvy::var("ANTHROPIC_API_KEY")?;
    let client = Client::new(api_key);

    let reply = client
        .message(MODEL, "Explain the rust borrow checker like i am five years old")
        .await?;

    println!("{reply}");
    Ok(())
}
