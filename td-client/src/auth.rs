use tokio::sync::mpsc;

use td_types::enums::AuthorizationState as State;
use td_types::fns;

use crate::client::{ClientHandle, UpdateReceiver};
use crate::error::{Error, Result};

pub struct Auth {
  pub client: ClientHandle,
  updates: UpdateReceiver,
  states: mpsc::Receiver<State>,
}

impl Auth {
  pub(crate) fn new(client: ClientHandle, updates: UpdateReceiver, states: mpsc::Receiver<State>) -> Self {
    Self { client, updates, states }
  }

  pub fn finish(self) -> (ClientHandle, UpdateReceiver) {
    tracing::info!("auth completed, returning client handle and update receiver");
    (self.client, self.updates)
  }

  pub async fn next(&mut self) -> Result<Option<State>> {
    loop {
      let state = self.states.recv().await.ok_or_else(|| {
        tracing::warn!("auth state channel disconnected");
        Error::Disconnected
      })?;

      match state {
        State::authorizationStateWaitTdlibParameters => {}
        State::authorizationStateReady => return Ok(None),
        State::authorizationStateClosed | State::authorizationStateClosing | State::authorizationStateLoggingOut => {
          tracing::warn!(?state, "client closed during authorization");
          return Err(Error::Disconnected);
        }
        state => {
          tracing::debug!(?state, "authorization state received");
          return Ok(Some(state));
        }
      }
    }
  }

  pub async fn bot(mut self, token: &str) -> Result<(ClientHandle, UpdateReceiver)> {
    while let Some(state) = self.next().await? {
      match state {
        State::authorizationStateWaitPhoneNumber => {
          self.client.execute(&fns::checkAuthenticationBotToken { token: token.into() }).await?;
        }
        other => return Err(Error::Auth(format!("unexpected bot auth state: {other:?}"))),
      }
    }

    Ok(self.finish())
  }
}
