//! Detached request client for concurrent Telegram operations.
//!
//! Obtain a cloneable [`Client`] from [`Session::client`](crate::session::Session::client).
//! Clients issue requests concurrently without borrowing update consumption.
//!
//! Direct function calls use [`Client::send`]. Normal message sends with terminal
//! delivery tracking use [`Client::track`] and [`Client::track_all`]. File
//! downloads use [`Client::download`].

use std::sync::{Arc, Weak};

use td_types::traits::Function;

use crate::connection::Connection;
use crate::error::{Error, Result};

/// A cloneable, non-owning capability for requests to one [`Session`](crate::session::Session).
///
/// Clones refer to the same session and may submit requests concurrently. They
/// cannot receive updates or initiate graceful closure. Keeping a client alive
/// does not keep the session operational: requests after owner loss or closure
/// admission fail with [`Error::Disconnected`].
///
/// Methods are asynchronous and do not submit until their futures are polled.
#[derive(Debug, Clone)]
pub struct Client(pub(crate) Weak<Connection>);

impl Client {
  /// Sends a generated function and returns its direct `TDLib` response.
  ///
  /// `F::Return` is supplied by `td-types::traits::Function`. This is the general
  /// entry point for the generated API, including getters and message edits.
  /// For normal sends, the direct response can still describe a pending temporary
  /// message; use [`track`](Self::track) or [`track_all`](Self::track_all) to
  /// wait for terminal outcomes.
  ///
  /// # Errors
  ///
  /// Returns [`Error::Json`] for serialization/decoding failures, [`Error::Td`]
  /// for `TDLib` error responses, and [`Error::Disconnected`] when the session
  /// cannot accept the request or its reply is abandoned during teardown.
  ///
  /// # Cancellation
  ///
  /// Dropping this future stops waiting, not the submitted native request.
  /// A timeout therefore does not establish that a side effect failed to occur.
  /// Retries and native cancellation are application decisions.
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use td_client::Client;
  /// # use td_client::Result;
  /// use td_types::{enums::User, fns};
  ///
  /// # async fn identify(client: &Client) -> Result {
  /// let User::user(user) = client.send(&fns::getMe {}).await?;
  /// println!("{}", user.first_name);
  /// # Ok(())
  /// # }
  /// ```
  pub async fn send<F: Function>(&self, request: &F) -> Result<F::Return> {
    self.connection()?.request(request).await
  }

  pub(crate) fn connection(&self) -> Result<Arc<Connection>> {
    self.0.upgrade().ok_or(Error::Disconnected)
  }
}
