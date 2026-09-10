//! UI rendering functions producing native Telegram Rich Messages via typed block composition.

use tdx::prelude::*;

use super::data::{RODS, SPOTS};
use super::state::{CatchRecord, Player};

/// View when a line is freshly cast into the water.
pub fn cast_initial(user_id: i64, user_name: &str, spot_name: &str, rod_name: &str) -> (types::inputMessageRichMessage, enums::ReplyMarkup) {
  let content = rich([
    heading(bold(format_args!("🎣 {user_name} cast a line into {spot_name}!")), 2),
    block_quote([paragraph(format_args!("Equipped Rod: {rod_name}"))]),
    paragraph("🌊 . . . 🎣 . . ."),
    paragraph(italic("Waiting patiently for a bite in the depths...")),
  ]);
  let keyboard = markup::column([
    markup::callback("🎣 Reel In!", format_args!("{user_id}:cast:reel")), //.
  ]);
  (content, keyboard)
}

/// Dynamic update when the line twitches.
pub fn cast_nibble() -> types::inputMessageRichMessage {
  rich([
    paragraph(bold("🌊 🐟 . . 🎣")), //.
    block_quote_expandable(italic("The bobber dipped slightly under the surface... something is inspecting the bait!")),
  ])
}

/// View when a bite is detected and the player must react immediately.
pub fn cast_bite(user_id: i64) -> (types::inputMessageRichMessage, enums::ReplyMarkup) {
  let content = rich([
    heading(bold("⚡ 🎣 BITE DETECTED! ⚡"), 1), //.
    paragraph(bold(italic("A massive tug shakes your rod — REEL IT IN NOW!"))),
  ]);
  let keyboard = markup::column([
    markup::callback("⚡ REEL IN NOW! ⚡", format_args!("{user_id}:cast:reel")), //.
  ]);
  (content, keyboard)
}

/// View when the fish escapes or the line was pulled prematurely.
pub fn cast_missed(user_id: i64, reason: &str) -> (types::inputMessageRichMessage, enums::ReplyMarkup) {
  let content = rich([
    heading(bold("💨 The line came up empty!"), 2), //.
    block_quote([paragraph(reason)]),
    paragraph(italic("Cast again when you're ready for another chance.")),
  ]);
  let keyboard = markup::inline([[
    markup::callback("🎣 Cast Again", format_args!("{user_id}:cast:again")),
    markup::callback("🎒 Open Bag", format_args!("{user_id}:bag:view")),
  ]]);
  (content, keyboard)
}

/// View displaying a successfully caught fish with stats and folklore lore formatted in a rich table.
pub fn cast_success(user_id: i64, user_name: &str, record: &CatchRecord, lore: &str) -> (types::inputMessageRichMessage, enums::ReplyMarkup) {
  let catch_table = table().header(("Species", "Rarity", "Weight", "Value")).bordered().striped().compact().caption(bold(record.species_name)).row((
    bold(record.species_name),
    record.rarity.badge(),
    cell(code(format_args!("{:.2} kg", record.weight))).center(),
    cell(bold(format_args!("{} 🪙", record.price))).right(),
  ));

  let content = rich([
    heading(bold(format_args!("🎉 {user_name} hooked a catch!")), 1), //.
    catch_table.into(),
    block_quote([paragraph(italic(lore))]),
  ]);
  let keyboard = markup::inline([[
    markup::callback("🎣 Cast Again", format_args!("{user_id}:cast:again")),
    markup::callback("🎒 Open Bag", format_args!("{user_id}:bag:view")),
  ]]);
  (content, keyboard)
}

