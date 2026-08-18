use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use td_types::traits::Function;
use td_types::{enums, fns, types};

#[track_caller]
fn check<T: Debug + PartialEq + Serialize + for<'de> Deserialize<'de>>(val: &T) {
  let json = serde_json::to_string(val).unwrap();
  let de: T = serde_json::from_str(&json).unwrap();
  assert_eq!(val, &de);
}

#[test]
fn int64() {
  check(&types::optionValueInteger { value: 0 });
  check(&types::optionValueInteger { value: -42 });
  check(&types::optionValueInteger { value: 1_234_567_890_123_456_789 });
  check(&types::optionValueInteger { value: i64::MIN });
  check(&types::optionValueInteger { value: i64::MAX });
}

#[test]
fn int64_vec() {
  check(&types::emojiStatusCustomEmojis { custom_emoji_ids: vec![] });
  check(&types::emojiStatusCustomEmojis { custom_emoji_ids: vec![0, -1, 100, 1_234_567_890_123] });
}

#[test]
fn float() {
  check(&types::location { latitude: 51.5074, longitude: -0.1278, horizontal_accuracy: 1.5 });
}

#[test]
fn bytes() {
  check(&types::minithumbnail { width: 0, height: 0, data: vec![] });
  check(&types::minithumbnail { width: 40, height: 40, data: (0..=255).collect() });
}

#[test]
fn tagged_enums() {
  check(&enums::OptionValue::optionValueEmpty);
  check(&enums::OptionValue::optionValueBoolean(types::optionValueBoolean { value: true }));
  check(&enums::OptionValue::optionValueInteger(types::optionValueInteger { value: 42 }));
  check(&enums::OptionValue::optionValueString(types::optionValueString { value: "hello".into() }));
}

#[test]
fn recursive_boxed_enums() {
  check(&enums::RichText::richTextBold(types::richTextBold {
    text: Box::new(enums::RichText::richTextItalic(types::richTextItalic {
      text: Box::new(enums::RichText::richTextPlain(types::richTextPlain { text: "nested rich text".into() })),
    })),
  }));
}

#[test]
fn function_return_types() {
  check(&<fns::banGroupCallParticipants as Function>::Return::ok);
  check(&<fns::getMe as Function>::Return::user(types::user {
    id: 12_345, //.
    first_name: "Alice".into(),
    last_name: "Smith".into(),
    ..Default::default()
  }));
}

#[test]
fn composite_structures() {
  check(&types::formattedText {
    text: "Telegram".into(),
    entities: vec![
      types::textEntity { offset: 0, length: 4, r#type: enums::TextEntityType::textEntityTypeBold },
      types::textEntity { offset: 4, length: 4, r#type: enums::TextEntityType::textEntityTypeItalic },
    ],
  });

  check(&enums::User::user(types::user {
    id: 9_876_543_210,
    first_name: "John".into(),
    last_name: "Doe".into(),
    usernames: Some(types::usernames {
      active_usernames: vec!["johndoe".into()],
      disabled_usernames: vec![],
      editable_username: "johndoe".into(),
      collectible_usernames: vec![],
    }),
    background_custom_emoji_id: 112_233_445_566,
    is_contact: true,
    is_premium: true,
    ..Default::default()
  }));
}
