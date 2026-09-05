//! Terminal message-send results and temporary message identities.
//!
//! A direct `TDLib` send response can contain a local temporary message. Successful
//! submission is not necessarily successful delivery. [`Sender::send_message`]
//! and [`Sender::send_messages`] bind pending identities before the request wakes,
//! then wait for send-success, send-failure, or non-cache deletion updates.
//! Every original update remains available through [`Client::recv`](crate::client::Client::recv).
//!
//! # Supported requests
//!
//! Use tracked methods for normal sends, such as `sendMessage`,
//! `sendMessageAlbum`, and normal non-preview `forwardMessages` calls.
//! Generic return-type bounds are not a promise to track every function returning
//! `Message` or `Messages`. Previews, getters, and edits belong on
//! [`Sender::send`]; an unsupported pending-looking response may never finish.
//! Already-final direct responses return without waiting for a terminal update.
//!
//! Edits use their direct correlated response. An `updateMessageEdited` event
//! cannot establish which edit request finished or whether another edit failed.
//!
//! # Cancellation
//!
//! A borrowed [`CancellationToken`] asks the operation to delete pending temporary
//! messages. It does not retract the initial request: even a pre-cancelled token
//! waits for the direct response so the temporary identity can be bound.
//!
//! An authoritative success already observed wins and returns the final message.
//! Deletion is requested only for a temporary ID still registered as pending;
//! this library never explicitly deletes a successful final ID. This is **not
//! server-atomic**: `TDLib` can itself delete a concurrently accepted message after
//! removing its pending record. Do not interpret cancellation as a guarantee that
//! the message was never visible or that a racing successful message remains.
//!
//! Dropping the future merely abandons local observation. Submitted native work
//! continues; token-triggered cleanup only runs while the future is driven.
//! Reusing a cancelled token asks every subsequent operation using it to cancel.
//!
//! If an application stop signal wins a race, cancel the token and keep awaiting
//! the same send future so native cleanup can finish:
//!
//! ```no_run
//! # use std::future::Future;
//! # use td_client::client::Sender;
//! # use td_client::error::Result;
//! # use td_client::transfer::CancellationToken;
//! # use td_types::{fns, types};
//! # async fn send_until_stop(
//! #   sender: &Sender, request: &fns::sendMessage, stop: impl Future<Output = ()>,
//! # ) -> Result<types::message> {
//! let cancel = CancellationToken::new();
//! let sending = sender.send_message(request, Some(&cancel), None);
//! tokio::pin!(sending);
//! tokio::select! {
//!   result = &mut sending => result,
//!   () = stop => {
//!     cancel.cancel();
//!     sending.await
//!   }
//! }
//! # }
//! ```
//!
//! # Upload measurements
//!
//! Pass an optional borrowed callback to observe primary media files. Supported
//! payloads are animations, audio, documents, photos (the last returned size),
//! stickers, videos, video notes, and voice notes. Thumbnails and recursive
//! attachment traversal are not tracked. There is no preliminary-upload API.
//!
//! Samples can coalesce across album items; neither a callback for every item nor
//! a final 100% sample is guaranteed. See [`Progress`] and the
//! [transfer guide](crate::transfer) for the common measurement contract.

use td_types::enums::{Message, Messages};
use td_types::traits::Function;
use td_types::types;

use crate::client::Sender;
use crate::connection::tracking::with_progress;
use crate::error::{Error, Result};
use crate::transfer::{CancellationToken, Progress};

/// The chat and temporary message ID identifying a tracked send failure.
///
/// Message IDs are scoped to a chat, so both fields are required. Keys carried
/// by [`Error::MessageFailed`] or [`Error::MessageDeleted`] refer to the
/// temporary send identity, not a replacement successful final message ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
  /// The chat containing the temporary message.
  pub chat_id: i64,
  /// The temporary message ID assigned by `TDLib`.
  pub message_id: i64,
}

