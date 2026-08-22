use td_types::enums::InputInlineQueryResult;
use td_types::types;

use super::App;
use crate::client_ext::ClientExt;

impl App {
  pub(super) async fn handle_inline_query(&self, query_id: i64, query: &str) -> td_client::Result {
    let trimmed = query.trim();
    tracing::debug!(%trimmed, "processing inline query");

    let results = vec![
      article("1", "👋 Help", "Rate users with + or -", "💡 Reply to any user message with `+` or `-` to adjust their karma!"),
      article("3", "🏆 Leaderboard", "View top members", "Use `/ratings` to view chat karma leaderboard!"),
    ];

    self.client.answer_inline_query(query_id, results, 10).await
  }
}

fn article(id: &str, title: &str, desc: &str, text: &str) -> InputInlineQueryResult {
  let (id, title, description) = (id.into(), title.into(), desc.into());
  let text = types::formattedText { text: text.into(), ..Default::default() };
  let input_message_content = types::inputMessageText { text, ..Default::default() }.into();
  types::inputInlineQueryResultArticle { id, title, description, input_message_content, ..Default::default() }.into()
}
