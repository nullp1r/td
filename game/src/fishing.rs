//! Pure deterministic fishing mechanics; persistence and transport live elsewhere.

use crate::{
  content::{Content, EncounterWeight, Species},
  ids::{BaitId, LocationId, SpeciesId},
  rng::Rng,
  world::Environment,
};

// Independent RNG domains keep one mechanic's extra draws from perturbing the others.
const SPECIES_SEED: u64 = 0xA5A5_4D4D_9292_1717;
const BITE_SEED: u64 = 0x7D23_1A69_2C8B_4F10;
const SPECIMEN_SEED: u64 = 0xC6BC_2796_92B5_CC83;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reaction {
  Excellent,
  Good,
  Late,
  Missed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StruggleAction {
  Pull,
  GiveLine,
}

#[derive(Clone, Copy, Debug)]
pub struct Specimen {
  pub length_mm: u32,
  pub weight_g: u32,
}

pub fn select_species(content: &Content, location: LocationId, bait: BaitId, environment: Environment, seed: u64) -> SpeciesId {
  let location = content.location(location);
  let total = location.encounters.iter().map(|encounter| encounter_weight(content, *encounter, bait, environment)).fold(0_u64, u64::saturating_add);

  let fallback = location.encounters.first().expect("validated fishable location has encounters").species_id;
  if total == 0 {
    return fallback;
  }

  let mut rng = Rng::new(seed ^ SPECIES_SEED);
  let mut roll = rng.below(total);
  for encounter in &location.encounters {
    let weight = encounter_weight(content, *encounter, bait, environment);
    if roll < weight {
      return encounter.species_id;
    }
    roll -= weight;
  }
  fallback
}

fn encounter_weight(content: &Content, encounter: EncounterWeight, bait: BaitId, environment: Environment) -> u64 {
  let species = content.species(encounter.species_id);
  if !eligible(species, environment) {
    return 0;
  }
  let multiplier = species
    .bait_modifiers
    .binary_search_by_key(&bait, |modifier| modifier.bait_id)
    .map_or(10_000_u64, |index| u64::from(species.bait_modifiers[index].multiplier_bp));
  u64::from(encounter.weight).saturating_mul(multiplier)
}

pub fn bite_delay_ms(species: &Species, bait_power: u32, seed: u64) -> u64 {
  let mut rng = Rng::new(seed ^ BITE_SEED);
  let base = rng.range_inclusive(species.bite_min_ms, species.bite_max_ms);
  let reduction_bp = bait_power.saturating_mul(80).min(3_500);
  base.saturating_mul(u64::from(10_000 - reduction_bp)) / 10_000
}

pub fn effective_control(base_control: u32, condition: u32) -> u32 {
  base_control.saturating_mul(75 + condition.clamp(1, 100) / 4) / 100
}

pub fn reaction(species: &Species, elapsed_ms: u64) -> Reaction {
  let [excellent, good, late] = species.reaction_ms;
  if elapsed_ms <= excellent {
    Reaction::Excellent
  } else if elapsed_ms <= good {
    Reaction::Good
  } else if elapsed_ms <= late {
    Reaction::Late
  } else {
    Reaction::Missed
  }
}

pub fn can_land(species: &Species, rod_control: u32, reaction: Reaction, first_catch: bool) -> bool {
  let timing = match reaction {
    Reaction::Excellent => 130,
    Reaction::Good => 110,
    Reaction::Late => 80,
    Reaction::Missed => return false,
  };
  first_catch || rod_control.saturating_mul(timing) >= species.difficulty.saturating_mul(100)
}

pub fn required_struggle_action(seed: u64) -> StruggleAction {
  if (seed.rotate_left(11) & 1) == 0 { StruggleAction::Pull } else { StruggleAction::GiveLine }
}

pub const fn struggle_observation(action: StruggleAction) -> &'static str {
  match action {
    StruggleAction::Pull => "The fish turns sharply toward jagged rocks. If it reaches them, the line will not survive.",
    StruggleAction::GiveLine => "The fish dives straight down. The line starts to sing under the sudden load.",
  }
}

pub fn struggle_succeeds(species: &Species, rod_control: u32, reaction: Reaction, chosen: StruggleAction, required: StruggleAction) -> bool {
  if chosen == required {
    return reaction != Reaction::Missed;
  }
  let timing = match reaction {
    Reaction::Excellent => 140,
    Reaction::Good => 115,
    Reaction::Late => 80,
    Reaction::Missed => return false,
  };
  rod_control.saturating_mul(timing) >= species.difficulty.saturating_mul(150)
}

