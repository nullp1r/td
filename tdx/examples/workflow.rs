//! Runs a small bot workflow: receive a message, reply, then edit the reply.

use std::{fs, pin::pin};

use anyhow::Context as _;
use serde::Deserialize;
use tokio::signal;
use tracing::Level;

use tdx::client::{self, Client, Session};
use tdx::enums::{Update, User};
use tdx::prelude::*;
use tdx::session;

#[derive(Deserialize)]
struct Config {
  api_id: i32,
  api_hash: String,
  bot_token: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
  tracing()?;

  let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/config.json");
  let bytes = fs::read(path).context("missing `config.json` (copy from `config.example.json`)")?;
  let Config { api_id, api_hash, bot_token } = serde_json::from_slice(&bytes).context("failed to parse `config.json`")?;

  let params = session::parameters(api_id, api_hash, ".tdx-session");
  let mut session = session::bot(params, &bot_token).await?;
  let finished = run(&mut session).await;
  let closed = session.close().await;

  finished?;
  closed?;

  Ok(())
}

async fn run(session: &mut Session) -> anyhow::Result<()> {
  // Reduce native logging after authorization; the callback stays installed.
  client::set_log_level(2)?;
  let client = session.client();

  let User::user(me) = client.send(&fns::getMe {}).await?;
  tracing::info!(username = ?me.username(), "ready; send the bot a message or press Ctrl-C");

  let mut ctrl_c = pin!(signal::ctrl_c());

  loop {
    let update = tokio::select! {
      update = session.recv() => update,
      signal = &mut ctrl_c => break signal?
    };

    match update {
      Some(update) => on_update(&client, update).await?,
      _ => break,
    }
  }

  Ok(())
}

async fn on_update(client: &Client, update: Update) -> anyhow::Result<()> {
  let Update::updateNewMessage(u) = update else { return Ok(()) };

  if u.message.is_outgoing || u.message.chat_id < 0 {
    return Ok(());
  }

  let Some(msg) = u.message.text() else { return Ok(()) };
  tracing::info!(chat_id = u.message.chat_id, message_id = u.message.id, text = %msg.text, "received message");

  let reply = send::reply(&u.message, line(("Received: ", bold(&msg.text))));
  let sent = client.track(&reply, None, None).await?;
  tracing::info!(chat_id = sent.chat_id, message_id = sent.id, "sent initial reply");

  let edit = edit::text(&sent, line(("Processed ", code("successfully"))));
  client.send(&edit).await?;
  tracing::info!(chat_id = sent.chat_id, message_id = sent.id, "edited reply");

  Ok(())
}

fn tracing() -> client::Result<()> {
  tracing_subscriber::fmt().without_time().with_max_level(Level::TRACE).init();

  client::execute(&fns::setLogStream { log_stream: enums::LogStream::logStreamEmpty })?;
  client::set_log_level(3)?;
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
      4 => tracing::debug!(target: "tdlib", %message),
      _ => tracing::trace!(target: "tdlib", %message),
    }
  });

  client::set_error_callback(|error| {
    tracing::error!(target: "tdlib", ?error);
  });

  Ok(())
}
