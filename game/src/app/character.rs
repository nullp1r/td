//! Character bootstrap, travel, world conditions, and exploration progression.

use rusqlite::{OptionalExtension as _, params};

use super::{App, DISCOVERY_SPECIES, Error, ITEM_RUSTED_KEY, Result, award_xp, character_location, has_active_encounter, knows_location, load_character, require_character_id};
use crate::{
  content::Content,
  fishing,
  ids::{LocationId, SpeciesId},
  view::{CharacterView, ConditionsView, ExploreView, LocationOptionView, LocationView, LocationsView},
  world::{environment_at, next_day_part_change_ms, next_weather_change_ms},
};

impl App {
  pub async fn ensure_character(&self, telegram_user_id: i64, now_ms: i64) -> Result<CharacterView> {
    let content = self.content.clone();
    self
      .run_db(move |connection| {
        // Bootstrap is idempotent and repairs missing starter rows from interrupted/older character creation.
        let tx = connection.transaction()?;
        tx.execute("INSERT OR IGNORE INTO accounts (telegram_user_id) VALUES (?1)", [telegram_user_id])?;
        let account_id: i64 = tx.query_row("SELECT id FROM accounts WHERE telegram_user_id = ?1", [telegram_user_id], |row| row.get(0))?;
        tx.execute("INSERT OR IGNORE INTO characters (account_id, name) VALUES (?1, 'Adventurer')", [account_id])?;
        let (character_id, location_id, rod_id): (i64, u32, u32) =
          tx.query_row("SELECT id, location_id, equipped_rod_id FROM characters WHERE account_id = ?1", [account_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
          })?;
        tx.execute("INSERT OR IGNORE INTO inventory_stacks (character_id, item_def_id, quantity) VALUES (?1, 1, 20)", [character_id])?;
        tx.execute(
          "INSERT OR IGNORE INTO character_locations (character_id, location_id, discovered_at_ms) VALUES (?1, 1, ?2)",
          params![character_id, now_ms],
        )?;
        tx.execute(
          "INSERT OR IGNORE INTO character_locations (character_id, location_id, discovered_at_ms) VALUES (?1, ?2, ?3)",
          params![character_id, location_id, now_ms],
        )?;
        tx.execute("INSERT OR IGNORE INTO character_rods (character_id, rod_id, acquired_at_ms) VALUES (?1, 1, ?2)", params![character_id, now_ms])?;
        tx.execute("INSERT OR IGNORE INTO character_rods (character_id, rod_id, acquired_at_ms) VALUES (?1, ?2, ?3)", params![character_id, rod_id, now_ms])?;
        tx.commit()?;
        Ok(load_character(connection, &content, telegram_user_id, now_ms)?)
      })
      .await
  }

  pub async fn character(&self, telegram_user_id: i64, now_ms: i64) -> Result<CharacterView> {
    let content = self.content.clone();
    self.run_db(move |connection| Ok(load_character(connection, &content, telegram_user_id, now_ms)?)).await
  }

  pub async fn locations(&self, telegram_user_id: i64) -> Result<LocationsView> {
    let content = self.content.clone();
    self.run_db(move |connection| load_locations(connection, &content, telegram_user_id)).await
  }

  pub async fn conditions(&self, telegram_user_id: i64, now_ms: i64) -> Result<ConditionsView> {
    let content = self.content.clone();
    self.run_db(move |connection| load_conditions(connection, &content, telegram_user_id, now_ms)).await
  }

  pub async fn travel(&self, telegram_user_id: i64, location_id: LocationId, now_ms: i64) -> Result<CharacterView> {
    let content = self.content.clone();
    self
      .run_db(move |connection| -> Result<_> {
        if content.locations.binary_search_by_key(&location_id, |location| location.id).is_err() {
          return Err(Error::UnknownLocation);
        }
        let tx = connection.transaction()?;
        let character_id = require_character_id(&tx, telegram_user_id)?;
        if !knows_location(&tx, character_id, location_id)? {
          return Err(Error::LocationLocked);
        }
        if has_active_encounter(&tx, character_id)? {
          return Err(Error::EncounterActive);
        }
        tx.execute("UPDATE characters SET location_id = ?1 WHERE id = ?2", params![location_id.0, character_id])?;
        tx.commit()?;
        load_character(connection, &content, telegram_user_id, now_ms).map_err(Error::from)
      })
      .await
  }

