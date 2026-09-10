//! Journal, Harbor Board contracts/objectives, NPC guidance, records, and titles.

use rusqlite::{OptionalExtension as _, params};

use super::{
  App, DISCOVERY_RELIC, DISCOVERY_SPECIES, Error, ITEM_RUSTED_KEY, RELIC_RUSTED_KEY, Result, award_xp, character_location, character_wallet,
  knows_location, owns_item, require_character_id, title_definition,
};
use crate::{
  content::{Content, Species},
  fishing::{self, Specimen},
  ids::{BaitId, LocationId, SpeciesId},
  view::{
    CatchSummaryView, ContractView, JournalView, NpcView, ObjectiveView, RecordEntryView, RecordsView, RewardView, SpeciesDiscoveryView, TasksView,
    TitleOptionView, TitlesView,
  },
  world::{GAME_DAY_REAL_MS, game_day},
};

const OBJECTIVE_FIRST_HAUL: u32 = 1;
const OBJECTIVE_FIELD_NOTES: u32 = 2;
const OBJECTIVE_DEEP_WATER: u32 = 3;
const REMOVAL_CONTRACT: i64 = 2;

struct ObjectiveDefinition {
  id: u32,
  name: &'static str,
  description: &'static str,
  reward: &'static str,
  reward_title: &'static str,
  reward_text: &'static str,
  target: u32,
  coins: u32,
  xp: u32,
  bait_reward: Option<(BaitId, u32)>,
}

const OBJECTIVES: [ObjectiveDefinition; 3] = [
  ObjectiveDefinition {
    id: OBJECTIVE_FIRST_HAUL,
    name: "First Haul",
    description: "Catch any five fish.",
    reward: "25 coins + 20 XP",
    reward_title: "First Haul complete",
    reward_text: "Mara pays for proof that you can keep a line wet without supervision.",
    target: 5,
    coins: 25,
    xp: 20,
    bait_reward: None,
  },
  ObjectiveDefinition {
    id: OBJECTIVE_FIELD_NOTES,
    name: "Field Notes",
    description: "Record six different species in your journal.",
    reward: "5 Glow Larvae + 30 XP",
    reward_title: "Field Notes complete",
    reward_text: "Your notes are useful enough that Mara hands over five Glow Larvae from the research box.",
    target: 6,
    coins: 0,
    xp: 30,
    bait_reward: Some((BaitId(3), 5)),
  },
  ObjectiveDefinition {
    id: OBJECTIVE_DEEP_WATER,
    name: "Deep Water",
    description: "Land a difficulty-11+ fish at the Broken Breakwater.",
    reward: "50 coins + 35 XP",
    reward_title: "Deep Water complete",
    reward_text: "Landing something serious off the breakwater earns a proper harbor bounty.",
    target: 1,
    coins: 50,
    xp: 35,
    bait_reward: None,
  },
];

fn objective_definition(id: u32) -> Option<&'static ObjectiveDefinition> {
  OBJECTIVES.iter().find(|objective| objective.id == id)
}

impl App {
  pub async fn journal(&self, telegram_user_id: i64) -> Result<JournalView> {
    let content = self.content.clone();
    self.run_db(move |connection| load_journal(connection, &content, telegram_user_id)).await
  }

  pub async fn tasks(&self, telegram_user_id: i64, now_ms: i64) -> Result<TasksView> {
    let content = self.content.clone();
    self.run_db(move |connection| load_tasks(connection, &content, telegram_user_id, now_ms).map_err(Error::from)).await
  }

