#![expect(
  non_camel_case_types,
  clippy::doc_link_with_quotes,
  clippy::doc_markdown,
  clippy::large_enum_variant,
  clippy::struct_excessive_bools,
  reason = "generated code follows TD API schema conventions" //.
)]

include!(concat!(env!("OUT_DIR"), "/generated.rs"));
