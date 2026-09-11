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
    paragraph(format!("Sold {} catches for {} coins.", sale.sold, sale.coins_gained)),
    paragraph(format!("Balance · {} coins", sale.coins)),
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
    paragraph(format!("🪙 {} coins · ⭐ Level {} · ✨ {} XP", view.coins, view.level, view.xp)),
  ];
  if view.level_up {
    blocks.push(block_quote([paragraph(format!("⭐ Level {} reached.", view.level))]));
  }
  blocks.extend(tasks_blocks(tasks));
  edit_panel(client, chat_id, message_id, rich(blocks), tasks_markup()).await
}

pub async fn npc(client: &Client, chat_id: i64, message_id: i64, view: &NpcView) -> client::Result<()> {
  let content = rich([heading(format!("{} · {}", view.name, view.title), 1), paragraph(view.text.as_str()), block_quote([paragraph(view.hint.as_str())])]);
  let reply_markup = markup::inline(vec![
    vec![markup::callback("📋 Harbor board", Callback::Tasks.encode()), markup::callback("🌦 Read the water", Callback::Conditions.encode())],
    vec![markup::callback("🏪 Tackle stall", Callback::Shop.encode()), markup::callback("🏠 Home", Callback::Home.encode())],
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
    paragraph(format!("{} produced {} ×{}.", result.recipe_name, result.bait_name, result.quantity)),
    block_quote([paragraph("The specimen is gone from your bag; its catch record remains in your journal.")]),
  ];
  blocks.extend(crafting_blocks(view));
  edit_panel(client, chat_id, message_id, rich(blocks), crafting_markup()).await
}

fn shop_content(view: &ShopView) -> types::inputMessageRichMessage {
  rich(shop_blocks(view))
}

fn shop_blocks(view: &ShopView) -> Vec<enums::InputPageBlock> {
  let rod_table = view.rods.iter().fold(table().header(("", "Rod", "Ctrl", "Cond", "Action")), |table, rod| {
    let marker = if rod.equipped { "●" } else { "○" };
    let action = if rod.equipped {
      inline_disabled_button("Equipped")
    } else if rod.owned {
      inline_disabled_button("Owned")
    } else if u64::from(rod.price) <= view.coins {
      inline_callback_button(format!("Buy · {}c", rod.price), Callback::BuyRod { rod_id: rod.id }.encode())
    } else {
      inline_disabled_button(format!("Need {}c", rod.price))
    };
    table.row((marker, rod.name.as_str(), rod.control, if rod.owned { format!("{}%", rod.condition) } else { "—".to_owned() }, action))
  });

  let bait_table = view.baits.iter().fold(table().header(("", "Bait", "Power", "Have", "Action")), |table, bait| {
    let marker = if bait.selected { "●" } else { "○" };
    let action = if u64::from(bait.price) <= view.coins {
      inline_callback_button(format!("Buy ×{} · {}c", bait.pack_size, bait.price), Callback::BuyBait { bait_id: bait.id }.encode())
    } else {
      inline_disabled_button(format!("Need {}c", bait.price))
    };
    table.row((marker, bait.name.as_str(), bait.power, bait.quantity, action))
  });

  let mut blocks = vec![
    heading("Harbor Tackle Stall", 1),
    paragraph(("Balance · ", bold(format!("{} coins", view.coins)))),
    heading("Rods", 2),
    rod_table.striped().compact().into(),
    heading("Bait", 2),
    bait_table.striped().compact().into(),
  ];

  if view.repair_all_cost > 0 {
    let action = if u64::from(view.repair_all_cost) <= view.coins {
      inline_callback_button(format!("Repair all · {}c", view.repair_all_cost), Callback::RepairRods.encode())
    } else {
      inline_disabled_button(format!("Repairs need {}c", view.repair_all_cost))
    };
    blocks.push(paragraph(("Worn tackle · ", action)));
  }

  if view.can_forage {
    blocks.push(paragraph(("Nothing left in the bait tin · ", inline_callback_button("Dig for 3 worms", Callback::ForageBait.encode()))));
  }

  blocks.push(details(
    "How tackle works",
    [
      paragraph("Control helps keep powerful catches from tearing free. Hard fights wear rods slowly; a worn rod loses some control but is never destroyed."),
      paragraph(
        "Fishing Power shortens the wait for a bite. Species preferences are separate and are meant to be learned from the water and your field notes.",
      ),
      paragraph("The ● marker shows what is currently equipped or selected. Change equipment from Inventory."),
    ],
  ));
  blocks
}

fn shop_markup() -> enums::ReplyMarkup {
  markup::inline([[markup::callback("← Inventory", Callback::Inventory.encode()), markup::callback("🏠 Home", Callback::Home.encode())]])
}

fn tasks_content(view: &TasksView) -> types::inputMessageRichMessage {
  rich(tasks_blocks(view))
}

fn tasks_blocks(view: &TasksView) -> Vec<enums::InputPageBlock> {
  let milestones = view.objectives.iter().fold(table().header(("Milestone", "Progress", "Reward", "Action")), |table, objective| {
    let progress = if objective.claimed {
      "done".to_owned()
    } else if objective.completed {
      "ready".to_owned()
    } else {
      format!("{} / {}", objective.progress, objective.target)
    };
    let action = if objective.claimed {
      inline_disabled_button("Claimed")
    } else if objective.completed {
      inline_callback_button("Claim", Callback::ClaimObjective { objective_id: objective.id }.encode())
    } else {
      inline_disabled_button("In progress")
    };
    table.row((objective.name, progress, objective.reward, action))
  });

  let contract_status = if view.contract.claimed {
    inline_disabled_button("Settled")
  } else if view.contract.ready {
    inline_callback_button("Turn in", Callback::TurnInContract.encode())
  } else {
    inline_disabled_button("Not ready")
  };
  let contract = table().header(("Wanted", "Reward", "Action")).row((
    view.contract.species_name.as_str(),
    format!("{}c · {} XP", view.contract.reward_coins, view.contract.reward_xp),
    contract_status,
  ));

  let mut blocks = vec![
    heading("Harbor Board", 1),
    paragraph("Mara keeps the useful work here: long-running milestones and one specimen request that changes with the harbor day."),
    milestones.striped().compact().into(),
    heading("Today's specimen", 2),
    contract.compact().into(),
    paragraph(format!("Board rotates in {}.", format_duration(view.contract.resets_in_ms))),
  ];

  let notes = view.objectives.iter().map(|objective| paragraph((bold(objective.name), " — ", objective.description))).collect::<Vec<_>>();
  blocks.push(details("Milestone notes", notes));
  blocks.push(paragraph(italic("Contract turn-ins use your smallest matching stored specimen, so the game will not sacrifice a record catch first.")));
  blocks
}

fn tasks_markup() -> enums::ReplyMarkup {
  markup::inline([[markup::callback("🎒 Inventory", Callback::Inventory.encode()), markup::callback("🏠 Home", Callback::Home.encode())]])
}

fn titles_content(view: &TitlesView) -> types::inputMessageRichMessage {
  let titles = view.titles.iter().fold(table().header(("", "Title", "Requirement", "Action")), |table, title| {
    let marker = if title.equipped { "●" } else { "○" };
    let action = if title.equipped {
      inline_disabled_button("Equipped")
    } else if title.unlocked {
      inline_callback_button("Equip", Callback::EquipTitle { title_id: title.id }.encode())
    } else {
      inline_disabled_button("Locked")
    };
    table.row((marker, title.name, title.description, action))
  });
  rich([
    heading("Titles", 1),
    paragraph("Titles are visible proof of what you have done in Rustwater. They do not add combat or fishing stats."),
    titles.striped().compact().into(),
  ])
}

fn titles_markup() -> enums::ReplyMarkup {
  markup::inline([[markup::callback("← Records", Callback::Records.encode()), markup::callback("🏠 Home", Callback::Home.encode())]])
}

fn crafting_content(view: &CraftingView) -> types::inputMessageRichMessage {
  rich(crafting_blocks(view))
}

fn crafting_blocks(view: &CraftingView) -> Vec<enums::InputPageBlock> {
  let recipes = view.recipes.iter().fold(table().header(("Preparation", "Makes", "Action")), |table, recipe| {
    let action =
      if recipe.ready { inline_callback_button("Prepare", Callback::Craft { recipe_id: recipe.id }.encode()) } else { inline_disabled_button("Need specimen") };
    table.row((recipe.name, format!("{} ×{}", recipe.output_name, recipe.output_quantity), action))
  });

  let notes = view.recipes.iter().map(|recipe| paragraph((bold(recipe.name), " — ", recipe.description))).collect::<Vec<_>>();
  vec![
    heading("Tackle Preparation", 1),
    paragraph(format!("Stored specimens available · {}", view.stored_catches)),
    recipes.striped().compact().into(),
    details("Preparation notes", notes),
    paragraph(italic("Preparing bait consumes one stored specimen. Its catch and record history remain in your journal.")),
  ]
}

fn crafting_markup() -> enums::ReplyMarkup {
  markup::inline([[markup::callback("← Inventory", Callback::Inventory.encode()), markup::callback("🏠 Home", Callback::Home.encode())]])
}

fn help_markup() -> enums::ReplyMarkup {
  markup::inline([[markup::primary("🏠 Home", Callback::Home.encode())]])
}

fn help_content() -> types::inputMessageRichMessage {
  rich([
    heading("Welcome to Rustwater", 1),
    paragraph(
      "You are an adventurer at the edge of an old harbor. Fishing is your first way into the world, not the limit of it: watch the water, keep field notes, explore what turns up, and follow threads that look strange enough to matter.",
    ),
    block_quote([paragraph((bold("Start here · "), "go Home and cast a line. When something bites, react quickly; your timing affects the catch."))]),
    paragraph((bold("Fish · "), "catch individual specimens and learn where and when species appear.")),
    paragraph((bold("Explore · "), "find places, relics, people, and routes that are not exposed as a checklist in advance.")),
    paragraph((bold("Prepare · "), "manage gear, sell ordinary specimens, and turn selected catches into bait.")),
    paragraph((bold("Record · "), "Journal and Records preserve discoveries and personal bests; notable catches can be shared through Telegram inline mode.")),
    paragraph((
      bold("Social · "),
      "group shoals are shared observations: read the clue privately, make a choice, and teach the chat what its water tends to do.",
    )),
    paragraph(italic("Coming back later? /start opens your current scene and /help opens this guide.")),
  ])
}
