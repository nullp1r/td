//! End-to-end application tests against isolated temporary SQLite databases.

use std::{assert_matches, path::PathBuf, process, sync::Arc};

use super::*;
use crate::{
  content::Content,
  db::Db,
  ids::{BaitId, SpeciesId},
  view::{GroupApproach, ReelOutcome, TimerOutcome},
  world::game_day,
};

const FIRST_HAUL_OBJECTIVE: u32 = 1;
const CONTRACT_REMOVAL_KIND: i64 = 2;
const FISH_CHUNKS_RECIPE: u32 = 1;
const FISH_CHUNKS_BAIT: BaitId = BaitId(6);

async fn app() -> App {
  use std::sync::atomic::{AtomicU64, Ordering};
  static NEXT_DB: AtomicU64 = AtomicU64::new(0);
  let suffix = NEXT_DB.fetch_add(1, Ordering::Relaxed);
  let path = PathBuf::from(format!("/tmp/telegram-mmo-test-{}-{suffix}.sqlite3", process::id()));
  let db = Db::open(path).await.expect("db");
  let content = Content::from_slice(include_bytes!("../../content/game.json")).expect("content");
  App::new(db, Arc::new(content))
}

async fn db_call<T, F>(app: &App, f: F) -> T
where
  T: Send + 'static,
  F: FnOnce(&mut rusqlite::Connection) -> rusqlite::Result<T> + Send + 'static,
{
  app.db.job(f).await.expect("database worker").expect("database query")
}

async fn set_all_bait(app: &App, telegram_user_id: i64, quantity: u32) {
  db_call(app, move |connection| {
    connection.execute(
      "UPDATE inventory_stacks SET quantity = ?1 WHERE character_id = (
         SELECT c.id FROM characters c JOIN accounts a ON a.id = c.account_id WHERE a.telegram_user_id = ?2
       )",
      params![quantity, telegram_user_id],
    )?;
    Ok(())
  })
  .await;
}

async fn next_timer_id(app: &App) -> i64 {
  let due_at = app.next_timer_due_at().await.expect("timer query").expect("timer");
  app.due_timer_ids(due_at, 1).await.expect("due timer query").into_iter().next().expect("due timer")
}

async fn cast_due_at(app: &App, telegram_user_id: i64, chat_id: i64, message_id: i64, seed: u64, now_ms: i64) -> i64 {
  app.cast(telegram_user_id, chat_id, message_id, seed, now_ms).await.expect("cast");
  app.next_timer_due_at().await.expect("timer query").expect("timer")
}

#[test]
fn level_curve_matches_expected_boundaries() {
  assert_eq!(level_for_xp(0), 1);
  assert_eq!(level_for_xp(49), 1);
  assert_eq!(level_for_xp(50), 2);
  assert_eq!(level_for_xp(199), 2);
  assert_eq!(level_for_xp(200), 3);
  assert_eq!(level_for_xp(u64::MAX), 607_400_100);
}

#[tokio::test]
async fn catch_is_persisted_and_duplicate_step_is_stale() {
  let app = app().await;
  let now = 1_000_000_000_i64;
  app.ensure_character(42, now).await.expect("character");
  let due_at = cast_due_at(&app, 42, 42, 100, 123, now).await;
  let timer_id = next_timer_id(&app).await;
  let outcome = app.fire_timer(timer_id, due_at).await.expect("bite");
  assert_matches!(outcome, TimerOutcome::Bite(_));
  let TimerOutcome::Bite(bite) = outcome else { unreachable!() };
  app.mark_presented(bite.encounter_id, bite.step, due_at).await.expect("presented");
  let outcome = app.reel(42, bite.encounter_id, bite.step, due_at + 2_000).await.expect("reel");
  assert_matches!(outcome, ReelOutcome::Caught(_));
  let duplicate = app.reel(42, bite.encounter_id, bite.step, due_at + 2_100).await;
  assert_matches!(duplicate, Err(Error::StaleEncounter));
}

