//! Classic inline-mode catch sharing.

use tdx::{client, prelude::*};

use crate::view::InlineCatchView;

use super::format_weight;

pub async fn inline_catches(client: &Client, update: &types::updateNewInlineQuery, catches: &[InlineCatchView]) -> client::Result<()> {
  let results = catches.iter().map(inline_catch_result).collect();
  let button = catches.is_empty().then(|| types::inlineQueryResultsButton {
    text: "Open Rustwater".into(),
    r#type: types::inlineQueryResultsButtonTypeStartBot { parameter: "inline".into() }.into(),
  });
  client.send(&fns::answerInlineQuery { inline_query_id: update.id, is_personal: true, button, results, cache_time: 0, next_offset: String::new() }).await?;
  Ok(())
}

fn inline_catch_result(catch: &InlineCatchView) -> enums::InputInlineQueryResult {
  let status = if catch.world_best {
    "World record"
  } else if catch.personal_best {
    "Personal best"
  } else {
    catch.location_name.as_str()
  };
  types::inputInlineQueryResultArticle {
    id: format!("catch:{}", catch.item_id),
    title: format!("{} · {}", catch.species_name, format_weight(catch.weight_g)),
    description: format!("{} · {:.1} cm · {}", catch.owner_name, f64::from(catch.length_mm) / 10.0, status),
    input_message_content: catch_card(catch).into(),
    ..Default::default()
  }
  .into()
}

fn catch_card(catch: &InlineCatchView) -> types::inputMessageRichMessage {
  let mut blocks = vec![
    heading("🎣 Rustwater catch", 1),
    heading(catch.species_name.as_str(), 2),
    table()
      .row(("Caught by", catch.owner_name.as_str()))
      .row(("Length", format_args!("{:.1} cm", f64::from(catch.length_mm) / 10.0)))
      .row(("Weight", format_weight(catch.weight_g)))
      .row(("Water", catch.location_name.as_str()))
      .compact()
      .into(),
  ];
  if catch.world_best {
    blocks.push(block_quote([paragraph("🌍 World record specimen.")]));
  } else if catch.personal_best {
    blocks.push(block_quote([paragraph("● Personal best for this species.")]));
  }
  blocks.push(footer("Rustwater · one persistent world, shared through Telegram"));
  rich(blocks)
}
