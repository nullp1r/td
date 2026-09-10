//! Shop, progression, crafting, title, NPC, and help presentation.

use tdx::{client, prelude::*};

use crate::{
  telegram::callback::Callback,
  view::{CraftResultView, CraftingView, NpcView, RewardView, SaleView, ShopView, TasksView, TitlesView},
};

use super::{edit_panel, format_duration};

pub async fn shop(client: &Client, chat_id: i64, message_id: i64, view: &ShopView) -> client::Result<()> {
  edit_panel(client, chat_id, message_id, shop_content(view), shop_markup()).await
}

pub async fn sold(client: &Client, chat_id: i64, message_id: i64, sale: SaleView, shop: &ShopView) -> client::Result<()> {
  let mut blocks = vec![
    heading("🪙 Catch sold", 1),
    paragraph(format_args!("Sold {} catches for {} coins.", sale.sold, sale.coins_gained)),
    paragraph(format_args!("🪙 Balance: {} coins", sale.coins)),
  ];
  blocks.extend(shop_blocks(shop));
  edit_panel(client, chat_id, message_id, rich(blocks), shop_markup()).await
}

pub async fn help(client: &Client, chat_id: i64, message_id: i64) -> client::Result<()> {
  edit_panel(client, chat_id, message_id, help_content(), help_markup()).await
}

pub async fn help_send(client: &Client, chat_id: i64) -> client::Result<types::message> {
  let mut request = send::message(chat_id, help_content());
  request.reply_markup = Some(help_markup());
  client.track(&request, None, None).await
}

pub async fn tasks(client: &Client, chat_id: i64, message_id: i64, view: &TasksView) -> client::Result<()> {
  edit_panel(client, chat_id, message_id, tasks_content(view), tasks_markup()).await
}

pub async fn reward(client: &Client, chat_id: i64, message_id: i64, view: &RewardView, tasks: &TasksView) -> client::Result<()> {
  let mut blocks = vec![
    heading(format!("🏆 {}", view.title), 1),
    paragraph(view.text.as_str()),
    paragraph(format_args!("🪙 {} coins · ⭐ Level {} · ✨ {} XP", view.coins, view.level, view.xp)),
  ];
  if view.level_up {
    blocks.push(block_quote([paragraph(format_args!("⭐ Level {} reached.", view.level))]));
  }
  blocks.extend(tasks_blocks(tasks));
  edit_panel(client, chat_id, message_id, rich(blocks), tasks_markup()).await
}

pub async fn npc(client: &Client, chat_id: i64, message_id: i64, view: &NpcView) -> client::Result<()> {
  let content = rich([
    heading(format_args!("💬 {} · {}", view.name, view.title), 1),
    paragraph(view.text.as_str()),
    block_quote([paragraph(view.hint.as_str())]),
  ]);
  let reply_markup = markup::inline(vec![
    vec![markup::callback("📋 Harbor board", Callback::Tasks.encode()), markup::callback("🏪 Tackle stall", Callback::Shop.encode())],
    vec![markup::callback("🌦 Conditions", Callback::Conditions.encode()), markup::callback("🏠 Home", Callback::Home.encode())],
  ]);
  edit_panel(client, chat_id, message_id, content, reply_markup).await
}

pub async fn titles(client: &Client, chat_id: i64, message_id: i64, view: &TitlesView) -> client::Result<()> {
  edit_panel(client, chat_id, message_id, titles_content(view), titles_markup()).await
}

pub async fn crafting(client: &Client, chat_id: i64, message_id: i64, view: &CraftingView) -> client::Result<()> {
  edit_panel(client, chat_id, message_id, crafting_content(view), crafting_markup()).await
}

pub async fn crafted(client: &Client, chat_id: i64, message_id: i64, result: &CraftResultView, view: &CraftingView) -> client::Result<()> {
  let mut blocks = vec![
    heading("🧰 Bait prepared", 1),
    paragraph(format_args!("{} produced {} ×{}.", result.recipe_name, result.bait_name, result.quantity)),
    block_quote([paragraph("The specimen was used up, but the catch still counts in Records.")]),
  ];
  blocks.extend(crafting_blocks(view));
  edit_panel(client, chat_id, message_id, rich(blocks), crafting_markup()).await
}

fn shop_content(view: &ShopView) -> types::inputMessageRichMessage {
  rich(shop_blocks(view))
}

