//! A production-grade Telegram fishing game bot built with `tdx`.
//!
//! Demonstrates single-buffer formatted text composition, array-based inline
//! keyboards, authoritative delivery tracking, live animations via edits,
//! callback queries with toasts and modal alerts, and entity-aware command routing.

mod data;
mod handler;
mod state;
mod ui;

use std::fs;
use std::pin::pin;
use std::sync::{Arc, Mutex};

use anyhow::Context as _;
use serde::Deserialize;
use tokio::signal;
use tracing::Level;

use tdx::client::{self, Session};
use tdx::enums::User;
use tdx::prelude::*;
use tdx::session;

use self::state::GameState;

const COMMANDS: [[&str; 2]; 6] = [
  ["fish", "Cast your line into the water"],
  ["bag", "Open your inventory and sell catches"],
  ["shop", "Browse and upgrade your fishing rods"],
  ["spots", "Travel between lakes and oceans"],
  ["brag", "Boast your greatest trophy catch"],
  ["help", "Show fishing guide and controls"],
];

#[derive(Deserialize)]
struct Config {
  api_id: i32,
  api_hash: String,
  bot_token: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
  init_tracing()?;

  let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/config.json");
  let bytes = fs::read(path).context("missing `config.json` (copy from `config.example.json`)")?;
  let Config { api_id, api_hash, bot_token } = serde_json::from_slice(&bytes).context("failed to parse `config.json`")?;

  let params = session::parameters(api_id, api_hash, ".tdx-fishing-session");
  let mut session = session::bot(params, &bot_token).await?;

  let finished = run(&mut session).await;
  let closed = session.close().await;

  finished?;
  closed?;

  Ok(())
}

async fn run(session: &mut Session) -> anyhow::Result<()> {
  let client = session.client();

  // Register commands in Telegram Bot Menu
  let commands = COMMANDS.iter().map(|&[cmd, desc]| {
    let [command, description] = [cmd, desc].map(Into::into);
    types::botCommand { command, description, ..Default::default() }
  });
  let req = fns::setCommands { commands: commands.collect(), ..Default::default() };
  client.send(&req).await?;

  let User::user(me) = client.send(&fns::getMe {}).await?;
  tracing::info!(username = ?me.username(), "Fishing bot ready! Send /fish or press Ctrl-C to stop");

  let state = Arc::new(Mutex::new(GameState::default()));
  let mut ctrl_c = pin!(signal::ctrl_c());

  loop {
    let update = tokio::select! {
      update = session.recv() => update,
      signal = &mut ctrl_c => {
        signal?;
        tracing::info!("Received Ctrl-C, closing session gracefully...");
        break;
      }
    };

    let Some(update) = update else { break };
    if let Err(err) = handler::on_update(&client, update, &me, &state).await {
      tracing::error!(error = ?err, "Error processing update");
    }
  }

  Ok(())
}

fn init_tracing() -> client::Result<()> {
  tracing_subscriber::fmt().without_time().with_max_level(Level::INFO).init();

  client::execute(&fns::setLogStream { log_stream: enums::LogStream::logStreamEmpty })?;
  client::set_log_level(2)?;
  client::set_log_callback(|level, message| {
    let message = message.to_str().unwrap_or("<invalid UTF-8>");
    let message = message.split_once('\t').map_or(message, |(_, body)| body).trim_ascii_end();
    if message.starts_with("Begin to wait for updates") || message.starts_with("End to wait for updates") {
      return;
    }
    match level {
      0 | 1 => tracing::error!(target: "tdlib", %message),
      2 => tracing::warn!(target: "tdlib", %message),
      3 => tracing::info!(target: "tdlib", %message),
      _ => tracing::debug!(target: "tdlib", %message),
    }
  });

  client::set_error_callback(|error| {
    tracing::error!(target: "tdlib", ?error);
  });

  Ok(())
}
