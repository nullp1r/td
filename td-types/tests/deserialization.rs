//! Verifies generated deserialization behavior and failures.

use std::assert_matches;

use td_types::{enums, types};

#[test]
fn raw_int64() {
  let res = serde_json::from_str::<types::optionValueInteger>(r#"{"value":123}"#);
  assert_matches!(res, Err(_));
}

#[test]
fn raw_int64_vec() {
  let res = serde_json::from_str::<types::emojiStatusCustomEmojis>(r#"{"custom_emoji_ids":[1,2,3]}"#);
  assert_matches!(res, Err(_));
}

#[test]
fn bytes_invalid_base64_error() {
  let res = serde_json::from_str::<types::minithumbnail>(r#"{"data":"???not-base64???"}"#);
  assert_matches!(res, Err(_));
}

#[test]
fn tagged_enum_unknown_variant_error() {
  let res = serde_json::from_str::<enums::OptionValue>(r#"{"@type":"unknownOptionValue"}"#);
  assert_matches!(res, Err(_));
}

#[test]
fn struct_defaults_and_ignored_fields() {
  // Partial / omitted fields fall back to default
  let partial: types::minithumbnail = serde_json::from_str(r#"{"width":10}"#).unwrap();
  assert_eq!(partial, types::minithumbnail { width: 10, height: 0, data: vec![] });

  // Completely empty payload
  let empty: types::minithumbnail = serde_json::from_str("{}").unwrap();
  assert_eq!(empty, types::minithumbnail::default());

  // Extra unknown fields are ignored
  let extra: types::minithumbnail = serde_json::from_str(r#"{"width":10,"unknown_field":"foo"}"#).unwrap();
  assert_eq!(extra.width, 10);
}

#[test]
fn enum_defaults() {
  // Unit default
  assert_eq!(enums::TextEntityType::default(), enums::TextEntityType::textEntityTypeBankCardNumber);

  // Struct-wrapping default (instantiates first variant)
  assert_eq!(enums::OptionValue::default(), types::optionValueBoolean { value: false }.into());
}