#[tokio::test]
async fn zero_bait_has_a_free_recovery_path() {
  let app = app().await;
  let now = 1_000_000_000_i64;
  app.ensure_character(43, now).await.expect("character");
  set_all_bait(&app, 43, 0).await;

  let before = app.character(43, now).await.expect("character");
  assert_eq!(before.total_bait, 0);
  assert_eq!(before.bait_left, 0);
  assert!(app.shop(43).await.expect("shop").can_forage);

  let shop = app.forage_bait(43).await.expect("forage");
  assert!(!shop.can_forage);
  let inventory = app.inventory(43).await.expect("inventory");
  let worms = inventory.baits.iter().find(|bait| bait.id == BaitId(1)).expect("worms");
  assert_eq!(worms.quantity, 3);
  assert!(worms.selected);

  assert_matches!(app.forage_bait(43).await, Err(Error::BaitStillAvailable));
  app.cast(43, 43, 101, 456, now).await.expect("cast after recovery");
}

#[tokio::test]
async fn buying_bait_can_recover_an_exhausted_selection() {
  let app = app().await;
  let now = 1_000_000_000_i64;
  app.ensure_character(44, now).await.expect("character");
  set_all_bait(&app, 44, 0).await;

  let shop = app.buy_bait(44, BaitId(2)).await.expect("buy bread");
  assert_eq!(shop.coins, 16);
  let bread = shop.baits.iter().find(|bait| bait.id == BaitId(2)).expect("bread");
  assert_eq!(bread.quantity, 5);
  assert!(bread.selected);
}

#[tokio::test]
async fn travel_is_blocked_only_while_an_encounter_is_active() {
  let app = app().await;
  let now = 1_000_000_000_i64;
  app.ensure_character(45, now).await.expect("character");
  let exploration = app.explore(45, now).await.expect("discover breakwater");
  assert_eq!(exploration.discovered_location.as_ref().map(|location| location.name.as_str()), Some("Broken Breakwater"));

  app.cast(45, 45, 102, 789, now).await.expect("cast");
  assert_matches!(app.travel(45, LocationId(2), now).await, Err(Error::EncounterActive));

  app.cancel_fishing(45, now).await.expect("cancel");
  let character = app.travel(45, LocationId(2), now).await.expect("travel");
  assert_eq!(character.location.name, "Broken Breakwater");
}

#[tokio::test]
async fn exploration_unlocks_locations_in_order() {
  let app = app().await;
  let now = 1_000_000_000_i64;
  app.ensure_character(46, now).await.expect("character");

  assert_matches!(app.travel(46, LocationId(2), now).await, Err(Error::LocationLocked));
  let first = app.explore(46, now).await.expect("first exploration");
  assert_eq!(first.discovered_location.as_ref().map(|location| location.name.as_str()), Some("Broken Breakwater"));
  let second = app.explore(46, now + 1).await.expect("second exploration");
  assert_eq!(second.discovered_location.as_ref().map(|location| location.name.as_str()), Some("Reed Pond"));

  let locations = app.locations(46).await.expect("locations");
  assert_eq!(locations.locations.len(), 3);
  assert_eq!(locations.undiscovered, 2);
}

#[tokio::test]
async fn rods_are_purchased_once_and_equipped_from_inventory() {
  let app = app().await;
  let now = 1_000_000_000_i64;
  app.ensure_character(48, now).await.expect("character");
  db_call(&app, |connection| {
    connection.execute("UPDATE characters SET coins = 100 WHERE account_id = (SELECT id FROM accounts WHERE telegram_user_id = 48)", [])?;
    Ok(())
  })
  .await;

  let shop = app.buy_rod(48, RodId(2), now).await.expect("buy reinforced rod");
  let rod = shop.rods.iter().find(|rod| rod.id == RodId(2)).expect("reinforced rod");
  assert!(rod.owned);
  assert!(!rod.equipped);
  assert_eq!(shop.coins, 55);
  assert_matches!(app.buy_rod(48, RodId(2), now + 1).await, Err(Error::RodAlreadyOwned));

  let inventory = app.equip_rod(48, RodId(2)).await.expect("equip reinforced rod");
  let rod = inventory.rods.iter().find(|rod| rod.id == RodId(2)).expect("reinforced rod");
  assert!(rod.equipped);
}

