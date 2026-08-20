//! Inline query handlers and result builders.

use td_client::Error as ClientError;
use td_types::{enums, fns, types};

use super::App;

impl App {
  #[tracing::instrument(skip(self))]
  pub(super) async fn handle_inline_query(&self, query_id: i64, query: &str) -> Result<(), ClientError> {
    let trimmed = query.trim();
    tracing::debug!(%trimmed, "processing inline query");

    let results = vec![
      make_article("1", "👋 Help", "Rate users with + or -", "💡 Reply to any user message with `+` or `-` to adjust their karma!"),
      make_article("2", "🎲 Roll Dice", "Rolls a random number", "🎲 Rolled: **42** (1-100)"),
      make_article("3", "🏆 Leaderboard", "View top members", "Use `/ratings` to view chat karma leaderboard!"),
    ];

    let req = fns::answerInlineQuery { inline_query_id: query_id, results, cache_time: 10, ..Default::default() };
    self.client.execute(&req).await?;
    Ok(())
  }
}

fn make_article(id: &str, title: &str, description: &str, text: &str) -> enums::InputInlineQueryResult {
  let content = enums::InputMessageContent::inputMessageText(types::inputMessageText {
    text: types::formattedText { text: text.into(), ..Default::default() },
    ..Default::default()
  });

  enums::InputInlineQueryResult::inputInlineQueryResultArticle(types::inputInlineQueryResultArticle {
    id: id.into(),
    title: title.into(),
    description: description.into(),
    input_message_content: content,
    ..Default::default()
  })
}
