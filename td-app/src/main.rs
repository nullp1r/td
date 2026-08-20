//! # `td-app`
//!
//! Example consumer demonstrating `td-client` using modern, expressive Rust.

mod app;
mod db;
mod util;

use std::error::Error;
use td_client::{Client, Config};
use tracing_subscriber::EnvFilter;

use crate::app::App;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  tracing_subscriber::fmt()
    .with_env_filter({
      EnvFilter::try_from_default_env() //.
        .unwrap_or_else(|_| EnvFilter::new("info,td_app=debug,td_client=debug"))
    })
    .init();

  tracing::info!("starting td-app client...");

  let (handle, updates) = Client::new(Config {
    api_id: 123_456, //.
    api_hash: "0123456789abcdef".into(),
    ..Default::default()
  })
  .auth_bot("123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11")
  .await?;

  tracing::info!("authenticated as bot");

  let app = App::new(handle);
  app.run(updates).await?;

  Ok(())
}