pub fn generate_specimen(species: &Species, seed: u64) -> Specimen {
  let mut rng = Rng::new(seed ^ SPECIMEN_SEED);
  let range = species.max_length_mm - species.min_length_mm;
  // Averaging three uniform draws clusters lengths around the middle without floating-point state.
  let sample = (u64::from(rng.next_u32()) + u64::from(rng.next_u32()) + u64::from(rng.next_u32())) / 3;
  let offset = (u64::from(range) * sample / u64::from(u32::MAX)) as u32;
  let length_mm = species.min_length_mm + offset;

  let length = u64::from(length_mm);
  let typical = u64::from(species.typical_length_mm);
  // Fish mass scales approximately with volume, then gets a small per-specimen condition variation.
  let cubic_ratio_bp = length.saturating_pow(3).saturating_mul(10_000) / typical.saturating_pow(3).max(1);
  let condition_bp = rng.range_inclusive(9_000, 11_000);
  let weight = u64::from(species.typical_weight_g).saturating_mul(cubic_ratio_bp).saturating_mul(condition_bp) / 100_000_000;
  let weight_g = weight.max(1).min(u64::from(u32::MAX)) as u32;

  Specimen { length_mm, weight_g }
}

pub fn sale_value(species: &Species, specimen: Specimen) -> u64 {
  let typical = u64::from(species.typical_weight_g).max(1);
  let weight = u64::from(specimen.weight_g);
  let ratio_bp = (weight.saturating_mul(10_000) / typical).clamp(5_000, 40_000);
  let value = u64::from(species.base_value).saturating_mul(5_000 + ratio_bp) / 15_000;
  value.max(1)
}

pub fn observation(species: &Species) -> &'static str {
  match species.difficulty {
    0..=6 => "The float disappears beneath the surface.",
    7..=10 => "The line snaps taut and starts moving sideways.",
    11..=14 => "Something heavy takes the bait and pulls toward deeper water.",
    _ => "Your rod bends violently. Whatever took the bait is extremely powerful.",
  }
}

pub fn eligible(species: &Species, environment: Environment) -> bool {
  let weather = environment.weather as u8;
  let day_part = environment.day_part as u8;
  (species.required_weather.is_empty() || species.required_weather.contains(&weather))
    && (species.required_day_parts.is_empty() || species.required_day_parts.contains(&day_part))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::world::{DayPart, Weather};

  fn content() -> Content {
    Content::from_slice(include_bytes!("../content/game.json")).expect("content")
  }

  #[test]
  fn deterministic_selection() {
    let content = content();
    let environment = Environment { weather: Weather::Clear, game_minute: 720, day_part: DayPart::Day };
    let first = select_species(&content, LocationId(1), BaitId(1), environment, 42);
    let second = select_species(&content, LocationId(1), BaitId(1), environment, 42);
    assert_eq!(first, second);
  }

  #[test]
  fn first_catch_is_forgiving() {
    let content = content();
    let sturgeon = content.species(SpeciesId(6));
    assert!(can_land(sturgeon, 10, Reaction::Late, true));
    assert!(!can_land(sturgeon, 10, Reaction::Excellent, false));
  }

  #[test]
  fn bait_power_shortens_wait_without_changing_seed_determinism() {
    let content = content();
    let species = content.species(SpeciesId(1));
    let weak = bite_delay_ms(species, 0, 999);
    let strong = bite_delay_ms(species, 20, 999);
    assert!(strong <= weak);
    assert_eq!(strong, bite_delay_ms(species, 20, 999));
  }

  #[test]
  fn worn_rods_keep_most_of_their_control() {
    assert_eq!(effective_control(20, 100), 20);
    assert!(effective_control(20, 1) >= 15);
    assert!(effective_control(20, 25) < effective_control(20, 100));
  }

  #[test]
  fn specimens_are_stable() {
    let content = content();
    let species = content.species(SpeciesId(1));
    let first = generate_specimen(species, 123);
    let second = generate_specimen(species, 123);
    assert_eq!(first.length_mm, second.length_mm);
    assert_eq!(first.weight_g, second.weight_g);
  }
}
