//! Request construction: targets, snapshot preservation, and editable defaults.

use std::assert_matches;

use anyhow::{Result, bail};

use tdx::enums::{InputMessageReplyTo, MessageTopic};
use tdx::prelude::*;

#[test]
fn array_keyboards_without_nested_vecs() -> Result<()> {
  let rows = [
    [markup::url("Home", "https://example.com"), markup::callback("Ping", b"ping")],
    [markup::switch_inline("Search", "rust"), markup::callback("Help", b"help")],
  ];
  let expected = rows.clone().map(Vec::from).to_vec();
  let keyboard = markup::inline(rows);

  let enums::ReplyMarkup::replyMarkupInlineKeyboard(keyboard) = keyboard else {
    bail!("expected an inline keyboard");
  };
  assert_eq!(keyboard.rows, expected);
  Ok(())
}

#[test]
fn flexible_message_targeting_and_reply() {
  // Coordinates identify the reply without requiring a cached message.
  let reply = send::reply(&(100, 200), "Hello".text());
  assert_eq!(reply.chat_id, 100);
  assert_matches!(
    reply.reply_to, //.
    Some(InputMessageReplyTo::inputMessageReplyToMessage(types::inputMessageReplyToMessage { message_id: 200, .. }))
  );

  // A snapshot also carries the original topic.
  let source = types::message {
    chat_id: 100, //.
    id: 200,
    topic_id: Some(types::messageTopicForum { forum_topic_id: 42 }.into()),
    ..Default::default()
  };

  let reply = send::reply(&source, "Hello".text());
  assert_eq!(reply.chat_id, 100);
  assert_matches!(
    reply.topic_id, //.
    Some(MessageTopic::messageTopicForum(types::messageTopicForum { forum_topic_id: 42 }))
  );
}

#[test]
fn uniform_content_constructors_with_captions() {
  let document = content::document(file::local("notes.pdf"), None, None);
  assert!(document.document.disable_content_type_detection);

  // Captions use the same rich text as ordinary messages.
  let caption = line("Watch this: ") + bold("cool clip");
  let video_with_caption = content::video(file::local("clip.mp4"), None, 60, [1920, 1080], Some(caption.into()));
  assert_eq!(video_with_caption.caption.unwrap().text, "Watch this: cool clip");

  let video_no_caption = content::video(file::local("clip.mp4"), None, 60, [1920, 1080], None);
  assert!(video_no_caption.caption.is_none());
}

#[test]
fn media_edits_preserve_only_the_supplied_snapshot() {
  let source = types::message {
    chat_id: 100, //.
    id: 200,
    reply_markup: Some(markup::inline([[markup::callback("Again", b"again")]])),
    ..Default::default()
  };
  let photo = content::photo(file::local("photo.jpg"), [1920, 1080], None);

  let snapshot = edit::media(&source, photo.clone());
  let coordinates = edit::media(&(100, 200), photo);

  assert_eq!((snapshot.chat_id, snapshot.message_id), (100, 200));
  assert_eq!(snapshot.reply_markup, source.reply_markup);
  assert_eq!(coordinates.reply_markup, None);
  assert_eq!(snapshot.input_message_content, coordinates.input_message_content);
}

#[test]
fn callback_answers_identify_the_query_and_alert_mode() {
  let answer = callback::answer(&300, "Done", true);
  assert_eq!(answer.callback_query_id, 300);
  assert_eq!(answer.text, "Done");
  assert!(answer.show_alert);

  let query = types::updateNewCallbackQuery { id: 777, ..Default::default() };
  let toast = callback::toast(&query, format_args!("Score: {}", 42));
  assert_eq!(toast.callback_query_id, 777);
  assert_eq!(toast.text, "Score: 42");
  assert!(!toast.show_alert);

  let ack = callback::ack(&query);
  assert_eq!(ack.callback_query_id, 777);
  assert_eq!(ack.text, "");
  assert!(!ack.show_alert);
}

#[test]
fn column_keyboards_place_each_button_in_its_own_row() -> Result<()> {
  let buttons = [markup::callback("One", b"1"), markup::callback("Two", b"2")];
  let keyboard = markup::column(buttons);

  let enums::ReplyMarkup::replyMarkupInlineKeyboard(keyboard) = keyboard else {
    bail!("expected an inline keyboard");
  };
  assert_eq!(keyboard.rows.len(), 2);
  assert_eq!(keyboard.rows[0].len(), 1);
  assert_eq!(keyboard.rows[1].len(), 1);
  Ok(())
}

#[test]
fn download_ranges_keep_synchronous_completion() {
  let mut download = transfer::download(400, 2);
  download.offset = 64;
  download.limit = 128;
  assert_eq!(download.file_id, 400);
  assert_eq!(download.priority, 2);

  assert_eq!((download.offset, download.limit), (64, 128));
  assert!(download.synchronous);
}

#[test]
fn rich_message_request_construction() {
  let html_msg = content::html("<b>Hello</b> <table><tr><td>Cell</td></tr></table>");
  let send_req = send::reply(&(123, 456), html_msg);
  assert_eq!(send_req.chat_id, 123);
  assert_matches!(
    send_req.input_message_content,
    enums::InputMessageContent::inputMessageRichMessage(types::inputMessageRichMessage {
      message: types::inputRichMessage {
        source: enums::RichMessageSource::richMessageSourceHtml(types::richMessageSourceHtml { text, .. }),
        ..
      },
      ..
    }) if text.contains("<table>")
  );

  let edit_req = edit::html(&(123, 456), "<blockquote>Quote</blockquote>");
  assert_eq!(edit_req.chat_id, 123);
  assert_eq!(edit_req.message_id, 456);
  assert_matches!(
    edit_req.input_message_content,
    enums::InputMessageContent::inputMessageRichMessage(types::inputMessageRichMessage {
      message: types::inputRichMessage {
        source: enums::RichMessageSource::richMessageSourceHtml(types::richMessageSourceHtml { text, .. }),
        ..
      },
      ..
    }) if text.contains("<blockquote>")
  );
}
