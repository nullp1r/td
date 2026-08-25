//! Internal correlation for authoritative `sendMessage` completion.
//!
//! `TDLib` first returns a local message with a temporary ID, then emits
//! `updateMessageSendSucceeded`, `updateMessageSendFailed`, or a non-cache
//! `updateDeleteMessages`. The generic request table can correlate only the direct
//! response's `@extra`, so this module bridges the two identities:
//!
//! 1. register a send by its `@extra` value before calling `td_send`;
//! 2. bind it to `(chat_id, temporary_message_id)` while the receiver handles the
//!    direct response and before the request future is woken;
//! 3. complete its one-shot when an authoritative terminal update names that
//!    temporary ID;
//! 4. still enqueue the original terminal update for the application.
//!
//! The direct-response-before-terminal-update ordering is derived from `TDLib`'s
//! implementation, not guaranteed by the generated schema. Re-audit it on every
//! `TDLib` upgrade. All registry locks are short and synchronous; no guard crosses
//! an `.await`.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use tokio::sync::oneshot;
use tokio::time::timeout_at;

use td_types::enums::{Message, Update};
use td_types::{fns, types};

use super::{ClientState, Error, MessageReply, PendingReply, Result, parse_td_error};

/// Receives the terminal outcome for one registered message send.
pub type MessageSendCompletion = oneshot::Receiver<Result<MessageSendOutcome>>;

impl ClientState {
  /// Runs a tracked `sendMessage`, optionally with compensating deadline deletion.
  pub async fn send_message(&self, request: &fns::sendMessage, deadline: Option<Instant>) -> Result<types::message> {
    if let Some(types::messageSendOptions { only_preview: true, .. }) = &request.options {
      return Err(Error::MessagePreview);
    }

    let (extra, serialized) = self.serialize_correlated_request(request)?;
    let mut result = self.message_sends.lock().unwrap().register(extra);
    let guard = MessageSendGuard { registration_id: extra, registry: &self.message_sends };
    let (reply, response) = oneshot::channel();
    self.register_and_send_request(extra, &serialized, false, PendingReply::Message(reply))?;
    let types::message { chat_id, id: message_id, .. } = response.await.map_err(|_| Error::Disconnected)??;
    let pending = MessageKey { chat_id, message_id };

    if let Some(deadline) = deadline {
      match timeout_at(deadline.into(), &mut result).await {
        Ok(result) => return result.map_err(|_| Error::Disconnected)??.into_result(pending),
        // A terminal update may win as timeout_at expires. Check registration
        // under the registry lock before issuing compensating deletion.
        Err(_) if guard.is_pending() => {
          self.cancel_message_send(pending, &mut result).await?;
          return Err(Error::MessageDeadline { chat_id, message_id });
        }
        Err(_) => {}
      }
    }

    result.await.map_err(|_| Error::Disconnected)??.into_result(pending)
  }

  /// Deletes a timed-out temporary message and any final ID that won the race.
  async fn cancel_message_send(&self, pending: MessageKey, result: &mut MessageSendCompletion) -> Result {
    self.delete_message(pending).await?;
    let outcome = result.await.map_err(|_| Error::Disconnected)??;
    // Deleting the temporary ID normally yields Deleted. Success can already be
    // in flight, in which case its different final ID also needs deletion.
    if let MessageSendOutcome::Succeeded(types::message { chat_id, id: message_id, .. }) = outcome {
      self.delete_message(MessageKey { chat_id, message_id }).await?;
    }
    Ok(())
  }

  /// Revokes one message ID through the ordinary correlated request path.
  async fn delete_message(&self, MessageKey { chat_id, message_id }: MessageKey) -> Result {
    let request = fns::deleteMessages { chat_id, message_ids: vec![message_id], revoke: true };
    self.execute_request(&request, false).await?;
    Ok(())
  }

  /// Parses and binds a `sendMessage` direct response before waking its requester.
  pub fn complete_message_request(&self, registration_id: u64, r#type: &str, raw: &[u8], reply: MessageReply) {
    let response = match r#type {
      "error" => Err(parse_td_error(raw)),
      _ => self.parse_and_bind_message_response(registration_id, raw),
    };
    if response.is_err() {
      self.message_sends.lock().unwrap().unregister(registration_id);
    }
    let _ = reply.send(response);
  }

  /// Decodes the temporary message and installs its terminal-update lookup key.
  fn parse_and_bind_message_response(&self, registration_id: u64, raw: &[u8]) -> Result<types::message> {
    let Message::message(message @ types::message { chat_id, id: message_id, .. }) = serde_json::from_slice(raw)?;
    // TDLib currently emits this correlated response before its terminal update.
    // Binding on the receiver thread makes the key visible before either the
    // requester wakes or the receiver performs its next native receive.
    self.message_sends.lock().unwrap().bind(registration_id, MessageKey { chat_id, message_id })?;
    Ok(message)
  }
}

/// The `TDLib` identity of a temporary or final message within one chat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MessageKey {
  /// Chat containing the message; message IDs are unique only within this scope.
  pub chat_id: i64,
  /// Temporary or final `TDLib` message ID.
  pub message_id: i64,
}

/// Two-stage correlation table for all tracked sends belonging to one client.
///
/// `registrations` owns each completion sender from request serialization onward.
/// Once the direct response arrives, `registrations_by_message` maps its temporary
/// key back to the same registration. Both entries are inserted and removed
/// together under the containing `ClientState` mutex.
#[derive(Default)]
pub struct Registry {
  /// Pending sends keyed by their original `@extra` registration ID.
  registrations: HashMap<u64, PendingMessageSend>,
  /// Bound temporary-message keys mapped back to registration IDs.
  registrations_by_message: HashMap<MessageKey, u64>,
}

