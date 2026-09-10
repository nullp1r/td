//! Telegram runtime: session setup, command registration, update routing, and timer output.

mod callback;
mod dispatch;
mod present;

use std::{env, fs, path::PathBuf, pin::pin, sync::Arc};

use anyhow::Context as _;
use tdx::{
  client::{self, Client, Session},
  command,
  enums::{BotCommandScope, Update},
  prelude::*,
  session,
};
use tokio::{
  signal,
  sync::{mpsc, watch},
};

use crate::{app::App, content::Content, db::Db, timer, view::TimerOutcome};

struct Config {
  api_id: i32,
  api_hash: String,
  bot_token: String,
  db_path: PathBuf,
  content_path: PathBuf,
  session_path: PathBuf,
}

/// Runs the Telegram MMO game daemon.
pub async fn run() -> anyhow::Result<()> {
  tracing()?;
  let config = Config::from_env()?;
  let content = Arc::new(Content::load(&config.content_path)?);
  fs::create_dir_all(config.session_path.join("db"))?;
  fs::create_dir_all(config.session_path.join("files"))?;
  if let Some(parent) = config.db_path.parent()
    && !parent.as_os_str().is_empty()
  {
    fs::create_dir_all(parent)?;
  }
  let db = Db::open(config.db_path).await?;
  let app = App::new(db, content);

  let mut params = session::parameters(config.api_id, config.api_hash, &config.session_path);
  params.use_message_database = false;
  let mut session = session::bot(params, &config.bot_token).await?;
  client::set_log_level(2)?;
  let client = session.client();

  register_commands(&client).await.context("failed to register Telegram bot commands")?;

  let (timer_tx, timer_rx) = mpsc::channel(128);
  let (wake_tx, wake_rx) = watch::channel(0_u64);
  tokio::spawn(timer::run(app.clone(), timer_tx, wake_rx));
  tokio::spawn(timer_output(app.clone(), client.clone(), timer_rx, wake_tx.clone()));

  let finished = recv_loop(&mut session, app, client, wake_tx).await;
  let closed = session.close().await;
  finished?;
  closed?;
  Ok(())
}

async fn register_commands(client: &Client) -> client::Result<()> {
  let private = [command::definition("start", "Open your character"), command::definition("help", "How to play Rustwater")];
  client.send(&command::set(BotCommandScope::botCommandScopeAllPrivateChats, private)).await?;

  let groups = [command::definition("fish", "Start the shared fishing activity"), command::ephemeral_definition("help", "How group fishing works")];
  client.send(&command::set(BotCommandScope::botCommandScopeAllGroupChats, groups)).await?;
  Ok(())
}

async fn recv_loop(session: &mut Session, app: App, client: Client, wake: watch::Sender<u64>) -> anyhow::Result<()> {
  let mut ctrl_c = pin!(signal::ctrl_c());
  loop {
    let update = tokio::select! {
      update = session.recv() => update,
      signal = &mut ctrl_c => break signal?,
    };
    let Some(update) = update else { break };

    match update {
      Update::updateNewMessage(update) if !update.message.is_outgoing => {
        let app = app.clone();
        let client = client.clone();
        tokio::spawn(async move {
          if let Err(error) = dispatch::message(&app, &client, &update.message).await {
            tracing::error!(?error, "message update failed");
          }
        });
      }
      Update::updateNewCallbackQuery(update) => {
        let app = app.clone();
        let client = client.clone();
        let wake = wake.clone();
        tokio::spawn(async move {
          if let Err(error) = dispatch::callback(&app, &client, &update, &wake).await {
            tracing::error!(?error, "callback update failed");
          }
        });
      }
      _ => {}
    }
  }
  Ok(())
}

fn wake_timer(wake: &watch::Sender<u64>) {
  let next = (*wake.borrow()).wrapping_add(1);
  let _ = wake.send(next);
}

async fn timer_output(app: App, client: Client, mut rx: mpsc::Receiver<TimerOutcome>, wake: watch::Sender<u64>) {
  // Reaction deadlines start only after Telegram has accepted the actionable Bite/Struggle message.
  // This prevents network latency from consuming the player's reaction window.
  while let Some(outcome) = rx.recv().await {
    let result = match outcome {
      TimerOutcome::Bite(bite) => {
        let sent = present::bite(&client, &bite).await;
        if sent.is_ok() {
          if let Err(error) = app.mark_presented(bite.encounter_id, bite.step, timer::now_ms()).await {
            tracing::error!(?error, encounter_id = bite.encounter_id.0, "failed to open reaction window");
          } else {
            wake_timer(&wake);
          }
        }
        sent
      }
      TimerOutcome::Escaped(escape) => present::escaped(&client, &escape).await,
      TimerOutcome::Stale => Ok(()),
    };
    if let Err(error) = result {
      tracing::error!(?error, "timer presentation failed");
    }
  }
}

impl Config {
  fn from_env() -> anyhow::Result<Self> {
    Ok(Self {
      api_id: env::var("TELEGRAM_API_ID").context("TELEGRAM_API_ID is required")?.parse().context("TELEGRAM_API_ID must be an i32")?,
      api_hash: env::var("TELEGRAM_API_HASH").context("TELEGRAM_API_HASH is required")?,
      bot_token: env::var("TELEGRAM_BOT_TOKEN").context("TELEGRAM_BOT_TOKEN is required")?,
      db_path: env::var_os("GAME_DB").map_or_else(|| PathBuf::from("game.sqlite3"), PathBuf::from),
      content_path: env::var_os("GAME_CONTENT").map_or_else(|| PathBuf::from("content/game.json"), PathBuf::from),
      session_path: env::var_os("TDLIB_SESSION").map_or_else(|| PathBuf::from(".tdx-session"), PathBuf::from),
    })
  }
}

fn tracing() -> client::Result<()> {
  tracing_subscriber::fmt().with_target(true).init();
  client::execute(&fns::setLogStream { log_stream: enums::LogStream::logStreamEmpty })?;
  client::set_log_callback(|level, message| {
    let message = message.to_string_lossy();
    match level {
      0 | 1 => tracing::error!(target: "tdlib", %message),
      2 => tracing::warn!(target: "tdlib", %message),
      3 => tracing::info!(target: "tdlib", %message),
      4 => tracing::debug!(target: "tdlib", %message),
      _ => tracing::trace!(target: "tdlib", %message),
    }
  });
  client::set_error_callback(|error| tracing::error!(target: "tdlib", ?error));
  Ok(())
}
