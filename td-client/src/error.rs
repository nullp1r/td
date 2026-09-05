//! Request, transfer, authentication, and lifecycle failures.
//!
//! Failures with a waiting caller are returned through [`Result`]. Unsolicited
//! native diagnostics use [`on_error`](crate::runtime::on_error), not every
//! client's update queue. This crate chooses neither logging nor retry policy.
//!
//! A failed wait is not necessarily a failed side effect. For example, losing a
//! reply during teardown does not prove a message was never accepted. Inspect
//! the variant and the relevant operation's cancellation contract before retrying.
//! Batch sends expose independent inner results so partial success is preserved.

use std::result;

use td_types::enums::AuthorizationState;
use td_types::types;

use crate::message::MessageKey;

/// The result of a client operation; the default success value is `()`.
pub type Result<T = ()> = result::Result<T, Error>;

/// An inspectable client failure.
///
/// `TDLib` codes and messages are preserved rather than classified into a
/// library-defined retry policy. This enum does not represent every unsolicited
/// update or native log message.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// `TDLib` reported an error, preserving its original code and message.
  ///
  /// Returned for request failures or delivered as an unsolicited diagnostic
  /// through [`on_error`](crate::runtime::on_error).
  #[error("TDLib: {} {}", .0.code, .0.message)]
  Td(types::error),
  /// A request could not be serialized or native output could not be decoded.
  #[error("JSON: {0}")]
  Json(#[from] serde_json::Error),
  /// The bot helper encountered an authorization state it does not handle.
  #[error("unexpected auth state: {0:?}")]
  Auth(AuthorizationState),
  /// Token-triggered cleanup won according to the operation's cancellation rules.
  #[error("operation cancelled")]
  Cancelled,
  /// A tracked send failed; contains its temporary key and the native error.
  #[error("message {} in chat {} failed: {} {}", .0.message_id, .0.chat_id, .1.code, .1.message)]
  MessageFailed(MessageKey, types::error),
  /// A tracked temporary message was deleted by a non-cache update.
  #[error("message {} in chat {} was deleted while being sent", .0.message_id, .0.chat_id)]
  MessageDeleted(MessageKey),
  /// The native response did not have the shape required by this operation.
  #[error("unexpected TDLib response: {0}")]
  UnexpectedResponse(&'static str),
  /// The owner is unavailable, admission is closed, or a reply was abandoned during teardown.
  #[error("client disconnected")]
  Disconnected,
}
