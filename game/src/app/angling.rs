//! Fishing encounter lifecycle: cast creation, player reactions, and durable timer transitions.

use rusqlite::{OptionalExtension as _, params};

use super::{
  App, CatchInput, DISCOVERY_RELIC, DISCOVERY_SPECIES, Error, ITEM_RUSTED_KEY, RELIC_RUSTED_KEY, Result, award_xp, has_active_encounter, load_character,
  owns_item, record_catch, require_character_id,
};
use crate::{
  content::{Content, Species},
  fishing::{self, Reaction},
  ids::{BaitId, EncounterId, LocationId, RodId, SpeciesId},
  view::{BiteView, CatchView, CharacterView, EscapeView, ReelOutcome, RelicView, StruggleView, TimerOutcome},
  world::{Environment, environment_at},
};

// Persisted fishing-encounter state and timer tags.
const SPECIAL_NONE: i64 = 0;
const SPECIAL_RUSTED_KEY: i64 = 1;
const PHASE_WAITING: i64 = 0;
const PHASE_BITE: i64 = 1;
const PHASE_STRUGGLE: i64 = 2;
const TIMER_BITE: i64 = 1;
const TIMER_DEADLINE: i64 = 2;

impl App {
  pub async fn cancel_fishing(&self, telegram_user_id: i64, now_ms: i64) -> Result<CharacterView> {
    let content = self.content.clone();
    self
      .run_db(move |connection| -> Result<_> {
        let tx = connection.transaction()?;
        let character_id = require_character_id(&tx, telegram_user_id)?;
        tx.execute("DELETE FROM timers WHERE entity_id IN (SELECT id FROM fishing_encounters WHERE character_id = ?1)", [character_id])?;
        tx.execute("DELETE FROM fishing_encounters WHERE character_id = ?1", [character_id])?;
        tx.commit()?;
        load_character(connection, &content, telegram_user_id, now_ms).map_err(Error::from)
      })
      .await
  }

