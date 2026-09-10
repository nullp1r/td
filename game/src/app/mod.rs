//! Transactional application boundary shared by Telegram and background timer drivers.

mod angling;
mod character;
mod economy;
mod progression;
mod social;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use rusqlite::{Connection, OptionalExtension as _, params};
use thiserror::Error;

use crate::{
  content::{Content, Species},
  db::{self, Db},
  fishing::Specimen,
  ids::{BaitId, LocationId, RodId, SpeciesId},
  view::{CharacterView, LocationView},
  world::environment_at,
};

// These numeric tags are persisted in SQLite; changing them requires a migration.
const DISCOVERY_SPECIES: i64 = 1;
const DISCOVERY_RELIC: i64 = 2;
const RELIC_RUSTED_KEY: i64 = 1;
const SPECIES_ITEM_BASE: u32 = 10_000;
const ITEM_RUSTED_KEY: u32 = 20_001;

const TITLES: [(&str, &str); 7] = [
  ("No title", "Display no earned title."),
  ("Angler", "Land at least 10 catches."),
  ("Naturalist", "Discover at least 10 species."),
  ("Relic Hunter", "Discover the Rusted Key."),
  ("Harbor Hand", "Claim all three harbor milestones."),
  ("Covekeeper", "Discover Lighthouse Cove."),
  ("Shoalbound", "Take part in a group-chat shoal."),
];

#[derive(Clone)]
pub struct App {
  db: Db,
  content: Arc<Content>,
}

#[derive(Debug, Error)]
pub enum Error {
  #[error(transparent)]
  Db(#[from] db::Error),
  #[error("character does not exist")]
  CharacterMissing,
  #[error("no bait remaining")]
  NoBait,
  #[error("a fishing encounter is already active")]
  EncounterActive,
  #[error("fishing encounter is no longer active")]
  StaleEncounter,
  #[error("callback belongs to a different player")]
  WrongPlayer,
  #[error("not enough coins")]
  NotEnoughCoins,
  #[error("there is nothing to sell")]
  NothingToSell,
  #[error("unknown location")]
  UnknownLocation,
  #[error("that location has not been discovered")]
  LocationLocked,
  #[error("there is nowhere to fish here")]
  NotFishable,
  #[error("unknown rod")]
  UnknownRod,
  #[error("you do not own that rod")]
  RodNotOwned,
  #[error("you already own that rod")]
  RodAlreadyOwned,
  #[error("you still have usable bait")]
  BaitStillAvailable,
  #[error("unknown objective")]
  UnknownObjective,
  #[error("that objective is not complete yet")]
  ObjectiveIncomplete,
  #[error("that objective reward was already claimed")]
  ObjectiveClaimed,
  #[error("the current harbor contract is not ready to turn in")]
  ContractNotReady,
  #[error("the current harbor contract has already been completed")]
  ContractClaimed,
  #[error("there is nobody to talk to here")]
  NoNpcHere,
  #[error("that bait is not sold by this shop")]
  BaitNotForSale,
  #[error("unknown crafting recipe")]
  UnknownRecipe,
  #[error("you do not have the required specimen")]
  RecipeNotReady,
  #[error("unknown title")]
  UnknownTitle,
  #[error("that title is still locked")]
  TitleLocked,
  #[error("that group shoal has already moved on")]
  GroupEventExpired,
  #[error("you already joined this group shoal")]
  GroupEventClaimed,
}

type Result<T> = std::result::Result<T, Error>;

impl From<rusqlite::Error> for Error {
  fn from(error: rusqlite::Error) -> Self {
    Self::Db(db::Error::Sql(error))
  }
}

impl App {
  pub fn new(db: Db, content: Arc<Content>) -> Self {
    Self { db, content }
  }

