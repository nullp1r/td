use std::io::{Cursor, Write as _};
use std::str;

use serde::de::{self, Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};

struct Int64(i64);

impl<'de> Deserialize<'de> for Int64 {
  fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
    <&str>::deserialize(d)?.parse().map(Self).map_err(de::Error::custom)
  }
}

impl Serialize for Int64 {
  fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    let mut buf = [0; 20];
    let len = {
      let mut w = Cursor::new(&mut buf[..]);
      let _ = write!(w, "{}", self.0);
      w.position() as usize
    };
    // SAFETY: `buf[..len]` was written by `i64`'s ASCII decimal formatter.
    s.serialize_str(unsafe { str::from_utf8_unchecked(&buf[..len]) })
  }
}

pub mod int64 {
  use serde::de::{Deserialize as _, Deserializer};
  use serde::ser::{Serialize as _, Serializer};

  use super::Int64;

  pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    Ok(Int64::deserialize(d)?.0)
  }

  #[expect(clippy::trivially_copy_pass_by_ref, reason = "serde signature")]
  pub fn serialize<S: Serializer>(&v: &i64, s: S) -> Result<S::Ok, S::Error> {
    Int64(v).serialize(s)
  }
}

pub mod int64_vec {
  use serde::de::{Deserialize as _, Deserializer};
  use serde::ser::{SerializeSeq as _, Serializer};

  use super::Int64;

  pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<i64>, D::Error> {
    Ok(Vec::deserialize(d)?.into_iter().map(|Int64(i)| i).collect())
  }

  pub fn serialize<S: Serializer>(ints: &[i64], s: S) -> Result<S::Ok, S::Error> {
    let mut seq = s.serialize_seq(Some(ints.len()))?;
    for &int in ints {
      seq.serialize_element(&Int64(int))?;
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
