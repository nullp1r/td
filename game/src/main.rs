//! Telegram MMO game daemon executable.

#[cfg(feature = "telegram")]
use game::telegram;

#[cfg(feature = "telegram")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
  telegram::run().await
}

#[cfg(not(feature = "telegram"))]
fn main() {
  eprintln!("build with the `telegram` feature to run the bot");
}
