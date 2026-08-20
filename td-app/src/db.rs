//! In-memory state and chat rating store for `td-app`.

use std::cmp::Reverse;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Database {
  /// `chat_id -> (user_id -> score)`
  ratings: HashMap<i64, HashMap<i64, i64>>,
}

impl Database {
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// Adjusts a user's rating by `delta` in a specific chat, returning the new rating.
  pub fn adjust_rating(&mut self, chat_id: i64, user_id: i64, delta: i64) -> i64 {
    let score = self.ratings.entry(chat_id).or_default().entry(user_id).or_default();
    *score += delta;
    *score
  }

  /// Gets the rating of a user in a chat.
  #[must_use]
  pub fn get_rating(&self, chat_id: i64, user_id: i64) -> i64 {
    self.ratings.get(&chat_id).and_then(|c| c.get(&user_id)).copied().unwrap_or_default()
  }

  /// Returns chat member ratings sorted by score in descending order.
  #[must_use]
  pub fn top_ratings(&self, chat_id: i64) -> Vec<(i64, i64)> {
    let Some(chat) = self.ratings.get(&chat_id) else {
      return Vec::new();
    };
    let mut scores: Vec<(i64, i64)> = chat.iter().map(|(&uid, &score)| (uid, score)).collect();
    scores.sort_unstable_by_key(|&(_, score)| Reverse(score));
    scores
  }
}