  pub async fn claim_objective(&self, telegram_user_id: i64, objective_id: u32, now_ms: i64) -> Result<RewardView> {
    let content = self.content.clone();
    self
      .run_db(move |connection| -> Result<_> {
        let objective = objective_definition(objective_id).ok_or(Error::UnknownObjective)?;
        let tx = connection.transaction()?;
        let (character_id, coins) = character_wallet(&tx, telegram_user_id)?;
        let claimed: bool = tx.query_row(
          "SELECT EXISTS(SELECT 1 FROM objective_claims WHERE character_id = ?1 AND objective_id = ?2)",
          params![character_id, objective_id],
          |row| row.get(0),
        )?;
        if claimed {
          return Err(Error::ObjectiveClaimed);
        }
        if objective_progress(&tx, &content, character_id, objective)? < objective.target {
          return Err(Error::ObjectiveIncomplete);
        }

        if let Some((bait_id, quantity)) = objective.bait_reward {
          tx.execute(
            "INSERT INTO inventory_stacks (character_id, item_def_id, quantity) VALUES (?1, ?2, ?3)
             ON CONFLICT(character_id, item_def_id) DO UPDATE SET quantity = quantity + excluded.quantity",
            params![character_id, bait_id.0, quantity],
          )?;
        }
        let (coins_gained, xp_gained) = (objective.coins, objective.xp);
        if coins_gained > 0 {
          tx.execute("UPDATE characters SET coins = coins + ?1 WHERE id = ?2", params![coins_gained, character_id])?;
        }
        let progress = award_xp(&tx, character_id, xp_gained)?;
        tx.execute(
          "INSERT INTO objective_claims (character_id, objective_id, claimed_at_ms) VALUES (?1, ?2, ?3)",
          params![character_id, objective_id, now_ms],
        )?;
        tx.commit()?;
        Ok(RewardView {
          title: objective.reward_title,
          text: objective.reward_text.to_owned(),
          coins: (coins.max(0) as u64).saturating_add(u64::from(coins_gained)),
          xp: progress.total_xp,
          level: progress.level,
          level_up: progress.level_up,
        })
      })
      .await
  }

  pub async fn turn_in_contract(&self, telegram_user_id: i64, now_ms: i64) -> Result<RewardView> {
    let content = self.content.clone();
    self
      .run_db(move |connection| -> Result<_> {
        let tx = connection.transaction()?;
        let (character_id, coins) = character_wallet(&tx, telegram_user_id)?;
        let cycle = game_day(now_ms);
        let species_id = contract_species(cycle);
        let already_claimed: bool =
          tx.query_row("SELECT EXISTS(SELECT 1 FROM contract_claims WHERE character_id = ?1 AND cycle = ?2)", params![character_id, cycle], |row| row.get(0))?;
        if already_claimed {
          return Err(Error::ContractClaimed);
        }
        // Retire the smallest matching specimen so a contract never sacrifices a better personal catch.
        let item_id = tx
          .query_row(
            concat!(
              "SELECT i.id FROM items i JOIN catches c ON c.item_id = i.id ",
              "WHERE i.owner_character_id = ?1 AND i.removed_at_ms IS NULL AND c.species_id = ?2 ",
              "ORDER BY c.weight_g, c.caught_at_ms, i.id LIMIT 1",
            ),
            params![character_id, species_id.0],
            |row| row.get::<_, i64>(0),
          )
          .optional()?;
        let Some(item_id) = item_id else {
          return Err(Error::ContractNotReady);
        };
        let species = content.species(species_id);
        let reward_coins = contract_reward_coins(species);
        let reward_xp = contract_reward_xp(species);
        tx.execute(
          "UPDATE items SET removed_at_ms = ?1, removal_kind = ?2 WHERE id = ?3 AND removed_at_ms IS NULL",
          params![now_ms, REMOVAL_CONTRACT, item_id],
        )?;
        tx.execute(
          "INSERT INTO contract_claims (character_id, cycle, species_id, item_id, claimed_at_ms) VALUES (?1, ?2, ?3, ?4, ?5)",
          params![character_id, cycle, species_id.0, item_id, now_ms],
        )?;
        tx.execute("UPDATE characters SET coins = coins + ?1 WHERE id = ?2", params![reward_coins, character_id])?;
        let progress = award_xp(&tx, character_id, reward_xp)?;
        tx.commit()?;
        Ok(RewardView {
          title: "Harbor contract complete",
          text: format!("Mara accepts your smallest stored {} specimen. Better specimens stay in your collection.", species.name),
          coins: (coins.max(0) as u64).saturating_add(u64::from(reward_coins)),
          xp: progress.total_xp,
          level: progress.level,
          level_up: progress.level_up,
        })
      })
      .await
  }

