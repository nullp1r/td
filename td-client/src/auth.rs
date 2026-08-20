use std::fmt;
use std::future::{Future, Ready};

use tokio::sync::watch;

use td_types::enums::AuthorizationState as AuthState;
use td_types::{enums, fns, types};

use crate::client::{ClientHandle, UpdateReceiver};
use crate::error::{Error, Result};

pub struct Authenticator {
  handle: ClientHandle,
  updates: Option<UpdateReceiver>,
  auth_rx: watch::Receiver<Option<AuthState>>,
}

impl fmt::Debug for Authenticator {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Authenticator").field("client_id", &self.handle.id()).finish_non_exhaustive()
  }
}

impl Authenticator {
  pub fn new(handle: ClientHandle, updates: UpdateReceiver, auth_rx: watch::Receiver<Option<AuthState>>) -> Self {
    Self { handle, updates: Some(updates), auth_rx }
  }

  pub const fn handle(&self) -> &ClientHandle {
    &self.handle
  }

  pub fn current_state(&self) -> Option<AuthState> {
    self.auth_rx.borrow().clone()
  }

  pub async fn wait_state(&mut self) -> Result<AuthState> {
    let state = self.auth_rx.wait_for(Option::is_some).await.map_err(|_| {
      tracing::warn!(client_id = self.handle.id(), "auth state channel disconnected");
      Error::Disconnected
    })?;

    let state = state.as_ref().cloned().ok_or_else(|| {
      tracing::warn!(client_id = self.handle.id(), "auth state missing from channel");
      Error::Disconnected
    })?;

    tracing::debug!(client_id = self.handle.id(), ?state, "current auth state resolved");
    Ok(state)
  }

  pub async fn wait_next_state(&mut self) -> Result<AuthState> {
    self.auth_rx.changed().await.map_err(|_| {
      tracing::warn!(client_id = self.handle.id(), "auth state channel disconnected while waiting for next state");
      Error::Disconnected
    })?;
    self.wait_state().await
  }

  pub async fn send_bot_token(&self, token: impl Into<String>) -> Result<()> {
    tracing::info!(client_id = self.handle.id(), "sending bot token for authentication");
    let req = fns::checkAuthenticationBotToken { token: token.into() };
    self.handle.execute(&req).await.map(|_| ())
  }

  pub async fn set_phone_number(&self, phone: impl Into<String>) -> Result<()> {
    tracing::info!(client_id = self.handle.id(), "setting authentication phone number");
    let req = fns::setAuthenticationPhoneNumber { phone_number: phone.into(), ..Default::default() };
    self.handle.execute(&req).await.map(|_| ())
  }

  pub async fn send_code(&self, code: impl Into<String>) -> Result<()> {
    tracing::info!(client_id = self.handle.id(), "sending authentication code");
    let req = fns::checkAuthenticationCode { code: code.into() };
    self.handle.execute(&req).await.map(|_| ())
  }

  pub async fn send_password(&self, password: impl Into<String>) -> Result<()> {
    tracing::info!(client_id = self.handle.id(), "sending 2-step verification password");
    let req = fns::checkAuthenticationPassword { password: password.into() };
    self.handle.execute(&req).await.map(|_| ())
  }

  pub async fn send_email_address(&self, email_address: impl Into<String>) -> Result<()> {
    let req = fns::setAuthenticationEmailAddress { email_address: email_address.into() };
    self.handle.execute(&req).await.map(|_| ())
  }

  pub async fn send_email_code(&self, code: impl Into<String>) -> Result<()> {
    let email_code = enums::EmailAddressAuthentication::emailAddressAuthenticationCode(types::emailAddressAuthenticationCode { code: code.into() });
    let req = fns::checkAuthenticationEmailCode { code: email_code };
    self.handle.execute(&req).await.map(|_| ())
  }

  pub async fn register_user(&self, first_name: impl Into<String>, last_name: impl Into<String>) -> Result<()> {
    let req = fns::registerUser { first_name: first_name.into(), last_name: last_name.into(), disable_notification: false };
    self.handle.execute(&req).await.map(|_| ())
  }

  pub async fn request_qr_code(&self) -> Result<()> {
    let req = fns::requestQrCodeAuthentication { other_user_ids: Vec::new() };
    self.handle.execute(&req).await.map(|_| ())
  }

