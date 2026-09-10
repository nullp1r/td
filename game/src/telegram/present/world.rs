//! Journal, records, locations, and live-world condition presentation.

use tdx::{client, prelude::*};

use crate::{
  telegram::callback::Callback,
  view::{ConditionsView, JournalView, LocationsView, RecordsView},
};

use super::{edit_panel, format_duration, format_game_time, format_weight, format_weight_u64};

pub async fn journal(client: &Client, chat_id: i64, message_id: i64, view: &JournalView) -> client::Result<()> {
  let markup = markup::inline(vec![
    vec![markup::callback("🏆 Records", Callback::Records.encode()), markup::callback("🏷 Titles", Callback::Titles.encode())],
    vec![markup::callback("🏠 Home", Callback::Home.encode())],
  ]);
  edit_panel(client, chat_id, message_id, journal_content(view), markup).await
}

pub async fn locations(client: &Client, chat_id: i64, message_id: i64, view: &LocationsView) -> client::Result<()> {
  edit_panel(client, chat_id, message_id, locations_content(view), locations_markup(view)).await
}

pub async fn conditions(client: &Client, chat_id: i64, message_id: i64, view: &ConditionsView) -> client::Result<()> {
  let markup = markup::inline(vec![
    vec![markup::callback("← Locations", Callback::Locations.encode()), markup::callback("📖 Journal", Callback::Journal.encode())],
    vec![markup::callback("🏠 Home", Callback::Home.encode())],
  ]);
  edit_panel(client, chat_id, message_id, conditions_content(view), markup).await
}

pub async fn records(client: &Client, chat_id: i64, message_id: i64, view: &RecordsView) -> client::Result<()> {
  let markup = markup::inline(vec![
    vec![markup::callback("← Journal", Callback::Journal.encode()), markup::callback("🏷 Titles", Callback::Titles.encode())],
    vec![markup::callback("🏠 Home", Callback::Home.encode())],
  ]);
  edit_panel(client, chat_id, message_id, records_content(view), markup).await
}

pub(super) fn journal_content(view: &JournalView) -> types::inputMessageRichMessage {
  let journal = table().header(("Species", "Best", "Observation")).rows(view.species.iter().map(|species| {
    if species.discovered {
      (
        species.name.as_str(),
        species.best_weight_g.map_or_else(|| "—".to_owned(), format_weight),
        if species.clue.is_empty() { "No useful notes yet." } else { species.clue.as_str() },
      )
    } else {
      ("???", String::new(), "Undiscovered")
    }
  }));
  let relic = if view.rusted_key_discovered { "Rusted Key ✓" } else { "No relics recorded yet." };
  rich([
    heading("📖 Discovery Journal", 1),
    paragraph(format_args!("Species: {} / {} · Locations: {} / {}", view.discovered, view.total, view.locations_discovered, view.locations_total)),
    paragraph(relic),
    journal.striped().compact().into(),
  ])
}

fn locations_content(view: &LocationsView) -> types::inputMessageRichMessage {
  let mut blocks = vec![heading("🗺 Known Waters", 1)];
  if view.undiscovered > 0 {
    blocks.push(paragraph(format_args!("{} location(s) are still missing from your map.", view.undiscovered)));
  }
  for location in &view.locations {
    blocks.push(heading(if location.current { format!("{} · here", location.name) } else { location.name.clone() }, 2));
    blocks.push(paragraph(location.description.as_str()));
    if !location.fishable {
      blocks.push(paragraph(italic("No fishing spot here.")));
    }
  }
  blocks.push(button_row([primary_callback_button("🔎 Explore here", Callback::Explore.encode())]));
  rich(blocks)
}

fn locations_markup(view: &LocationsView) -> enums::ReplyMarkup {
  let mut rows = Vec::new();
  for location in &view.locations {
    if !location.current {
      rows.push(vec![markup::callback(format!("⛵ Travel to {}", location.name), Callback::Travel { location_id: location.id }.encode())]);
    }
  }
  rows.push(vec![markup::callback("🌦 Conditions", Callback::Conditions.encode()), markup::callback("🏠 Home", Callback::Home.encode())]);
  markup::inline(rows)
}

fn conditions_content(view: &ConditionsView) -> types::inputMessageRichMessage {
  let mut blocks = vec![
    heading(format!("🌦 Conditions · {}", view.location_name), 1),
    table()
      .header(("Signal", "Now"))
      .row(("Game time", format_game_time(view.environment.game_minute)))
      .row(("Day part", view.environment.day_part.label()))
      .row(("Weather", view.environment.weather.label()))
      .row(("Next weather", format_args!("{} in {}", view.next_weather, format_duration(view.weather_changes_in_ms))))
      .row(("Day-part change", format_args!("in {}", format_duration(view.day_part_changes_in_ms))))
      .striped()
      .compact()
      .into(),
  ];
  if view.active_known_species.is_empty() {
    blocks.push(paragraph("None of your recorded species are known to be especially available under the current conditions here."));
  } else {
    blocks.push(heading("🐟 Known active species", 2));
    blocks.push(paragraph(view.active_known_species.join(", ")));
  }
  if view.known_but_inactive > 0 {
    blocks.push(paragraph(format_args!(
      "{} recorded species at this location are currently outside their known weather/time conditions.",
      view.known_but_inactive,
    )));
  }
  blocks.push(paragraph(italic("This panel only reveals relationships your character has already discovered. Unknown species remain unknown.")));
  blocks.push(if view.fishable {
    button_row([success_callback_button("🎣 Cast a line", Callback::Cast.encode())])
  } else {
    button_row([primary_callback_button("🔎 Explore", Callback::Explore.encode())])
  });
  rich(blocks)
}

pub(super) fn records_content(view: &RecordsView) -> types::inputMessageRichMessage {
  let mut blocks = vec![
    heading("🏆 Catch Records", 1),
    table()
      .header(("Lifetime", "Value"))
      .row(("Recorded catches", view.lifetime_catches))
      .row(("Total landed mass", format_weight_u64(view.lifetime_weight_g)))
      .striped()
      .compact()
      .into(),
  ];
  if let Some(catch) = &view.heaviest {
    blocks.push(block_quote([paragraph(format_args!(
      "Heaviest specimen: {} · {} · {:.1} cm",
      catch.species_name,
      format_weight(catch.weight_g),
      f64::from(catch.length_mm) / 10.0
    ))]));
  }
  if !view.entries.is_empty() {
    let records = table()
      .header(("Species", "Your best", "World best"))
      .rows(view.entries.iter().map(|entry| (entry.species_name.as_str(), format_weight(entry.personal_best_g), format_weight(entry.world_best_g))));
    blocks.extend([heading("⭐ Personal bests", 2), records.striped().compact().into()]);
  }
  blocks.push(paragraph(italic("Records include specimens that were later sold, turned in, or processed into bait.")));
  rich(blocks)
}