  pub async fn cast(&self, telegram_user_id: i64, chat_id: i64, message_id: i64, seed: u64, now_ms: i64) -> Result<()> {
    let content = self.content.clone();
    self
      .run_db(move |connection| -> Result<_> {
        let tx = connection.transaction()?;
        let Some((character_id, location_id, bait_id, rod_id)) = tx
          .query_row(
            "SELECT c.id, c.location_id, c.selected_bait_id, c.equipped_rod_id
             FROM characters c JOIN accounts a ON a.id = c.account_id WHERE a.telegram_user_id = ?1",
            [telegram_user_id],
            |row| Ok((row.get::<_, i64>(0)?, LocationId(row.get(1)?), BaitId(row.get(2)?), RodId(row.get(3)?))),
          )
          .optional()?
        else {
          return Err(Error::CharacterMissing);
        };

        if has_active_encounter(&tx, character_id)? {
          return Err(Error::EncounterActive);
        }
        let location = content.location(location_id);
        if !location.fishable {
          return Err(Error::NotFishable);
        }

        // Bait consumption, encounter creation, and its wake-up timer commit together.
        let changed = tx.execute(
          "UPDATE inventory_stacks SET quantity = quantity - 1 WHERE character_id = ?1 AND item_def_id = ?2 AND quantity > 0",
          params![character_id, bait_id.0],
        )?;
        if changed == 0 {
          return Err(Error::NoBait);
        }

        let environment = environment_at(now_ms);
        let (species_id, special_id) = cast_target(&tx, &content, character_id, location_id, bait_id, environment, seed)?;
        let species = content.species(species_id);
        let bait = content.bait(bait_id);
        let due_at_ms = now_ms + fishing::bite_delay_ms(species, bait.fishing_power, seed) as i64;
        let seed_i64 = i64::from_ne_bytes(seed.to_ne_bytes());

        tx.execute(
          "INSERT INTO fishing_encounters
           (character_id, chat_id, message_id, step, phase, species_id, special_id, seed, location_id, bait_id, rod_id, weather, game_minute, created_at_ms)
           VALUES (?1, ?2, ?3, 0, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
          params![
            character_id,
            chat_id,
            message_id,
            PHASE_WAITING,
            species_id.0,
            special_id,
            seed_i64,
            location_id.0,
            bait_id.0,
            rod_id.0,
            environment.weather as i64,
            environment.game_minute,
            now_ms
          ],
        )?;
        let encounter_id = tx.last_insert_rowid();
        tx.execute("INSERT INTO timers (due_at_ms, kind, entity_id, step) VALUES (?1, ?2, ?3, 0)", params![due_at_ms, TIMER_BITE, encounter_id])?;
        tx.commit()?;
        Ok(())
      })
      .await
  }

  /// Starts the reaction deadline only after Telegram has accepted the actionable message; retries are idempotent.
  pub async fn mark_presented(&self, encounter_id: EncounterId, step: u32, now_ms: i64) -> Result<()> {
    let content = self.content.clone();
    self
      .run_db(move |connection| {
        let tx = connection.transaction()?;
        let row = tx
          .query_row("SELECT species_id, step, phase, opened_at_ms FROM fishing_encounters WHERE id = ?1", [encounter_id.0], |row| {
            Ok((SpeciesId(row.get(0)?), row.get::<_, u32>(1)?, row.get::<_, i64>(2)?, row.get::<_, Option<i64>>(3)?))
          })
          .optional()?;
        let Some((species_id, current_step, phase, opened_at)) = row else { return Ok(()) };
        if current_step != step || !matches!(phase, PHASE_BITE | PHASE_STRUGGLE) || opened_at.is_some() {
          return Ok(());
        }

        let species = content.species(species_id);
        let extra = if phase == PHASE_STRUGGLE { 2_000 } else { 0 };
        let deadline = now_ms + species.reaction_ms[2] as i64 + extra;
        tx.execute("UPDATE fishing_encounters SET opened_at_ms = ?1 WHERE id = ?2 AND opened_at_ms IS NULL", params![now_ms, encounter_id.0])?;
        tx.execute(
          "INSERT OR IGNORE INTO timers (due_at_ms, kind, entity_id, step) VALUES (?1, ?2, ?3, ?4)",
          params![deadline, TIMER_DEADLINE, encounter_id.0, step],
        )?;
        tx.commit()?;
        Ok(())
      })
      .await
  }

  pub async fn reel(&self, telegram_user_id: i64, encounter_id: EncounterId, step: u32, received_at_ms: i64) -> Result<ReelOutcome> {
    let content = self.content.clone();
    self
      .run_db(move |connection| -> Result<_> {
        let tx = connection.transaction()?;
        let encounter = require_encounter(&tx, encounter_id, telegram_user_id, step, PHASE_BITE)?;
        let species = content.species(encounter.species_id);
        let reaction = encounter_reaction(species, &encounter, received_at_ms);
        let outcome = if encounter.special_id == SPECIAL_RUSTED_KEY {
          resolve_relic_reel(&tx, encounter_id, &encounter, reaction, received_at_ms)?
        } else {
          resolve_fish_reel(&tx, &content, encounter_id, &encounter, species, reaction, received_at_ms)?
        };
        tx.commit()?;
        Ok(outcome)
      })
      .await
  }

  pub async fn struggle_action(
    &self,
    telegram_user_id: i64,
    encounter_id: EncounterId,
    step: u32,
    action: fishing::StruggleAction,
    received_at_ms: i64,
  ) -> Result<ReelOutcome> {
    let content = self.content.clone();
    self
      .run_db(move |connection| -> Result<_> {
        let tx = connection.transaction()?;
        let encounter = require_encounter(&tx, encounter_id, telegram_user_id, step, PHASE_STRUGGLE)?;
        let species = content.species(encounter.species_id);
        let rod_control = encounter_rod_control(&tx, &content, &encounter)?;
        let reaction = encounter_reaction(species, &encounter, received_at_ms);
        let required = fishing::required_struggle_action(encounter.seed);
        if !fishing::struggle_succeeds(species, rod_control, reaction, action, required) {
          clear_encounter(&tx, encounter_id)?;
          tx.commit()?;
          let reason = if reaction == Reaction::Missed {
            "You hesitate too long during the struggle. The fish uses the slack moment to tear free."
          } else {
            match required {
              fishing::StruggleAction::Pull => "You give the fish room when it needed to be turned. It reaches the rocks and the line parts instantly.",
              fishing::StruggleAction::GiveLine => "You pull into the dive instead of yielding. The tension spikes and the hook rips free.",
            }
          };
          return Ok(ReelOutcome::Escaped(EscapeView { chat_id: encounter.chat_id, message_id: encounter.message_id, reason }));
        }
        let catch = finish_catch(&tx, encounter_id, &encounter, species, reaction, received_at_ms)?;
        tx.commit()?;
        Ok(ReelOutcome::Caught(catch))
      })
      .await
  }

  pub async fn next_timer_due_at(&self) -> Result<Option<i64>> {
    self.run_db(|connection| Ok(connection.query_row("SELECT due_at_ms FROM timers ORDER BY due_at_ms, id LIMIT 1", [], |row| row.get(0)).optional()?)).await
  }

  pub async fn due_timer_ids(&self, now_ms: i64, limit: u32) -> Result<Vec<i64>> {
    self
      .run_db(move |connection| {
        let mut statement = connection.prepare("SELECT id FROM timers WHERE due_at_ms <= ?1 ORDER BY due_at_ms, id LIMIT ?2")?;
        let rows = statement.query_map(params![now_ms, limit], |row| row.get(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
      })
      .await
  }

  pub async fn fire_timer(&self, timer_id: i64, now_ms: i64) -> Result<TimerOutcome> {
    let content = self.content.clone();
    self
      .run_db(move |connection| {
        let tx = connection.transaction()?;
        let Some(timer) = due_timer(&tx, timer_id, now_ms)? else {
          tx.commit()?;
          return Ok(TimerOutcome::Stale);
        };
        let Some(encounter) = timer_encounter(&tx, timer.encounter_id)? else {
          delete_timer(&tx, timer_id)?;
          tx.commit()?;
          return Ok(TimerOutcome::Stale);
        };
        if encounter.step != timer.step {
          delete_timer(&tx, timer_id)?;
          tx.commit()?;
          return Ok(TimerOutcome::Stale);
        }
        let outcome = advance_timer(&tx, &content, timer_id, &timer, &encounter)?;
        tx.commit()?;
        Ok(outcome)
      })
      .await
  }
}

struct DueTimer {
  kind: i64,
  encounter_id: EncounterId,
  step: u32,
}

struct TimerEncounter {
  chat_id: i64,
  message_id: i64,
  step: u32,
  phase: i64,
  species_id: SpeciesId,
  special_id: i64,
}

fn due_timer(tx: &rusqlite::Transaction<'_>, timer_id: i64, now_ms: i64) -> rusqlite::Result<Option<DueTimer>> {
  tx.query_row("SELECT kind, entity_id, step FROM timers WHERE id = ?1 AND due_at_ms <= ?2", params![timer_id, now_ms], |row| {
    Ok(DueTimer { kind: row.get(0)?, encounter_id: EncounterId(row.get(1)?), step: row.get(2)? })
  })
  .optional()
}

fn timer_encounter(tx: &rusqlite::Transaction<'_>, encounter_id: EncounterId) -> rusqlite::Result<Option<TimerEncounter>> {
  tx.query_row("SELECT chat_id, message_id, step, phase, species_id, special_id FROM fishing_encounters WHERE id = ?1", [encounter_id.0], |row| {
    Ok(TimerEncounter {
      chat_id: row.get(0)?,
      message_id: row.get(1)?,
      step: row.get(2)?,
      phase: row.get(3)?,
      species_id: SpeciesId(row.get(4)?),
      special_id: row.get(5)?,
    })
  })
  .optional()
}

fn advance_timer(
  tx: &rusqlite::Transaction<'_>,
  content: &Content,
  timer_id: i64,
  timer: &DueTimer,
  encounter: &TimerEncounter,
) -> rusqlite::Result<TimerOutcome> {
  match timer.kind {
    TIMER_BITE if encounter.phase == PHASE_WAITING => {
      let next_step = encounter.step + 1;
      tx.execute(
        "UPDATE fishing_encounters SET step = ?1, phase = ?2, opened_at_ms = NULL WHERE id = ?3",
        params![next_step, PHASE_BITE, timer.encounter_id.0],
      )?;
      delete_timer(tx, timer_id)?;
      let observation = if encounter.special_id == SPECIAL_RUSTED_KEY {
        "The hook drags against something hard. A metallic shape shifts between the submerged stones."
      } else {
        fishing::observation(content.species(encounter.species_id))
      };
      Ok(TimerOutcome::Bite(BiteView {
        encounter_id: timer.encounter_id,
        step: next_step,
        chat_id: encounter.chat_id,
        message_id: encounter.message_id,
        observation,
      }))
    }
    TIMER_DEADLINE if matches!(encounter.phase, PHASE_BITE | PHASE_STRUGGLE) => {
      clear_encounter(tx, timer.encounter_id)?;
      Ok(TimerOutcome::Escaped(EscapeView {
        chat_id: encounter.chat_id,
        message_id: encounter.message_id,
        reason: "You wait too long. Whatever was on the line is gone.",
      }))
    }
    _ => {
      delete_timer(tx, timer_id)?;
      Ok(TimerOutcome::Stale)
    }
  }
}

fn delete_timer(tx: &rusqlite::Transaction<'_>, timer_id: i64) -> rusqlite::Result<()> {
  tx.execute("DELETE FROM timers WHERE id = ?1", [timer_id])?;
  Ok(())
}

/// Selects the normal fish or the hidden Rusted Key encounter at the breakwater.
fn cast_target(
  tx: &rusqlite::Transaction<'_>,
  content: &Content,
  character_id: i64,
  location: LocationId,
  bait: BaitId,
  environment: Environment,
  seed: u64,
) -> rusqlite::Result<(SpeciesId, i64)> {
  if location == LocationId(2) && !owns_item(tx, character_id, ITEM_RUSTED_KEY)? {
    let catches: u32 = tx.query_row(
      "SELECT count(*) FROM catches c JOIN items i ON i.id = c.item_id WHERE i.owner_character_id = ?1 AND c.location_id = 2",
      [character_id],
      |row| row.get(0),
    )?;
    if catches >= 4 || seed.is_multiple_of(8) {
      return Ok((SpeciesId(1), SPECIAL_RUSTED_KEY));
    }
  }
  Ok((fishing::select_species(content, location, bait, environment, seed), SPECIAL_NONE))
}

fn resolve_relic_reel(
  tx: &rusqlite::Transaction<'_>,
  encounter_id: EncounterId,
  encounter: &ActiveEncounter,
  reaction: Reaction,
  received_at_ms: i64,
) -> rusqlite::Result<ReelOutcome> {
  if reaction == Reaction::Missed {
    clear_encounter(tx, encounter_id)?;
    return Ok(ReelOutcome::Escaped(EscapeView {
      chat_id: encounter.chat_id,
      message_id: encounter.message_id,
      reason: "You react too late. Something metallic scrapes across the stones and drops back into the black water.",
    }));
  }

  tx.execute(
    "INSERT INTO items (owner_character_id, item_def_id, created_at_ms) VALUES (?1, ?2, ?3)",
    params![encounter.character_id, ITEM_RUSTED_KEY, received_at_ms],
  )?;
  tx.execute(
    "INSERT OR IGNORE INTO discoveries (character_id, kind, subject_id, discovered_at_ms) VALUES (?1, ?2, ?3, ?4)",
    params![encounter.character_id, DISCOVERY_RELIC, RELIC_RUSTED_KEY, received_at_ms],
  )?;
  let global_first = tx.execute(
    "INSERT OR IGNORE INTO global_discoveries (kind, subject_id, character_id, discovered_at_ms) VALUES (?1, ?2, ?3, ?4)",
    params![DISCOVERY_RELIC, RELIC_RUSTED_KEY, encounter.character_id, received_at_ms],
  )? == 1;
  let progress = award_xp(tx, encounter.character_id, 30)?;
  clear_encounter(tx, encounter_id)?;
  Ok(ReelOutcome::Relic(RelicView {
    chat_id: encounter.chat_id,
    message_id: encounter.message_id,
    name: "Rusted Key",
    description: "Corroded almost beyond recognition. A faded lighthouse emblem is still visible beneath the salt.",
    global_first,
    xp_gained: 30,
    level: progress.level,
    level_up: progress.level_up,
  }))
}

fn resolve_fish_reel(
  tx: &rusqlite::Transaction<'_>,
  content: &Content,
  encounter_id: EncounterId,
  encounter: &ActiveEncounter,
  species: &Species,
  reaction: Reaction,
  received_at_ms: i64,
) -> rusqlite::Result<ReelOutcome> {
  let rod_control = encounter_rod_control(tx, content, encounter)?;
  let discovery_count: i64 =
    tx.query_row("SELECT count(*) FROM discoveries WHERE character_id = ?1 AND kind = ?2", params![encounter.character_id, DISCOVERY_SPECIES], |row| {
      row.get(0)
    })?;
  let first_catch = discovery_count == 0;

  if !fishing::can_land(species, rod_control, reaction, first_catch) {
    clear_encounter(tx, encounter_id)?;
    let reason = if reaction == Reaction::Missed {
      "You react too late. The line goes slack and the catch disappears."
    } else {
      "The fish overpowers your equipment and tears free. You may need more control."
    };
    return Ok(ReelOutcome::Escaped(EscapeView { chat_id: encounter.chat_id, message_id: encounter.message_id, reason }));
  }

  if species.difficulty >= 13 && !first_catch {
    let next_step = encounter.step.saturating_add(1);
    tx.execute("UPDATE fishing_encounters SET step = ?1, phase = ?2, opened_at_ms = NULL WHERE id = ?3", params![next_step, PHASE_STRUGGLE, encounter_id.0])?;
    let required = fishing::required_struggle_action(encounter.seed);
    return Ok(ReelOutcome::Struggle(StruggleView {
      encounter_id,
      step: next_step,
      chat_id: encounter.chat_id,
      message_id: encounter.message_id,
      observation: fishing::struggle_observation(required),
    }));
  }

  finish_catch(tx, encounter_id, encounter, species, reaction, received_at_ms).map(ReelOutcome::Caught)
}

fn clear_encounter(tx: &rusqlite::Transaction<'_>, encounter_id: EncounterId) -> rusqlite::Result<()> {
  tx.execute("DELETE FROM timers WHERE entity_id = ?1", [encounter_id.0])?;
  tx.execute("DELETE FROM fishing_encounters WHERE id = ?1", [encounter_id.0])?;
  Ok(())
}

struct ActiveEncounter {
  character_id: i64,
  chat_id: i64,
  message_id: i64,
  step: u32,
  phase: i64,
  species_id: SpeciesId,
  special_id: i64,
  seed: u64,
  location_id: LocationId,
  bait_id: BaitId,
  rod_id: RodId,
  weather: i64,
  game_minute: u16,
  opened_at: Option<i64>,
  telegram_user_id: i64,
}

fn require_encounter(tx: &rusqlite::Transaction<'_>, encounter_id: EncounterId, telegram_user_id: i64, step: u32, phase: i64) -> Result<ActiveEncounter> {
  let Some(encounter) = load_encounter(tx, encounter_id)? else { return Err(Error::StaleEncounter) };
  if encounter.telegram_user_id != telegram_user_id {
    return Err(Error::WrongPlayer);
  }
  if encounter.step != step || encounter.phase != phase {
    return Err(Error::StaleEncounter);
  }
  Ok(encounter)
}

fn encounter_reaction(species: &Species, encounter: &ActiveEncounter, received_at_ms: i64) -> Reaction {
  let opened_at = encounter.opened_at.unwrap_or(received_at_ms);
  fishing::reaction(species, received_at_ms.saturating_sub(opened_at) as u64)
}

fn encounter_rod_control(tx: &rusqlite::Transaction<'_>, content: &Content, encounter: &ActiveEncounter) -> rusqlite::Result<u32> {
  let condition =
    tx.query_row("SELECT condition FROM character_rods WHERE character_id = ?1 AND rod_id = ?2", params![encounter.character_id, encounter.rod_id.0], |row| {
      row.get(0)
    })?;
  Ok(fishing::effective_control(content.rod(encounter.rod_id).control, condition))
}

fn load_encounter(tx: &rusqlite::Transaction<'_>, encounter_id: EncounterId) -> rusqlite::Result<Option<ActiveEncounter>> {
  tx.query_row(
    "SELECT e.character_id, e.chat_id, e.message_id, e.step, e.phase, e.species_id, e.special_id, e.seed, e.location_id, e.bait_id, e.rod_id,
            e.weather, e.game_minute, e.opened_at_ms, a.telegram_user_id
     FROM fishing_encounters e
     JOIN characters c ON c.id = e.character_id
     JOIN accounts a ON a.id = c.account_id
     WHERE e.id = ?1",
    [encounter_id.0],
    |row| {
      Ok(ActiveEncounter {
        character_id: row.get(0)?,
        chat_id: row.get(1)?,
        message_id: row.get(2)?,
        step: row.get(3)?,
        phase: row.get(4)?,
        species_id: SpeciesId(row.get(5)?),
        special_id: row.get(6)?,
        seed: u64::from_ne_bytes(row.get::<_, i64>(7)?.to_ne_bytes()),
        location_id: LocationId(row.get(8)?),
        bait_id: BaitId(row.get(9)?),
        rod_id: RodId(row.get(10)?),
        weather: row.get(11)?,
        game_minute: row.get(12)?,
        opened_at: row.get(13)?,
        telegram_user_id: row.get(14)?,
      })
    },
  )
  .optional()
}

fn finish_catch(
  tx: &rusqlite::Transaction<'_>,
  encounter_id: EncounterId,
  encounter: &ActiveEncounter,
  species: &Species,
  reaction: Reaction,
  received_at_ms: i64,
) -> rusqlite::Result<CatchView> {
  let specimen = fishing::generate_specimen(species, encounter.seed);
  let record = record_catch(
    tx,
    species,
    &CatchInput {
      character_id: encounter.character_id,
      species_id: encounter.species_id,
      specimen,
      seed: encounter.seed,
      location_id: encounter.location_id,
      bait_id: encounter.bait_id,
      weather: encounter.weather,
      game_minute: encounter.game_minute,
      caught_at_ms: received_at_ms,
    },
  )?;
  let wear = 1_u32.saturating_add(species.difficulty / 8);
  tx.execute(
    "UPDATE character_rods SET condition = max(1, condition - ?1) WHERE character_id = ?2 AND rod_id = ?3",
    params![wear, encounter.character_id, encounter.rod_id.0],
  )?;
  clear_encounter(tx, encounter_id)?;
  Ok(CatchView {
    chat_id: encounter.chat_id,
    message_id: encounter.message_id,
    species_name: species.name.clone(),
    length_mm: specimen.length_mm,
    weight_g: specimen.weight_g,
    reaction,
    new_species: record.new_species,
    global_first: record.global_first,
    xp_gained: record.xp_gained,
    level: record.level,
    level_up: record.level_up,
    sale_value: fishing::sale_value(species, specimen),
  })
}
