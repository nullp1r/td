//! Extension trait for inspecting user details.

use td_types::types;

/// Extension trait for inspecting user details.
pub trait UserExt {
  /// Returns the primary active username without leading `@`.
  fn username(&self) -> Option<&str>;

  /// Returns the user's first name, or their username, or `"User"` if neither is set.
  fn display_name(&self) -> &str;
}

impl UserExt for types::user {
  fn username(&self) -> Option<&str> {
    match &self.usernames {
      Some(all) if let [main, ..] = &*all.active_usernames => Some(main),
      _ => None,
    }
  }

  fn display_name(&self) -> &str {
    if !self.first_name.is_empty() {
      &self.first_name
    } else if let Some(username) = self.username() {
      username
    } else {
      "User"
    }
  }
}
