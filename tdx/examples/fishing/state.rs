//! In-memory state tracking players, inventory, and active fishing sessions.

use std::collections::HashMap;
use std::time::Instant;

use super::data::{self, RODS, Rarity, SPOTS, SplitMix64};

/// A stored record of a successfully caught fish.
#[derive(Clone, Copy, Debug)]
pub struct CatchRecord {
  /// Name of the species.
  pub species_name: &'static str,
  /// Rarity category.
  pub rarity: Rarity,
  /// Catch weight in kilograms.
  pub weight: f32,
  /// Gold value of the fish.
  pub price: u32,
}

/// Persistent player profile and equipment.
#[derive(Debug)]
pub struct Player {
  /// Angler display name.
  pub name: String,
  /// Available coin balance.
  pub coins: u32,
  /// Currently equipped rod index.
  pub rod_id: usize,
  /// Currently active fishing spot index.
  pub spot_id: usize,
  /// Bitmask of owned rod IDs.
  pub owned_rods: u32,
  /// Current catches stored in the bag.
  pub bag: Vec<CatchRecord>,
  /// Total number of fish caught across all sessions.
  pub total_caught: u32,
  /// Best (heaviest or most valuable) catch.
  pub best_catch: Option<CatchRecord>,
}

impl Default for Player {
  fn default() -> Self {
    Self {
      name: String::new(),
      coins: 0,
      rod_id: 0,
      spot_id: 0,
      owned_rods: 1, // Bamboo rod is unlocked by default
      bag: Vec::new(),
      total_caught: 0,
      best_catch: None,
    }
  }
}

impl Player {
  /// Checks whether the player owns a specific rod.
  pub fn owns_rod(&self, rod_id: usize) -> bool {
    (self.owned_rods & (1 << rod_id)) != 0
  }

  /// Returns the display name or a fallback if unset.
  pub fn display_name(&self) -> &str {
    if self.name.is_empty() { "Angler" } else { &self.name }
  }
}

/// An in-flight cast session waiting for the player to reel in.
#[derive(Debug)]
pub struct ActiveCast {
  /// User who initiated the cast.
  pub user_id: i64,
  /// Rod in use during the cast.
  pub rod_id: usize,
  /// Fishing spot where the line was cast.
  pub spot_id: usize,
  /// When the bite window began, if triggered.
  pub bite_window: Option<(Instant, Instant)>,
}

/// Result of attempting to reel in a line.
pub enum ReelOutcome {
  /// Successfully hooked a fish with species, weight, price, and lore.
  Hooked {
    /// Caught fish details.
    record: CatchRecord,
    /// Folklore or lore text for this species.
    lore: &'static str,
  },
  /// Reeled in before the fish took the bait.
  TooEarly,
  /// Reeled in after the fish ate the bait and swam off.
  TooLate,
  /// The cast was already completed, expired, or belongs to another angler.
  NotActive,
}

/// Complete game state managing players and live cast sessions.
#[derive(Default)]
pub struct GameState {
  players: HashMap<i64, Player>,
  active_casts: HashMap<(i64, i64), ActiveCast>,
}

impl GameState {
  /// Retrieves or creates a player profile.
  pub fn player_mut(&mut self, user_id: i64) -> &mut Player {
    self.players.entry(user_id).or_default()
  }

  /// Looks up a player profile immutably.
  pub fn player(&self, user_id: i64) -> Option<&Player> {
    self.players.get(&user_id)
  }

  /// Checks whether a cast session is active.
  pub fn is_cast_active(&self, chat_id: i64, message_id: i64) -> bool {
    self.active_casts.contains_key(&(chat_id, message_id))
  }

  /// Registers a newly initiated line cast.
  pub fn register_cast(&mut self, user_id: i64, chat_id: i64, message_id: i64, rod_id: usize, spot_id: usize) {
    let cast = ActiveCast { user_id, rod_id, spot_id, bite_window: None };
    self.active_casts.insert((chat_id, message_id), cast);
  }

  /// Sets the active bite window when the bobber triggers.
  pub fn set_bite_window(&mut self, chat_id: i64, message_id: i64, start: Instant, end: Instant) {
    if let Some(cast) = self.active_casts.get_mut(&(chat_id, message_id)) {
      cast.bite_window = Some((start, end));
    }
  }

  /// Attempts to reel in the line.
  pub fn reel(&mut self, user_id: i64, chat_id: i64, message_id: i64, now: Instant, rng: &mut SplitMix64) -> ReelOutcome {
    let Some(cast) = self.active_casts.get(&(chat_id, message_id)) else {
      return ReelOutcome::NotActive;
    };

    if cast.user_id != user_id {
      return ReelOutcome::NotActive;
    }

    let Some((start, end)) = cast.bite_window else {
      self.active_casts.remove(&(chat_id, message_id));
      return ReelOutcome::TooEarly;
    };

    if now < start {
      self.active_casts.remove(&(chat_id, message_id));
      ReelOutcome::TooEarly
    } else if now > end {
      self.active_casts.remove(&(chat_id, message_id));
      ReelOutcome::TooLate
    } else {
      let cast = self.active_casts.remove(&(chat_id, message_id)).expect("cast exists");
      let rod = RODS.get(cast.rod_id).unwrap_or(&RODS[0]);
      let (species, weight, price) = data::roll_catch(cast.spot_id, rod.luck_bonus, rng);
      let record = CatchRecord { species_name: species.name, rarity: species.rarity, weight, price };

      // Record in player inventory
      let player = self.players.entry(user_id).or_default();
      player.bag.push(record);
      player.total_caught = player.total_caught.saturating_add(1);

      if player.best_catch.as_ref().is_none_or(|b| record.weight > b.weight) {
        player.best_catch = Some(record);
      }

      ReelOutcome::Hooked { record, lore: species.lore }
    }
  }

  /// Marks a cast as expired if it timed out unreeled.
  pub fn expire_cast(&mut self, chat_id: i64, message_id: i64) -> bool {
    self.active_casts.remove(&(chat_id, message_id)).is_some()
  }

  /// Sells all fish currently in the player's bag.
  pub fn sell_all(&mut self, user_id: i64) -> (usize, u32) {
    let player = self.players.entry(user_id).or_default();
    let count = player.bag.len();
    let total_earned: u32 = player.bag.iter().map(|c| c.price).sum();
    player.coins = player.coins.saturating_add(total_earned);
    player.bag.clear();
    (count, total_earned)
  }

  /// Buys or equips a rod.
  pub fn equip_or_buy_rod(&mut self, user_id: i64, rod_id: usize) -> Result<&'static str, &'static str> {
    let Some(rod) = RODS.get(rod_id) else {
      return Err("Invalid rod ID");
    };

    let player = self.players.entry(user_id).or_default();
    if player.owns_rod(rod_id) {
      player.rod_id = rod_id;
      return Ok("Equipped rod");
    }

    if player.coins < rod.price {
      return Err("Not enough coins");
    }

    player.coins -= rod.price;
    player.owned_rods |= 1 << rod_id;
    player.rod_id = rod_id;
    Ok("Purchased and equipped rod")
  }

  /// Travels to a fishing spot if the player has the required rod.
  pub fn travel_to_spot(&mut self, user_id: i64, spot_id: usize) -> Result<&'static str, &'static str> {
    let Some(spot) = SPOTS.get(spot_id) else {
      return Err("Invalid fishing spot");
    };

    let player = self.players.entry(user_id).or_default();
    if player.rod_id < spot.required_rod {
      return Err("You need a stronger rod to fish here");
    }

    player.spot_id = spot_id;
    Ok("Traveled to new spot")
  }
}