  pub async fn npc(&self, telegram_user_id: i64, now_ms: i64) -> Result<NpcView> {
    let content = self.content.clone();
    self.run_db(move |connection| load_npc(connection, &content, telegram_user_id, now_ms)).await
  }

  pub async fn records(&self, telegram_user_id: i64) -> Result<RecordsView> {
    let content = self.content.clone();
    self.run_db(move |connection| load_records(connection, &content, telegram_user_id)).await
  }

  pub async fn titles(&self, telegram_user_id: i64) -> Result<TitlesView> {
    self
      .run_db(move |connection| -> Result<_> {
        let Some((character_id, equipped_id)) = connection
          .query_row(
            "SELECT c.id, c.title_id FROM characters c JOIN accounts a ON a.id = c.account_id WHERE a.telegram_user_id = ?1",
            [telegram_user_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, u32>(1)?)),
          )
          .optional()?
        else {
          return Err(Error::CharacterMissing);
        };
        load_titles(connection, character_id, equipped_id).map_err(Error::from)
      })
      .await
  }

  pub async fn equip_title(&self, telegram_user_id: i64, title_id: u32) -> Result<TitlesView> {
    if title_definition(title_id).is_none() {
      return Err(Error::UnknownTitle);
    }
    self
      .run_db(move |connection| -> Result<_> {
        let character_id = require_character_id(connection, telegram_user_id)?;
        let unlocks = title_unlocks(connection, character_id)?;
        let title_index = usize::try_from(title_id).unwrap_or(usize::MAX);
        if !unlocks.get(title_index).copied().unwrap_or(false) {
          return Err(Error::TitleLocked);
        }
        connection.execute("UPDATE characters SET title_id = ?1 WHERE id = ?2", params![title_id, character_id])?;
        Ok(build_titles(title_id, unlocks))
      })
      .await
  }
}

fn load_journal(connection: &rusqlite::Connection, content: &Content, telegram_user_id: i64) -> Result<JournalView> {
  let character_id = require_character_id(connection, telegram_user_id)?;
  let mut statement = connection.prepare(
    "WITH best AS (
       SELECT c.species_id, max(c.weight_g) AS weight_g FROM catches c JOIN items i ON i.id = c.item_id
       WHERE i.owner_character_id = ?1 GROUP BY c.species_id
     )
     SELECT d.subject_id, best.weight_g FROM discoveries d LEFT JOIN best ON best.species_id = d.subject_id
     WHERE d.character_id = ?1 AND d.kind = ?2 ORDER BY d.subject_id",
  )?;
  let discoveries = statement
    .query_map(params![character_id, DISCOVERY_SPECIES], |row| Ok((SpeciesId(row.get(0)?), row.get::<_, Option<u32>>(1)?)))?
    .collect::<rusqlite::Result<Vec<_>>>()?;
  let species = content
    .species
    .iter()
    .map(|species| {
      let discovery = discoveries.binary_search_by_key(&species.id, |&(id, _)| id).ok().and_then(|index| discoveries.get(index));
      SpeciesDiscoveryView {
        name: species.name.clone(),
        discovered: discovery.is_some(),
        best_weight_g: discovery.and_then(|&(_, best)| best),
        clue: species.clue.clone(),
      }
    })
    .collect::<Vec<_>>();
  let rusted_key_discovered = connection.query_row(
    "SELECT EXISTS(SELECT 1 FROM discoveries WHERE character_id = ?1 AND kind = ?2 AND subject_id = ?3)",
    params![character_id, DISCOVERY_RELIC, RELIC_RUSTED_KEY],
    |row| row.get(0),
  )?;
  let locations_discovered = connection.query_row("SELECT count(*) FROM character_locations WHERE character_id = ?1", [character_id], |row| row.get(0))?;
  Ok(JournalView {
    discovered: discoveries.len() as u32,
    total: species.len() as u32,
    species,
    rusted_key_discovered,
    locations_discovered,
    locations_total: content.locations.len() as u32,
  })
}

