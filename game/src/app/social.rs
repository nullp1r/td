//! Group-chat shoals: deterministic event rotation and one free claim per character.

use rusqlite::params;

use super::{App, CatchInput, Error, Result, record_catch, require_character_id};
use crate::{
  content::Content,
  fishing,
  ids::{BaitId, LocationId, SpeciesId},
  view::{GroupCatchView, GroupEventView},
  world::environment_at,
};

const GROUP_EVENT_REAL_MS: i64 = 20 * 60 * 1000;
const GROUP_SPECIES_ECHO_HERRING: u32 = 25;
const GROUP_SPECIES_RUMOR_CARP: u32 = 26;
const GROUP_LOCATION: LocationId = LocationId(1);
const GROUP_BAIT: BaitId = BaitId(1);

impl App {
  pub async fn group_event(&self, chat_id: i64, now_ms: i64) -> Result<GroupEventView> {
    let content = self.content.clone();
    self.run_db(move |connection| Ok(load_group_event(connection, &content, chat_id, now_ms)?)).await
  }

  pub async fn group_cast(&self, telegram_user_id: i64, chat_id: i64, cycle: i64, seed: u64, now_ms: i64) -> Result<GroupCatchView> {
    let content = self.content.clone();
    self
      .run_db(move |connection| -> Result<_> {
        if group_cycle(now_ms) != cycle {
          return Err(Error::GroupEventExpired);
        }
        let tx = connection.transaction()?;
        let catch = claim_group_catch(&tx, &content, telegram_user_id, chat_id, cycle, seed, now_ms)?;
        tx.commit()?;
        Ok(catch)
      })
      .await
  }
}

/// Claims the chat/cycle once and records the resulting catch in the same transaction.
fn claim_group_catch(
  tx: &rusqlite::Transaction<'_>,
  content: &Content,
  telegram_user_id: i64,
  chat_id: i64,
  cycle: i64,
  seed: u64,
  now_ms: i64,
) -> Result<GroupCatchView> {
  let character_id = require_character_id(tx, telegram_user_id)?;
  let already_claimed: bool = tx.query_row(
    "SELECT EXISTS(SELECT 1 FROM group_event_claims WHERE chat_id = ?1 AND cycle = ?2 AND character_id = ?3)",
    params![chat_id, cycle, character_id],
    |row| row.get(0),
  )?;
  if already_claimed {
    return Err(Error::GroupEventClaimed);
  }

  let species_id = group_species(chat_id, cycle);
  let species = content.species(species_id);
  let specimen = fishing::generate_specimen(species, seed);
  let environment = environment_at(now_ms);
  // Shared casts never consume or wear private equipment. Catch provenance uses the group's
  // Old Harbor/basic-bait context so the ordinary catch schema remains authoritative.
  let record = record_catch(
    tx,
    species,
    &CatchInput {
      character_id,
      species_id,
      specimen,
      seed,
      location_id: GROUP_LOCATION,
      bait_id: GROUP_BAIT,
      weather: environment.weather as i64,
      game_minute: environment.game_minute,
      caught_at_ms: now_ms,
    },
  )?;
  tx.execute(
    "INSERT INTO group_event_claims (chat_id, cycle, character_id, species_id, item_id, claimed_at_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    params![chat_id, cycle, character_id, species_id.0, record.item_id, now_ms],
  )?;
  Ok(GroupCatchView {
    species_name: species.name.clone(),
    length_mm: specimen.length_mm,
    weight_g: specimen.weight_g,
    new_species: record.new_species,
    global_first: record.global_first,
    xp_gained: record.xp_gained,
    level: record.level,
    event: load_group_event(tx, content, chat_id, now_ms)?,
  })
}

// Event identity is derived from wall time and chat ID; only per-character claims need persistence.
fn group_cycle(now_ms: i64) -> i64 {
  now_ms.div_euclid(GROUP_EVENT_REAL_MS)
}

fn group_species(chat_id: i64, cycle: i64) -> SpeciesId {
  let chat_bits = u64::from_ne_bytes(chat_id.to_ne_bytes());
  let cycle_bits = u64::from_ne_bytes(cycle.to_ne_bytes());
  let mixed = chat_bits.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ cycle_bits.rotate_left(23);
  if mixed & 1 == 0 { SpeciesId(GROUP_SPECIES_ECHO_HERRING) } else { SpeciesId(GROUP_SPECIES_RUMOR_CARP) }
}

fn load_group_event(connection: &rusqlite::Connection, content: &Content, chat_id: i64, now_ms: i64) -> rusqlite::Result<GroupEventView> {
  let cycle = group_cycle(now_ms);
  let species_id = group_species(chat_id, cycle);
  let participants: u32 =
    connection.query_row("SELECT count(*) FROM group_event_claims WHERE chat_id = ?1 AND cycle = ?2", params![chat_id, cycle], |row| row.get(0))?;
  let next_cycle = cycle.saturating_add(1).saturating_mul(GROUP_EVENT_REAL_MS);
  Ok(GroupEventView {
    cycle,
    species_name: content.species(species_id).name.clone(),
    participants,
    resets_in_ms: next_cycle.saturating_sub(now_ms),
    ends_at_unix: next_cycle.div_euclid(1_000) as i32,
  })
}
