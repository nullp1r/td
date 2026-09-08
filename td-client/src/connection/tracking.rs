// The receiver binds/settles; operation futures await and invoke callbacks.
// Registry entries never contain a borrowed callback or application update cache.

use std::future::{self, Future};

use tokio::sync::{oneshot, watch};

use td_types::enums::{Message, MessageContent, MessageSendingState, Messages, Update};
use td_types::traits::Function;
use td_types::{fns, types};

use crate::error::{Error, Result};
use crate::message::MessageKey;
use crate::transfer::{CancellationToken, Progress};

use super::{Connection, PendingReply, Registry};

pub type Sample = (usize, Progress);

pub struct PendingMessage {
  key: MessageKey,
  terminal: oneshot::Receiver<Result<types::message>>,
}

pub struct PendingMessages {
  pub pending: Vec<PendingMessage>,
  // One latest sample for the entire batch. No allocation without a callback
  // and a primary file; the initial zero watch value is not an observation.
  pub samples: Option<watch::Receiver<Sample>>,
}

enum Scope {
  Upload(usize),
  Download { offset: i64, limit: i64 },
}

pub struct Observation {
  file_id: i32,
  scope: Scope,
  samples: watch::Sender<Sample>,
}

impl Connection {
  pub async fn messages<F: Function>(&self, request: &F, progress: bool) -> Result<PendingMessages> {
    let (reply, response) = oneshot::channel();
    self.submit(request, PendingReply::Messages { progress, reply })?;
    response.await.map_err(|_| Error::Disconnected)?
  }

  pub fn observe_download(&self, request: &fns::downloadFile) -> watch::Receiver<Sample> {
    // Register before submission, including for immediately cached downloads.
    let (samples, receiver) = watch::channel((0, Progress::default()));
    let scope = Scope::Download { offset: request.offset, limit: request.limit };
    let mut registry = self.registry.lock().unwrap();
    registry.prune_observers();
    registry.file_observers.push(Observation { file_id: request.file_id, scope, samples });
    receiver
  }
}

impl PendingMessages {
  pub async fn finish(
    self,
    connection: &Connection,
    cancel: Option<&CancellationToken>,
    callback: Option<&mut (dyn FnMut(usize, Progress) + Send)>,
  ) -> Vec<Result<types::message>> {
    // Waiters are already bound. Sequential awaits order results, not native
    // sends; with_progress stays driven during cancellation cleanup as well.
    let completion = async {
      let mut results = Vec::with_capacity(self.pending.len());
      for message in self.pending {
        results.push(message.finish(connection, cancel).await);
      }
      results
    };
    with_progress(completion, self.samples, callback).await
  }
}

impl PendingMessage {
  pub async fn finish(mut self, connection: &Connection, cancel: Option<&CancellationToken>) -> Result<types::message> {
    tokio::select! {
      biased;
      result = &mut self.terminal => result.unwrap_or(Err(Error::Disconnected)),
      () = cancelled(cancel) => self.cancel(connection).await,
    }
  }

  async fn cancel(self, connection: &Connection) -> Result<types::message> {
    let Self { key, terminal } = self;
    let pending = || connection.registry.lock().unwrap().pending_messages.contains_key(&key);
    // Settlement removes the key before publishing its result. This prevents
    // knowingly deleting a completed send, not a server-side acceptance race.
    if pending() {
      let request = fns::deleteMessages { chat_id: key.chat_id, message_ids: vec![key.message_id], revoke: true };
      let deletion = connection.request(&request).await;
      // A terminal result wins if it arrived during deletion. Never delete a final ID.
      if let Err(error) = deletion
        && pending()
      {
        return Err(error);
      }
    }
    match terminal.await.map_err(|_| Error::Disconnected)? {
      Err(Error::MessageDeleted(_)) => Err(Error::Cancelled),
      result => result,
    }
  }
}

impl Registry {
  pub fn bind(&mut self, messages: impl IntoIterator<Item = types::message>, progress: bool) -> PendingMessages {
    let messages = messages.into_iter();
    let mut pending = Vec::with_capacity(messages.size_hint().0);
    let mut samples = None;
    if progress {
      self.prune_observers();
    }
    for (index, message) in messages.enumerate() {
      let key = MessageKey { chat_id: message.chat_id, message_id: message.id };
      if progress && let Some(file) = primary_file(&message) {
        let (sender, _) = samples.get_or_insert_with(|| watch::channel((0, Progress::default())));
        self.file_observers.push(Observation {
          file_id: file.id, //.
          scope: Scope::Upload(index),
          samples: sender.clone(),
        });
      }
      let (reply, terminal) = oneshot::channel();
      // Live temporary keys are unique. Abandoned waiters remain until terminal
      // updates or teardown. Already-final responses share the completion path.
      if let Some(MessageSendingState::messageSendingStatePending(_)) = message.sending_state {
        self.pending_messages.insert(key, reply);
      } else {
        let _ = reply.send(Ok(message));
      }
      pending.push(PendingMessage { key, terminal });
    }
    PendingMessages { pending, samples: samples.map(|(_, receiver)| receiver) }
  }

  fn prune_observers(&mut self) {
    // Registration also prunes files that never emit another update after the
    // operation drops. Dropping observation performs no native cancellation.
    self.file_observers.retain(|observer| observer.samples.receiver_count() != 0);
  }