fn load_npc(connection: &rusqlite::Connection, content: &Content, telegram_user_id: i64, now_ms: i64) -> Result<NpcView> {
  let (character_id, location_id) = character_location(connection, telegram_user_id)?;
  if location_id != LocationId(1) {
    return Err(Error::NoNpcHere);
  }
  let story = npc_story(
    owns_item(connection, character_id, ITEM_RUSTED_KEY)?,
    knows_location(connection, character_id, LocationId(4))?,
    knows_location(connection, character_id, LocationId(5))?,
  );
  let contract = load_contract(connection, content, character_id, now_ms)?;
  let stored_catches: u32 = connection.query_row(
    "SELECT count(*) FROM catches c JOIN items i ON i.id = c.item_id WHERE i.owner_character_id = ?1 AND i.removed_at_ms IS NULL",
    [character_id],
    |row| row.get(0),
  )?;
  let mut hint = contract_hint(&contract);
  if stored_catches > 0 {
    hint.push_str(" And don't sell everything blindly—the tackle bench can turn ordinary specimens into useful bait.");
  }
  Ok(NpcView { name: "Mara", title: "Harbor Warden", text: story.to_owned(), hint })
}

fn load_records(connection: &rusqlite::Connection, content: &Content, telegram_user_id: i64) -> Result<RecordsView> {
  let character_id = require_character_id(connection, telegram_user_id)?;
  let (lifetime_catches, lifetime_weight_g): (i64, i64) = connection.query_row(
    "SELECT count(*), coalesce(sum(c.weight_g), 0) FROM catches c JOIN items i ON i.id = c.item_id WHERE i.owner_character_id = ?1",
    [character_id],
    |row| Ok((row.get(0)?, row.get(1)?)),
  )?;
  let heaviest = connection
    .query_row(
      "SELECT c.species_id, c.length_mm, c.weight_g FROM catches c JOIN items i ON i.id = c.item_id
       WHERE i.owner_character_id = ?1 ORDER BY c.weight_g DESC, c.item_id ASC LIMIT 1",
      [character_id],
      |row| Ok((SpeciesId(row.get(0)?), row.get::<_, u32>(1)?, row.get::<_, u32>(2)?)),
    )
    .optional()?
    .map(|(species_id, length_mm, weight_g)| {
      let species = content.species(species_id);
      CatchSummaryView { species_name: species.name.clone(), length_mm, weight_g, value: fishing::sale_value(species, Specimen { length_mm, weight_g }) }
    });
  let mut statement = connection.prepare(
    "WITH personal AS (
       SELECT c.species_id, max(c.weight_g) AS best FROM catches c JOIN items i ON i.id = c.item_id
       WHERE i.owner_character_id = ?1 GROUP BY c.species_id
     ), world AS (SELECT species_id, max(weight_g) AS best FROM catches GROUP BY species_id)
     SELECT personal.species_id, personal.best, world.best FROM personal JOIN world USING (species_id)
     ORDER BY personal.best DESC LIMIT 12",
  )?;
  let entries = statement
    .query_map([character_id], |row| Ok((SpeciesId(row.get(0)?), row.get::<_, u32>(1)?, row.get::<_, u32>(2)?)))?
    .map(|row| {
      let (species_id, personal_best_g, world_best_g) = row?;
      Ok(RecordEntryView { species_name: content.species(species_id).name.clone(), personal_best_g, world_best_g })
    })
    .collect::<rusqlite::Result<_>>()?;
  Ok(RecordsView { lifetime_catches: lifetime_catches.max(0) as u64, lifetime_weight_g: lifetime_weight_g.max(0) as u64, heaviest, entries })
}

