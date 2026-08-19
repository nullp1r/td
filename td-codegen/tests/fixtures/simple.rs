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
      Self::entity(types::entity::default())
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
      Self::itemData(types::itemData::default())
    }
  }

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  #[serde(tag = "@type")]
  pub enum Loop {
    r#loop(types::r#loop),
  }

  impl Default for Loop {
    fn default() -> Self {
      Self::r#loop(types::r#loop::default())
    }
  }

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  #[serde(tag = "@type")]
  pub enum Menu {
    menu(types::menu),
  }

  impl Default for Menu {
    fn default() -> Self {
      Self::menu(types::menu::default())
    }
  }

  #[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
  #[serde(tag = "@type")]
  pub enum Tree {
    #[default]
    leaf,
    node(types::node),
  }

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  #[serde(tag = "@type")]
  pub enum User {
    user(types::user),
    userGroup(types::userGroup),
  }

  impl Default for User {
    fn default() -> Self {
      Self::user(types::user::default())
    }
  }
}

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
