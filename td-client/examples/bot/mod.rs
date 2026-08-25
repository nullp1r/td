use std::fs;

use anyhow::Context as _;
use serde::Deserialize;

use td_client::Client;

pub type Result<T = ()> = anyhow::Result<T>;

#[derive(Deserialize)]
struct Config {
  api_id: i32,
  api_hash: String,
  bot_token: String,
}

pub async fn run(task: impl AsyncFnOnce(&mut Client) -> Result) -> Result {
  tracing_subscriber::fmt().without_time().init();
  td_client::set_log_level(1);

  let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/bot/config.json");
  let bytes = fs::read(path).context("missing `config.json` (copy from `config.example.json`)")?;
  let parsed = serde_json::from_slice(&bytes).context("failed to parse `config.json`")?;

  let Config { api_id, api_hash, bot_token } = parsed;
  let params = td_client::params(api_id, api_hash, ".td");
  let mut client = Client::bot(params, &bot_token).await?;

  if let Err(error) = task(&mut client).await {
    tracing::error!(%error, "failed to run");
  }
  if let Err(error) = client.shutdown().await {
    tracing::error!(%error, "failed to shut down");
  }
  Ok(())
}
