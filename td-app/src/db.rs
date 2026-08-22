use std::cmp::Reverse;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub(crate) struct Database {
  ratings: HashMap<i64, HashMap<i64, i64>>,
}

impl Database {
  pub fn adjust_rating(&mut self, chat_id: i64, user_id: i64, delta: i64) -> i64 {
    let score = self.ratings.entry(chat_id).or_default().entry(user_id).or_default();
    *score += delta;
    *score
  }

  pub fn get_rating(&self, chat_id: i64, user_id: i64) -> i64 {
    self.ratings.get(&chat_id).and_then(|c| c.get(&user_id)).copied().unwrap_or_default()
  }

  /// Returns chat member ratings sorted by score in descending order.
  pub fn top_ratings(&self, chat_id: i64) -> Vec<(i64, i64)> {
    let Some(chat) = self.ratings.get(&chat_id) else {
      return Vec::new();
    };
    let mut scores: Vec<(i64, i64)> = chat.iter().map(|(&uid, &score)| (uid, score)).collect();
    scores.sort_unstable_by_key(|&(_, score)| Reverse(score));
    scores
  }
}
