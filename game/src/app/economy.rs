//! Inventory, tackle economy, specimen sales, repairs, and bait preparation.

use rusqlite::{OptionalExtension as _, params};

use super::{
  App, Error, ITEM_RUSTED_KEY, Result, character_wallet, has_active_encounter, owns_item, pair_value, quantity, require_character_id, stack_quantities,
};
use crate::{
  content::Content,
  fishing::{self, Specimen},
  ids::{BaitId, RodId, SpeciesId},
  view::{
    BaitStackView, CatchSummaryView, CraftResultView, CraftingView, InventoryView, RecipeView, RodInventoryView, SaleView, ShopBaitView, ShopRodView, ShopView,
  },
};

const REMOVAL_SOLD: i64 = 1;

struct RecipeDefinition {
  id: u32,
  name: &'static str,
  description: &'static str,
  material: Option<SpeciesId>,
  output: BaitId,
  quantity: u32,
}

// Recipe ids are persisted by craft_consumptions; append new recipes instead of renumbering them.
const RECIPES: [RecipeDefinition; 2] = [
  RecipeDefinition {
    id: 1,
    name: "Cut bait",
    description: "Process your smallest stored catch into strong general-purpose fish chunks.",
    material: None,
    output: BaitId(6),
    quantity: 4,
  },
  RecipeDefinition {
    id: 2,
    name: "Crab paste",
    description: "Use one stored Mud Crab to make pungent bait that eels strongly prefer.",
    material: Some(SpeciesId(3)),
    output: BaitId(7),
    quantity: 3,
  },
];

fn recipe(id: u32) -> Option<&'static RecipeDefinition> {
  RECIPES.iter().find(|recipe| recipe.id == id)
}

impl App {
  pub async fn inventory(&self, telegram_user_id: i64) -> Result<InventoryView> {
    let content = self.content.clone();
    self.run_db(move |connection| load_inventory(connection, &content, telegram_user_id).map_err(Error::from)).await
  }

  pub async fn shop(&self, telegram_user_id: i64) -> Result<ShopView> {
    let content = self.content.clone();
    self.run_db(move |connection| load_shop(connection, &content, telegram_user_id).map_err(Error::from)).await
  }

  pub async fn repair_rods(&self, telegram_user_id: i64) -> Result<ShopView> {
    let content = self.content.clone();
    self
      .run_db(move |connection| -> Result<_> {
        let tx = connection.transaction()?;
        let (character_id, coins) = character_wallet(&tx, telegram_user_id)?;
        if has_active_encounter(&tx, character_id)? {
          return Err(Error::EncounterActive);
        }
        let missing: u32 =
          tx.query_row("SELECT coalesce(sum(100 - condition), 0) FROM character_rods WHERE character_id = ?1", [character_id], |row| row.get(0))?;
        let cost = repair_cost(missing);
        if coins < i64::from(cost) {
          return Err(Error::NotEnoughCoins);
        }
        if cost > 0 {
          tx.execute("UPDATE characters SET coins = coins - ?1 WHERE id = ?2", params![cost, character_id])?;
          tx.execute("UPDATE character_rods SET condition = 100 WHERE character_id = ?1", [character_id])?;
        }
        tx.commit()?;
        load_shop(connection, &content, telegram_user_id).map_err(Error::from)
      })
      .await
  }

  pub async fn crafting(&self, telegram_user_id: i64) -> Result<CraftingView> {
    let content = self.content.clone();
    self
      .run_db(move |connection| -> Result<_> {
        let character_id = require_character_id(connection, telegram_user_id)?;
        let counts = stored_catch_counts(connection, character_id)?;
        let stored_catches = counts.iter().fold(0_u32, |total, &(_, count)| total.saturating_add(count));
        let recipes = RECIPES
          .iter()
          .map(|recipe| RecipeView {
            id: recipe.id,
            name: recipe.name,
            description: recipe.description,
            output_name: content.bait(recipe.output).name.clone(),
            output_quantity: recipe.quantity,
            ready: recipe.material.map_or(stored_catches > 0, |species| pair_value(&counts, species).unwrap_or(0) > 0),
          })
          .collect();
        Ok(CraftingView { recipes, stored_catches })
      })
      .await
  }

