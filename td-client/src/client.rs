use std::ffi::CStr;
use std::sync::Arc;

use serde::Deserialize;
use tokio::sync::mpsc;

use td_types::enums;
use td_types::traits::Function;

use crate::auth::Authenticator;
use crate::config::Config;
use crate::error::{Error, Result};
use crate::router::ClientState;

pub type UpdateReceiver = mpsc::Receiver<enums::Update>;
pub type ClientHandle = Arc<ClientState>;

#[derive(Debug, Clone)]
pub struct Client {
  config: Config,
}

impl Client {
  pub fn new(config: Config) -> Self {
    Self { config }
  }

  pub fn start(self) -> (ClientHandle, UpdateReceiver) {
    let (state, updates, _) = ClientState::create(self.config.td_log_level);
    (state, updates)
  }

  pub async fn auth(self) -> Result<Authenticator> {
    let (state, updates, auth_rx) = ClientState::create(self.config.td_log_level);
    tracing::info!("beginning client authentication");

    tracing::debug!("setting tdlib parameters");
    state.execute(&self.config.td).await?;

    Ok(Authenticator::new(state, updates, auth_rx))
  }
}

#[derive(Deserialize)]
struct RawStatelessEnvelope<'a> {
  #[serde(rename = "@type")]
  type_name: &'a str,
}

/// Synchronously executes a stateless request without creating a client instance.
#[tracing::instrument(name = "td::execute_sync", skip(req))]
pub fn execute_sync<F: Function>(req: &F) -> Result<F::Return> {
  let mut c_req = serde_json::to_vec(req)?;
  c_req.push(0);

  tracing::debug!("executing synchronous stateless request");

  let res_ptr = unsafe { td_sys::td_execute(c_req.as_ptr().cast()) };
  if res_ptr.is_null() {
    tracing::error!("td_execute returned null pointer");
    return Err(Error::Disconnected);
  }

  let res_bytes = unsafe { CStr::from_ptr(res_ptr) }.to_bytes();
  let res_str = unsafe { str::from_utf8_unchecked(res_bytes) };
  tracing::trace!(raw = res_str, "td_execute received response");

  if let Ok(env) = serde_json::from_str::<RawStatelessEnvelope<'_>>(res_str)
    && let "error" = env.type_name
    && let Ok(err) = serde_json::from_str(res_str)
  {
    tracing::debug!(?err, "td_execute returned error");
    return Err(Error::Td(err));
  }

  serde_json::from_str(res_str).map_err(|source| {
    tracing::error!(%source, raw = res_str, "failed to parse td_execute response payload");
    Error::Json { source, raw: Some(res_str.to_string()) }
  })
}