fn shop_blocks(view: &ShopView) -> Vec<enums::InputPageBlock> {
  let bait_table = view.baits.iter().fold(table().header(("Bait", "Power", "Pack", "Price", "Owned")), |table, bait| {
    table.row((
      if bait.selected { format!("✓ {}", bait.name) } else { bait.name.clone() },
      bait.power,
      format_args!("×{}", bait.pack_size),
      bait.price,
      bait.quantity,
    ))
  });
  let rod_table = table().header(("Rod", "Control", "Condition", "Price", "Status")).rows(view.rods.iter().map(|rod| {
    let status = if rod.equipped {
      "equipped"
    } else if rod.owned {
      "owned"
    } else {
      ""
    };
    (
      rod.name.as_str(),
      rod.control,
      if rod.owned { format!("{}%", rod.condition) } else { "—".to_owned() },
      if rod.price == 0 { "—".to_owned() } else { rod.price.to_string() },
      status,
    )
  }));
  let mut blocks = vec![
    heading("🏪 Harbor Tackle Stall", 1),
    paragraph(format_args!("🪙 Balance: {} coins", view.coins)),
    heading("🎣 Rods", 2),
    rod_table.striped().compact().into(),
    paragraph(concat!(
      "Control determines whether your equipment can keep powerful catches from tearing free. Difficult catches wear rods slowly; ",
      "low condition only reduces control modestly and never destroys the rod.",
    )),
    heading("🪱 Bait", 2),
    bait_table.striped().compact().into(),
    paragraph(concat!(
      "Fishing Power shortens the wait for a bite. Species preferences are a separate, mostly discoverable effect. ",
      "Buying bait auto-selects it only when the current bait has run out.",
    )),
    paragraph(if view.can_forage {
      "Out of bait? You can dig up three basic worms for free. Once you have bait again, you won’t need this option."
    } else {
      "Digging for emergency worms is available whenever you run completely out of bait."
    }),
  ];
  blocks.extend(
    view
      .rods
      .iter()
      .filter(|rod| !rod.owned && rod.price > 0)
      .map(|rod| button_row([success_callback_button(format_args!("Buy {} · {} coins", rod.name, rod.price), Callback::BuyRod { rod_id: rod.id }.encode())])),
  );
  if view.repair_all_cost > 0 {
    blocks.push(button_row([callback_button(format_args!("Repair all rods · {} coins", view.repair_all_cost), Callback::RepairRods.encode())]));
  }
  blocks.extend(view.baits.iter().map(|bait| {
    button_row([success_callback_button(
      format_args!("Buy {} ×{} · {} coins", bait.name, bait.pack_size, bait.price),
      Callback::BuyBait { bait_id: bait.id }.encode(),
    )])
  }));
  if view.can_forage {
    blocks.push(button_row([callback_button("🪱 Dig for 3 worms · free", Callback::ForageBait.encode())]));
  }
  blocks
}

fn shop_markup() -> enums::ReplyMarkup {
  markup::inline([[markup::callback("← Inventory", Callback::Inventory.encode()), markup::callback("🏠 Home", Callback::Home.encode())]])
}

fn tasks_content(view: &TasksView) -> types::inputMessageRichMessage {
  rich(tasks_blocks(view))
}

fn tasks_blocks(view: &TasksView) -> Vec<enums::InputPageBlock> {
  let milestones = table().header(("Milestone", "Progress", "Reward")).rows(view.objectives.iter().map(|objective| {
    let status = if objective.claimed {
      "claimed".to_owned()
    } else if objective.completed {
      "ready".to_owned()
    } else {
      format!("{} / {}", objective.progress, objective.target)
    };
    (objective.name, status, objective.reward)
  }));
  let mut blocks = vec![heading("📋 Harbor Board", 1), milestones.striped().compact().into()];
  blocks.extend(view.objectives.iter().map(|objective| paragraph(format_args!("{} — {}", objective.name, objective.description))));
  blocks.push(heading("🎯 Harbor contract", 2));
  let contract_status = if view.contract.claimed {
    "Completed this game day"
  } else if view.contract.ready {
    "Matching specimen stored — ready to turn in"
  } else {
    "No matching stored specimen yet"
  };
  blocks.extend([
    paragraph(format_args!(
      "Bring one {}. Reward: {} coins + {} XP. {}.",
      view.contract.species_name, view.contract.reward_coins, view.contract.reward_xp, contract_status
    )),
    paragraph(format_args!("Board rotation in {}.", format_duration(view.contract.resets_in_ms))),
    paragraph(italic("Contract turn-ins use your smallest matching stored specimen, so a record catch is not sacrificed automatically.")),
  ]);
  blocks.extend(view.objectives.iter().filter(|objective| objective.completed && !objective.claimed).map(|objective| {
    button_row([success_callback_button(format_args!("Claim {}", objective.name), Callback::ClaimObjective { objective_id: objective.id }.encode())])
  }));
  if view.contract.ready && !view.contract.claimed {
    blocks.push(button_row([success_callback_button(format_args!("Turn in {}", view.contract.species_name), Callback::TurnInContract.encode())]));
  }
  blocks
}