  pub async fn craft(&self, telegram_user_id: i64, recipe_id: u32, now_ms: i64) -> Result<CraftResultView> {
    let content = self.content.clone();
    self
      .run_db(move |connection| -> Result<_> {
        let recipe = recipe(recipe_id).ok_or(Error::UnknownRecipe)?;
        let tx = connection.transaction()?;
        let character_id = require_character_id(&tx, telegram_user_id)?;
        let item_id = tx
          .query_row(
            "SELECT c.item_id FROM catches c JOIN items i ON i.id = c.item_id
             WHERE i.owner_character_id = ?1 AND i.removed_at_ms IS NULL AND (?2 IS NULL OR c.species_id = ?2)
             ORDER BY c.weight_g ASC, c.item_id ASC LIMIT 1",
            params![character_id, recipe.material.map(|species| species.0)],
            |row| row.get::<_, i64>(0),
          )
          .optional()?
          .ok_or(Error::RecipeNotReady)?;
        tx.execute("UPDATE items SET removed_at_ms = ?1, removal_kind = NULL WHERE id = ?2", params![now_ms, item_id])?;
        tx.execute("INSERT INTO craft_consumptions (item_id, recipe_id, consumed_at_ms) VALUES (?1, ?2, ?3)", params![item_id, recipe.id, now_ms])?;
        add_bait(&tx, character_id, recipe.output, recipe.quantity)?;
        tx.commit()?;
        Ok(CraftResultView { recipe_name: recipe.name, bait_name: content.bait(recipe.output).name.clone(), quantity: recipe.quantity })
      })
      .await
  }

  pub async fn buy_bait(&self, telegram_user_id: i64, bait_id: BaitId) -> Result<ShopView> {
    let content = self.content.clone();
    self
      .run_db(move |connection| -> Result<_> {
        let Ok(index) = content.baits.binary_search_by_key(&bait_id, |bait| bait.id) else {
          return Err(Error::NoBait);
        };
        let bait = &content.baits[index];
        if !bait.buyable {
          return Err(Error::BaitNotForSale);
        }
        let (buy_price, pack_size) = (bait.buy_price, bait.pack_size);
        let tx = connection.transaction()?;
        let (character_id, coins) = character_wallet(&tx, telegram_user_id)?;
        if coins < i64::from(buy_price) {
          return Err(Error::NotEnoughCoins);
        }
        tx.execute("UPDATE characters SET coins = coins - ?1 WHERE id = ?2", params![buy_price, character_id])?;
        add_bait(&tx, character_id, bait_id, pack_size)?;
        tx.commit()?;
        load_shop(connection, &content, telegram_user_id).map_err(Error::from)
      })
      .await
  }

  pub async fn buy_rod(&self, telegram_user_id: i64, rod_id: RodId, now_ms: i64) -> Result<ShopView> {
    let content = self.content.clone();
    self
      .run_db(move |connection| -> Result<_> {
        let Ok(index) = content.rods.binary_search_by_key(&rod_id, |rod| rod.id) else {
          return Err(Error::UnknownRod);
        };
        let rod = &content.rods[index];
        let tx = connection.transaction()?;
        let (character_id, coins) = character_wallet(&tx, telegram_user_id)?;
        if owns_rod(&tx, character_id, rod_id)? {
          return Err(Error::RodAlreadyOwned);
        }
        if coins < i64::from(rod.buy_price) {
          return Err(Error::NotEnoughCoins);
        }
        tx.execute("UPDATE characters SET coins = coins - ?1 WHERE id = ?2", params![rod.buy_price, character_id])?;
        tx.execute("INSERT INTO character_rods (character_id, rod_id, acquired_at_ms) VALUES (?1, ?2, ?3)", params![character_id, rod_id.0, now_ms])?;
        tx.commit()?;
        load_shop(connection, &content, telegram_user_id).map_err(Error::from)
      })
      .await
  }

