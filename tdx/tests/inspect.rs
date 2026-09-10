//! Borrowed command, caption, file, and identity inspection.

use std::assert_matches;

use tdx::{command, enums::TextEntityType};
use tdx::prelude::*;

#[test]
fn commands_agree_across_text_content_and_message() {
  let text = types::formattedText {
    text: "/start@MyBot now".into(), //.
    entities: vec![types::textEntity { offset: 0, length: 12, r#type: TextEntityType::textEntityTypeBotCommand }],
  };
  let expected = Some(Command { name: "start", target: Some("MyBot"), args: "now" });

  assert_eq!(text.command(), expected);
  assert_eq!(text.text.command(), expected);

  let message = message(types::messageText { text, ..Default::default() });
  assert_eq!(message.command(), expected);
  assert_eq!(message.content.command(), expected);

  let command = message.command().unwrap();
  assert!(command.is_for(Some("mybot")));
  assert!(!command.is_for(Some("otherbot")));
  assert_eq!(message.command_for(Some("mybot")), expected);
  assert_eq!(message.command_for(Some("otherbot")), None);
}

#[test]
fn command_registration_builders_keep_generated_requests_editable() {
  let mut request = command::set(
    enums::BotCommandScope::botCommandScopeAllPrivateChats,
    [command::definition("start", "Open the game"), command::ephemeral_definition("help", "Private help in groups")],
  );
  request.language_code = "en".into();

  assert_matches!(request.scope, Some(enums::BotCommandScope::botCommandScopeAllPrivateChats));
  assert_eq!(request.language_code, "en");
  let [start, help] = request.commands.as_slice() else { unreachable!("expected two commands") };
  assert_eq!((&*start.command, &*start.description, start.is_ephemeral), ("start", "Open the game", false));
  assert_eq!((&*help.command, &*help.description, help.is_ephemeral), ("help", "Private help in groups", true));
}

#[test]
fn formatted_commands_require_a_leading_command_entity() {
  // The raw parser trusts a slash prefix; formatted input trusts Telegram's entities.
  let mut text: types::formattedText = "/start".text().into();
  assert!(text.text.command().is_some());
  assert_eq!(text.command(), None);

  text.entities.push(types::textEntity { offset: 0, length: 6, r#type: TextEntityType::textEntityTypeCode });
  assert_eq!(text.command(), None);

  text.text = "See /start".into();
  text.entities = vec![types::textEntity { offset: 4, length: 6, r#type: TextEntityType::textEntityTypeBotCommand }];
  assert_eq!(text.command(), None);
}

#[test]
fn empty_text_is_present_not_missing() {
  let message = message(types::messageText::default());
  let text = message.text();
  assert_matches!(text, Some(text) if text.text.is_empty());
  assert_eq!(message.content.text(), text);
  assert_eq!(message.caption(), None);
}

#[test]
fn photo_inspection_selects_the_largest_size_and_preserves_caption() {
  let sizes = [11, 777].map(|id| types::photoSize {
    photo: types::file { id, ..Default::default() }, //.
    ..Default::default()
  });
  let photo = types::photo { sizes: sizes.into(), ..Default::default() };
  let message = message(types::messagePhoto {
    photo, //.
    caption: "photo caption".text().into(),
    ..Default::default()
  });

  let caption = message.caption();
  assert_matches!(caption, Some(text) if text.text == "photo caption");
  assert_eq!(message.content.caption(), caption);
  assert_eq!(message.content.text(), caption);
  assert_eq!(message.primary_file_id(), Some(777));
  assert_eq!(message.content.primary_file_id(), Some(777));
}

#[test]
fn sender_identity_keeps_users_distinct_from_chats() {
  let user: enums::MessageSender = types::messageSenderUser { user_id: 12345 }.into();
  let chat: enums::MessageSender = types::messageSenderChat { chat_id: 12345 }.into();

  assert_eq!((user.id(), user.user_id(), user.chat_id()), (12345, Some(12345), None));
  assert_eq!((chat.id(), chat.user_id(), chat.chat_id()), (12345, None, Some(12345)));

  let message = types::message { sender_id: user, ..Default::default() };
  assert_eq!(message.sender_id(), 12345);
}

#[test]
fn username_is_the_first_active_name() {
  let mut user = types::user::default();
  assert_eq!(user.username(), None);

  let active_usernames = ["primary", "alias"].map(Into::into).to_vec();
  user.usernames = Some(types::usernames { active_usernames, ..Default::default() });
  assert_eq!(user.username(), Some("primary"));
}

#[test]
fn message_reply_inspection_extracts_id() {
  let mut message = types::message::default();
  assert_eq!(message.replied_message_id(), None);
  assert!(!message.is_reply());

  message.reply_to = Some(types::messageReplyToMessage { message_id: 4242, ..Default::default() }.into());
  assert_eq!(message.replied_message_id(), Some(4242));
  assert!(message.is_reply());
}

#[test]
fn user_display_name_falls_back_gracefully() {
  let mut user = types::user::default();
  assert_eq!(user.display_name(), "User");

  let active_usernames = ["fisherbot"].map(Into::into).to_vec();
  user.usernames = Some(types::usernames { active_usernames, ..Default::default() });
  assert_eq!(user.display_name(), "fisherbot");

  user.first_name = "Captain".into();
  assert_eq!(user.display_name(), "Captain");
}

fn message(content: impl Into<enums::MessageContent>) -> types::message {
  types::message { content: content.into(), ..Default::default() }
}
