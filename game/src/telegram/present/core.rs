//! Scene-first Home and compact inventory presentation.

use tdx::{client, prelude::*};

use crate::{
  telegram::callback::Callback,
  view::{CharacterView, InventoryView},
};

use super::{edit_panel, format_weight, share_target};

pub async fn home(client: &Client, chat_id: i64, character: &CharacterView) -> client::Result<types::message> {
  let mut request = send::message(chat_id, home_content(character));
  request.reply_markup = Some(home_markup(character));
  client.track(&request, None, None).await
}

pub async fn home_edit(client: &Client, chat_id: i64, message_id: i64, character: &CharacterView) -> client::Result<()> {
  edit_panel(client, chat_id, message_id, home_content(character), home_markup(character)).await
}

pub async fn inventory(client: &Client, chat_id: i64, message_id: i64, view: &InventoryView) -> client::Result<()> {
  edit_panel(client, chat_id, message_id, inventory_content(view), inventory_markup()).await
}

fn home_content(character: &CharacterView) -> types::inputMessageRichMessage {
  let mut blocks = vec![
    heading(format!("🌊 {}", character.location.name), 1),
    paragraph(character.location.description.as_str()),
    pull_quote(format_args!("{} · {}", character.environment.day_part.label(), character.environment.weather.label())),
    paragraph((bold("Right now — "), home_thread(character))),
  ];

  if character.has_rusted_key {
    blocks.push(block_quote([paragraph("🗝 The Rusted Key bears the old lighthouse crest. It is not a fishing trophy; it is a way farther into Rustwater.")]));
  }
  if character.bait_left == 0 && character.location.fishable {
    blocks.push(block_quote([paragraph(if character.total_bait == 0 {
      "Your tackle pouch is empty. The stall can restock you, or you can dig up a few emergency worms."
    } else {
      "Your selected bait is empty. Another bait in your inventory is still usable."
    })]));
  }
  if !character.location.fishable {
    blocks.push(block_quote([paragraph("There is nowhere useful to cast here. This is a place to investigate, not another fishing menu.")]));
  }

  blocks.push(if character.fishing_active {
    button_row([danger_callback_button("✂️ Cut active line", Callback::CancelFishing.encode())])
  } else if character.location.fishable && character.bait_left > 0 {
    button_row([success_callback_button("🎣 Cast a line", Callback::Cast.encode()), primary_callback_button("🔎 Explore", Callback::Explore.encode())])
  } else if character.location.fishable && character.total_bait > 0 {
    button_row([callback_button("🎒 Choose bait", Callback::Inventory.encode()), primary_callback_button("🔎 Explore", Callback::Explore.encode())])
  } else if character.location.fishable {
    button_row([success_callback_button("🪱 Dig for worms", Callback::ForageBait.encode()), primary_callback_button("🔎 Explore", Callback::Explore.encode())])
  } else {
    button_row([primary_callback_button("🔎 Explore", Callback::Explore.encode())])
  });

  blocks.push(details(
    "Your kit & standing",
    [Into::<enums::InputPageBlock>::into(
      table()
        .row(("Adventurer", character.name.as_str()))
        .row(("Level", character.level))
        .row(("Title", character.title_name.unwrap_or("—")))
        .row(("Coins", character.coins))
        .row(("Rod", format_args!("{} · {}%", character.rod_name, character.rod_condition)))
        .row(("Bait", format_args!("{} ×{}", character.bait_name, character.bait_left)))
        .row(("Field notes", format_args!("{} species · {} catches", character.discovered_species, character.lifetime_catches)))
        .compact(),
    )],
  ));
  rich(blocks)
}

fn home_thread(character: &CharacterView) -> &'static str {
  if character.lifetime_catches == 0 {
    "Mara Reed has given you enough tackle to test the harbor. Land something, then see what that tells you about the water."
  } else if character.has_rusted_key {
    "The harbor has already stopped being only about fish. Follow the lighthouse marks when you are ready."
  } else if character.discovered_species < 4 {
    "You have only begun to learn the local water. Conditions, bait, and location all leave patterns worth noticing."
  } else {
    "Choose your own thread: improve a record, work the Harbor Board, explore farther, or follow whatever looks out of place."
  }
}