  pub async fn equip_rod(&self, telegram_user_id: i64, rod_id: RodId) -> Result<InventoryView> {
    let content = self.content.clone();
    self
      .run_db(move |connection| -> Result<_> {
        if content.rods.binary_search_by_key(&rod_id, |rod| rod.id).is_err() {
          return Err(Error::UnknownRod);
        }
        let character_id = require_character_id(connection, telegram_user_id)?;
        if !owns_rod(connection, character_id, rod_id)? {
          return Err(Error::RodNotOwned);
        }
        connection.execute("UPDATE characters SET equipped_rod_id = ?1 WHERE id = ?2", params![rod_id.0, character_id])?;
        load_inventory(connection, &content, telegram_user_id).map_err(Error::from)
      })
      .await
  }

  pub async fn forage_bait(&self, telegram_user_id: i64) -> Result<ShopView> {
    const EMERGENCY_WORMS: u32 = 3;
    const WORM_ID: BaitId = BaitId(1);

    let content = self.content.clone();
    self
      .run_db(move |connection| -> Result<_> {
        let tx = connection.transaction()?;
        let character_id = require_character_id(&tx, telegram_user_id)?;
        let stacks = stack_quantities(&tx, character_id)?;
        let total_bait = content.baits.iter().map(|bait| u64::from(quantity(&stacks, bait.id.0))).fold(0_u64, u64::saturating_add);
        if total_bait != 0 {
          return Err(Error::BaitStillAvailable);
        }
        add_bait(&tx, character_id, WORM_ID, EMERGENCY_WORMS)?;
        tx.commit()?;
        load_shop(connection, &content, telegram_user_id).map_err(Error::from)
      })
      .await
  }

  pub async fn select_bait(&self, telegram_user_id: i64, bait_id: BaitId) -> Result<InventoryView> {
    let content = self.content.clone();
    self
      .run_db(move |connection| -> Result<_> {
        if content.baits.binary_search_by_key(&bait_id, |bait| bait.id).is_err() {
          return Err(Error::NoBait);
        }
        let character_id = require_character_id(connection, telegram_user_id)?;
        let quantity = connection
          .query_row("SELECT quantity FROM inventory_stacks WHERE character_id = ?1 AND item_def_id = ?2", params![character_id, bait_id.0], |row| {
            row.get::<_, u32>(0)
          })
          .optional()?
          .unwrap_or(0);
        if quantity == 0 {
          return Err(Error::NoBait);
        }
        connection.execute("UPDATE characters SET selected_bait_id = ?1 WHERE id = ?2", params![bait_id.0, character_id])?;
        load_inventory(connection, &content, telegram_user_id).map_err(Error::from)
      })
      .await
  }

  pub async fn sell_all_catches(&self, telegram_user_id: i64, now_ms: i64) -> Result<SaleView> {
    let content = self.content.clone();
    self
      .run_db(move |connection| -> Result<_> {
        let tx = connection.transaction()?;
        let (character_id, coins) = character_wallet(&tx, telegram_user_id)?;
        let (sold, gained) = {
          let mut statement = tx.prepare(concat!(
            "SELECT c.species_id, c.length_mm, c.weight_g FROM catches c JOIN items i ON i.id = c.item_id ",
            "WHERE i.owner_character_id = ?1 AND i.removed_at_ms IS NULL",
          ))?;
          let rows = statement.query_map([character_id], |row| Ok((SpeciesId(row.get(0)?), row.get::<_, u32>(1)?, row.get::<_, u32>(2)?)))?;
          let mut sold = 0_u32;
          let mut gained = 0_u64;
          for row in rows {
            let (species_id, length_mm, weight_g) = row?;
            sold = sold.saturating_add(1);
            let species = content.species(species_id);
            gained = gained.saturating_add(fishing::sale_value(species, Specimen { length_mm, weight_g }));
          }
          (sold, gained)
        };
        if sold == 0 {
          return Err(Error::NothingToSell);
        }
        tx.execute(
          concat!(
            "UPDATE items SET removed_at_ms = ?1, removal_kind = ?2 ",
            "WHERE owner_character_id = ?3 AND removed_at_ms IS NULL AND id IN (SELECT item_id FROM catches)",
          ),
          params![now_ms, REMOVAL_SOLD, character_id],
        )?;
        tx.execute("UPDATE characters SET coins = coins + ?1 WHERE id = ?2", params![gained as i64, character_id])?;
        tx.commit()?;
        Ok(SaleView { sold, coins_gained: gained, coins: (coins.max(0) as u64).saturating_add(gained) })
      })
      .await
  }
}

