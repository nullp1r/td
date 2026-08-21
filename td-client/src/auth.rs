use std::future::{Future, Ready};

use tokio::sync::mpsc;

use td_types::enums::AuthorizationState as AuthState;
use td_types::fns;

use crate::client::{ClientHandle, UpdateReceiver};
use crate::error::{Error, Result};

pub struct Authenticator {
  client: ClientHandle,
  updates: UpdateReceiver,
  auth_rx: mpsc::Receiver<AuthState>,
}

impl Authenticator {
  pub(crate) fn new(client: ClientHandle, updates: UpdateReceiver, auth_rx: mpsc::Receiver<AuthState>) -> Self {
    Self { client, updates, auth_rx }
  }

  fn finish(self) -> (ClientHandle, UpdateReceiver) {
    tracing::info!("auth completed, returning client handle and update receiver");
    (self.client, self.updates)
  }

  async fn next_state(&mut self) -> Result<AuthState> {
    self.auth_rx.recv().await.ok_or_else(|| {
      tracing::warn!("auth state channel disconnected");
      Error::Disconnected
    })
  }

  pub async fn bot(mut self, token: impl Into<String>) -> Result<(ClientHandle, UpdateReceiver)> {
    let token = token.into();
    tracing::info!("starting bot authorization workflow");

    loop {
      let state = self.next_state().await?;
      tracing::debug!(?state, "handling bot auth state");
      match state {
        AuthState::authorizationStateWaitPhoneNumber => {
          tracing::info!("sending bot token for authentication");
          let req = fns::checkAuthenticationBotToken { token: token.clone() };
          self.client.execute(&req).await?;
        }
        AuthState::authorizationStateReady => return Ok(self.finish()),
        other => handle_common_state(&other, "bot")?,
      }
    }
  }

  pub async fn user<C, CFut>(self, phone: impl Into<String>, code: C) -> Result<(ClientHandle, UpdateReceiver)>
  where
    C: FnMut() -> CFut,
    CFut: Future<Output = String>,
  {
    self.user_internal(phone, code, None::<fn() -> Ready<_>>).await
  }

  pub async fn user_with_password<C, CFut, P, PFut>(self, phone: impl Into<String>, code: C, pw: P) -> Result<(ClientHandle, UpdateReceiver)>
  where
    C: FnMut() -> CFut,
    CFut: Future<Output = String>,
    P: FnMut() -> PFut,
    PFut: Future<Output = String>,
  {
    self.user_internal(phone, code, Some(pw)).await
  }

  async fn user_internal<C, CFut, P, PFut>(mut self, phone: impl Into<String>, mut code: C, mut pw: Option<P>) -> Result<(ClientHandle, UpdateReceiver)>
  where
    C: FnMut() -> CFut,
    CFut: Future<Output = String>,
    P: FnMut() -> PFut,
    PFut: Future<Output = String>,
  {
    let phone = phone.into();
    tracing::info!("starting user authorization workflow");

    loop {
      let state = self.next_state().await?;
      tracing::debug!(?state, "handling user auth state");
      match state {
        AuthState::authorizationStateWaitPhoneNumber => {
          tracing::info!("setting authentication phone number");
          let req = fns::setAuthenticationPhoneNumber { phone_number: phone.clone(), ..Default::default() };
          self.client.execute(&req).await?;
        }
        AuthState::authorizationStateWaitCode(_) => {
          tracing::info!("prompting for authorization code");
          let req = fns::checkAuthenticationCode { code: code().await };
          self.client.execute(&req).await?;
        }
        AuthState::authorizationStateWaitPassword(_) => {
          let Some(p_fn) = pw.as_mut() else {
            tracing::error!("2-step verification password required but no password_fn provided");
            return Err(Error::Auth("2-step verification password required".into()));
          };
          tracing::info!("prompting for 2-step verification password");
          let req = fns::checkAuthenticationPassword { password: p_fn().await };
          self.client.execute(&req).await?;
        }
        AuthState::authorizationStateReady => return Ok(self.finish()),
        other => handle_common_state(&other, "user")?,
      }
    }
  }
}

fn handle_common_state(state: &AuthState, context: &str) -> Result<()> {
  match state {
    AuthState::authorizationStateWaitTdlibParameters => Ok(()),
    AuthState::authorizationStateClosed | AuthState::authorizationStateClosing => {
      tracing::warn!("client was closed during authorization");
      Err(Error::Auth("client was closed during authorization".into()))
    }
    AuthState::authorizationStateLoggingOut => {
      tracing::warn!("client is logging out");
      Err(Error::Auth("client is logging out".into()))
    }
    other => {
      tracing::error!(?other, "unexpected auth state for {context}");
      Err(Error::Auth(format!("unexpected auth state for {context}: {other:?}")))
    }
  }
}
