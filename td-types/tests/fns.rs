use td_types::{enums, fns, types};

#[test]
fn unit_function() {
  let json = serde_json::to_string(&fns::getMe {}).unwrap();
  assert_eq!(json, r#"{"@type":"getMe"}"#);
}

#[test]
fn function_with_args() {
  let ban = fns::banGroupCallParticipants { group_call_id: 123, user_ids: vec![456, 789] };
  let json = serde_json::to_string(&ban).unwrap();
  assert_eq!(json, r#"{"@type":"banGroupCallParticipants","group_call_id":123,"user_ids":["456","789"]}"#);
}

#[test]
fn function_with_enum_arg() {
  let set = fns::setOption {
    name: "online".into(), //.
    value: Some(enums::OptionValue::optionValueBoolean(types::optionValueBoolean { value: true })),
  };
  let json = serde_json::to_string(&set).unwrap();
  assert_eq!(json, r#"{"@type":"setOption","name":"online","value":{"@type":"optionValueBoolean","value":true}}"#);
}
