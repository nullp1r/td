//! Small typed identifiers keep domain IDs distinct until the SQLite/TDLib boundaries.

use serde::{Deserialize, Serialize};

macro_rules! id {
  ($(#[$meta:meta])* $name:ident, $inner:ty) => {
    $(#[$meta])*
    #[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
    #[serde(transparent)]
    pub struct $name(pub $inner);
  };
}

id!(EncounterId, i64);
id!(LocationId, u32);
id!(SpeciesId, u32);
id!(BaitId, u32);
id!(RodId, u32);
