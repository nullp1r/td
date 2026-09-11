//! Group-chat shoals: shared observations, private reads, and persistent chat familiarity.

use rusqlite::params;

use super::{App, CatchInput, Error, Result, record_catch, require_character_id};
use crate::{
  content::{Content, Species},
  fishing::{self, Specimen},
  ids::{BaitId, LocationId, SpeciesId},
  view::{GroupApproach, GroupCatchView, GroupEventView},
  world::environment_at,
};

const GROUP_EVENT_REAL_MS: i64 = 20 * 60 * 1000;
const GROUP_SPECIES_ECHO_HERRING: u32 = 25;
const GROUP_SPECIES_RUMOR_CARP: u32 = 26;
const GROUP_LOCATION: LocationId = LocationId(1);
const GROUP_BAIT: BaitId = BaitId(1);

struct GroupRead {
  clue: &'static str,
  memory_hint: &'static str,
  correct: GroupApproach,
}

#[derive(Clone, Copy)]
struct GroupClaimInput {
  telegram_user_id: i64,
  chat_id: i64,
  cycle: i64,
  approach: GroupApproach,
  seed: u64,
  now_ms: i64,
}

impl App {
  pub async fn group_event(&self, chat_id: i64, now_ms: i64) -> Result<GroupEventView> {
    let content = self.content.clone();
    self.run_db(move |connection| Ok(load_group_event(connection, &content, chat_id, now_ms)?)).await
  }

  pub async fn group_cast(&self, telegram_user_id: i64, chat_id: i64, cycle: i64, approach: GroupApproach, seed: u64, now_ms: i64) -> Result<GroupCatchView> {
    let content = self.content.clone();
    self
      .run_db(move |connection| -> Result<_> {
        if group_cycle(now_ms) != cycle {
          return Err(Error::GroupEventExpired);
        }
        let tx = connection.transaction()?;
        let catch = claim_group_catch(&tx, &content, GroupClaimInput { telegram_user_id, chat_id, cycle, approach, seed, now_ms })?;
        tx.commit()?;
        Ok(catch)
      })
      .await
  }
}

/// Claims the chat/cycle once and records the resulting catch in the same transaction.
fn claim_group_catch(tx: &rusqlite::Transaction<'_>, content: &Content, input: GroupClaimInput) -> Result<GroupCatchView> {
  let GroupClaimInput { telegram_user_id, chat_id, cycle, approach, seed, now_ms } = input;
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
  let read = group_read(species_id);
  let read_correct = approach == read.correct;
  let specimen = read_specimen(species, seed, read_correct);
  let environment = environment_at(now_ms);
  // Shared reads never consume or wear private equipment. Catch provenance uses the group's
  // Old Harbor/basic-bait context so the ordinary catch ledger remains authoritative.
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
    "INSERT INTO group_event_claims
     (chat_id, cycle, character_id, species_id, item_id, claimed_at_ms, approach, read_correct)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    params![chat_id, cycle, character_id, species_id.0, record.item_id, now_ms, approach_code(approach), read_correct],
  )?;
  Ok(GroupCatchView {
    item_id: record.item_id,
    species_name: species.name.clone(),
    length_mm: specimen.length_mm,
    weight_g: specimen.weight_g,
    new_species: record.new_species,
    global_first: record.global_first,
    personal_best: record.personal_best,
    world_best: record.world_best,
    xp_gained: record.xp_gained,
    level: record.level,
    approach,
    read_correct,
    event: load_group_event(tx, content, chat_id, now_ms)?,
  })
}

// Event identity is derived from wall time and chat ID; only player reads need persistence.
fn group_cycle(now_ms: i64) -> i64 {
  now_ms.div_euclid(GROUP_EVENT_REAL_MS)
}

fn group_species(chat_id: i64, cycle: i64) -> SpeciesId {
  let chat_bits = u64::from_ne_bytes(chat_id.to_ne_bytes());
  let cycle_bits = u64::from_ne_bytes(cycle.to_ne_bytes());
  let mixed = chat_bits.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ cycle_bits.rotate_left(23);
  if mixed & 1 == 0 { SpeciesId(GROUP_SPECIES_ECHO_HERRING) } else { SpeciesId(GROUP_SPECIES_RUMOR_CARP) }
}

fn group_read(species_id: SpeciesId) -> GroupRead {
  match species_id.0 {
    GROUP_SPECIES_ECHO_HERRING => GroupRead {
      clue: "Small rings repeat in pairs. After the second ripple, the loose line starts to travel sideways.",
      memory_hint: "This chat has seen that doubled ripple before. The fish tends to follow movement instead of pressure.",
      correct: GroupApproach::Drift,
    },
    GROUP_SPECIES_RUMOR_CARP => GroupRead {
      clue: "The surface keeps going unnaturally flat, then something heavy nudges the line from directly below.",
      memory_hint: "People here remember the same pattern: chasing the first twitch usually makes it vanish.",
      correct: GroupApproach::Hold,
    },
    _ => unreachable!("group species table is closed"),
  }
}

fn read_specimen(species: &Species, seed: u64, correct: bool) -> Specimen {
  let first = fishing::generate_specimen(species, seed);
  let second = fishing::generate_specimen(species, seed.rotate_left(17) ^ 0xA5D3_4E91_79B2_C60F);
  if correct {
    if first.weight_g >= second.weight_g { first } else { second }
  } else if first.weight_g <= second.weight_g {
    first
  } else {
    second
  }
}

const fn approach_code(approach: GroupApproach) -> i64 {
  match approach {
    GroupApproach::Drift => 1,
    GroupApproach::Hold => 2,
  }
}

fn load_group_event(connection: &rusqlite::Connection, content: &Content, chat_id: i64, now_ms: i64) -> rusqlite::Result<GroupEventView> {
  let cycle = group_cycle(now_ms);
  let species_id = group_species(chat_id, cycle);
  let participants: u32 =
    connection.query_row("SELECT count(*) FROM group_event_claims WHERE chat_id = ?1 AND cycle = ?2", params![chat_id, cycle], |row| row.get(0))?;
  let total_catches: u32 = connection.query_row("SELECT count(*) FROM group_event_claims WHERE chat_id = ?1", [chat_id], |row| row.get(0))?;
  let (standing_name, standing_level) = group_standing(total_catches);
  let read = group_read(species_id);
  let next_cycle = cycle.saturating_add(1).saturating_mul(GROUP_EVENT_REAL_MS);
  Ok(GroupEventView {
    cycle,
    species_name: (standing_level >= 2).then(|| content.species(species_id).name.clone()),
    clue: read.clue,
    memory_hint: (standing_level >= 1).then_some(read.memory_hint),
    participants,
    total_catches,
    standing_name,
    mastered_approach: (standing_level >= 3).then_some(read.correct),
    resets_in_ms: next_cycle.saturating_sub(now_ms),
    ends_at_unix: next_cycle.div_euclid(1_000) as i32,
  })
}

const fn group_standing(total_catches: u32) -> (&'static str, u8) {
  match total_catches {
    0..=4 => ("Ripple", 0),
    5..=14 => ("Current", 1),
    15..=39 => ("Tidebound", 2),
    _ => ("Harbor Chorus", 3),
  }
}
