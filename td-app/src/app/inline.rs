use td_client::Error as ClientError;
use td_types::{enums, types};

use super::App;
use crate::client_ext::ClientExt;

impl App {
  pub(super) async fn handle_inline_query(&self, query_id: i64, query: &str) -> Result<(), ClientError> {
    let trimmed = query.trim();
    tracing::debug!(%trimmed, "processing inline query");

    let results = vec![
      article("1", "👋 Help", "Rate users with + or -", "💡 Reply to any user message with `+` or `-` to adjust their karma!"),
      article("2", "🎲 Roll Dice", "Rolls a random number", "🎲 Rolled: **42** (1-100)"),
      article("3", "🏆 Leaderboard", "View top members", "Use `/ratings` to view chat karma leaderboard!"),
    ];

    self.client.answer_inline_query(query_id, results, 10).await
  }
}

fn article(id: &str, title: &str, desc: &str, text: &str) -> enums::InputInlineQueryResult {
  let content = enums::InputMessageContent::inputMessageText(types::inputMessageText {
    text: types::formattedText { text: text.into(), ..Default::default() },
    ..Default::default()
  });

  enums::InputInlineQueryResult::inputInlineQueryResultArticle(types::inputInlineQueryResultArticle {
    id: id.into(),
    title: title.into(),
    description: desc.into(),
    input_message_content: content,
    ..Default::default()
  })
}
