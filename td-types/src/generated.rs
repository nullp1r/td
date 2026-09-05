#![doc(hidden)]
#![expect(
  non_camel_case_types,
  clippy::doc_link_with_quotes,
  clippy::doc_markdown,
  clippy::large_enum_variant,
  clippy::struct_excessive_bools,
  rustdoc::bare_urls,
  rustdoc::broken_intra_doc_links,
  rustdoc::invalid_html_tags,
  reason = "generated code follows TD API schema conventions"
)]

//! Build-script-generated `TDLib` objects and functions.
//!
//! The checked-in module is only a stable include point. `build.rs` parses the
//! active `td_api.tl` into `$OUT_DIR/generated.rs`, so the Rust API and its
//! upstream documentation always come from the same schema revision.

include!(concat!(env!("OUT_DIR"), "/generated.rs"));