  pub async fn explore(&self, telegram_user_id: i64, now_ms: i64) -> Result<ExploreView> {
    let content = self.content.clone();
    self
      .run_db(move |connection| -> Result<_> {
        let tx = connection.transaction()?;
        let Some((character_id, location_id, active, has_key)) = tx
          .query_row(
            concat!(
              "SELECT c.id, c.location_id, ",
              "EXISTS(SELECT 1 FROM fishing_encounters e WHERE e.character_id = c.id), ",
              "EXISTS(SELECT 1 FROM items i WHERE i.owner_character_id = c.id AND i.item_def_id = ?2) ",
              "FROM characters c JOIN accounts a ON a.id = c.account_id WHERE a.telegram_user_id = ?1",
            ),
            params![telegram_user_id, ITEM_RUSTED_KEY],
            |row| Ok((row.get::<_, i64>(0)?, LocationId(row.get(1)?), row.get::<_, bool>(2)?, row.get::<_, bool>(3)?)),
          )
          .optional()?
        else {
          return Err(Error::CharacterMissing);
        };
        if active {
          return Err(Error::EncounterActive);
        }

        let mut statement = tx.prepare("SELECT location_id FROM character_locations WHERE character_id = ?1 ORDER BY location_id")?;
        let rows = statement.query_map([character_id], |row| Ok(LocationId(row.get(0)?)))?;
        let unlocked = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        drop(statement);

        // Discovery is committed before XP is awarded, so repeated exploration cannot farm the same unlock.
        let finding = exploration_finding(location_id, has_key, &unlocked);
        let mut xp_gained = 0;
        let discovered_location = if let Some(target) = finding.location {
          let inserted = tx.execute(
            "INSERT OR IGNORE INTO character_locations (character_id, location_id, discovered_at_ms) VALUES (?1, ?2, ?3)",
            params![character_id, target.0, now_ms],
          )? == 1;
          if inserted {
            xp_gained = 12;
            award_xp(&tx, character_id, xp_gained)?;
          }
          let location = content.location(target);
          Some(LocationView { name: location.name.clone(), description: location.description.clone(), fishable: location.fishable })
        } else {
          None
        };
        tx.commit()?;
        Ok(ExploreView { title: finding.title, text: finding.text.to_owned(), discovered_location, xp_gained })
      })
      .await
  }
}

