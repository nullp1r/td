//! Serde adapters for `TDLib` JSON values whose wire form differs from Rust.
//!
//! `TDLib` encodes signed 64-bit integers as decimal JSON strings and byte arrays
//! as base64 strings. Generated fields reference these modules through
//! `#[serde(with = ...)]`, keeping the public Rust representation as `i64`,
//! `Vec<i64>`, or `Vec<u8>`.

use core::fmt::NumBuffer;

use serde::de::{self, Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};

/// Stack-backed adapter for one decimal-string `i64`.
struct Int64(i64);

impl<'de> Deserialize<'de> for Int64 {
  fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
    <&str>::deserialize(d)?.parse().map(Self).map_err(de::Error::custom)
  }
}

impl Serialize for Int64 {
  fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(self.0.format_into(&mut NumBuffer::new()))
  }
}

pub mod int64 {
  //! Serialization of one Rust `i64` as a decimal JSON string.

  use serde::de::{Deserialize as _, Deserializer};
  use serde::ser::{Serialize as _, Serializer};

  use super::Int64;

  /// Deserializes a signed decimal string into an `i64`.
  pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    Ok(Int64::deserialize(d)?.0)
  }

  /// Serializes an `i64` as its signed decimal string.
  #[expect(clippy::trivially_copy_pass_by_ref, reason = "serde signature")]
  pub fn serialize<S: Serializer>(&v: &i64, s: S) -> Result<S::Ok, S::Error> {
    Int64(v).serialize(s)
  }
}

pub mod int64_vec {
  //! Serialization of `Vec<i64>` as a JSON array of decimal strings.

  use serde::de::{Deserialize as _, Deserializer};
  use serde::ser::{SerializeSeq as _, Serializer};

  use super::Int64;

  /// Deserializes an array of signed decimal strings.
  pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<i64>, D::Error> {
    Ok(Vec::deserialize(d)?.into_iter().map(|Int64(i)| i).collect())
  }

  /// Serializes a slice of integers as signed decimal strings.
  pub fn serialize<S: Serializer>(ints: &[i64], s: S) -> Result<S::Ok, S::Error> {
    let mut seq = s.serialize_seq(Some(ints.len()))?;
    for &int in ints {
      seq.serialize_element(&Int64(int))?;
    }
    seq.end()
  }
}

pub mod bytes {
  //! Serialization of byte vectors as standard padded base64 strings.

  use serde::de::{Deserialize as _, Deserializer, Error as _};
  use serde::ser::Serializer;

  use crate::base64;

  /// Decodes a base64 JSON string.
  pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    let s = <&str>::deserialize(d)?;
    base64::decode(s).ok_or_else(|| D::Error::custom("invalid base64"))
  }

  /// Encodes bytes as a standard padded base64 JSON string.
  pub fn serialize<S: Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&base64::encode(v, 0))
  }
}
