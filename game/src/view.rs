//! Committed domain projections returned by `App` and consumed by presentation code.
//! They deliberately contain no Telegram/TDLib types, so persistence/gameplay stays transport-agnostic.

use crate::{
  fishing::Reaction,
  ids::{BaitId, EncounterId, LocationId, RodId},
  world::Environment,
};

#[derive(Debug)]
pub struct CharacterView {
  pub name: String,
  pub xp: u64,
  pub level: u32,
  pub coins: u64,
  pub location: LocationView,
  pub bait_name: String,
  pub bait_left: u32,
  pub total_bait: u32,
  pub rod_name: String,
  pub rod_condition: u32,
  pub environment: Environment,
  pub fishing_active: bool,
  pub has_rusted_key: bool,
  pub has_npc: bool,
  pub title_name: Option<&'static str>,
}

#[derive(Debug)]
pub struct LocationView {
  pub name: String,
  pub description: String,
  pub fishable: bool,
}

#[derive(Debug)]
pub struct BaitStackView {
  pub id: BaitId,
  pub name: String,
  pub power: u32,
  pub quantity: u32,
  pub selected: bool,
}

#[derive(Debug)]
pub struct CatchSummaryView {
  pub species_name: String,
  pub length_mm: u32,
  pub weight_g: u32,
  pub value: u64,
}

#[derive(Debug)]
pub struct RodInventoryView {
  pub id: RodId,
  pub name: String,
  pub control: u32,
  pub condition: u32,
  pub owned: bool,
  pub equipped: bool,
}

#[derive(Debug)]
pub struct InventoryView {
  pub coins: u64,
  pub rod_name: String,
  pub rods: Vec<RodInventoryView>,
  pub baits: Vec<BaitStackView>,
  pub recent_catches: Vec<CatchSummaryView>,
  pub catch_count: u32,
  pub sell_value: u64,
  pub has_rusted_key: bool,
}

#[derive(Debug)]
pub struct SpeciesDiscoveryView {
  pub name: String,
  pub discovered: bool,
  pub best_weight_g: Option<u32>,
  pub clue: String,
}

#[derive(Debug)]
pub struct JournalView {
  pub discovered: u32,
  pub total: u32,
  pub species: Vec<SpeciesDiscoveryView>,
  pub rusted_key_discovered: bool,
  pub locations_discovered: u32,
  pub locations_total: u32,
}

#[derive(Debug)]
pub struct LocationOptionView {
  pub id: LocationId,
  pub name: String,
  pub description: String,
  pub fishable: bool,
  pub current: bool,
}

#[derive(Debug)]
pub struct LocationsView {
  pub locations: Vec<LocationOptionView>,
  pub undiscovered: u32,
}

#[derive(Debug)]
pub struct ShopBaitView {
  pub id: BaitId,
  pub name: String,
  pub power: u32,
  pub quantity: u32,
  pub pack_size: u32,
  pub price: u32,
  pub selected: bool,
}

#[derive(Debug)]
pub struct ShopRodView {
  pub id: RodId,
  pub name: String,
  pub control: u32,
  pub condition: u32,
  pub price: u32,
  pub owned: bool,
  pub equipped: bool,
}

#[derive(Debug)]
pub struct ShopView {
  pub coins: u64,
  pub baits: Vec<ShopBaitView>,
  pub rods: Vec<ShopRodView>,
  pub can_forage: bool,
  pub repair_all_cost: u32,
}

#[derive(Debug)]
pub struct ObjectiveView {
  pub id: u32,
  pub name: &'static str,
  pub description: &'static str,
  pub progress: u32,
  pub target: u32,
  pub reward: &'static str,
  pub completed: bool,
  pub claimed: bool,
}

#[derive(Debug)]
pub struct ContractView {
  pub species_name: String,
  pub reward_coins: u32,
  pub reward_xp: u32,
  pub ready: bool,
  pub claimed: bool,
  pub resets_in_ms: i64,
}

#[derive(Debug)]
pub struct TasksView {
  pub objectives: Vec<ObjectiveView>,
  pub contract: ContractView,
}

#[derive(Debug)]
pub struct RewardView {
  pub title: &'static str,
  pub text: String,
  pub coins: u64,
  pub xp: u64,
  pub level: u32,
  pub level_up: bool,
}