  pub fn finish(mut self) -> Result<(ClientHandle, UpdateReceiver)> {
    let Some(updates) = self.updates.take() else {
      tracing::error!(client_id = self.handle.id(), "authenticator already finished or missing updates receiver");
      return Err(Error::Auth("authenticator already finished or updates channel missing".into()));
    };
    tracing::info!(client_id = self.handle.id(), "auth completed, returning client handle and update receiver");
    Ok((self.handle, updates))
  }

  pub async fn auth_bot(mut self, token: impl Into<String>) -> Result<(ClientHandle, UpdateReceiver)> {
    let token = token.into();
    let client_id = self.handle.id();
    tracing::info!(client_id, "starting bot authorization workflow");

    loop {
      let state = self.wait_state().await?;
      tracing::debug!(client_id, ?state, "handling bot auth state");
      match state {
        AuthState::authorizationStateWaitPhoneNumber => self.send_bot_token(&token).await?,
        AuthState::authorizationStateReady => {
          tracing::info!(client_id, "bot authorization successful");
          return self.finish();
        }
        other => self.handle_common_state(&other, "bot")?,
      }
      self.wait_next_state().await?;
    }
  }

  pub async fn auth_user<C, CFut>(self, phone: impl Into<String>, code: C) -> Result<(ClientHandle, UpdateReceiver)>
  where
    C: FnMut() -> CFut,
    CFut: Future<Output = String>,
  {
    self.auth_user_internal(phone, code, Option::<fn() -> Ready<String>>::None).await
  }

  pub async fn auth_user_with_password<C, CFut, P, PFut>(self, phone: impl Into<String>, code: C, pw: P) -> Result<(ClientHandle, UpdateReceiver)>
  where
    C: FnMut() -> CFut,
    CFut: Future<Output = String>,
    P: FnMut() -> PFut,
    PFut: Future<Output = String>,
  {
    self.auth_user_internal(phone, code, Some(pw)).await
  }

  async fn auth_user_internal<C, CFut, P, PFut>(mut self, phone: impl Into<String>, mut code: C, mut pw: Option<P>) -> Result<(ClientHandle, UpdateReceiver)>
  where
    C: FnMut() -> CFut,
    CFut: Future<Output = String>,
    P: FnMut() -> PFut,
    PFut: Future<Output = String>,
  {
    let phone = phone.into();
    let client_id = self.handle.id();
    tracing::info!(client_id, "starting user authorization workflow");

    loop {
      let state = self.wait_state().await?;
      tracing::debug!(client_id, ?state, "handling user auth state");
      match state {
        AuthState::authorizationStateWaitPhoneNumber => self.set_phone_number(&phone).await?,
        AuthState::authorizationStateWaitCode(_) => {
          tracing::info!(client_id, "prompting for authorization code");
          self.send_code(code().await).await?;
        }
        AuthState::authorizationStateWaitPassword(_) => {
          let Some(p_fn) = pw.as_mut() else {
            tracing::error!(client_id, "2-step verification password required but no password_fn provided");
            return Err(Error::Auth("2-step verification password required".into()));
          };
          tracing::info!(client_id, "prompting for 2-step verification password");
          self.send_password(p_fn().await).await?;
        }
        AuthState::authorizationStateReady => {
          tracing::info!(client_id, "user authorization successful");
          return self.finish();
        }
        other => self.handle_common_state(&other, "user")?,
      }
      self.wait_next_state().await?;
    }
  }

  fn handle_common_state(&self, state: &AuthState, context: &str) -> Result<()> {
    match state {
      AuthState::authorizationStateWaitTdlibParameters => Ok(()),
      AuthState::authorizationStateClosed | AuthState::authorizationStateClosing => {
        tracing::warn!(client_id = self.handle.id(), "client was closed during authorization");
        Err(Error::Auth("client was closed during authorization".into()))
      }
      AuthState::authorizationStateLoggingOut => {
        tracing::warn!(client_id = self.handle.id(), "client is logging out");
        Err(Error::Auth("client is logging out".into()))
      }
      other => {
        tracing::error!(client_id = self.handle.id(), ?other, "unexpected auth state for {context}");
        Err(Error::Auth(format!("unexpected auth state for {context}: {other:?}")))
      }
    }
  }
}