fn tasks_markup() -> enums::ReplyMarkup {
  markup::inline([[markup::callback("🎒 Inventory", Callback::Inventory.encode()), markup::callback("🏠 Home", Callback::Home.encode())]])
}

fn titles_content(view: &TitlesView) -> types::inputMessageRichMessage {
  let titles = table().header(("Title", "Requirement", "Status")).rows(view.titles.iter().map(|title| {
    let status = if title.equipped {
      "equipped"
    } else if title.unlocked {
      "unlocked"
    } else {
      "locked"
    };
    (title.name, title.description, status)
  }));
  let mut blocks = vec![
    heading("🏷 Titles", 1),
    paragraph(
      "Titles show what you’ve accomplished in Rustwater. They don’t change your fishing strength, and you can swap any unlocked title whenever you like.",
    ),
    titles.striped().compact().into(),
  ];
  blocks.extend(
    view
      .titles
      .iter()
      .filter(|title| title.unlocked && !title.equipped)
      .map(|title| button_row([callback_button(format_args!("Equip {}", title.name), Callback::EquipTitle { title_id: title.id }.encode())])),
  );
  rich(blocks)
}

fn titles_markup() -> enums::ReplyMarkup {
  markup::inline([[markup::callback("← Records", Callback::Records.encode()), markup::callback("🏠 Home", Callback::Home.encode())]])
}

fn crafting_content(view: &CraftingView) -> types::inputMessageRichMessage {
  rich(crafting_blocks(view))
}

fn crafting_blocks(view: &CraftingView) -> Vec<enums::InputPageBlock> {
  let recipes = view.recipes.iter().fold(table().header(("Preparation", "Output", "Status")), |table, recipe| {
    table.row((recipe.name, format_args!("{} ×{}", recipe.output_name, recipe.output_quantity), if recipe.ready { "ready" } else { "missing specimen" }))
  });
  let mut blocks = vec![
    heading("🧰 Tackle Preparation", 1),
    paragraph(format_args!("Stored catches available as materials: {}", view.stored_catches)),
    recipes.striped().compact().into(),
  ];
  blocks.extend(view.recipes.iter().map(|recipe| paragraph(format_args!("{} — {}", recipe.name, recipe.description))));
  blocks.push(paragraph(italic("Preparing bait uses one stored specimen. The catch still counts in Records after the specimen is used.")));
  blocks.extend(
    view
      .recipes
      .iter()
      .filter(|recipe| recipe.ready)
      .map(|recipe| button_row([success_callback_button(format_args!("Prepare {}", recipe.output_name), Callback::Craft { recipe_id: recipe.id }.encode())])),
  );
  blocks
}

fn crafting_markup() -> enums::ReplyMarkup {
  markup::inline([[markup::callback("← Inventory", Callback::Inventory.encode()), markup::callback("🏠 Home", Callback::Home.encode())]])
}

fn help_markup() -> enums::ReplyMarkup {
  markup::inline([[markup::primary("🏠 Home", Callback::Home.encode())]])
}

fn help_content() -> types::inputMessageRichMessage {
  rich([
    heading("🌊 Welcome to Rustwater", 1),
    paragraph(
      "You’re an angler working the old harbor and the waters around it. Fish, explore, keep interesting specimens, and follow whatever the shoreline turns up.",
    ),
    block_quote([paragraph(("🎣 ", bold("Start here:"), " go Home and tap “Cast a line”. When something bites, react to what you see."))]),
    paragraph("🎣 Fish — catch specimens and learn what lives where."),
    paragraph("🧭 Explore — discover places, people, and clues."),
    paragraph("🎒 Prepare — choose gear, sell catches, and make bait."),
    paragraph("📖 Record — Journal and Records track discoveries and personal bests."),
    paragraph("📋 Progress — the Harbor Board offers milestones and a changing specimen contract."),
    paragraph(italic("Coming back later? /start opens Home and /help opens this guide.")),
  ])
}