#[tokio::test]
async fn selling_a_catch_preserves_its_personal_record() {
  let app = app().await;
  let now = 1_000_000_000_i64;
  app.ensure_character(49, now).await.expect("character");
  let due_at = cast_due_at(&app, 49, 49, 104, 123, now).await;
  let timer_id = next_timer_id(&app).await;
  let outcome = app.fire_timer(timer_id, due_at).await.expect("bite");
  assert_matches!(outcome, TimerOutcome::Bite(_));
  let TimerOutcome::Bite(bite) = outcome else { unreachable!() };
  app.mark_presented(bite.encounter_id, bite.step, due_at).await.expect("presented");
  let caught = app.reel(49, bite.encounter_id, bite.step, due_at + 2_000).await.expect("reel");
  assert_matches!(caught, ReelOutcome::Caught(_));

  let before = app.inventory(49).await.expect("inventory");
  assert_eq!(before.catch_count, 1);
  let item_id = before.recent_catches[0].item_id;
  app.sell_all_catches(49, now + 10_000).await.expect("sell");
  assert_eq!(app.inventory(49).await.expect("inventory").catch_count, 0);
  let journal = app.journal(49).await.expect("journal");
  assert!(journal.species.iter().any(|species| species.discovered && species.best_weight_g.is_some()));
  let shared = app.inline_catches(49, format!("catch:{item_id}")).await.expect("inline catch");
  assert_eq!(shared.len(), 1);
  assert_eq!(shared[0].item_id, item_id);
}

#[tokio::test]
async fn rusted_key_opens_the_lighthouse_progression() {
  let app = app().await;
  let now = 1_000_000_000_i64;
  app.ensure_character(47, now).await.expect("character");
  app.explore(47, now).await.expect("discover breakwater");
  app.travel(47, LocationId(2), now).await.expect("travel");
  let due_at = cast_due_at(&app, 47, 47, 103, 320, now).await;
  let timer_id = next_timer_id(&app).await;
  let outcome = app.fire_timer(timer_id, due_at).await.expect("bite");
  assert_matches!(outcome, TimerOutcome::Bite(_));
  let TimerOutcome::Bite(bite) = outcome else { unreachable!() };
  app.mark_presented(bite.encounter_id, bite.step, due_at).await.expect("presented");
  let outcome = app.reel(47, bite.encounter_id, bite.step, due_at + 1_000).await.expect("reel");
  assert_matches!(outcome, ReelOutcome::Relic(_));

  let clue = app.explore(47, now + 2_000).await.expect("follow key clue");
  assert_eq!(clue.discovered_location.as_ref().map(|location| location.name.as_str()), Some("Old Lighthouse"));
  app.travel(47, LocationId(4), now + 2_001).await.expect("lighthouse");
  let opened = app.explore(47, now + 2_002).await.expect("open lighthouse");
  assert_eq!(opened.discovered_location.as_ref().map(|location| location.name.as_str()), Some("Lighthouse Cove"));
}