  /// Runs one database job and flattens worker, SQL, and gameplay failures into the application error model.
  async fn run_db<T, F>(&self, job: F) -> Result<T>
  where
    T: Send + 'static,
    F: FnOnce(&mut Connection) -> Result<T> + Send + 'static,
  {
    self.db.job(job).await?
  }
}

fn require_character_id(connection: &Connection, telegram_user_id: i64) -> Result<i64> {
  connection
    .query_row("SELECT c.id FROM characters c JOIN accounts a ON a.id = c.account_id WHERE a.telegram_user_id = ?1", [telegram_user_id], |row| row.get(0))
    .optional()?
    .ok_or(Error::CharacterMissing)
}

fn character_wallet(connection: &Connection, telegram_user_id: i64) -> Result<(i64, i64)> {
  connection
    .query_row("SELECT c.id, c.coins FROM characters c JOIN accounts a ON a.id = c.account_id WHERE a.telegram_user_id = ?1", [telegram_user_id], |row| {
      Ok((row.get(0)?, row.get(1)?))
    })
    .optional()?
    .ok_or(Error::CharacterMissing)
}

fn character_location(connection: &Connection, telegram_user_id: i64) -> Result<(i64, LocationId)> {
  connection
    .query_row("SELECT c.id, c.location_id FROM characters c JOIN accounts a ON a.id = c.account_id WHERE a.telegram_user_id = ?1", [telegram_user_id], |row| {
      Ok((row.get(0)?, LocationId(row.get(1)?)))
    })
    .optional()?
    .ok_or(Error::CharacterMissing)
}

fn has_active_encounter(connection: &Connection, character_id: i64) -> rusqlite::Result<bool> {
  connection.query_row("SELECT EXISTS(SELECT 1 FROM fishing_encounters WHERE character_id = ?1)", [character_id], |row| row.get(0))
}

fn owns_item(connection: &Connection, character_id: i64, item_def_id: u32) -> rusqlite::Result<bool> {
  connection.query_row(
    "SELECT EXISTS(SELECT 1 FROM items WHERE owner_character_id = ?1 AND item_def_id = ?2)",
    params![character_id, item_def_id],
    |row| row.get(0),
  )
}

fn knows_location(connection: &Connection, character_id: i64, location_id: LocationId) -> rusqlite::Result<bool> {
  connection.query_row(
    "SELECT EXISTS(SELECT 1 FROM character_locations WHERE character_id = ?1 AND location_id = ?2)",
    params![character_id, location_id.0],
    |row| row.get(0),
  )
}

fn stack_quantities(connection: &Connection, character_id: i64) -> rusqlite::Result<Vec<(u32, u32)>> {
  let mut statement = connection.prepare("SELECT item_def_id, quantity FROM inventory_stacks WHERE character_id = ?1 ORDER BY item_def_id")?;
  let rows = statement.query_map([character_id], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))?;
  rows.collect()
}

fn pair_value<K: Copy + Ord, V: Copy>(pairs: &[(K, V)], key: K) -> Option<V> {
  pairs.binary_search_by_key(&key, |&(key, _)| key).ok().map(|index| pairs[index].1)
}

fn quantity(stacks: &[(u32, u32)], item_def_id: u32) -> u32 {
  pair_value(stacks, item_def_id).unwrap_or(0)
}

struct CharacterState {
  id: i64,
  name: String,
  xp: i64,
  level: u32,
  coins: i64,
  location_id: LocationId,
  bait_id: BaitId,
  rod_id: RodId,
  title_id: u32,
}

fn load_character(connection: &Connection, content: &Content, telegram_user_id: i64, now_ms: i64) -> rusqlite::Result<CharacterView> {
  let state = connection.query_row(
    "SELECT c.id, c.name, c.xp, c.level, c.coins, c.location_id, c.selected_bait_id, c.equipped_rod_id, c.title_id
     FROM characters c JOIN accounts a ON a.id = c.account_id WHERE a.telegram_user_id = ?1",
    [telegram_user_id],
    |row| {
      Ok(CharacterState {
        id: row.get(0)?,
        name: row.get(1)?,
        xp: row.get(2)?,
        level: row.get(3)?,
        coins: row.get(4)?,
        location_id: LocationId(row.get(5)?),
        bait_id: BaitId(row.get(6)?),
        rod_id: RodId(row.get(7)?),
        title_id: row.get(8)?,
      })
    },
  )?;
  let stacks = stack_quantities(connection, state.id)?;
  let bait_left = quantity(&stacks, state.bait_id.0);
  let total_bait = content.baits.iter().map(|bait| u64::from(quantity(&stacks, bait.id.0))).fold(0_u64, u64::saturating_add);
  let fishing_active = has_active_encounter(connection, state.id)?;
  let has_rusted_key = owns_item(connection, state.id, ITEM_RUSTED_KEY)?;
  let location = content.location(state.location_id);
  let bait = content.bait(state.bait_id);
  let rod = content.rod(state.rod_id);
  let rod_condition = connection.query_row(
    "SELECT condition FROM character_rods WHERE character_id = ?1 AND rod_id = ?2",
    params![state.id, state.rod_id.0],
    |row| row.get::<_, u32>(0),
  )?;
  Ok(CharacterView {
    name: state.name,
    xp: state.xp.max(0) as u64,
    level: state.level,
    coins: state.coins.max(0) as u64,
    location: LocationView { name: location.name.clone(), description: location.description.clone(), fishable: location.fishable },
    bait_name: bait.name.clone(),
    bait_left,
    total_bait: u32::try_from(total_bait).unwrap_or(u32::MAX),
    rod_name: rod.name.clone(),
    rod_condition,
    environment: environment_at(now_ms),
    fishing_active,
    has_rusted_key,
    has_npc: state.location_id == LocationId(1),
    title_name: title_definition(state.title_id).and_then(|(name, _)| (state.title_id != 0).then_some(name)),
  })
}