impl Sender {
  /// Sends one normal message and waits for its terminal result.
  ///
  /// Accepts a generated function returning `td-types::enums::Message`, subject
  /// to the [normal-send contract](crate::message#supported-requests). The result
  /// is the final concrete message, not merely the initial pending response.
  /// Internal tracking does not depend on the application receiving updates.
  ///
  /// `progress` receives `(0, measurement)`; it runs synchronously on the task
  /// polling this future and must not block or panic. See [`Progress`] for
  /// coalescing and unknown totals.
  ///
  /// # Errors
  ///
  /// Returns direct-request errors, [`Error::MessageFailed`] on terminal send
  /// failure, or [`Error::MessageDeleted`] on non-cache deletion. Token-driven
  /// deletion maps to [`Error::Cancelled`]. Native deletion failures are returned
  /// while the message remains pending; terminal outcomes or teardown can win
  /// during deletion. Teardown can produce [`Error::Disconnected`]; an unexpected
  /// direct response produces
  /// [`Error::UnexpectedResponse`] or [`Error::Json`].
  ///
  /// # Cancellation
  ///
  /// See the [cancellation contract](crate::message#cancellation). Dropping this
  /// future performs no native cancellation and does not undo a sent message.
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use td_client::client::Sender;
  /// # use td_client::error::Result;
  /// use td_types::{fns, types};
  ///
  /// # async fn greet(sender: &Sender, chat_id: i64) -> Result {
  /// let text = types::formattedText { text: "Hello!".into(), ..Default::default() };
  /// let content = types::inputMessageText { text, ..Default::default() };
  /// let request = fns::sendMessage {
  ///   chat_id,
  ///   input_message_content: content.into(),
  ///   ..Default::default()
  /// };
  /// let message = sender.send_message(&request, None, None).await?;
  /// println!("Sent message {}", message.id);
  /// # Ok(())
  /// # }
  /// ```
  pub async fn send_message<F: Function<Return = Message>>(
    &self,
    request: &F,
    cancel: Option<&CancellationToken>,
    progress: Option<&mut (dyn FnMut(usize, Progress) + Send)>,
  ) -> Result<types::message> {
    let connection = self.connection()?;
    let batch = connection.messages(request, progress.is_some()).await?;
    let [message] = batch.pending.try_into().map_err(|_| Error::UnexpectedResponse("expected one message"))?;
    with_progress(message.finish(&connection, cancel), batch.samples, progress).await
  }

  /// Sends a normal-message batch and returns individual terminal results.
  ///
  /// Accepts a generated function returning `td-types::enums::Messages`, subject
  /// to the [normal-send contract](crate::message#supported-requests). It is not
  /// restricted to albums. Results retain direct-response order, not completion
  /// order; an empty batch returns an empty vector.
  ///
  /// The outer result describes submission and direct-response handling. Once
  /// bound, each message has its own result: one terminal failure does not erase
  /// successful messages elsewhere in the batch.
  ///
  /// `progress` receives the zero-based direct-response item index and a
  /// measurement. One shared observation channel coalesces samples across items;
  /// intermediate callbacks for every item are not guaranteed.
  ///
  /// # Errors
  ///
  /// The outer result reports direct-request, decoding, and pre-binding
  /// disconnection errors. Each inner result has the terminal/cancellation errors
  /// documented on [`send_message`](Self::send_message).
  ///
  /// # Cancellation
  ///
  /// One token applies to the whole batch, not one item. Pending items are awaited
  /// and, when requested, cancelled sequentially in response order; later items
  /// may finish before their cancellation is attempted. Successful results remain
  /// successful. There is no all-or-nothing send or rollback guarantee.
  ///
  /// Dropping this future abandons observation of the whole batch without native
  /// cancellation. See the [shared cancellation contract](crate::message#cancellation).
  ///
  /// # Examples
  ///
  /// Handle partial failure instead of assuming the outer `Ok` means every send
  /// succeeded:
  ///
  /// ```no_run
  /// # use td_client::client::Sender;
  /// # use td_client::error::Result;
  /// # use td_types::fns;
  /// # async fn album(sender: &Sender, request: &fns::sendMessageAlbum) -> Result {
  /// let results = sender.send_messages(request, None, None).await?;
  /// for (index, result) in results.into_iter().enumerate() {
  ///   match result {
  ///     Ok(message) => println!("Item {index}: message {}", message.id),
  ///     Err(error) => eprintln!("Item {index}: {error}"),
  ///   }
  /// }
  /// # Ok(())
  /// # }
  /// ```
  pub async fn send_messages<F: Function<Return = Messages>>(
    &self,
    request: &F,
    cancel: Option<&CancellationToken>,
    progress: Option<&mut (dyn FnMut(usize, Progress) + Send)>,
  ) -> Result<Vec<Result<types::message>>> {
    let connection = self.connection()?;
    let batch = connection.messages(request, progress.is_some()).await?;
    Ok(batch.finish(&connection, cancel, progress).await)
  }
}