async fn insert_test_catch(app: &App, telegram_user_id: i64, species_id: SpeciesId, weight_g: u32, now_ms: i64) -> i64 {
  db_call(app, move |connection| {
    let character_id: i64 = connection.query_row(
      "SELECT c.id FROM characters c JOIN accounts a ON a.id = c.account_id WHERE a.telegram_user_id = ?1",
      [telegram_user_id],
      |row| row.get(0),
    )?;
    connection.execute(
      "INSERT INTO items (owner_character_id, item_def_id, created_at_ms) VALUES (?1, ?2, ?3)",
      params![character_id, SPECIES_ITEM_BASE + species_id.0, now_ms],
    )?;
    let item_id = connection.last_insert_rowid();
    connection.execute(
      concat!(
        "INSERT INTO catches (item_id, species_id, length_mm, weight_g, seed, generator_version, caught_at_ms, ",
        "location_id, bait_id, weather, game_minute) VALUES (?1, ?2, 250, ?3, 1, 1, ?4, 1, 1, 0, 720)",
      ),
      params![item_id, species_id.0, weight_g, now_ms],
    )?;
    Ok(item_id)
  })
  .await
}
#[tokio::test]
async fn milestone_rewards_are_claimed_once() {
  let app = app().await;
  let now = 1_000_000_000_i64;
  app.ensure_character(50, now).await.expect("character");
  for offset in 0..5 {
    insert_test_catch(&app, 50, SpeciesId(1), 200 + offset, now + i64::from(offset)).await;
  }
  let tasks = app.tasks(50, now + 10).await.expect("tasks");
  let first = tasks.objectives.iter().find(|objective| objective.id == FIRST_HAUL_OBJECTIVE).expect("first haul");
  assert!(first.completed);
  assert!(!first.claimed);

  let reward = app.claim_objective(50, FIRST_HAUL_OBJECTIVE, now + 11).await.expect("claim");
  assert_eq!(reward.coins, 45);
  assert_matches!(app.claim_objective(50, FIRST_HAUL_OBJECTIVE, now + 12).await, Err(Error::ObjectiveClaimed));
}

#[tokio::test]
async fn contract_uses_smallest_matching_specimen_and_keeps_history() {
  let app = app().await;
  let now = 1_000_000_000_i64;
  app.ensure_character(51, now).await.expect("character");
  let target = match game_day(now).rem_euclid(3) {
    0 => SpeciesId(1),
    1 => SpeciesId(2),
    _ => SpeciesId(3),
  };
  let large = insert_test_catch(&app, 51, target, 900, now + 1).await;
  let small = insert_test_catch(&app, 51, target, 120, now + 2).await;
  assert!(app.tasks(51, now).await.expect("tasks").contract.ready);

  app.turn_in_contract(51, now + 3).await.expect("turn in");
  let (small_removed, small_kind, large_removed): (Option<i64>, Option<i64>, Option<i64>) = db_call(&app, move |connection| {
    let small_row = connection.query_row("SELECT removed_at_ms, removal_kind FROM items WHERE id = ?1", [small], |row| {
      Ok((row.get::<_, Option<i64>>(0)?, row.get::<_, Option<i64>>(1)?))
    })?;
    let large_removed = connection.query_row("SELECT removed_at_ms FROM items WHERE id = ?1", [large], |row| row.get::<_, Option<i64>>(0))?;
    Ok((small_row.0, small_row.1, large_removed))
  })
  .await;
  assert!(small_removed.is_some());
  assert_eq!(small_kind, Some(CONTRACT_REMOVAL_KIND));
  assert!(large_removed.is_none());
  assert!(app.tasks(51, now + 4).await.expect("tasks").contract.claimed);
}

