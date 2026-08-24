/// Concrete payloads carried by `TDLib` object constructors.
///
/// Unit constructors appear only as variants in [`enums`].
pub mod types {
  use serde::{Deserialize, Serialize};
  use crate::serde_with;
  use super::enums;

  /// An entity referencing other types
  #[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
  #[serde(default)]
  pub struct entity {
    pub target: enums::User,
    pub extra: self::CustomType,
    pub parent: enums::Tree,
    /// Filter expression; pass null for all
    pub filter: Option<String>,
  }

  /// Data variant item with fields
  #[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
  #[serde(default)]
  pub struct itemData {
    /// Item identifier
    pub id: i32,
    /// Additional item details;
    /// may be null if omitted
    pub details: Option<String>,
  }

  #[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
  #[serde(default)]
  pub struct r#loop {
    pub r#for: bool,
    pub r#match: i32,
  }

  #[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
  #[serde(default)]
  pub struct menu {
    pub id: i32,
    pub items: Vec<enums::Menu>,
  }

  #[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
  #[serde(default)]
  pub struct node {
    pub value: i32,
    pub left: Box<enums::Tree>,
    pub right: Box<enums::Tree>,
  }

  #[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
  #[serde(default)]
  pub struct user {
    pub id: i32,
    pub token: i64,
    #[serde(with = "serde_with::int64")]
    pub score: i64,
    pub ratio: f64,
    pub name: String,
    #[serde(with = "serde_with::bytes")]
    pub payload: Vec<u8>,
    pub active: bool,
  }

  #[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
  #[serde(default)]
  pub struct userGroup {
    #[serde(with = "serde_with::int64_vec")]
    pub ids: Vec<i64>,
    pub matrix: Vec<Vec<String>>,
  }
}

/// Polymorphic `TDLib` objects grouped by their TL result type.
///
/// Enums use the JSON `@type` field to select a constructor payload from [`types`].
pub mod enums {
  use serde::{Deserialize, Serialize};
  use super::types;

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  #[serde(tag = "@type")]
  pub enum Entity {
    entity(types::entity),
  }

  impl Default for Entity {
    fn default() -> Self {
      types::entity::default().into()
    }
  }

  impl From<types::entity> for Entity {
    fn from(value: types::entity) -> Self {
      Self::entity(value)
    }
  }

  /// Represents a polymorphic item hierarchy
  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  #[serde(tag = "@type")]
  pub enum Item {
    /// Data variant item with fields
    itemData(types::itemData),
    /// Unit variant item with no fields
    itemEmpty,
  }

  impl Default for Item {
    fn default() -> Self {
      types::itemData::default().into()
    }
  }

  impl From<types::itemData> for Item {
    fn from(value: types::itemData) -> Self {
      Self::itemData(value)
    }
  }

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  #[serde(tag = "@type")]
  pub enum Loop {
    r#loop(types::r#loop),
  }

  impl Default for Loop {
    fn default() -> Self {
      types::r#loop::default().into()
    }
  }

  impl From<types::r#loop> for Loop {
    fn from(value: types::r#loop) -> Self {
      Self::r#loop(value)
    }
  }

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  #[serde(tag = "@type")]
  pub enum Menu {
    menu(types::menu),
  }

  impl Default for Menu {
    fn default() -> Self {
      types::menu::default().into()
    }
  }

  impl From<types::menu> for Menu {
    fn from(value: types::menu) -> Self {
      Self::menu(value)
    }
  }

  #[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
  #[serde(tag = "@type")]
  pub enum Tree {
    #[default]
    leaf,
    node(types::node),
  }

  impl From<types::node> for Tree {
    fn from(value: types::node) -> Self {
      Self::node(value)
    }
  }

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  #[serde(tag = "@type")]
  pub enum User {
    user(types::user),
    userGroup(types::userGroup),
  }

  impl Default for User {
    fn default() -> Self {
      types::user::default().into()
    }
  }

  impl From<types::user> for User {
    fn from(value: types::user) -> Self {
      Self::user(value)
    }
  }

  impl From<types::userGroup> for User {
    fn from(value: types::userGroup) -> Self {
      Self::userGroup(value)
    }
  }
}

/// Serializable `TDLib` requests and their typed response associations.
///
/// Each request implements [`crate::traits::Function`].
pub mod fns {
  use serde::Serialize;
  use crate::{serde_with, traits::Function};
  use super::{enums, types};

  /// Fetches an entity by ID
  #[derive(Debug, Clone, PartialEq, Default, Serialize)]
  #[serde(tag = "@type")]
  pub struct fetchEntity {
    /// Entity ID
    #[serde(with = "serde_with::int64")]
    pub id: i64,
    /// Optional search query;
    /// may be null
    pub query: Option<String>,
  }

  impl Function for fetchEntity {
    type Return = enums::Entity;
  }

  #[derive(Debug, Clone, PartialEq, Default, Serialize)]
  #[serde(tag = "@type")]
  pub struct ping {
  }

  impl Function for ping {
    type Return = bool;
  }

  #[derive(Debug, Clone, PartialEq, Default, Serialize)]
  #[serde(tag = "@type")]
  pub struct r#match {
    pub r#type: i32,
    pub custom: types::CustomType,
  }

  impl Function for r#match {
    type Return = i32;
  }
}