/// View displaying player inventory, balance, and recent catches in a structured table.
pub fn bag_view(user_id: i64, player: &Player) -> (types::inputMessageRichMessage, enums::ReplyMarkup) {
  let spot = SPOTS.get(player.spot_id).unwrap_or(&SPOTS[0]);
  let rod = RODS.get(player.rod_id).unwrap_or(&RODS[0]);
  let total_val: u32 = player.bag.iter().map(|c| c.price).sum();

  let overview_table = table().header(("🪙 Balance", "🎣 Rod", "🗺 Water", "🐟 Caught")).bordered().compact().row((
    cell(bold(format_args!("{} 🪙", player.coins))).center(),
    bold(rod.name),
    italic(spot.name),
    cell(code(player.total_caught)).center(),
  ));

  let mut blocks = vec![
    heading(bold(format_args!("🎒 {}'s Angler Bag", player.display_name())), 1), //.
    overview_table.into(),
  ];

  if let Some(best) = &player.best_catch {
    blocks.push(block_quote_expandable(format_args!("⭐ All-Time Best Trophy: {} ({:.2} kg, {} 🪙)", best.species_name, best.weight, best.price)));
  }

  if player.bag.is_empty() {
    blocks.push(paragraph(italic("Your bag is empty. Use /fish to cast a line!")));
  } else {
    let mut catch_table = table().header(("#", "Fish", "Weight", "Value")).bordered().striped().compact();
    for (i, c) in player.bag.iter().rev().take(5).enumerate() {
      catch_table = catch_table.row((
        cell(i + 1).center(),
        bold(c.species_name),
        cell(code(format_args!("{:.2} kg", c.weight))).center(),
        cell(bold(format_args!("{} 🪙", c.price))).right(),
      ));
    }

    blocks.push(paragraph(bold(format_args!("📦 Recent Catches ({} items, {} 🪙 total):", player.bag.len(), total_val))));
    blocks.push(catch_table.into());

    if player.bag.len() > 5 {
      let mut archive_table = table().header(("#", "Fish", "Weight", "Value")).bordered().compact();
      for (i, c) in player.bag.iter().rev().skip(5).enumerate() {
        archive_table = archive_table.row((
          cell(i + 6).center(),
          bold(c.species_name),
          cell(code(format_args!("{:.2} kg", c.weight))).center(),
          cell(bold(format_args!("{} 🪙", c.price))).right(),
        ));
      }
      blocks.push(details(bold(format_args!("📜 Older Catch Archive ({} catches)", player.bag.len() - 5)), [archive_table]));
    }
  }

  let sell = markup::callback("💰 Sell All Fish", format_args!("{user_id}:bag:sell"));
  let cast = markup::callback("🎣 Cast Line", format_args!("{user_id}:cast:again"));
  let shop = markup::callback("🛒 Tackle Shop", format_args!("{user_id}:shop:view"));
  let spots = markup::callback("🗺 Fishing Spots", format_args!("{user_id}:spot:view"));
  let keyboard = markup::inline([[sell, cast], [shop, spots]]);

  (rich(blocks), keyboard)
}

/// View showing the tackle shop for rod upgrades in a structured table.
pub fn shop_view(user_id: i64, player: &Player) -> (types::inputMessageRichMessage, enums::ReplyMarkup) {
  let mut shop_table = table()
    .header(("Rod", "Price", "Luck", "Status")) //.
    .bordered()
    .striped()
    .compact()
    .caption(bold("Available Rods & Tackle"));

  let mut buttons = Vec::with_capacity(RODS.len() + 1);

  for rod in &RODS {
    let is_equipped = player.rod_id == rod.id;
    let is_owned = player.owns_rod(rod.id);
    let status = if is_equipped {
      bold("Equipped")
    } else if is_owned {
      italic("Owned")
    } else {
      bold("Available")
    };

    shop_table = shop_table.row((
      bold(rod.name), //.
      cell(format_args!("{} 🪙", rod.price)).right(),
      cell(format_args!("+{}", rod.luck_bonus)).center(),
      status,
    ));

    let btn = if is_equipped {
      markup::callback(format_args!("✓ {}", rod.name), format_args!("{user_id}:shop:info:{}", rod.id))
    } else if is_owned {
      markup::callback(format_args!("Equip {}", rod.name), format_args!("{user_id}:shop:buy:{}", rod.id))
    } else {
      markup::callback(format_args!("Buy {} ({}🪙)", rod.name, rod.price), format_args!("{user_id}:shop:buy:{}", rod.id))
    };

    buttons.push(vec![btn]);
  }

  buttons.push(vec![
    markup::callback("🎒 Back to Bag", format_args!("{user_id}:bag:view")), //.
    markup::callback("🎣 Cast Line", format_args!("{user_id}:cast:again")),
  ]);

  let msg = rich([
    heading(bold("🛒 Tackle & Rod Shop"), 1), //.
    block_quote([paragraph(format_args!("Current Balance: {} 🪙", player.coins))]),
    shop_table.into(),
  ]);

  (msg, markup::inline(buttons))
}