#[tokio::test]
async fn catches_wear_rods_and_shop_repairs_them() {
  let app = app().await;
  let now = 1_000_000_000_i64;
  app.ensure_character(52, now).await.expect("character");
  let due_at = cast_due_at(&app, 52, 52, 105, 123, now).await;
  let timer_id = next_timer_id(&app).await;
  let outcome = app.fire_timer(timer_id, due_at).await.expect("bite");
  let TimerOutcome::Bite(bite) = outcome else { unreachable!("expected bite") };
  app.mark_presented(bite.encounter_id, bite.step, due_at).await.expect("presented");
  let outcome = app.reel(52, bite.encounter_id, bite.step, due_at + 2_000).await.expect("reel");
  assert_matches!(outcome, ReelOutcome::Caught(_));

  let before = app.inventory(52).await.expect("inventory");
  let starter = before.rods.iter().find(|rod| rod.id == RodId(1)).expect("starter");
  assert!(starter.condition < 100);
  assert!(app.shop(52).await.expect("shop").repair_all_cost > 0);
  app.repair_rods(52).await.expect("repair");
  let after = app.inventory(52).await.expect("inventory");
  assert_eq!(after.rods.iter().find(|rod| rod.id == RodId(1)).expect("starter").condition, 100);
}

#[tokio::test]
async fn crafting_consumes_a_specimen_but_preserves_records() {
  let app = app().await;
  let now = 1_000_000_000_i64;
  app.ensure_character(53, now).await.expect("character");
  let item_id = insert_test_catch(&app, 53, SpeciesId(1), 240, now + 1).await;
  let before = app.records(53).await.expect("records");
  assert_eq!(before.lifetime_catches, 1);

  let result = app.craft(53, FISH_CHUNKS_RECIPE, now + 2).await.expect("craft");
  assert_eq!(result.quantity, 4);
  let inventory = app.inventory(53).await.expect("inventory");
  assert_eq!(inventory.catch_count, 0);
  assert_eq!(inventory.baits.iter().find(|bait| bait.id == FISH_CHUNKS_BAIT).expect("fish chunks").quantity, 4);
  assert_eq!(app.records(53).await.expect("records").lifetime_catches, 1);

  let consumed: bool =
    db_call(&app, move |connection| connection.query_row("SELECT EXISTS(SELECT 1 FROM craft_consumptions WHERE item_id = ?1)", [item_id], |row| row.get(0)))
      .await;
  assert!(consumed);
}

#[tokio::test]
async fn group_shoal_is_once_per_character_and_unlocks_a_title() {
  let app = app().await;
  let now = 1_000_000_000_i64;
  let chat_id = -100_123_i64;
  app.ensure_character(54, now).await.expect("character");
  let event = app.group_event(chat_id, now).await.expect("event");
  assert_eq!(i64::from(event.ends_at_unix) * 1_000, now + event.resets_in_ms);
  let catch = app.group_cast(54, chat_id, event.cycle, GroupApproach::Drift, 99, now + 1).await.expect("group cast");
  assert_eq!(catch.event.participants, 1);
  assert_matches!(app.group_cast(54, chat_id, event.cycle, GroupApproach::Hold, 100, now + 2).await, Err(Error::GroupEventClaimed));

  let titles = app.titles(54).await.expect("titles");
  let shoalbound = titles.titles.iter().find(|title| title.id == 6).expect("shoalbound");
  assert!(shoalbound.unlocked);
  let equipped = app.equip_title(54, 6).await.expect("equip title");
  assert!(equipped.titles.iter().any(|title| title.id == 6 && title.equipped));
  assert_eq!(app.character(54, now + 3).await.expect("character").title_name, Some("Shoalbound"));
}

#[tokio::test]
async fn crafted_bait_is_not_sold_but_can_be_selected() {
  let app = app().await;
  let now = 1_000_000_000_i64;
  app.ensure_character(55, now).await.expect("character");
  insert_test_catch(&app, 55, SpeciesId(1), 180, now + 1).await;
  app.craft(55, FISH_CHUNKS_RECIPE, now + 2).await.expect("craft");
  assert_matches!(app.buy_bait(55, FISH_CHUNKS_BAIT).await, Err(Error::BaitNotForSale));
  let inventory = app.select_bait(55, FISH_CHUNKS_BAIT).await.expect("select crafted bait");
  assert!(inventory.baits.iter().any(|bait| bait.id == FISH_CHUNKS_BAIT && bait.selected));
}