fn home_markup(character: &CharacterView) -> enums::ReplyMarkup {
  let mut rows = Vec::new();
  if character.lifetime_catches == 0 {
    if character.has_npc {
      rows.push(vec![markup::primary("💬 Mara Reed", Callback::Talk.encode())]);
    }
    rows.push(vec![markup::callback("🎒 Pack & tackle", Callback::Inventory.encode()), markup::callback("ℹ️ Guide", Callback::Help.encode())]);
    return markup::inline(rows);
  }

  if character.has_npc {
    rows.push(vec![markup::primary("💬 Mara Reed", Callback::Talk.encode()), markup::callback("📋 Harbor Board", Callback::Tasks.encode())]);
  } else {
    rows.push(vec![markup::callback("📋 Harbor Board", Callback::Tasks.encode())]);
  }
  rows.push(vec![markup::callback("🎒 Inventory", Callback::Inventory.encode()), markup::callback("📖 Journal", Callback::Journal.encode())]);
  rows.push(vec![markup::callback("🗺 Places", Callback::Locations.encode()), markup::callback("🌦 Water & weather", Callback::Conditions.encode())]);
  rows.push(vec![markup::callback("ℹ️ Guide", Callback::Help.encode())]);
  markup::inline(rows)
}

fn inventory_content(view: &InventoryView) -> types::inputMessageRichMessage {
  let rods = view.rods.iter().filter(|rod| rod.owned).fold(table().header(("", "Rod", "Ctrl", "Cond.", "")), |table, rod| {
    let action =
      if rod.equipped { inline_disabled_button("Equipped") } else { inline_callback_button("Equip", Callback::EquipRod { rod_id: rod.id }.encode()) };
    table.row((if rod.equipped { "●" } else { "○" }, rod.name.as_str(), rod.control, format!("{}%", rod.condition), action))
  });
  let baits = view.baits.iter().fold(table().header(("", "Bait", "Power", "Owned", "")), |table, bait| {
    let action = if bait.selected {
      inline_disabled_button("Selected")
    } else if bait.quantity > 0 {
      inline_callback_button("Use", Callback::SelectBait { bait_id: bait.id }.encode())
    } else {
      inline_disabled_button("Empty")
    };
    table.row((if bait.selected { "●" } else { "○" }, bait.name.as_str(), bait.power, format!("×{}", bait.quantity), action))
  });

  let mut blocks = vec![
    heading("🎒 Pack & tackle", 1),
    paragraph(format_args!("🪙 {} coins · {} stored catches worth about {} coins", view.coins, view.catch_count, view.sell_value)),
    heading("🎣 Rods", 2),
    rods.striped().compact().into(),
    heading("🪱 Bait", 2),
    baits.striped().compact().into(),
  ];
  if view.has_rusted_key {
    blocks.extend([heading("🗝 Relics", 2), block_quote([paragraph("Rusted Key · old lighthouse crest · kept separate from ordinary saleable catch.")])]);
  }

  blocks.push(heading("🐟 Recent specimens", 2));
  if view.recent_catches.is_empty() {
    blocks.push(paragraph(italic("Nothing stored yet.")));
  } else {
    let catches = view.recent_catches.iter().fold(table().header(("Species", "Size", "Value", "Action")), |table, catch| {
      let share = inline_button(switch_inline_button("Share", format!("catch:{}", catch.item_id), share_target()));
      table.row((
        catch.species_name.as_str(),
        format!("{} · {:.1} cm", format_weight(catch.weight_g), f64::from(catch.length_mm) / 10.0),
        format!("{}c", catch.value),
        share,
      ))
    });
    blocks.push(catches.striped().compact().into());
  }
  if view.catch_count > 0 {
    blocks.push(button_row([danger_callback_button(format_args!("Sell all stored fish · {}c", view.sell_value), Callback::SellAll.encode())]));
  }
  rich(blocks)
}

fn inventory_markup() -> enums::ReplyMarkup {
  markup::inline(vec![
    vec![markup::callback("🧰 Prepare bait", Callback::Crafting.encode()), markup::callback("🏪 Tackle stall", Callback::Shop.encode())],
    vec![markup::switch_inline_target("↗ Share catches", "", share_target())],
    vec![markup::callback("🏠 Home", Callback::Home.encode())],
  ])
}