/// Completion state retained between registration and a terminal update.
struct PendingMessageSend {
  /// Temporary-message key, absent until the direct response is bound.
  temporary_message: Option<MessageKey>,
  /// One-shot used by the public send future to await the terminal outcome.
  completion: oneshot::Sender<Result<MessageSendOutcome>>,
}

/// Authoritative terminal states recognized for a tracked message send.
#[derive(Debug)]
pub enum MessageSendOutcome {
  /// `TDLib` replaced the temporary message with this final sent message.
  Succeeded(types::message),
  /// `TDLib` reported a terminal send failure and its associated message.
  Failed(types::updateMessageSendFailed),
  /// A non-cache deletion removed the temporary message before another result.
  Deleted,
}

impl MessageSendOutcome {
  /// Converts a terminal update into the public message result and contextual error.
  fn into_result(self, MessageKey { chat_id, message_id }: MessageKey) -> Result<types::message> {
    match self {
      Self::Succeeded(message) => Ok(message),
      Self::Failed(update) => Err(Error::MessageFailed(Box::new(update))),
      Self::Deleted => Err(Error::MessageDeleted { chat_id, message_id }),
    }
  }
}

impl Registry {
  /// Registers `registration_id` and returns its terminal-outcome receiver.
  pub fn register(&mut self, registration_id: u64) -> MessageSendCompletion {
    let (completion, receiver) = oneshot::channel();
    self.registrations.insert(registration_id, PendingMessageSend { temporary_message: None, completion });
    receiver
  }

  /// Associates a registered `@extra` value with its temporary message key.
  ///
  /// A missing registration means its public future was dropped before the direct
  /// response arrived, so there is no observer left to bind. Reusing a live key is
  /// an explicit correlation failure rather than silently replacing its waiter.
  pub fn bind(&mut self, registration_id: u64, temporary_message: MessageKey) -> Result {
    let Some(registration) = self.registrations.get_mut(&registration_id) else { return Ok(()) };
    let Entry::Vacant(message_entry) = self.registrations_by_message.entry(temporary_message) else {
      let MessageKey { chat_id, message_id } = temporary_message;
      return Err(Error::MessageCorrelation { chat_id, message_id });
    };
    registration.temporary_message = Some(temporary_message);
    message_entry.insert(registration_id);
    Ok(())
  }

  /// Observes terminal message updates without consuming or changing them.
  pub fn observe(&mut self, update: &Update) {
    match *update {
      Update::updateMessageSendSucceeded(ref upd) => {
        let &types::updateMessageSendSucceeded { ref message, old_message_id: message_id } = upd;
        let temporary_message = MessageKey { chat_id: message.chat_id, message_id };
        self.complete(temporary_message, MessageSendOutcome::Succeeded(message.clone()));
      }
      Update::updateMessageSendFailed(ref upd) => {
        let &types::updateMessageSendFailed { ref message, old_message_id: message_id, .. } = upd;
        let temporary_message = MessageKey { chat_id: message.chat_id, message_id };
        self.complete(temporary_message, MessageSendOutcome::Failed(upd.clone()));
      }
      Update::updateDeleteMessages(ref upd) if !upd.from_cache => {
        let &types::updateDeleteMessages { chat_id, ref message_ids, .. } = upd;
        for &message_id in message_ids {
          self.complete(MessageKey { chat_id, message_id }, MessageSendOutcome::Deleted);
        }
      }
      _ => {}
    }
  }

  /// Completes and removes the registration bound to `temporary_message`.
  fn complete(&mut self, temporary_message: MessageKey, outcome: MessageSendOutcome) {
    let Some(registration_id) = self.registrations_by_message.remove(&temporary_message) else { return };
    let Some(PendingMessageSend { completion, .. }) = self.registrations.remove(&registration_id) else { return };
    let _ = completion.send(Ok(outcome));
  }

  /// Reports whether deadline cancellation still has a live registration to cancel.
  fn is_registered(&self, registration_id: u64) -> bool {
    self.registrations.contains_key(&registration_id)
  }

  /// Removes a local observer and its bound temporary-message index, if any.
  fn unregister(&mut self, registration_id: u64) {
    let registration = self.registrations.remove(&registration_id);
    if let Some(PendingMessageSend { temporary_message: Some(temporary_message), .. }) = registration {
      self.registrations_by_message.remove(&temporary_message);
    }
  }

  /// Drops all send waiters and indexes when their client disconnects.
  pub fn disconnect(&mut self) {
    self.registrations.clear();
    self.registrations_by_message.clear();
  }

  /// Fails all sends after update JSON can no longer be correlated reliably.
  pub fn fail_json(&mut self, error: &Arc<serde_json::Error>) {
    for (_, PendingMessageSend { completion, .. }) in self.registrations.drain() {
      let _ = completion.send(Err(Error::Json(Arc::clone(error))));
    }
    self.registrations_by_message.clear();
  }
}

/// Scope guard that unregisters tracking when the public send future is dropped.
///
/// This is local cancellation only: dropping it never calls `TDLib` or deletes a
/// message. It also removes tracking after every normal return path.
struct MessageSendGuard<'a> {
  /// Original request `@extra` value used as the registration ID.
  registration_id: u64,
  /// Per-client registry from which the entry must be removed.
  registry: &'a Mutex<Registry>,
}

impl MessageSendGuard<'_> {
  /// Tests the timeout-vs-terminal-update race under the registry lock.
  fn is_pending(&self) -> bool {
    self.registry.lock().unwrap().is_registered(self.registration_id)
  }
}

impl Drop for MessageSendGuard<'_> {
  fn drop(&mut self) {
    self.registry.lock().unwrap().unregister(self.registration_id);
  }
}
