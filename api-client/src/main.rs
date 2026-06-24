mod api_client;
use api_client::client::Client;
use std::io::{self, BufRead, Write};

use crate::api_client::conversation::Conversation;

const MODEL: &str = "claude-opus-4-8";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let streaming: bool = std::env::args().any(|arg| arg == "--streaming");
    let api_key = dotenvy::var("ANTHROPIC_API_KEY")?;
    let client = Client::new(api_key);
    let mut chat = Conversation::new(MODEL);
    // Lock stdin once for the whole session instead of re-locking each turn.
    let mut stdin = io::stdin().lock();
    println!("Chat started, Type 'quit' or press Ctrl-C to exit.\n");
    loop {
        // 1. Print the prompt WITHOUT a newline, then flush.
        print!("❯ ");
        io::stdout().flush()?;

        // 2. Read one line.
        let mut line = String::new();
        let bytes = stdin.read_line(&mut line)?;
        if bytes == 0 {
            // EOF Ctrl+D
            println!();
            break;
        }

        // 3. read_line keeps the trailing '\n', so trim it
        let prompt = line.trim();
        if prompt.is_empty() {
            continue;
        }
        if prompt == "/quit" || prompt == "/exit" {
            break;
        }
        // 4. send and print. handle errors without killing the session.
        if streaming {
            chat.say_stream(&client, prompt, |piece| {
                print!("{piece}");
                let _ = io::stdout().flush();
            })
            .await?;
        } else {
            match chat.say(&client, prompt).await {
                Ok(reply) => println!("\n{reply}\n"),
                Err(e) => eprintln!("\n[error] {e}\n"),
            }
        }
    }
    println!("Bye! ({} messages exchanged)", chat.history().len());
    Ok(())
}
