use std::fs;

use serde::Deserialize;
use tracing_subscriber::EnvFilter;

use td_client::{Client, Config};
use td_types::fns;

use self::app::App;

mod app;
mod client_ext;
mod db;
mod util;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
  pub api_id: i32,
  pub api_hash: String,
  pub bot_token: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  tracing_subscriber::fmt()
    .with_env_filter(match EnvFilter::try_from_default_env() {
      Err(_) => EnvFilter::new("info,td_app=debug,td_client=debug"),
      Ok(filter) => filter,
    })
    .without_time()
    .init();
  td_client::set_log_verbosity_level(1);

  let cfg = fs::read_to_string("config.json")?;
  let cfg = serde_json::from_str::<AppConfig>(&cfg)?;
  let AppConfig { api_id, api_hash, bot_token } = cfg;

  let td = fns::setTdlibParameters { api_id, api_hash, ..Config::default().td };
  let auth = Client::new(Config { td }).auth().await?;
  let (client, updates) = auth.bot(&bot_token).await?;

  let app = App::new(client);
  app.run(updates).await?;

  Ok(())
}
