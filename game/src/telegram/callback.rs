//! Stable binary payloads stored in Telegram buttons.
//!
//! Messages can outlive a bot process, so version/kind bytes are a wire protocol: append new kinds,
//! but never silently renumber existing ones. Unknown versions decode to `None` and are acknowledged.

use crate::ids::{BaitId, EncounterId, LocationId, RodId};

const VERSION: u8 = 1;
const CAST: u8 = 1;
const REEL: u8 = 2;
const HOME: u8 = 3;
const INVENTORY: u8 = 4;
const JOURNAL: u8 = 5;
const LOCATIONS: u8 = 6;
const SHOP: u8 = 7;
const HELP: u8 = 8;
const BUY_BAIT: u8 = 9;
const SELECT_BAIT: u8 = 10;
const SELL_ALL: u8 = 11;
const TRAVEL: u8 = 12;
const CANCEL_FISHING: u8 = 13;
const FORAGE_BAIT: u8 = 14;
const EXPLORE: u8 = 15;
const BUY_ROD: u8 = 16;
const EQUIP_ROD: u8 = 17;
const PULL: u8 = 18;
const GIVE_LINE: u8 = 19;
const TASKS: u8 = 20;
const CLAIM_OBJECTIVE: u8 = 21;
const TURN_IN_CONTRACT: u8 = 22;
const TALK: u8 = 23;
const CONDITIONS: u8 = 24;
const REPAIR_RODS: u8 = 25;
const RECORDS: u8 = 26;
const TITLES: u8 = 27;
const EQUIP_TITLE: u8 = 28;
const CRAFTING: u8 = 29;
const CRAFT: u8 = 30;
const GROUP_CAST: u8 = 31;
const GROUP_JOURNAL: u8 = 32;
const GROUP_RECORDS: u8 = 33;
const GROUP_HELP: u8 = 34;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Callback {
  Cast,
  Reel { encounter_id: EncounterId, step: u32 },
  Home,
  Inventory,
  Journal,
  Locations,
  Shop,
  Help,
  BuyBait { bait_id: BaitId },
  SelectBait { bait_id: BaitId },
  SellAll,
  Travel { location_id: LocationId },
  CancelFishing,
  ForageBait,
  Explore,
  BuyRod { rod_id: RodId },
  EquipRod { rod_id: RodId },
  Pull { encounter_id: EncounterId, step: u32 },
  GiveLine { encounter_id: EncounterId, step: u32 },
  Tasks,
  ClaimObjective { objective_id: u32 },
  TurnInContract,
  Talk,
  Conditions,
  RepairRods,
  Records,
  Titles,
  EquipTitle { title_id: u32 },
  Crafting,
  Craft { recipe_id: u32 },
  GroupCast { cycle: i64 },
  GroupJournal,
  GroupRecords,
  GroupHelp,
}