fn stored_catch_counts(connection: &rusqlite::Connection, character_id: i64) -> rusqlite::Result<Vec<(SpeciesId, u32)>> {
  let mut statement = connection.prepare(
    "SELECT c.species_id, count(*) FROM catches c JOIN items i ON i.id = c.item_id
     WHERE i.owner_character_id = ?1 AND i.removed_at_ms IS NULL GROUP BY c.species_id ORDER BY c.species_id",
  )?;
  statement.query_map([character_id], |row| Ok((SpeciesId(row.get(0)?), row.get(1)?)))?.collect()
}

struct InventoryState {
  character_id: i64,
  coins: i64,
  selected_bait_id: BaitId,
  equipped_rod_id: RodId,
}

fn inventory_state(connection: &rusqlite::Connection, telegram_user_id: i64) -> rusqlite::Result<InventoryState> {
  connection.query_row(
    "SELECT c.id, c.coins, c.selected_bait_id, c.equipped_rod_id
     FROM characters c JOIN accounts a ON a.id = c.account_id WHERE a.telegram_user_id = ?1",
    [telegram_user_id],
    |row| {
      Ok(InventoryState { character_id: row.get(0)?, coins: row.get(1)?, selected_bait_id: BaitId(row.get(2)?), equipped_rod_id: RodId(row.get(3)?) })
    },
  )
}

fn owns_rod(connection: &rusqlite::Connection, character_id: i64, rod_id: RodId) -> rusqlite::Result<bool> {
  connection.query_row(
    "SELECT EXISTS(SELECT 1 FROM character_rods WHERE character_id = ?1 AND rod_id = ?2)",
    params![character_id, rod_id.0],
    |row| row.get(0),
  )
}

fn owned_rods(connection: &rusqlite::Connection, character_id: i64) -> rusqlite::Result<Vec<(RodId, u32)>> {
  let mut statement = connection.prepare("SELECT rod_id, condition FROM character_rods WHERE character_id = ?1 ORDER BY rod_id")?;
  statement.query_map([character_id], |row| Ok((RodId(row.get(0)?), row.get(1)?)))?.collect()
}

/// Adds bait atomically and switches selection only when the current stack is empty.
fn add_bait(tx: &rusqlite::Transaction<'_>, character_id: i64, bait_id: BaitId, amount: u32) -> rusqlite::Result<()> {
  tx.execute(
    "INSERT INTO inventory_stacks (character_id, item_def_id, quantity) VALUES (?1, ?2, ?3)
     ON CONFLICT(character_id, item_def_id) DO UPDATE SET quantity = quantity + excluded.quantity",
    params![character_id, bait_id.0, amount],
  )?;
  let selected_quantity: u32 = tx.query_row(
    "SELECT coalesce(s.quantity, 0) FROM characters c
     LEFT JOIN inventory_stacks s ON s.character_id = c.id AND s.item_def_id = c.selected_bait_id WHERE c.id = ?1",
    [character_id],
    |row| row.get(0),
  )?;
  if selected_quantity == 0 {
    tx.execute("UPDATE characters SET selected_bait_id = ?1 WHERE id = ?2", params![bait_id.0, character_id])?;
  }
  Ok(())
}

fn repair_cost(missing_condition: u32) -> u32 {
  missing_condition.saturating_add(9) / 10
}

