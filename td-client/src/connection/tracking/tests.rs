//! Send completion, response decoding, and transfer observation.

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
  types::message {
    id, //.
    chat_id: 9,
    sending_state: Some(types::messageSendingStatePending::default().into()),
    ..Default::default()
  }
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

    // Settle out of order: success, failure, then deletion. Results must still
    // occupy their original response positions.
    let message = types::message { id: 21, ..pending(11) };
    registry.observe_message(&types::updateMessageSendSucceeded { old_message_id: 11, message }.into());
    let error = types::error { code: 400, message: "failed".into() };
    registry.observe_message(
      &types::updateMessageSendFailed {
        old_message_id: 10, //.
        message: pending(10),
        error,
      }
      .into(),
    );
    let deletion = |from_cache| {
      types::updateDeleteMessages {
        chat_id: 9, //.
        message_ids: vec![12],
        from_cache,
        ..Default::default()
      }
      .into()
    };

    // Cache eviction is not an authoritative message deletion.
    registry.observe_message(&deletion(true));
    assert!(registry.pending_messages.contains_key(&MessageKey { chat_id: 9, message_id: 12 }));
    registry.observe_message(&deletion(false));
    batch
  };
  let results = timeout(Duration::from_secs(1), batch.finish(&connection, None, None)).await.unwrap();
  let [first, second, third]: [Result<types::message>; 3] = results.try_into().unwrap();
  assert_matches!(first, Err(Error::MessageFailed(MessageKey { message_id: 10, .. }, error)) if error.code == 400);
  assert_matches!(second, Ok(types::message { id: 21, .. }));
  assert_matches!(third, Err(Error::MessageDeleted(MessageKey { message_id: 12, .. })));
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
  // Success is already authoritative when cancellation becomes observable.
  let cancel = CancellationToken::new();
  cancel.cancel();
  let mut samples = Vec::new();
  let mut callback = |index, progress| samples.push((index, progress));
  let results = timeout(Duration::from_secs(1), batch.finish(&connection, Some(&cancel), Some(&mut callback))) //.
    .await
    .unwrap();
  let [result]: [Result<types::message>; 1] = results.try_into().unwrap();
  assert_matches!(result, Ok(types::message { id: 20, .. }));
  assert_eq!(samples, []);
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
    // Poll explicitly so every progress sample is consumed before the next
    // native update; no scheduler timing is involved.
    let mut context = Context::from_waker(Waker::noop());
    let initial = finish.as_mut().poll(&mut context);
    assert_matches!(initial, Poll::Pending);

    let remote = types::remoteFile { uploaded_size: 80, ..Default::default() };
    let file = types::file { id: 7, expected_size: 200, remote, ..Default::default() };
    connection.registry.lock().unwrap().observe_file(&file);
    let partial = finish.as_mut().poll(&mut context);
    assert_matches!(partial, Poll::Pending);

    // Native estimates and byte counts may decrease. Preserve those measurements.
    let file = types::file {
      expected_size: 90, //.
      remote: types::remoteFile { uploaded_size: 30, ..Default::default() },
      ..file
    };
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

  // A cached suffix cannot satisfy a request whose prefix is still missing.
  file.local.download_offset = 30;
  assert_eq!(Progress::download(&file, 20, 50), Progress { current: 0, total: 50 });

  // Unknown file size leaves total unknown, while an explicit limit still bounds current.
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
  // Abandoned observation is pruned on the next update, without native cancellation.
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
  let result = timeout(Duration::from_secs(1), with_progress(completion, Some(samples), Some(&mut callback))) //.
    .await
    .unwrap();
  assert_eq!(result, 42);
  assert!(!called);
}