impl Callback {
  pub fn encode(self) -> Vec<u8> {
    match self {
      Self::Cast => vec![VERSION, CAST],
      Self::Reel { encounter_id, step } => encode_encounter(REEL, encounter_id, step),
      Self::Home => vec![VERSION, HOME],
      Self::Inventory => vec![VERSION, INVENTORY],
      Self::Journal => vec![VERSION, JOURNAL],
      Self::Locations => vec![VERSION, LOCATIONS],
      Self::Shop => vec![VERSION, SHOP],
      Self::Help => vec![VERSION, HELP],
      Self::BuyBait { bait_id } => encode_u32(BUY_BAIT, bait_id.0),
      Self::SelectBait { bait_id } => encode_u32(SELECT_BAIT, bait_id.0),
      Self::SellAll => vec![VERSION, SELL_ALL],
      Self::Travel { location_id } => encode_u32(TRAVEL, location_id.0),
      Self::CancelFishing => vec![VERSION, CANCEL_FISHING],
      Self::ForageBait => vec![VERSION, FORAGE_BAIT],
      Self::Explore => vec![VERSION, EXPLORE],
      Self::BuyRod { rod_id } => encode_u32(BUY_ROD, rod_id.0),
      Self::EquipRod { rod_id } => encode_u32(EQUIP_ROD, rod_id.0),
      Self::Pull { encounter_id, step } => encode_encounter(PULL, encounter_id, step),
      Self::GiveLine { encounter_id, step } => encode_encounter(GIVE_LINE, encounter_id, step),
      Self::Tasks => vec![VERSION, TASKS],
      Self::ClaimObjective { objective_id } => encode_u32(CLAIM_OBJECTIVE, objective_id),
      Self::TurnInContract => vec![VERSION, TURN_IN_CONTRACT],
      Self::Talk => vec![VERSION, TALK],
      Self::Conditions => vec![VERSION, CONDITIONS],
      Self::RepairRods => vec![VERSION, REPAIR_RODS],
      Self::Records => vec![VERSION, RECORDS],
      Self::Titles => vec![VERSION, TITLES],
      Self::EquipTitle { title_id } => encode_u32(EQUIP_TITLE, title_id),
      Self::Crafting => vec![VERSION, CRAFTING],
      Self::Craft { recipe_id } => encode_u32(CRAFT, recipe_id),
      Self::GroupCast { cycle } => encode_i64(GROUP_CAST, cycle),
      Self::GroupJournal => vec![VERSION, GROUP_JOURNAL],
      Self::GroupRecords => vec![VERSION, GROUP_RECORDS],
      Self::GroupHelp => vec![VERSION, GROUP_HELP],
    }
  }

  pub fn decode(bytes: &[u8]) -> Option<Self> {
    match bytes {
      [VERSION, CAST] => Some(Self::Cast),
      [VERSION, REEL, rest @ ..] if rest.len() == 12 => {
        let (encounter, step) = rest.split_at(8);
        let encounter_id = EncounterId(i64::from_le_bytes(encounter.try_into().ok()?));
        let step = u32::from_le_bytes(step.try_into().ok()?);
        Some(Self::Reel { encounter_id, step })
      }
      [VERSION, HOME] => Some(Self::Home),
      [VERSION, INVENTORY] => Some(Self::Inventory),
      [VERSION, JOURNAL] => Some(Self::Journal),
      [VERSION, LOCATIONS] => Some(Self::Locations),
      [VERSION, SHOP] => Some(Self::Shop),
      [VERSION, HELP] => Some(Self::Help),
      [VERSION, BUY_BAIT, rest @ ..] if rest.len() == 4 => Some(Self::BuyBait { bait_id: BaitId(decode_u32(rest)?) }),
      [VERSION, SELECT_BAIT, rest @ ..] if rest.len() == 4 => Some(Self::SelectBait { bait_id: BaitId(decode_u32(rest)?) }),
      [VERSION, SELL_ALL] => Some(Self::SellAll),
      [VERSION, TRAVEL, rest @ ..] if rest.len() == 4 => Some(Self::Travel { location_id: LocationId(decode_u32(rest)?) }),
      [VERSION, CANCEL_FISHING] => Some(Self::CancelFishing),
      [VERSION, FORAGE_BAIT] => Some(Self::ForageBait),
      [VERSION, EXPLORE] => Some(Self::Explore),
      [VERSION, BUY_ROD, rest @ ..] if rest.len() == 4 => Some(Self::BuyRod { rod_id: RodId(decode_u32(rest)?) }),
      [VERSION, EQUIP_ROD, rest @ ..] if rest.len() == 4 => Some(Self::EquipRod { rod_id: RodId(decode_u32(rest)?) }),
      [VERSION, PULL, rest @ ..] if rest.len() == 12 => decode_encounter(rest).map(|(encounter_id, step)| Self::Pull { encounter_id, step }),
      [VERSION, GIVE_LINE, rest @ ..] if rest.len() == 12 => decode_encounter(rest).map(|(encounter_id, step)| Self::GiveLine { encounter_id, step }),
      [VERSION, TASKS] => Some(Self::Tasks),
      [VERSION, CLAIM_OBJECTIVE, rest @ ..] if rest.len() == 4 => Some(Self::ClaimObjective { objective_id: decode_u32(rest)? }),
      [VERSION, TURN_IN_CONTRACT] => Some(Self::TurnInContract),
      [VERSION, TALK] => Some(Self::Talk),
      [VERSION, CONDITIONS] => Some(Self::Conditions),
      [VERSION, REPAIR_RODS] => Some(Self::RepairRods),
      [VERSION, RECORDS] => Some(Self::Records),
      [VERSION, TITLES] => Some(Self::Titles),
      [VERSION, EQUIP_TITLE, rest @ ..] if rest.len() == 4 => Some(Self::EquipTitle { title_id: decode_u32(rest)? }),
      [VERSION, CRAFTING] => Some(Self::Crafting),
      [VERSION, CRAFT, rest @ ..] if rest.len() == 4 => Some(Self::Craft { recipe_id: decode_u32(rest)? }),
      [VERSION, GROUP_CAST, rest @ ..] if rest.len() == 8 => Some(Self::GroupCast { cycle: decode_i64(rest)? }),
      [VERSION, GROUP_JOURNAL] => Some(Self::GroupJournal),
      [VERSION, GROUP_RECORDS] => Some(Self::GroupRecords),
      [VERSION, GROUP_HELP] => Some(Self::GroupHelp),
      _ => None,
    }
  }
}

