//! Loads, validates, and indexes immutable gameplay content.

use std::{fs, io, path::Path};

use serde::Deserialize;
use thiserror::Error;

use crate::ids::{BaitId, LocationId, RodId, SpeciesId};

#[derive(Debug, Error)]
pub enum Error {
  #[error("failed to read content: {0}")]
  Io(#[from] io::Error),
  #[error("failed to parse content: {0}")]
  Json(#[from] serde_json::Error),
  #[error("invalid content: {0}")]
  Invalid(String),
}

#[derive(Clone, Debug, Deserialize)]
pub struct Content {
  pub baits: Vec<Bait>,
  pub rods: Vec<Rod>,
  pub locations: Vec<Location>,
  pub species: Vec<Species>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Bait {
  pub id: BaitId,
  pub name: String,
  pub fishing_power: u32,
  #[serde(default = "default_bait_buy_price")]
  pub buy_price: u32,
  #[serde(default = "default_bait_pack_size")]
  pub pack_size: u32,
  #[serde(default = "default_true")]
  pub buyable: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Rod {
  pub id: RodId,
  pub name: String,
  pub control: u32,
  pub buy_price: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Location {
  pub id: LocationId,
  pub name: String,
  pub description: String,
  #[serde(default = "default_true")]
  pub fishable: bool,
  pub encounters: Vec<EncounterWeight>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct EncounterWeight {
  pub species_id: SpeciesId,
  pub weight: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Species {
  pub id: SpeciesId,
  pub name: String,
  pub min_length_mm: u32,
  pub typical_length_mm: u32,
  pub max_length_mm: u32,
  pub typical_weight_g: u32,
  pub difficulty: u32,
  pub xp: u32,
  pub base_value: u32,
  #[serde(default)]
  pub clue: String,
  pub bite_min_ms: u64,
  pub bite_max_ms: u64,
  pub reaction_ms: [u64; 3],
  pub required_weather: Vec<u8>,
  pub required_day_parts: Vec<u8>,
  pub bait_modifiers: Vec<BaitModifier>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct BaitModifier {
  pub bait_id: BaitId,
  pub multiplier_bp: u32,
}

impl Content {
  pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
    let bytes = fs::read(path)?;
    Self::from_slice(&bytes)
  }

  /// Loads and validates content from JSON bytes.
  pub fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
    let mut content: Self = serde_json::from_slice(bytes)?;
    content.prepare()?;
    Ok(content)
  }

  pub fn location(&self, id: LocationId) -> &Location {
    let index = self.locations.binary_search_by_key(&id, |location| location.id).expect("validated location id");
    &self.locations[index]
  }

  pub fn species(&self, id: SpeciesId) -> &Species {
    let index = self.species.binary_search_by_key(&id, |species| species.id).expect("validated species id");
    &self.species[index]
  }

  pub fn bait(&self, id: BaitId) -> &Bait {
    let index = self.baits.binary_search_by_key(&id, |bait| bait.id).expect("validated bait id");
    &self.baits[index]
  }

  pub fn rod(&self, id: RodId) -> &Rod {
    let index = self.rods.binary_search_by_key(&id, |rod| rod.id).expect("validated rod id");
    &self.rods[index]
  }

  fn prepare(&mut self) -> Result<(), Error> {
    // Runtime lookups use binary_search, so every keyed collection is normalized once at load time.
    self.locations.sort_unstable_by_key(|value| value.id);
    self.species.sort_unstable_by_key(|value| value.id);
    self.baits.sort_unstable_by_key(|value| value.id);
    self.rods.sort_unstable_by_key(|value| value.id);
    for species in &mut self.species {
      species.bait_modifiers.sort_unstable_by_key(|modifier| modifier.bait_id);
    }
    self.validate()
  }

  fn validate(&self) -> Result<(), Error> {
    unique_sorted(self.locations.iter().map(|value| value.id.0), "location")?;
    unique_sorted(self.species.iter().map(|value| value.id.0), "species")?;
    unique_sorted(self.baits.iter().map(|value| value.id.0), "bait")?;
    unique_sorted(self.rods.iter().map(|value| value.id.0), "rod")?;
    self.validate_locations()?;
    self.validate_equipment()?;
    self.validate_species()
  }

  fn validate_locations(&self) -> Result<(), Error> {
    for id in 1..=5 {
      if self.locations.binary_search_by_key(&LocationId(id), |value| value.id).is_err() {
        return Err(Error::Invalid(format!("required MVP location {id} is missing")));
      }
    }
    for location in &self.locations {
      if location.fishable == location.encounters.is_empty() {
        let issue = if location.fishable { "fishable location has no encounters" } else { "non-fishable location has encounters" };
        return Err(Error::Invalid(format!("location {}: {issue}", location.id.0)));
      }
      for encounter in &location.encounters {
        if encounter.weight == 0 {
          return Err(Error::Invalid(format!("location {} has zero encounter weight", location.id.0)));
        }
        if self.species.binary_search_by_key(&encounter.species_id, |value| value.id).is_err() {
          return Err(Error::Invalid(format!("location {} references unknown species {}", location.id.0, encounter.species_id.0)));
        }
      }
    }
    Ok(())
  }

  fn validate_equipment(&self) -> Result<(), Error> {
    if self.baits.binary_search_by_key(&BaitId(1), |value| value.id).is_err() {
      return Err(Error::Invalid("starter bait 1 is missing".into()));
    }
    if self.rods.binary_search_by_key(&RodId(1), |value| value.id).is_err() {
      return Err(Error::Invalid("starter rod 1 is missing".into()));
    }
    for rod in &self.rods {
      if rod.id != RodId(1) && rod.buy_price == 0 {
        return Err(Error::Invalid(format!("rod {} has zero buy price", rod.id.0)));
      }
    }
    for bait in &self.baits {
      if bait.buyable && bait.buy_price == 0 {
        return Err(Error::Invalid(format!("buyable bait {} has zero buy price", bait.id.0)));
      }
      if bait.pack_size == 0 {
        return Err(Error::Invalid(format!("bait {} has zero pack size", bait.id.0)));
      }
    }
    Ok(())
  }

  fn validate_species(&self) -> Result<(), Error> {
    for species in &self.species {
      if !(species.min_length_mm..=species.max_length_mm).contains(&species.typical_length_mm) {
        return Err(Error::Invalid(format!("species {} has invalid length bounds", species.id.0)));
      }
      let [excellent, good, late] = species.reaction_ms;
      if !(excellent < good && good < late) {
        return Err(Error::Invalid(format!("species {} has invalid reaction windows", species.id.0)));
      }
      if species.bite_min_ms > species.bite_max_ms {
        return Err(Error::Invalid(format!("species {} has invalid bite delay", species.id.0)));
      }
      unique_sorted(species.bait_modifiers.iter().map(|modifier| modifier.bait_id.0), "bait modifier")?;
      for modifier in &species.bait_modifiers {
        if self.baits.binary_search_by_key(&modifier.bait_id, |value| value.id).is_err() {
          return Err(Error::Invalid(format!("species {} references unknown bait {}", species.id.0, modifier.bait_id.0)));
        }
      }
    }
    Ok(())
  }
}

fn unique_sorted(values: impl IntoIterator<Item = u32>, kind: &str) -> Result<(), Error> {
  let mut previous = None;
  for value in values {
    if previous == Some(value) {
      return Err(Error::Invalid(format!("duplicate {kind} id {value}")));
    }
    previous = Some(value);
  }
  Ok(())
}

const fn default_true() -> bool {
  true
}

const fn default_bait_buy_price() -> u32 {
  5
}

const fn default_bait_pack_size() -> u32 {
  5
}

#[cfg(test)]
mod tests {
  use super::Content;

  #[test]
  fn bundled_content_is_valid() {
    Content::from_slice(include_bytes!("../content/game.json")).expect("valid content");
  }
}