struct ProgressRecord {
  total_xp: u64,
  level: u32,
  level_up: bool,
}

fn award_xp(tx: &rusqlite::Transaction<'_>, character_id: i64, xp_gained: u32) -> rusqlite::Result<ProgressRecord> {
  let (old_xp, old_level): (i64, u32) = tx.query_row("SELECT xp, level FROM characters WHERE id = ?1", [character_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
  let total_xp = (old_xp.max(0) as u64).saturating_add(u64::from(xp_gained)).min(i64::MAX as u64);
  let level = level_for_xp(total_xp);
  tx.execute("UPDATE characters SET xp = ?1, level = ?2 WHERE id = ?3", params![total_xp as i64, level, character_id])?;
  Ok(ProgressRecord { total_xp, level, level_up: level > old_level })
}
struct CatchRecord {
  item_id: i64,
  new_species: bool,
  global_first: bool,
  xp_gained: u32,
  level: u32,
  level_up: bool,
}

struct CatchInput {
  character_id: i64,
  species_id: SpeciesId,
  specimen: Specimen,
  seed: u64,
  location_id: LocationId,
  bait_id: BaitId,
  weather: i64,
  game_minute: u16,
  caught_at_ms: i64,
}

/// Inserts the immutable catch ledger row, discovery facts, and XP as one transaction step.
fn record_catch(tx: &rusqlite::Transaction<'_>, species: &Species, input: &CatchInput) -> rusqlite::Result<CatchRecord> {
  let &CatchInput { character_id, species_id, specimen, seed, location_id, bait_id, weather, game_minute, caught_at_ms } = input;
  let seed = i64::from_ne_bytes(seed.to_ne_bytes());
  tx.execute(
    "INSERT INTO items (owner_character_id, item_def_id, created_at_ms) VALUES (?1, ?2, ?3)",
    params![character_id, SPECIES_ITEM_BASE + species_id.0, caught_at_ms],
  )?;
  let item_id = tx.last_insert_rowid();
  tx.execute(
    "INSERT INTO catches
     (item_id, species_id, length_mm, weight_g, seed, generator_version, caught_at_ms, location_id, bait_id, weather, game_minute)
     VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7, ?8, ?9, ?10)",
    params![item_id, species_id.0, specimen.length_mm, specimen.weight_g, seed, caught_at_ms, location_id.0, bait_id.0, weather, game_minute],
  )?;

  let new_species = tx.execute(
    "INSERT OR IGNORE INTO discoveries (character_id, kind, subject_id, discovered_at_ms) VALUES (?1, ?2, ?3, ?4)",
    params![character_id, DISCOVERY_SPECIES, species_id.0, caught_at_ms],
  )? == 1;
  let global_first = tx.execute(
    "INSERT OR IGNORE INTO global_discoveries (kind, subject_id, character_id, discovered_at_ms) VALUES (?1, ?2, ?3, ?4)",
    params![DISCOVERY_SPECIES, species_id.0, character_id, caught_at_ms],
  )? == 1;

  let xp_gained = species.xp + if new_species { 10 } else { 0 };
  let progress = award_xp(tx, character_id, xp_gained)?;

  Ok(CatchRecord { item_id, new_species, global_first, xp_gained, level: progress.level, level_up: progress.level_up })
}

fn title_definition(id: u32) -> Option<(&'static str, &'static str)> {
  TITLES.get(id as usize).copied()
}

fn level_for_xp(xp: u64) -> u32 {
  let completed_levels = (xp / 50).isqrt();
  u32::try_from(completed_levels).unwrap_or(u32::MAX - 1).saturating_add(1)
}