/// View showing fishing spots and requirements in a structured table.
pub fn spots_view(user_id: i64, player: &Player) -> (types::inputMessageRichMessage, enums::ReplyMarkup) {
  let mut spots_table = table()
    .header(("Location", "Required Tackle", "Status")) //.
    .bordered()
    .striped()
    .compact()
    .caption(bold("Water Boundaries & Requirements"));

  let mut buttons = Vec::with_capacity(SPOTS.len() + 1);

  for spot in &SPOTS {
    let is_current = player.spot_id == spot.id;
    let req_rod = RODS.get(spot.required_rod).map_or("Unknown", |r| r.name);
    let unlocked = player.rod_id >= spot.required_rod;
    let status = if is_current {
      bold("Current")
    } else if unlocked {
      italic("Available")
    } else {
      strike("Locked")
    };

    spots_table = spots_table.row((format_args!("{} - {}", spot.name, spot.description), code(req_rod), status));

    let btn = if is_current {
      markup::callback(format_args!("📍 {}", spot.name), format_args!("{user_id}:spot:info:{}", spot.id))
    } else if unlocked {
      markup::callback(format_args!("Travel to {}", spot.name), format_args!("{user_id}:spot:go:{}", spot.id))
    } else {
      markup::callback(format_args!("🔒 {} (Needs {})", spot.name, req_rod), format_args!("{user_id}:spot:locked:{}", spot.id))
    };

    buttons.push(vec![btn]);
  }

  buttons.push(vec![
    markup::callback("🎒 Back to Bag", format_args!("{user_id}:bag:view")), //.
    markup::callback("🎣 Cast Line", format_args!("{user_id}:cast:again")),
  ]);

  let msg = rich([heading(bold("🗺 Fishing Locations & Waters"), 1), spots_table.into()]);

  (msg, markup::inline(buttons))
}

/// View showing the trophy brag card in a structured table.
pub fn brag_view(player: &Player) -> types::inputMessageRichMessage {
  let name = player.display_name();
  let Some(record) = player.best_catch.as_ref() else {
    return rich([
      paragraph(bold(format_args!("🎣 {name} has not landed any catches yet!"))), //.
      paragraph("Use /fish to cast your first line."),
    ]);
  };

  rich([
    heading(bold(format_args!("🏆 {name}'s Greatest Trophy!")), 1),
    table()
      .header(("Species", "Rarity", "Weight", "Appraised Value"))
      .bordered()
      .striped()
      .compact()
      .caption(bold(record.species_name))
      .row((
        format_args!("🐟 {}", record.species_name),
        record.rarity.badge(),
        cell(code(format_args!("{:.2} kg", record.weight))).center(),
        cell(bold(format_args!("{} 🪙", record.price))).right(),
      ))
      .into(),
    block_quote_expandable(italic("A truly magnificent catch from deep waters!")),
  ])
}

/// General help tour and bot description with clickable commands.
pub fn help_view() -> Text {
  lines((
    bold("📖 Fishing Bot Manual & Guide"),
    "",
    "Welcome to the waters! Cast your line, catch rare and mythical fish, upgrade your rods in the tackle shop, and travel to deep trenches.",
    "",
    bold("Command Index:"),
    (bot_command("/fish"), " — Cast your line into the water"),
    (bot_command("/cast"), " — Alias to cast your line"),
    (bot_command("/bag"), " — View inventory table & sell catches"),
    (bot_command("/shop"), " — Upgrade rods for better luck and bites"),
    (bot_command("/spots"), " — Travel between lakes, rivers, and trenches"),
    (bot_command("/brag"), " — Boast your greatest trophy catch"),
    (bot_command("/help"), " — Show this command guide"),
    "",
    bold("💡 Angler Tips & Secrets:"),
    "• Better rods unlock deeper waters with legendary sea monsters.",
    "• Rarity scale: Common → Uncommon → Rare → Epic → Legendary → Mythic.",
    "• You can sell your catches at the Tackle Shop or boast your all-time record using /brag.",
  ))
}
