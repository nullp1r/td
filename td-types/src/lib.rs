#![doc(html_no_source)]

#[doc(hidden)]
pub use generated::*;

pub mod traits {
  use serde::{de, ser};

  pub trait Function: ser::Serialize {
    type Return: de::DeserializeOwned;
  }
}

mod base64;
mod generated;
mod serde_with;