fn encode_encounter(kind: u8, encounter_id: EncounterId, step: u32) -> Vec<u8> {
  let mut bytes = Vec::with_capacity(14);
  bytes.extend_from_slice(&[VERSION, kind]);
  bytes.extend_from_slice(&encounter_id.0.to_le_bytes());
  bytes.extend_from_slice(&step.to_le_bytes());
  bytes
}

fn decode_encounter(bytes: &[u8]) -> Option<(EncounterId, u32)> {
  if bytes.len() != 12 {
    return None;
  }
  let (encounter, step) = bytes.split_at(8);
  Some((EncounterId(i64::from_le_bytes(encounter.try_into().ok()?)), u32::from_le_bytes(step.try_into().ok()?)))
}

fn encode_u32(kind: u8, value: u32) -> Vec<u8> {
  let mut bytes = Vec::with_capacity(6);
  bytes.extend_from_slice(&[VERSION, kind]);
  bytes.extend_from_slice(&value.to_le_bytes());
  bytes
}

fn decode_u32(bytes: &[u8]) -> Option<u32> {
  Some(u32::from_le_bytes(bytes.try_into().ok()?))
}

fn encode_i64(kind: u8, value: i64) -> Vec<u8> {
  let mut bytes = Vec::with_capacity(10);
  bytes.extend_from_slice(&[VERSION, kind]);
  bytes.extend_from_slice(&value.to_le_bytes());
  bytes
}

fn decode_i64(bytes: &[u8]) -> Option<i64> {
  Some(i64::from_le_bytes(bytes.try_into().ok()?))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn callback_round_trip() {
    let values = [
      Callback::Reel { encounter_id: EncounterId(123_456), step: 7 },
      Callback::BuyBait { bait_id: BaitId(3) },
      Callback::Travel { location_id: LocationId(2) },
      Callback::Home,
      Callback::ForageBait,
      Callback::Explore,
      Callback::BuyRod { rod_id: RodId(4) },
      Callback::EquipRod { rod_id: RodId(2) },
      Callback::Pull { encounter_id: EncounterId(77), step: 3 },
      Callback::GiveLine { encounter_id: EncounterId(77), step: 3 },
      Callback::Tasks,
      Callback::ClaimObjective { objective_id: 3 },
      Callback::TurnInContract,
      Callback::Talk,
      Callback::Conditions,
      Callback::RepairRods,
      Callback::Records,
      Callback::Titles,
      Callback::EquipTitle { title_id: 6 },
      Callback::Crafting,
      Callback::Craft { recipe_id: 2 },
      Callback::GroupCast { cycle: -123_456 },
      Callback::GroupJournal,
      Callback::GroupRecords,
      Callback::GroupHelp,
    ];
    for value in values {
      assert_eq!(Callback::decode(&value.encode()), Some(value));
    }
  }
}