fn npc_story(has_key: bool, knows_lighthouse: bool, knows_cove: bool) -> &'static str {
  if knows_cove {
    concat!(
      "So the old service stair really does reach the water. Keep what you find down there catalogued; ",
      "half this harbor was built by people nobody remembers.",
    )
  } else if knows_lighthouse {
    "If that key belongs anywhere, it belongs up at the lighthouse. The old keepers used maintenance passages that don't appear on modern charts."
  } else if has_key {
    "That's a lighthouse key, or what's left of one. I saw the same crest on the breakwater plates when I was a kid. Look around those stones carefully."
  } else {
    "Fishing keeps this place fed, but attention keeps you alive. Watch the weather, write down what bites, and don't assume the harbor ends at the last pier."
  }
}

fn contract_hint(contract: &ContractView) -> String {
  if contract.claimed {
    "Today's harbor contract is already settled. The board will rotate with the next game day.".to_owned()
  } else if contract.ready {
    format!("You already have the {} I asked for. Bring it to the board when you're ready.", contract.species_name)
  } else {
    format!("The current harbor contract is for one {}. I only need an ordinary specimen.", contract.species_name)
  }
}

fn objective_progress(connection: &rusqlite::Connection, content: &Content, character_id: i64, objective: &ObjectiveDefinition) -> rusqlite::Result<u32> {
  let progress = match objective.id {
    OBJECTIVE_FIRST_HAUL => connection.query_row(
      "SELECT count(*) FROM catches c JOIN items i ON i.id = c.item_id WHERE i.owner_character_id = ?1",
      [character_id],
      |row| row.get(0),
    )?,
    OBJECTIVE_FIELD_NOTES => connection.query_row(
      "SELECT count(*) FROM discoveries WHERE character_id = ?1 AND kind = ?2",
      params![character_id, DISCOVERY_SPECIES],
      |row| row.get(0),
    )?,
    OBJECTIVE_DEEP_WATER => {
      let mut statement = connection
        .prepare("SELECT DISTINCT c.species_id FROM catches c JOIN items i ON i.id = c.item_id WHERE i.owner_character_id = ?1 AND c.location_id = 2")?;
      let mut found = false;
      for species_id in statement.query_map([character_id], |row| Ok(SpeciesId(row.get(0)?)))? {
        if content.species(species_id?).difficulty >= 11 {
          found = true;
          break;
        }
      }
      u32::from(found)
    }
    _ => 0,
  };
  Ok(progress.min(objective.target))
}

fn load_tasks(connection: &rusqlite::Connection, content: &Content, telegram_user_id: i64, now_ms: i64) -> rusqlite::Result<TasksView> {
  let character_id: i64 =
    connection
      .query_row("SELECT c.id FROM characters c JOIN accounts a ON a.id = c.account_id WHERE a.telegram_user_id = ?1", [telegram_user_id], |row| row.get(0))?;
  let mut claim_statement = connection.prepare("SELECT objective_id FROM objective_claims WHERE character_id = ?1 ORDER BY objective_id")?;
  let claim_rows = claim_statement.query_map([character_id], |row| row.get::<_, u32>(0))?;
  let claimed = claim_rows.collect::<rusqlite::Result<Vec<_>>>()?;
  let mut objectives = Vec::with_capacity(OBJECTIVES.len());
  for definition in &OBJECTIVES {
    let progress = objective_progress(connection, content, character_id, definition)?;
    objectives.push(ObjectiveView {
      id: definition.id,
      name: definition.name,
      description: definition.description,
      progress,
      target: definition.target,
      reward: definition.reward,
      completed: progress >= definition.target,
      claimed: claimed.binary_search(&definition.id).is_ok(),
    });
  }
  Ok(TasksView { objectives, contract: load_contract(connection, content, character_id, now_ms)? })
}

