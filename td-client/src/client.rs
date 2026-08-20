use std::ffi::CStr;
use std::fmt;
use std::future::Future;
use std::sync::Arc;

use serde::Deserialize;
use tokio::sync::mpsc;

use td_types::enums;
use td_types::traits::Function;

use crate::auth::Authenticator;
use crate::config::Config;
use crate::error::{Error, Result};
use crate::router::ClientState;

/// Sets the global `TDLib` log verbosity level.
pub fn set_log_verbosity_level(level: i32) {
  // SAFETY: Setting global TDLib logging verbosity level.
  unsafe { td_sys::td_set_log_verbosity_level(level) };
}

pub type UpdateReceiver = mpsc::Receiver<enums::Update>;

#[derive(Clone)]
pub struct ClientHandle(Arc<ClientState>);

impl fmt::Debug for ClientHandle {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("ClientHandle").field("client_id", &self.id()).finish_non_exhaustive()
  }
}

impl ClientHandle {
  pub fn id(&self) -> i32 {
    self.0.id()
  }

  pub async fn execute<F: Function>(&self, fn_req: &F) -> Result<F::Return> {
    self.0.execute(fn_req).await
  }
}

#[derive(Debug, Clone)]
pub struct Client {
  config: Config,
}

impl Client {
  pub fn new(config: Config) -> Self {
    Self { config }
  }

  pub fn start(self) -> (ClientHandle, UpdateReceiver) {
    let (state, updates, _) = ClientState::create(self.config.log_verbosity_level);
    (ClientHandle(state), updates)
  }

  pub async fn authenticate(self) -> Result<Authenticator> {
    let (state, updates, auth_rx) = ClientState::create(self.config.log_verbosity_level);
    let handle = ClientHandle(state);
    tracing::info!(client_id = handle.id(), "beginning client authentication");

    let auth = Authenticator::new(handle, updates, auth_rx);
    tracing::debug!(client_id = auth.handle().id(), "setting tdlib parameters");
    auth.handle().execute(&self.config.td).await?;

    Ok(auth)
  }

  pub async fn auth_bot(self, token: impl Into<String>) -> Result<(ClientHandle, UpdateReceiver)> {
    self.authenticate().await?.auth_bot(token).await
  }

  pub async fn auth_user<C, CFut>(self, phone: impl Into<String>, code_fn: C) -> Result<(ClientHandle, UpdateReceiver)>
  where
    C: FnMut() -> CFut,
    CFut: Future<Output = String>,
  {
    self.authenticate().await?.auth_user(phone, code_fn).await
  }

  pub async fn auth_user_with_password<C, CFut, P, PFut>(self, phone: impl Into<String>, code_fn: C, password_fn: P) -> Result<(ClientHandle, UpdateReceiver)>
  where
    C: FnMut() -> CFut,
    CFut: Future<Output = String>,
    P: FnMut() -> PFut,
    PFut: Future<Output = String>,
  {
    self.authenticate().await?.auth_user_with_password(phone, code_fn, password_fn).await
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

  // SAFETY: Passing a valid null-terminated JSON C string to stateless td_execute.
  let res_ptr = unsafe { td_sys::td_execute(c_req.as_ptr().cast()) };
  if res_ptr.is_null() {
    tracing::error!("td_execute returned null pointer");
    return Err(Error::Disconnected);
  }

  // SAFETY: TDLib guarantees res_ptr points to a valid null-terminated C string when non-null.
  let Ok(res_str) = (unsafe { CStr::from_ptr(res_ptr) }).to_str() else {
    tracing::error!("invalid UTF-8 in td_execute response");
    return Err(Error::Sys("invalid UTF-8 in td_execute response".into()));
  };

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
