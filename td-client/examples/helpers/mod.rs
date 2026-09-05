//! Shared runner and configuration loader for example bots.

use std::fs;

use anyhow::Context as _;
use serde::Deserialize;

use td_client::{Client, Session, parameters, set_log_level};

/// Result alias used by examples and handlers.
pub type Result<T = ()> = anyhow::Result<T>;

#[derive(Deserialize)]
struct Config {
  api_id: i32,
  api_hash: String,
  bot_token: String,
}

/// Runs a bot example task, closing the session gracefully when complete.
pub async fn run(task: impl AsyncFnOnce(&mut Session, Client) -> Result) -> Result {
  tracing_subscriber::fmt().without_time().init();
  set_log_level(1);

  let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/helpers/config.json");
  let bytes = fs::read(path).context("missing `config.json` (copy from `config.example.json`)")?;
  let Config { api_id, api_hash, bot_token } = serde_json::from_slice(&bytes).context("failed to parse `config.json`")?;

  let mut session = Session::bot(parameters(api_id, api_hash, ".td"), &bot_token).await?;
  let client = session.client();

  if let Err(error) = task(&mut session, client).await {
    tracing::error!(%error, "failed to run");
  }
  if let Err(error) = session.close().await {
    tracing::error!(%error, "failed to close");
  }
  Ok(())
}
