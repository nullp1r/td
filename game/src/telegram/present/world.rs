//! Journal, records, locations, and live-world condition presentation.

use tdx::{client, prelude::*};

use crate::{
  telegram::callback::Callback,
  view::{ConditionsView, JournalView, LocationsView, RecordsView},
};

use super::{edit_panel, format_duration, format_game_time, format_weight, format_weight_u64, share_target};

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
    vec![markup::callback("← Places", Callback::Locations.encode()), markup::callback("📖 Journal", Callback::Journal.encode())],
    vec![markup::callback("🏠 Home", Callback::Home.encode())],
  ]);
  edit_panel(client, chat_id, message_id, conditions_content(view), markup).await
}

pub async fn records(client: &Client, chat_id: i64, message_id: i64, view: &RecordsView) -> client::Result<()> {
  let markup = markup::inline(vec![
    vec![markup::switch_inline_target("↗ Share catches", "", share_target())],
    vec![markup::callback("← Journal", Callback::Journal.encode()), markup::callback("🏷 Titles", Callback::Titles.encode())],
    vec![markup::callback("🏠 Home", Callback::Home.encode())],
  ]);
  edit_panel(client, chat_id, message_id, records_content(view), markup).await
}

pub(super) fn journal_content(view: &JournalView) -> types::inputMessageRichMessage {
  let discovered = view.species.iter().filter(|species| species.discovered).collect::<Vec<_>>();
  let mut blocks = vec![
    heading("Field Journal", 1),
    paragraph(format!("{} of {} species observed · {} of {} places mapped", view.discovered, view.total, view.locations_discovered, view.locations_total)),
  ];

  if view.rusted_key_discovered {
    blocks.push(block_quote([paragraph("🗝 Rusted Key · lighthouse crest. Found in the harbor, but clearly connected to something beyond fishing.")]));
  }

  if discovered.is_empty() {
    blocks.push(paragraph("The pages are mostly empty. Land a specimen and the journal will begin recording what your character actually knows."));
  } else {
    let notes = discovered.iter().fold(table().header(("Species", "Best", "Field note")), |table, species| {
      table.row((
        species.name.as_str(),
        species.best_weight_g.map_or_else(|| "—".to_owned(), format_weight),
        if species.clue.is_empty() { "No useful pattern yet." } else { species.clue.as_str() },
      ))
    });
    blocks.push(notes.striped().compact().into());
  }

  let unknown = view.total.saturating_sub(view.discovered);
  if unknown > 0 {
    blocks.push(paragraph(italic(format!("{unknown} species remain unrecorded. The journal does not name things you have not found."))));
  }
  rich(blocks)
}

fn locations_content(view: &LocationsView) -> types::inputMessageRichMessage {
  let mut blocks = vec![heading("Known Places", 1)];
  if view.undiscovered > 0 {
    blocks.push(paragraph(format!("Your map still has {} blank edge(s). They are found through play, not listed in advance.", view.undiscovered)));
  }
  for location in &view.locations {
    blocks.push(heading(if location.current { format!("● {} · here", location.name) } else { format!("○ {}", location.name) }, 2));
    blocks.push(paragraph(location.description.as_str()));
    if !location.fishable {
      blocks.push(paragraph(italic("There is no useful fishing spot here; this place matters for other reasons.")));
    }
  }
  blocks.push(button_row([primary_callback_button("🔎 Explore where you are", Callback::Explore.encode())]));
  rich(blocks)
}

fn locations_markup(view: &LocationsView) -> enums::ReplyMarkup {
  let mut rows = Vec::new();
  for location in &view.locations {
    if !location.current {
      rows.push(vec![markup::callback(format!("⛵ {}", location.name), Callback::Travel { location_id: location.id }.encode())]);
    }
  }
  rows.push(vec![markup::callback("🌦 Read the water", Callback::Conditions.encode()), markup::callback("🏠 Home", Callback::Home.encode())]);
  markup::inline(rows)
}

fn conditions_content(view: &ConditionsView) -> types::inputMessageRichMessage {
  let mut blocks = vec![
    heading(format!("Water & Weather · {}", view.location_name), 1),
    pull_quote(format!("{} · {}", view.environment.day_part.label(), view.environment.weather.label())),
  ];

  if view.active_known_species.is_empty() {
    blocks.push(paragraph("Nothing in your field notes is strongly associated with these conditions here yet."));
  } else {
    blocks.push(paragraph((bold("Your notes point to · "), view.active_known_species.join(", "))));
  }
  if view.known_but_inactive > 0 {
    blocks
      .push(paragraph(format!("{} other recorded species here are outside the weather or time patterns you have learned for them.", view.known_but_inactive)));
  }

  blocks.push(details(
    "Clock & forecast",
    [Into::<enums::InputPageBlock>::into(
      table()
        .row(("Harbor time", format_game_time(view.environment.game_minute)))
        .row(("Weather shifts", format!("{} · in {}", view.next_weather, format_duration(view.weather_changes_in_ms))))
        .row(("Light changes", format!("in {}", format_duration(view.day_part_changes_in_ms))))
        .compact(),
    )],
  ));
  blocks.push(paragraph(italic("Only relationships your character has discovered are named here. Unknown species stay unknown.")));
  blocks.push(if view.fishable {
    button_row([success_callback_button("🎣 Cast a line", Callback::Cast.encode())])
  } else {
    button_row([primary_callback_button("🔎 Explore", Callback::Explore.encode())])
  });
  rich(blocks)
}

pub(super) fn records_content(view: &RecordsView) -> types::inputMessageRichMessage {
  let mut blocks = vec![
    heading("Catch Records", 1),
    paragraph(format!("{} recorded catches · {} landed in total", view.lifetime_catches, format_weight_u64(view.lifetime_weight_g))),
  ];
  if let Some(catch) = &view.heaviest {
    blocks.push(block_quote([paragraph((
      bold("Heaviest · "),
      catch.species_name.as_str(),
      " · ",
      format_weight(catch.weight_g),
      format!(" · {:.1} cm", f64::from(catch.length_mm) / 10.0),
    ))]));
  }
  if !view.entries.is_empty() {
    let records = table()
      .header(("Species", "Your best", "World"))
      .rows(view.entries.iter().map(|entry| (entry.species_name.as_str(), format_weight(entry.personal_best_g), format_weight(entry.world_best_g))));
    blocks.extend([heading("Personal bests", 2), records.striped().compact().into()]);
  }
  blocks
    .push(paragraph(italic("Catch history survives selling, contracts, and bait preparation. Inventory ownership and recorded history are separate things.")));
  rich(blocks)
}
