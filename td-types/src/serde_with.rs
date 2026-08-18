use std::fmt::Display;

use serde::de;
use serde::ser::{Serialize, Serializer};

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum Int64<'a> {
  Raw(i64),
  Str(&'a str),
}

impl Int64<'_> {
  fn into_i64<E: de::Error>(self) -> Result<i64, E> {
    match self {
      Self::Raw(n) => Ok(n),
      Self::Str(s) => s.parse().map_err(de::Error::custom),
    }
  }
}

struct Str<T>(T);

impl<T: Display> Serialize for Str<T> {
  fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    s.collect_str(&self.0)
  }
}

pub mod int64 {
  use serde::de::{Deserialize as _, Deserializer};
  use serde::ser::Serializer;

  use super::Int64;

  pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    Int64::deserialize(d)?.into_i64()
  }

  #[expect(clippy::trivially_copy_pass_by_ref, reason = "required by serde serialize_with signature")]
  pub fn serialize<S: Serializer>(v: &i64, s: S) -> Result<S::Ok, S::Error> {
    s.collect_str(v)
  }
}

pub mod int64_vec {
  use serde::de::{Deserialize as _, Deserializer};
  use serde::ser::{SerializeSeq as _, Serializer};

  use super::{Int64, Str};

  pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<i64>, D::Error> {
    Vec::deserialize(d)?.into_iter().map(Int64::into_i64).collect()
  }

  pub fn serialize<S: Serializer>(v: &[i64], s: S) -> Result<S::Ok, S::Error> {
    let mut seq = s.serialize_seq(Some(v.len()))?;
    for &item in v {
      seq.serialize_element(&Str(item))?;
    }
    seq.end()
  }
}

pub mod bytes {
  use serde::de::{Deserialize as _, Deserializer, Error as _};
  use serde::ser::Serializer;

  use crate::base64;

  pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    let s = <&str>::deserialize(d)?;
    base64::decode(s).ok_or_else(|| D::Error::custom("invalid base64"))
  }

  pub fn serialize<S: Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&base64::encode(v, 0))
  }
}
