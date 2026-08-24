#![doc(html_no_source)]

//! Generated Rust types for `TDLib`'s JSON API.
//!
//! The crate compiles the local `td_api.tl` schema at build time and preserves
//! its constructor, function, and field names. The generated surface is split by
//! role:
//!
//! - [`types`] contains concrete constructor payload structs;
//! - [`enums`] contains `@type`-tagged unions of constructors sharing a TL result
//!   type, including [`enums::Update`];
//! - [`fns`] contains serializable requests, each associated with its response by
//!   [`traits::Function`].
//!
//! ```
//! use td_types::{fns, traits::Function};
//!
//! fn accepts_get_me<F: Function<Return = td_types::enums::User>>(_: &F) {}
//!
//! let request = fns::getMe {};
//! accepts_get_me(&request);
//! assert_eq!(serde_json::to_value(request).unwrap(), serde_json::json!({ "@type": "getMe" }));
//! ```
//!
//! Object and enum values implement both [`serde::Serialize`] and
//! [`serde::Deserialize`]. Requests implement only `Serialize`, because their
//! corresponding response type is the deserialization boundary. Generated
//! object structs use Serde defaults for fields omitted from a response;
//! generated Rust construction remains explicit through ordinary public fields
//! and `Default`.
//!
//! # Wire representation
//!
//! Generated Serde attributes follow `TDLib`'s JSON interface: `int64` values are
//! decimal strings, `bytes` values are standard padded base64, polymorphic objects
//! carry `@type`, and fields documented by the schema as nullable use `Option`.
//! `int32`, `int53`, `double`, booleans, strings, and vectors use their natural
//! JSON representations.
//!
//! This crate describes the protocol but does not execute it. Use `td-client`
//! for correlated asynchronous requests, ordered updates, and client lifecycle.

pub use generated::*;

/// Traits implemented by generated protocol operations.
pub mod traits {
  use serde::{de, ser};

  /// A serializable `TDLib` request with a statically known response type.
  ///
  /// Generated structs in [`crate::fns`] implement this marker. Generic transports
  /// can serialize the request and deserialize the correlated response as
  /// [`Self::Return`] without maintaining a separate function-to-result table.
  pub trait Function: ser::Serialize {
    /// Successful response object declared by the function's TL result type.
    type Return: de::DeserializeOwned;
  }
}

mod base64;
mod generated;
mod serde_with;
