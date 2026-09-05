// The receiver binds/settles; operation futures await and invoke callbacks.
// Registry entries never contain a borrowed callback or application update cache.
use std::future::{self, Future};

use td_types::enums::{Message, MessageContent, MessageSendingState, Messages, Update};
use td_types::traits::Function;
use td_types::{fns, types};
use tokio::sync::{oneshot, watch};

use super::{Connection, PendingReply, Registry};
use crate::error::{Error, Result};
use crate::message::Key;
use crate::transfer::{CancellationToken, Progress};

pub type Sample = (usize, Progress);

pub struct PendingMessage {
  key: Key,
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
      let key = Key { chat_id: message.chat_id, message_id: message.id };
      if progress && let Some(file) = primary_file(&message) {
        let (sender, _) = samples.get_or_insert_with(|| watch::channel((0, Progress::default())));
        self.file_observers.push(Observation { file_id: file.id, scope: Scope::Upload(index), samples: sender.clone() });
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
        Scope::Upload(index) => (index, Progress { current: file.remote.uploaded_size, total: file.size.max(file.expected_size) }),
        Scope::Download { offset, limit } => (0, Progress::download(file, offset, limit)),
      };
      observer.samples.send(sample).is_ok()
    });
  }

  pub fn observe_message(&mut self, update: &Update) {
    match update {
      Update::updateMessageSendSucceeded(update) => {
        let key = Key { chat_id: update.message.chat_id, message_id: update.old_message_id };
        self.settle(key, || Ok(update.message.clone()));
      }
      Update::updateMessageSendFailed(update) => {
        let key = Key { chat_id: update.message.chat_id, message_id: update.old_message_id };
        self.settle(key, || Err(Error::MessageFailed(key, update.error.clone())));
      }
      Update::updateDeleteMessages(update) if !update.from_cache => {
        for &message_id in &update.message_ids {
          let key = Key { chat_id: update.chat_id, message_id };
          self.settle(key, || Err(Error::MessageDeleted(key)));
        }
      }
      _ => {}
    }
  }

  fn settle(&mut self, key: Key, result: impl FnOnce() -> Result<types::message>) {
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
mod tests {
  use std::assert_matches;
  use std::future::Future;
  use std::pin::pin;
  use std::task::{Context, Poll, Waker};
  use std::time::Duration;

  use tokio::task::yield_now;
  use tokio::time::timeout;

  use super::*;

  #[test]
  fn primary_file_mapping_covers_supported_media() {
    let file = |id| types::file { id, ..Default::default() };
    let primary_id = |content| {
      let message = types::message { content, ..Default::default() };
      primary_file(&message).map(|file| file.id)
    };

    let animation = types::animation { animation: file(1), ..Default::default() };
    assert_eq!(primary_id(types::messageAnimation { animation, ..Default::default() }.into()), Some(1));
    let audio = types::audio { audio: file(2), ..Default::default() };
    assert_eq!(primary_id(types::messageAudio { audio, ..Default::default() }.into()), Some(2));
    let document = types::document { document: file(3), ..Default::default() };
    assert_eq!(primary_id(types::messageDocument { document, ..Default::default() }.into()), Some(3));
    let sticker = types::sticker { sticker: file(4), ..Default::default() };
    assert_eq!(primary_id(types::messageSticker { sticker, ..Default::default() }.into()), Some(4));
    let video = types::video { video: file(5), ..Default::default() };
    assert_eq!(primary_id(types::messageVideo { video, ..Default::default() }.into()), Some(5));
    let video_note = types::videoNote { video: file(6), ..Default::default() };
    assert_eq!(primary_id(types::messageVideoNote { video_note, ..Default::default() }.into()), Some(6));
    let voice_note = types::voiceNote { voice: file(7), ..Default::default() };
    assert_eq!(primary_id(types::messageVoiceNote { voice_note, ..Default::default() }.into()), Some(7));

    let sizes = [8, 9].map(|id| types::photoSize { photo: file(id), ..Default::default() });
    let photo = types::photo { sizes: sizes.into(), ..Default::default() };
    assert_eq!(primary_id(types::messagePhoto { photo, ..Default::default() }.into()), Some(9));

    assert_eq!(primary_id(types::messageText::default().into()), None);
  }

  fn pending(id: i64) -> types::message {
    types::message { id, chat_id: 9, sending_state: Some(types::messageSendingStatePending::default().into()), ..Default::default() }
  }

  fn document(id: i32, size: i64) -> MessageContent {
    let file = types::file { id, size, ..Default::default() };
    let document = types::document { document: file, ..Default::default() };
    types::messageDocument { document, ..Default::default() }.into()
  }

  #[tokio::test]
  async fn batch_preserves_response_order_and_independent_outcomes() {
    let (connection, _updates) = Connection::fixture();
    let batch = {
      let mut registry = connection.registry.lock().unwrap();
      let batch = registry.bind(vec![pending(10), pending(11), pending(12)], false);
      let message = types::message { id: 21, ..pending(11) };
      registry.observe_message(&types::updateMessageSendSucceeded { old_message_id: 11, message }.into());
      let error = types::error { code: 400, message: "failed".into() };
      registry.observe_message(&types::updateMessageSendFailed { old_message_id: 10, message: pending(10), error }.into());
      let deletion = |from_cache| types::updateDeleteMessages { chat_id: 9, message_ids: vec![12], from_cache, ..Default::default() }.into();
      registry.observe_message(&deletion(true));
      assert!(registry.pending_messages.contains_key(&Key { chat_id: 9, message_id: 12 }));
      registry.observe_message(&deletion(false));
      batch
    };
    let results = timeout(Duration::from_secs(1), batch.finish(&connection, None, None)).await.unwrap();
    let [first, second, third]: [Result<types::message>; 3] = results.try_into().unwrap();
    assert_matches!(first, Err(Error::MessageFailed(Key { message_id: 10, .. }, error)) if error.code == 400);
    assert_matches!(second, Ok(types::message { id: 21, .. }));
    assert_matches!(third, Err(Error::MessageDeleted(Key { message_id: 12, .. })));
  }

  #[tokio::test]
  async fn authoritative_success_beats_cancellation_without_synthetic_progress() {
    let (connection, _updates) = Connection::fixture();
    let batch = {
      let mut registry = connection.registry.lock().unwrap();
      let message = types::message { content: document(7, 100), ..pending(10) };
      let batch = registry.bind(vec![message], true);
      assert!(registry.file_observers.iter().any(|observer| observer.file_id == 7));
      let message = types::message { id: 20, content: document(8, 120), ..pending(10) };
      registry.observe_message(&types::updateMessageSendSucceeded { old_message_id: 10, message }.into());
      batch
    };
    let cancel = CancellationToken::new();
    cancel.cancel();
    let mut samples = Vec::new();
    let mut callback = |index, progress| samples.push((index, progress));
    let results = timeout(Duration::from_secs(1), batch.finish(&connection, Some(&cancel), Some(&mut callback))).await.unwrap();
    let [result]: [Result<types::message>; 1] = results.try_into().unwrap();
    assert_matches!(result, Ok(types::message { id: 20, .. }));
    assert!(samples.is_empty());
  }

  #[test]
  fn malformed_and_missing_message_responses_remain_errors() {
    let malformed = parse_messages(b"{", "message").err();
    assert_matches!(malformed, Some(Error::Json(_)));
    let missing = parse_messages(br#"{"@type":"messages","total_count":0}"#, "messages").err();
    assert_matches!(missing, Some(Error::UnexpectedResponse("batch response omitted messages")));
    let unexpected = parse_messages(br#"{"@type":"ok"}"#, "ok").err();
    assert_matches!(unexpected, Some(Error::UnexpectedResponse(_)));
  }

  #[test]
  fn response_iterators_preserve_single_and_batch_shapes() {
    let single = parse_messages(br#"{"@type":"message","id":7}"#, "message").unwrap();
    assert_eq!(single.map(|message| message.id).collect::<Vec<_>>(), [7]);
    let batch = parse_messages(br#"{"@type":"messages","messages":[{"id":9},{"id":8}]}"#, "messages").unwrap();
    assert_eq!(batch.map(|message| message.id).collect::<Vec<_>>(), [9, 8]);
    let mut empty = parse_messages(br#"{"@type":"messages","messages":[]}"#, "messages").unwrap();
    assert_eq!(empty.next(), None);
  }

  #[test]
  fn measurements_can_regress_and_completion_does_not_fabricate_progress() {
    let (connection, _updates) = Connection::fixture();
    let message = types::message { content: document(7, 100), ..pending(10) };
    let batch = connection.registry.lock().unwrap().bind(vec![message], true);
    let mut samples = Vec::new();
    {
      let mut callback = |_, progress| samples.push(progress);
      let mut finish = pin!(batch.finish(&connection, None, Some(&mut callback)));
      let mut context = Context::from_waker(Waker::noop());
      let initial = finish.as_mut().poll(&mut context);
      assert_matches!(initial, Poll::Pending);

      let remote = types::remoteFile { uploaded_size: 80, ..Default::default() };
      let file = types::file { id: 7, expected_size: 200, remote, ..Default::default() };
      connection.registry.lock().unwrap().observe_file(&file);
      let partial = finish.as_mut().poll(&mut context);
      assert_matches!(partial, Poll::Pending);

      let file = types::file { expected_size: 90, remote: types::remoteFile { uploaded_size: 30, ..Default::default() }, ..file };
      connection.registry.lock().unwrap().observe_file(&file);
      let regressed = finish.as_mut().poll(&mut context);
      assert_matches!(regressed, Poll::Pending);

      let message = types::message { id: 20, content: document(8, 120), ..pending(10) };
      let success = types::updateMessageSendSucceeded { old_message_id: 10, message }.into();
      connection.registry.lock().unwrap().observe_message(&success);
      let terminal = finish.as_mut().poll(&mut context);
      assert_matches!(terminal, Poll::Ready(_));
    }
    let expected = [Progress { current: 80, total: 200 }, Progress { current: 30, total: 90 }];
    assert_eq!(samples, expected);
  }

  #[test]
  fn range_progress_counts_only_the_available_requested_prefix() {
    let local = types::localFile { download_offset: 0, downloaded_prefix_size: 60, ..Default::default() };
    let mut file = types::file { size: 100, local, ..Default::default() };
    assert_eq!(Progress::download(&file, 20, 50), Progress { current: 40, total: 50 });
    assert_eq!(Progress::download(&file, 90, 50), Progress { current: 0, total: 10 });
    file.local.downloaded_prefix_size = 100;
    assert_eq!(Progress::download(&file, 20, 50), Progress { current: 50, total: 50 });
    assert_eq!(Progress::download(&file, 90, 50), Progress { current: 10, total: 10 });
    file.local.download_offset = 30;
    assert_eq!(Progress::download(&file, 20, 50), Progress { current: 0, total: 50 });
    file.size = 0;
    assert_eq!(Progress::download(&file, 30, 50), Progress { current: 50, total: 0 });
    assert_eq!(Progress::download(&file, 30, 0), Progress { current: 100, total: 0 });
  }

  #[test]
  fn observers_coalesce_by_operation_and_prune_abandoned_waits() {
    let (connection, _updates) = Connection::fixture();
    let (upload, mut uploads) = watch::channel((0, Progress::default()));
    let request = fns::downloadFile { file_id: 7, offset: 20, limit: 50, ..Default::default() };
    let mut downloads = connection.observe_download(&request);
    let mut registry = connection.registry.lock().unwrap();
    registry.file_observers.push(Observation { file_id: 7, scope: Scope::Upload(2), samples: upload.clone() });
    registry.file_observers.push(Observation { file_id: 8, scope: Scope::Upload(3), samples: upload });
    let remote = types::remoteFile { uploaded_size: 60, ..Default::default() };
    let local = types::localFile { downloaded_prefix_size: 100, ..Default::default() };
    let file = types::file { id: 7, size: 100, local, remote, ..Default::default() };
    registry.observe_file(&file);
    assert_eq!(*uploads.borrow_and_update(), (2, Progress { current: 60, total: 100 }));
    assert_eq!(*downloads.borrow_and_update(), (0, Progress { current: 50, total: 50 }));

    registry.observe_file(&file);
    registry.observe_file(&types::file { id: 8, ..file });
    assert_eq!(*uploads.borrow_and_update(), (3, Progress { current: 60, total: 100 }));
    drop((uploads, downloads));
    registry.observe_file(&types::file::default());
    assert!(registry.file_observers.is_empty());
  }

  #[tokio::test]
  async fn a_closed_progress_channel_does_not_starve_completion() {
    let (sender, samples) = watch::channel((0, Progress::default()));
    drop(sender);
    let completion = async {
      yield_now().await;
      42
    };
    let mut called = false;
    let mut callback = |_, _| called = true;
    let result = timeout(Duration::from_secs(1), with_progress(completion, Some(samples), Some(&mut callback))).await.unwrap();
    assert_eq!(result, 42);
    assert!(!called);
  }
}
