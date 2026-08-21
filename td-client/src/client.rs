use std::sync::Arc;

use serde::Deserialize;
use tokio::sync::mpsc;

use td_types::enums;
use td_types::traits::Function;

use crate::auth::Authenticator;
use crate::config::Config;
use crate::error::{Error, Result};
use crate::router::ClientState;
use crate::util;

pub type UpdateReceiver = mpsc::Receiver<enums::Update>;
pub type ClientHandle = Arc<ClientState>;

/// Sets `TDLib`'s process-wide log verbosity level.
pub fn set_log_verbosity_level(level: i32) {
  // SAFETY: TDLib accepts any integer verbosity level and this call has no pointer arguments.
  unsafe { td_sys::td_set_log_verbosity_level(level) };
}

#[derive(Debug, Clone)]
pub struct Client {
  config: Config,
}

impl Client {
  pub fn new(config: impl Into<Config>) -> Self {
    Self { config: config.into() }
  }

  pub fn start(self) -> (ClientHandle, UpdateReceiver) {
    let (state, updates, _) = ClientState::create();
    (state, updates)
  }

  pub async fn auth(self) -> Result<Authenticator> {
    let (state, updates, auth_rx) = ClientState::create();
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

pub fn execute_sync<F: Function>(req: &F) -> Result<F::Return> {
  let c_req = util::to_c_json(req)?;

  tracing::debug!("executing synchronous stateless request");

  // SAFETY: `c_req` is a valid, nul-terminated JSON request kept alive until `td_execute` returns.
  let res_ptr = unsafe { td_sys::td_execute(c_req.as_ptr().cast()) };
  if res_ptr.is_null() {
    tracing::error!("td_execute returned null pointer");
    return Err(Error::Disconnected);
  }

  tracing::trace!("td_execute received response");

  // SAFETY: TDLib guarantees a valid nul-terminated response when the pointer is non-null.
  let res_bytes = unsafe { util::c_json(res_ptr) };

  if let Ok(env) = util::from_c_json::<RawStatelessEnvelope<'_>>(res_bytes)
    && let "error" = env.type_name
    && let Ok(err) = util::from_c_json(res_bytes)
  {
    tracing::debug!(?err, "td_execute returned error");
    return Err(Error::Td(err));
  }

  util::from_c_json(res_bytes).map_err(|source| {
    tracing::error!(%source, "failed to parse td_execute response payload");
    Error::json(source, res_bytes)
  })
}