#[derive(Debug)]
pub struct NpcView {
  pub name: &'static str,
  pub title: &'static str,
  pub text: String,
  pub hint: String,
}

#[derive(Debug)]
pub struct ConditionsView {
  pub location_name: String,
  pub fishable: bool,
  pub environment: Environment,
  pub next_weather: &'static str,
  pub weather_changes_in_ms: i64,
  pub day_part_changes_in_ms: i64,
  pub active_known_species: Vec<String>,
  pub known_but_inactive: u32,
}

#[derive(Debug)]
pub struct RecordEntryView {
  pub species_name: String,
  pub personal_best_g: u32,
  pub world_best_g: u32,
}

#[derive(Debug)]
pub struct RecordsView {
  pub lifetime_catches: u64,
  pub lifetime_weight_g: u64,
  pub heaviest: Option<CatchSummaryView>,
  pub entries: Vec<RecordEntryView>,
}

#[derive(Copy, Debug)]
pub struct TitleOptionView {
  pub id: u32,
  pub name: &'static str,
  pub description: &'static str,
  pub unlocked: bool,
  pub equipped: bool,
}

#[derive(Debug)]
pub struct TitlesView {
  pub titles: Vec<TitleOptionView>,
}

#[derive(Debug)]
pub struct RecipeView {
  pub id: u32,
  pub name: &'static str,
  pub description: &'static str,
  pub output_name: String,
  pub output_quantity: u32,
  pub ready: bool,
}

#[derive(Debug)]
pub struct CraftingView {
  pub recipes: Vec<RecipeView>,
  pub stored_catches: u32,
}

#[derive(Debug)]
pub struct CraftResultView {
  pub recipe_name: &'static str,
  pub bait_name: String,
  pub quantity: u32,
}

#[derive(Debug)]
pub struct GroupEventView {
  pub cycle: i64,
  pub species_name: String,
  pub participants: u32,
  pub resets_in_ms: i64,
  pub ends_at_unix: i32,
}

#[derive(Debug)]
pub struct GroupCatchView {
  pub species_name: String,
  pub length_mm: u32,
  pub weight_g: u32,
  pub new_species: bool,
  pub global_first: bool,
  pub xp_gained: u32,
  pub level: u32,
  pub event: GroupEventView,
}

#[derive(Copy, Debug)]
pub struct SaleView {
  pub sold: u32,
  pub coins_gained: u64,
  pub coins: u64,
}

#[derive(Copy, Debug)]
pub struct CastStarted {
  pub due_at_ms: i64,
}

#[derive(Debug)]
pub struct BiteView {
  pub encounter_id: EncounterId,
  pub step: u32,
  pub chat_id: i64,
  pub message_id: i64,
  pub observation: &'static str,
}

#[derive(Debug)]
pub struct CatchView {
  pub chat_id: i64,
  pub message_id: i64,
  pub species_name: String,
  pub length_mm: u32,
  pub weight_g: u32,
  pub reaction: Reaction,
  pub new_species: bool,
  pub global_first: bool,
  pub xp_gained: u32,
  pub level: u32,
  pub level_up: bool,
  pub sale_value: u64,
}

#[derive(Debug)]
pub struct StruggleView {
  pub encounter_id: EncounterId,
  pub step: u32,
  pub chat_id: i64,
  pub message_id: i64,
  pub observation: &'static str,
}

#[derive(Debug)]
pub struct RelicView {
  pub chat_id: i64,
  pub message_id: i64,
  pub name: &'static str,
  pub description: &'static str,
  pub global_first: bool,
  pub xp_gained: u32,
  pub level: u32,
  pub level_up: bool,
}

#[derive(Debug)]
pub struct ExploreView {
  pub title: &'static str,
  pub text: String,
  pub discovered_location: Option<LocationView>,
  pub xp_gained: u32,
}

#[derive(Debug)]
pub struct EscapeView {
  pub chat_id: i64,
  pub message_id: i64,
  pub reason: &'static str,
}

#[derive(Debug)]
pub enum ReelOutcome {
  Caught(CatchView),
  Struggle(StruggleView),
  Relic(RelicView),
  Escaped(EscapeView),
}

#[derive(Debug)]
pub enum TimerOutcome {
  Bite(BiteView),
  Escaped(EscapeView),
  Stale,
}