fn load_inventory(connection: &rusqlite::Connection, content: &Content, telegram_user_id: i64) -> rusqlite::Result<InventoryView> {
  let state = inventory_state(connection, telegram_user_id)?;
  let stacks = stack_quantities(connection, state.character_id)?;
  let baits = content
    .baits
    .iter()
    .map(|bait| BaitStackView {
      id: bait.id,
      name: bait.name.clone(),
      power: bait.fishing_power,
      quantity: quantity(&stacks, bait.id.0),
      selected: bait.id == state.selected_bait_id,
    })
    .collect();

  let owned_rods = owned_rods(connection, state.character_id)?;
  let rods = content
    .rods
    .iter()
    .map(|rod| {
      let condition = pair_value(&owned_rods, rod.id);
      RodInventoryView {
        id: rod.id,
        name: rod.name.clone(),
        control: rod.control,
        condition: condition.unwrap_or(100),
        owned: condition.is_some(),
        equipped: rod.id == state.equipped_rod_id,
      }
    })
    .collect();

  let (recent_catches, catch_count, sell_value) = stored_catch_summary(connection, content, state.character_id)?;
  let has_rusted_key = owns_item(connection, state.character_id, ITEM_RUSTED_KEY)?;
  Ok(InventoryView {
    coins: state.coins.max(0) as u64,
    rod_name: content.rod(state.equipped_rod_id).name.clone(),
    rods,
    baits,
    recent_catches,
    catch_count,
    sell_value,
    has_rusted_key,
  })
}

// One scan feeds both the inventory preview and its aggregate sell action.
fn stored_catch_summary(connection: &rusqlite::Connection, content: &Content, character_id: i64) -> rusqlite::Result<(Vec<CatchSummaryView>, u32, u64)> {
  let mut statement = connection.prepare(
    "SELECT c.species_id, c.length_mm, c.weight_g
     FROM catches c JOIN items i ON i.id = c.item_id
     WHERE i.owner_character_id = ?1 AND i.removed_at_ms IS NULL
     ORDER BY c.caught_at_ms DESC, c.item_id DESC",
  )?;
  let rows = statement.query_map([character_id], |row| Ok((SpeciesId(row.get(0)?), row.get::<_, u32>(1)?, row.get::<_, u32>(2)?)))?;
  let (mut recent, mut count, mut value) = (Vec::new(), 0_u32, 0_u64);
  for row in rows {
    let (species_id, length_mm, weight_g) = row?;
    let species = content.species(species_id);
    let catch_value = fishing::sale_value(species, Specimen { length_mm, weight_g });
    count = count.saturating_add(1);
    value = value.saturating_add(catch_value);
    if recent.len() < 8 {
      recent.push(CatchSummaryView { species_name: species.name.clone(), length_mm, weight_g, value: catch_value });
    }
  }
  Ok((recent, count, value))
}

fn load_shop(connection: &rusqlite::Connection, content: &Content, telegram_user_id: i64) -> rusqlite::Result<ShopView> {
  let state = inventory_state(connection, telegram_user_id)?;
  let stacks = stack_quantities(connection, state.character_id)?;
  let baits = content
    .baits
    .iter()
    .filter(|bait| bait.buyable)
    .map(|bait| ShopBaitView {
      id: bait.id,
      name: bait.name.clone(),
      power: bait.fishing_power,
      quantity: quantity(&stacks, bait.id.0),
      pack_size: bait.pack_size,
      price: bait.buy_price,
      selected: bait.id == state.selected_bait_id,
    })
    .collect();
  let owned_rods = owned_rods(connection, state.character_id)?;
  let rods = content
    .rods
    .iter()
    .map(|rod| {
      let condition = pair_value(&owned_rods, rod.id);
      ShopRodView {
        id: rod.id,
        name: rod.name.clone(),
        control: rod.control,
        condition: condition.unwrap_or(100),
        price: rod.buy_price,
        owned: condition.is_some(),
        equipped: rod.id == state.equipped_rod_id,
      }
    })
    .collect();
  let missing: u32 = owned_rods.iter().map(|&(_, condition)| 100_u32.saturating_sub(condition)).sum();
  let total_bait = content.baits.iter().map(|bait| u64::from(quantity(&stacks, bait.id.0))).fold(0_u64, u64::saturating_add);
  let can_forage = total_bait == 0;
  Ok(ShopView { coins: state.coins.max(0) as u64, baits, rods, can_forage, repair_all_cost: repair_cost(missing) })
}
