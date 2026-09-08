//! Credentialed end-to-end scenarios. Run serially with `--ignored --test-threads=1`.
//! Each scenario records remote messages for cleanup; authorization persists between runs.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{env, process};

use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use tokio::time::timeout;

use tdx::client;
use tdx::prelude::*;
use tdx::session::{bot, parameters};

use self::live::scenarios::exercise;

mod live {
  pub mod media;
  pub mod scenarios;
}

const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/live/config.json");
const SESSION: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/live/session");
const OPERATION_TIMEOUT: Duration = Duration::from_secs(120);
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(10);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
const PHOTO_COUNT: usize = 10;

#[derive(Deserialize)]
struct Config {
  api_id: i32,
  api_hash: String,
  bot_token: String,
  chat_id: i64,
}

#[tokio::test]
#[ignore = "requires tdx/tests/live/config.json, FFmpeg, and a real Telegram test chat"]
async fn telegram_boundary() -> Result<()> {
  let config = read_config()?;
  let root = temporary_directory()?;
  fs::create_dir_all(&root).context("failed to create the live-test directory")?;

  let result = run(config, &root).await;
  let cleanup = fs::remove_dir_all(&root).context("failed to remove the live-test directory");
  result.and(cleanup)
}

async fn run(config: Config, root: &Path) -> Result<()> {
  let Config { api_id, api_hash, bot_token, chat_id } = config;
  ensure!(chat_id != 0, "chat_id must identify the dedicated Telegram test chat");
  let mut params = parameters(api_id, api_hash, SESSION);
  params.files_directory = root.to_string_lossy().into_owned();
  params.use_file_database = false;
  params.use_chat_info_database = false;
  params.use_message_database = false;

  client::set_log_level(0)?;
  client::set_receive_timeout(Duration::from_millis(50));
  let mut session = timeout(OPERATION_TIMEOUT, bot(params, &bot_token)).await.context("bot construction timed out")??;
  let client = session.client();

  let mut messages = Vec::new();

  // Capture all three outcomes before propagating any error. A failed scenario
  // must not skip remote cleanup or graceful shutdown.
  let exercise = timeout(OPERATION_TIMEOUT, exercise(&mut session, &client, chat_id, root, &mut messages))
    .await
    .context("live Telegram exercise timed out")
    .and_then(|result| result);

  let cleanup = timeout(CLEANUP_TIMEOUT, delete_messages(&client, chat_id, &messages)) //.
    .await
    .context("remote cleanup timed out")
    .and_then(|result| result);

  let shutdown = timeout(SHUTDOWN_TIMEOUT, session.close()) //.
    .await
    .context("shutdown timed out")
    .and_then(|result| result.map_err(Into::into));

  exercise?;
  cleanup?;
  shutdown
}

fn read_config() -> Result<Config> {
  let bytes = fs::read(CONFIG).context("missing live-test config")?;
  serde_json::from_slice(&bytes).context("invalid live-test config")
}

fn temporary_directory() -> Result<PathBuf> {
  let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
  Ok(env::temp_dir().join(format!("tdx-live-{}-{nonce}", process::id())))
}

async fn delete_messages(client: &Client, chat_id: i64, message_ids: &[i64]) -> Result<()> {
  if message_ids.is_empty() {
    return Ok(());
  }
  let request = delete::messages(chat_id, message_ids.iter().copied(), true);
  client.send(&request).await.context("failed to delete live-test messages")?;
  Ok(())
}