fn load_locations(connection: &rusqlite::Connection, content: &Content, telegram_user_id: i64) -> Result<LocationsView> {
  let (character_id, current_location) = character_location(connection, telegram_user_id)?;
  let mut statement = connection.prepare("SELECT location_id FROM character_locations WHERE character_id = ?1 ORDER BY location_id")?;
  let unlocked = statement.query_map([character_id], |row| Ok(LocationId(row.get(0)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
  let locations = content
    .locations
    .iter()
    .filter(|location| unlocked.binary_search(&location.id).is_ok())
    .map(|location| LocationOptionView {
      id: location.id,
      name: location.name.clone(),
      description: location.description.clone(),
      fishable: location.fishable,
      current: location.id == current_location,
    })
    .collect();
  Ok(LocationsView { locations, undiscovered: (content.locations.len() as u32).saturating_sub(unlocked.len() as u32) })
}

fn load_conditions(connection: &rusqlite::Connection, content: &Content, telegram_user_id: i64, now_ms: i64) -> Result<ConditionsView> {
  let (character_id, location_id) = character_location(connection, telegram_user_id)?;
  let environment = environment_at(now_ms);
  let mut statement = connection.prepare("SELECT subject_id FROM discoveries WHERE character_id = ?1 AND kind = ?2 ORDER BY subject_id")?;
  let discovered = statement
    .query_map(params![character_id, DISCOVERY_SPECIES], |row| Ok(SpeciesId(row.get(0)?)))?
    .collect::<rusqlite::Result<Vec<_>>>()?;
  let location = content.location(location_id);
  let mut active_known_species = Vec::new();
  let mut known_but_inactive = 0_u32;
  for encounter in &location.encounters {
    if discovered.binary_search(&encounter.species_id).is_err() {
      continue;
    }
    let species = content.species(encounter.species_id);
    if fishing::eligible(species, environment) {
      if !active_known_species.contains(&species.name) {
        active_known_species.push(species.name.clone());
      }
    } else {
      known_but_inactive = known_but_inactive.saturating_add(1);
    }
  }
  let weather_change = next_weather_change_ms(now_ms);
  Ok(ConditionsView {
    location_name: location.name.clone(),
    fishable: location.fishable,
    environment,
    next_weather: environment_at(weather_change.saturating_add(1)).weather.label(),
    weather_changes_in_ms: weather_change.saturating_sub(now_ms),
    day_part_changes_in_ms: next_day_part_change_ms(now_ms).saturating_sub(now_ms),
    active_known_species,
    known_but_inactive,
  })
}

struct Finding {
  title: &'static str,
  text: &'static str,
  location: Option<LocationId>,
}

const fn finding(title: &'static str, text: &'static str, location: Option<LocationId>) -> Finding {
  Finding { title, text, location }
}

/// Match order is the exploration progression: the first unmet discovery wins.
fn exploration_finding(location_id: LocationId, has_key: bool, unlocked: &[LocationId]) -> Finding {
  let known = |id| unlocked.binary_search(&LocationId(id)).is_ok();
  match location_id.0 {
    1 if !known(2) => finding(
      "A path along the seawall",
      concat!(
        "Beyond the last warehouse, a narrow service path follows the harbor wall. ",
        "It ends where old storm damage broke the stones open to much deeper water.",
      ),
      Some(LocationId(2)),
    ),
    1 if !known(3) => finding(
      "Tracks through the reeds",
      concat!(
        "Fresh footprints leave the harbor road and disappear through shoulder-high reeds. ",
        "Following them reveals a quiet freshwater pond hidden behind the warehouses.",
      ),
      Some(LocationId(3)),
    ),
    1 => finding("Old Harbor", "You know the nearby alleys, piers, and seawall well enough now. Nothing else obvious reveals itself today.", None),
    2 if has_key && !known(4) => finding(
      "The lighthouse emblem",
      concat!(
        "The emblem on your rusted key matches an iron marker half-buried among the collapsed stones. ",
        "A maintenance path climbs from here toward the abandoned lighthouse.",
      ),
      Some(LocationId(4)),
    ),
    2 if has_key => {
      finding("Broken Breakwater", "The rusted key and the old maintenance marker clearly belong to the lighthouse above. The path is already known to you.", None)
    }
    2 => finding(
      "A familiar crest",
      concat!(
        "Between two collapsed blocks you find an iron plate stamped with a lighthouse crest. ",
        "Something used to connect this place to the cliffs above.",
      ),
      None,
    ),
    3 => finding(
      "Reed Pond",
      concat!(
        "You find old bait tins, tiny scales on the reeds, and several places where larger animals pushed through the grass. ",
        "The pond is more active than it first appears.",
      ),
      None,
    ),
    4 if has_key && !known(5) => finding(
      "The key turns",
      concat!(
        "The rusted key resists, then turns with a metallic crack. Inside the lighthouse, a service stair descends through the cliff ",
        "to a sealed sea door. Beyond it lies a pale hidden cove.",
      ),
      Some(LocationId(5)),
    ),
    4 if has_key => finding("Old Lighthouse", "The unlocked service stair still leads down toward the hidden cove beneath the cliffs.", None),
    4 => finding("Old Lighthouse", "The iron door is locked. Its corroded keyhole is surrounded by the same lighthouse crest seen around the old harbor works.", None),
    5 => finding(
      "Lighthouse Cove",
      concat!(
        "There are old anchor points cut into the rock and faint marks beneath the waterline. ",
        "Someone used this cove long before the lighthouse went dark.",
      ),
      None,
    ),
    _ => finding("Exploration", "You search the area but find nothing that changes what you know about it.", None),
  }
}
