use td_types::{enums, functions, types};

#[test]
fn serialize_unit_function() {
  let json = serde_json::to_string(&functions::getMe {}).unwrap();
  assert_eq!(json, r#"{"@type":"getMe"}"#);
}

#[test]
fn serialize_function_with_args() {
  let ban = functions::banGroupCallParticipants { group_call_id: 123, user_ids: vec![456, 789] };
  let json = serde_json::to_string(&ban).unwrap();
  assert_eq!(json, r#"{"@type":"banGroupCallParticipants","group_call_id":123,"user_ids":["456","789"]}"#);
}

#[test]
fn serialize_function_with_enum_arg() {
  let set = functions::setOption {
    name: "online".into(), //.
    value: Some(enums::OptionValue::optionValueBoolean(types::optionValueBoolean { value: true })),
  };
  let json = serde_json::to_string(&set).unwrap();
  assert_eq!(json, r#"{"@type":"setOption","name":"online","value":{"@type":"optionValueBoolean","value":true}}"#);
}