  pub fn observe_file(&mut self, file: &types::file) {
    // ponytail: O(active observers), contiguous and allocation-light for small sets.
    // Restore keyed routing if deployments need many simultaneous transfers.
    self.file_observers.retain(|observer| {
      if observer.file_id != file.id {
        return observer.samples.receiver_count() != 0;
      }
      let sample = match observer.scope {
        Scope::Upload(index) => (
          index,
          Progress {
            current: file.remote.uploaded_size, //.
            total: file.size.max(file.expected_size),
          },
        ),
        Scope::Download { offset, limit } => (0, Progress::download(file, offset, limit)),
      };
      observer.samples.send(sample).is_ok()
    });
  }

  pub fn observe_message(&mut self, update: &Update) {
    match update {
      Update::updateMessageSendSucceeded(update) => {
        let key = MessageKey { chat_id: update.message.chat_id, message_id: update.old_message_id };
        self.settle(key, || Ok(update.message.clone()));
      }
      Update::updateMessageSendFailed(update) => {
        let key = MessageKey { chat_id: update.message.chat_id, message_id: update.old_message_id };
        self.settle(key, || Err(Error::MessageFailed(key, update.error.clone())));
      }
      Update::updateDeleteMessages(update) if !update.from_cache => {
        for &message_id in &update.message_ids {
          let key = MessageKey { chat_id: update.chat_id, message_id };
          self.settle(key, || Err(Error::MessageDeleted(key)));
        }
      }
      _ => {}
    }
  }

  fn settle(&mut self, key: MessageKey, result: impl FnOnce() -> Result<types::message>) {
    // Clone only for live waiters: the original update goes to the application.
    if let Some(reply) = self.pending_messages.remove(&key)
      && !reply.is_closed()
    {
      let _ = reply.send(result());
    }
  }
}

impl Progress {
  fn download(file: &types::file, offset: i64, limit: i64) -> Self {
    let local = &file.local;
    let available = (file.size.max(file.expected_size) - offset).max(0);
    let total = if limit > 0 { available.min(limit) } else { available };
    let prefix_end = local.download_offset + local.downloaded_prefix_size;
    // Bytes beyond a missing prefix do not count. Range arithmetic must fit i64;
    // clamping describes overlap, not sanitization of invalid request arguments.
    let current = if local.download_offset <= offset { (prefix_end - offset).max(0) } else { 0 };
    // Unknown size stays total=0, but an explicit limit still bounds current.
    let bound = match (total, limit) {
      (1.., _) => total,
      (_, 1..) => limit,
      _ => current,
    };
    Self { current: current.min(bound), total }
  }
}

pub async fn with_progress<T>(
  completion: impl Future<Output = T>,
  samples: Option<watch::Receiver<Sample>>,
  callback: Option<&mut (dyn FnMut(usize, Progress) + Send)>,
) -> T {
  tokio::pin!(completion);
  if let (Some(mut samples), Some(callback)) = (samples, callback) {
    loop {
      tokio::select! {
        biased;
        // Completion wins; the callback contract promises no final-sample flush.
        result = &mut completion => return result,
        changed = samples.changed() => {
          let Ok(()) = changed else { break };
          // Release the watch borrow before invoking application code.
          let (index, progress) = *samples.borrow_and_update();
          callback(index, progress);
        }
      }
    }
  }
  // A closed observer is not completion. Do not spin on its ready error branch.
  completion.await
}

pub async fn cancelled(token: Option<&CancellationToken>) {
  match token {
    Some(token) => token.cancelled().await,
    None => future::pending().await,
  }
}

fn primary_file(message: &types::message) -> Option<&types::file> {
  // Track the primary payload, not thumbnails or a recursive walk of generated types.
  match &message.content {
    MessageContent::messageAnimation(content) => Some(&content.animation.animation),
    MessageContent::messageAudio(content) => Some(&content.audio.audio),
    MessageContent::messageDocument(content) => Some(&content.document.document),
    MessageContent::messagePhoto(content) => content.photo.sizes.last().map(|size| &size.photo),
    MessageContent::messageSticker(content) => Some(&content.sticker.sticker),
    MessageContent::messageVideo(content) => Some(&content.video.video),
    MessageContent::messageVideoNote(content) => Some(&content.video_note.video),
    MessageContent::messageVoiceNote(content) => Some(&content.voice_note.voice),
    _ => None,
  }
}

pub fn parse_messages(raw: &[u8], kind: &str) -> Result<impl Iterator<Item = types::message>> {
  // Keep the singleton inline instead of allocating a temporary binding vector.
  let (single, batch) = match kind {
    "message" => {
      let Message::message(message) = serde_json::from_slice(raw)?;
      (Some(message), Vec::new())
    }
    "messages" => {
      let Messages::messages(messages) = serde_json::from_slice(raw)?;
      (None, messages.messages.ok_or(Error::UnexpectedResponse("batch response omitted messages"))?)
    }
    _ => return Err(Error::UnexpectedResponse("tracked send returned an unexpected type")),
  };
  Ok(single.into_iter().chain(batch))
}

#[cfg(test)]
mod tests;
