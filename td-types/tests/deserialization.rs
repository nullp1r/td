use std::assert_matches;

use td_types::{enums, types};

#[test]
fn int64_from_raw_numbers() {
  let de: types::optionValueInteger = serde_json::from_str(r#"{"value":42}"#).unwrap();
  assert_eq!(de.value, 42);

  let de: types::optionValueInteger = serde_json::from_str(r#"{"value":-42}"#).unwrap();
  assert_eq!(de.value, -42);

  let res = serde_json::from_str::<types::optionValueInteger>(r#"{"value":"invalid"}"#);
  assert_matches!(res, Err(_));
}

#[test]
fn int64_vec_flexible_formats() {
  let json = r#"{"custom_emoji_ids":[100, "200", -300]}"#;
  let de: types::emojiStatusCustomEmojis = serde_json::from_str(json).unwrap();
  assert_eq!(de.custom_emoji_ids, vec![100, 200, -300]);
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
  assert_eq!(
    enums::TextEntityType::default(), // Unit default
    enums::TextEntityType::textEntityTypeBankCardNumber
  );

  assert_eq!(
    enums::OptionValue::default(), // Struct-wrapping default (instantiates first variant)
    enums::OptionValue::optionValueBoolean(types::optionValueBoolean { value: false })
  );
}