fn load_contract(connection: &rusqlite::Connection, content: &Content, character_id: i64, now_ms: i64) -> rusqlite::Result<ContractView> {
  let cycle = game_day(now_ms);
  let species_id = contract_species(cycle);
  let species = content.species(species_id);
  let claimed: bool =
    connection
      .query_row("SELECT EXISTS(SELECT 1 FROM contract_claims WHERE character_id = ?1 AND cycle = ?2)", params![character_id, cycle], |row| row.get(0))?;
  let ready: bool = if claimed {
    false
  } else {
    connection.query_row(
      concat!(
        "SELECT EXISTS(SELECT 1 FROM items i JOIN catches c ON c.item_id = i.id ",
        "WHERE i.owner_character_id = ?1 AND i.removed_at_ms IS NULL AND c.species_id = ?2)",
      ),
      params![character_id, species_id.0],
      |row| row.get(0),
    )?
  };
  let next_cycle = cycle.saturating_add(1).saturating_mul(GAME_DAY_REAL_MS);
  Ok(ContractView {
    species_name: species.name.clone(),
    reward_coins: contract_reward_coins(species),
    reward_xp: contract_reward_xp(species),
    ready,
    claimed,
    resets_in_ms: next_cycle.saturating_sub(now_ms).max(0),
  })
}

fn contract_species(cycle: i64) -> SpeciesId {
  match cycle.rem_euclid(3) {
    0 => SpeciesId(1),
    1 => SpeciesId(2),
    _ => SpeciesId(3),
  }
}

fn contract_reward_coins(species: &Species) -> u32 {
  species.base_value.saturating_mul(4).saturating_add(8)
}

fn contract_reward_xp(species: &Species) -> u32 {
  species.xp.saturating_add(12)
}

fn load_titles(connection: &rusqlite::Connection, character_id: i64, equipped_id: u32) -> rusqlite::Result<TitlesView> {
  Ok(build_titles(equipped_id, title_unlocks(connection, character_id)?))
}

fn build_titles(equipped_id: u32, unlocks: [bool; 7]) -> TitlesView {
  let titles = unlocks
    .into_iter()
    .enumerate()
    .filter_map(|(id, unlocked)| {
      let id = id as u32;
      title_definition(id).map(|(name, description)| TitleOptionView { id, name, description, unlocked, equipped: equipped_id == id })
    })
    .collect();
  TitlesView { titles }
}

// Array indices intentionally match persisted title IDs and the TITLES table in app/mod.rs.
fn title_unlocks(connection: &rusqlite::Connection, character_id: i64) -> rusqlite::Result<[bool; 7]> {
  let (catches, species, relic, objectives, cove, group): (u32, u32, bool, u32, bool, bool) = connection.query_row(
    "SELECT
       (SELECT count(*) FROM catches c JOIN items i ON i.id = c.item_id WHERE i.owner_character_id = ?1),
       (SELECT count(*) FROM discoveries WHERE character_id = ?1 AND kind = ?2),
       EXISTS(SELECT 1 FROM discoveries WHERE character_id = ?1 AND kind = ?3 AND subject_id = ?4),
       (SELECT count(*) FROM objective_claims WHERE character_id = ?1),
       EXISTS(SELECT 1 FROM character_locations WHERE character_id = ?1 AND location_id = 5),
       EXISTS(SELECT 1 FROM group_event_claims WHERE character_id = ?1)",
    params![character_id, DISCOVERY_SPECIES, DISCOVERY_RELIC, RELIC_RUSTED_KEY],
    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
  )?;
  Ok([true, catches >= 10, species >= 10, relic, objectives >= 3, cove, group])
}
