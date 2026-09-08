//! Session setup conveniences built on the public `td-client` API.
//!
//! The returned [`Session`] remains the sole lifecycle owner. Consume application
//! updates after authorization and call [`Session::close`] before dropping it.

use std::path::Path;

use td_types::{enums::AuthorizationState, fns};

use crate::{Session, client};

/// A failure while opening or authorizing a bot session.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// Native, serialization or lifecycle failure from the client.
  #[error(transparent)]
  Client(#[from] client::Error),
  /// Authorization requires an interaction the bot-token flow does not support.
  #[error("unexpected bot authorization state: {0:?}")]
  Authorization(AuthorizationState),
}

/// Opens a session and waits for bot authorization.
///
/// An already-authorized directory is reused without comparing its account with
/// `token`. Use a separate directory when switching accounts. Unexpected states
/// return an error; non-auth updates remain buffered for [`Session::recv`].
/// On authorization failure, graceful closure is attempted before the original
/// error is returned. Drive this future to completion: dropping it abandons cleanup.
pub async fn bot(params: fns::setTdlibParameters, token: &str) -> Result<Session, Error> {
  let mut session = Session::open(params).await?;
  if let Err(error) = authorize_bot(&mut session, token).await {
    let _ = session.close().await;
    return Err(error);
  }
  Ok(session)
}

async fn authorize_bot(session: &mut Session, token: &str) -> Result<(), Error> {
  let client = session.client();
  loop {
    match session.recv_auth().await {
      AuthorizationState::authorizationStateReady => return Ok(()),
      AuthorizationState::authorizationStateWaitTdlibParameters => {}
      AuthorizationState::authorizationStateWaitPhoneNumber => {
        let request = fns::checkAuthenticationBotToken { token: token.into() };
        client.send(&request).await?;
      }
      state => return Err(Error::Authorization(state)),
    }
  }
}

/// Builds editable `TDLib` parameters with local database and file directories.
///
/// Uses `directory/db` and `directory/files`; this function does not create them.
/// Enables file, chat-info, and message databases. Language defaults to `en`,
/// device model to `Server`, and application version to this crate's version.
/// Other fields retain their generated defaults, including the encryption key.
///
/// These are conveniences, not validated configuration or a security policy.
/// Disable databases you do not need and set application metadata/encryption
/// explicitly where appropriate. Paths are converted with lossy UTF-8 conversion.
///
/// # Examples
///
/// ```
/// let mut params = tdx::session::parameters(12345, "api hash", "session");
/// params.use_message_database = false;
/// params.device_model = "My application".into();
/// assert!(!params.use_message_database);
/// ```
pub fn parameters(api_id: i32, api_hash: impl Into<String>, directory: impl AsRef<Path>) -> fns::setTdlibParameters {
  let directory = directory.as_ref();
  fns::setTdlibParameters {
    api_id,
    api_hash: api_hash.into(),
    database_directory: directory.join("db").to_string_lossy().into_owned(),
    files_directory: directory.join("files").to_string_lossy().into_owned(),
    use_file_database: true,
    use_chat_info_database: true,
    use_message_database: true,
    system_language_code: "en".into(),
    device_model: "Server".into(),
    application_version: env!("CARGO_PKG_VERSION").into(),
    ..Default::default()
  }
}
