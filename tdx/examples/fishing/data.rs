//! Static data tables and definitions for fish species, fishing spots, and rods.

/// Rarity tier of a catchable fish.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rarity {
  /// Common fish found in abundance.
  Common,
  /// Uncommon fish with higher market value.
  Uncommon,
  /// Rare prize catch.
  Rare,
  /// Epic trophy fish.
  Epic,
  /// Legendary creature of ancient myth.
  Legendary,
}

impl Rarity {
  /// Returns a display badge for the rarity.
  pub const fn badge(self) -> &'static str {
    match self {
      Self::Common => "⚪ Common",
      Self::Uncommon => "🟢 Uncommon",
      Self::Rare => "🔵 Rare",
      Self::Epic => "🟣 Epic",
      Self::Legendary => "🟡 Legendary",
    }
  }

  /// Score weight for rolling rarity tiers.
  pub const fn weight(self) -> u32 {
    match self {
      Self::Common => 60,
      Self::Uncommon => 25,
      Self::Rare => 10,
      Self::Epic => 4,
      Self::Legendary => 1,
    }
  }
}

/// A species of fish available to catch.
#[derive(Clone, Copy, Debug)]
pub struct FishSpecies {
  /// Common name of the fish.
  pub name: &'static str,
  /// Rarity category.
  pub rarity: Rarity,
  /// Minimum catch weight in kilograms.
  pub min_weight: f32,
  /// Maximum catch weight in kilograms.
  pub max_weight: f32,
  /// Base market price in coins.
  pub base_price: u32,
  /// Journal excerpt or folklore description.
  pub lore: &'static str,
}

/// A fishing rod available for purchase or equipped by the player.
#[derive(Clone, Copy, Debug)]
pub struct Rod {
  /// Unique identifier of the rod.
  pub id: usize,
  /// Display name of the rod.
  pub name: &'static str,
  /// Purchase price in coins.
  pub price: u32,
  /// Luck bonus added to rare rolls.
  pub luck_bonus: u32,
  /// Range of delays before a bite triggers, in milliseconds.
  pub bite_delay_ms: (u64, u64),
}

/// A distinct fishing location with its own ecosystem.
#[derive(Clone, Copy, Debug)]
pub struct Spot {
  /// Unique identifier of the spot.
  pub id: usize,
  /// Name of the location.
  pub name: &'static str,
  /// Atmospheric description.
  pub description: &'static str,
  /// Index of the rod required to fish here safely.
  pub required_rod: usize,
  /// Pool of catchable species at this location.
  pub species: &'static [FishSpecies],
}

/// Available fishing rods in order of progression.
pub const RODS: [Rod; 4] = [
  Rod { id: 0, name: "Bamboo Pole", price: 0, luck_bonus: 0, bite_delay_ms: (3000, 5000) },
  Rod { id: 1, name: "Fiberglass Rod", price: 60, luck_bonus: 8, bite_delay_ms: (2500, 4500) },
  Rod { id: 2, name: "Carbon Fiber Pro", price: 220, luck_bonus: 20, bite_delay_ms: (2000, 3500) },
  Rod { id: 3, name: "Golden Master Rod", price: 700, luck_bonus: 40, bite_delay_ms: (1500, 2800) },
];

const POND_SPECIES: [FishSpecies; 5] = [
  FishSpecies {
    name: "Pond Guppy",
    rarity: Rarity::Common,
    min_weight: 0.1,
    max_weight: 0.5,
    base_price: 5,
    lore: "A tiny, cheerful swimmer that bites on almost anything.",
  },
  FishSpecies {
    name: "Silver Crucian",
    rarity: Rarity::Common,
    min_weight: 0.8,
    max_weight: 2.2,
    base_price: 12,
    lore: "Its shimmering scales glint like silver coins under the morning sun.",
  },
  FishSpecies {
    name: "Golden Carp",
    rarity: Rarity::Rare,
    min_weight: 3.5,
    max_weight: 8.0,
    base_price: 45,
    lore: "Said to bring good fortune to anglers patient enough to wait past noon.",
  },
  FishSpecies {
    name: "Speckled Pike",
    rarity: Rarity::Epic,
    min_weight: 6.0,
    max_weight: 14.5,
    base_price: 110,
    lore: "An ambush predator lurking among the reeds. Striking with lightning speed.",
  },
  FishSpecies {
    name: "Mossback Ancient Turtle",
    rarity: Rarity::Legendary,
    min_weight: 25.0,
    max_weight: 48.0,
    base_price: 320,
    lore: "Carrying centuries of river secrets on its moss-encrusted shell.",
  },
];

const RIVER_SPECIES: [FishSpecies; 5] = [
  FishSpecies {
    name: "River Perch",
    rarity: Rarity::Common,
    min_weight: 0.5,
    max_weight: 1.8,
    base_price: 10,
    lore: "A feisty striped swimmer thriving in turbulent currents.",
  },
  FishSpecies {
    name: "Rainbow Trout",
    rarity: Rarity::Uncommon,
    min_weight: 1.5,
    max_weight: 4.2,
    base_price: 24,
    lore: "Adorned with iridescent colors reflecting mountain cascades.",
  },
  FishSpecies {
    name: "King Salmon",
    rarity: Rarity::Rare,
    min_weight: 5.0,
    max_weight: 16.0,
    base_price: 75,
    lore: "Migrating tirelessly upstream against ferocious rapids.",
  },
  FishSpecies {
    name: "Electric Catfish",
    rarity: Rarity::Epic,
    min_weight: 8.0,
    max_weight: 22.0,
    base_price: 180,
    lore: "Sparks hum along its barbels, numbing unwary fingers upon contact.",
  },
  FishSpecies {
    name: "Azure River Dragon",
    rarity: Rarity::Legendary,
    min_weight: 30.0,
    max_weight: 65.0,
    base_price: 450,
    lore: "A mythical river serpent spoken of only in dusty waterside tavern ballads.",
  },
];

const ABYSS_SPECIES: [FishSpecies; 5] = [
  FishSpecies {
    name: "Sunken Leather Boot",
    rarity: Rarity::Common,
    min_weight: 0.9,
    max_weight: 1.4,
    base_price: 2,
    lore: "A waterlogged relic of an unfortunate sailor. Has seen better tides.",
  },
  FishSpecies {
    name: "Lantern Angler",
    rarity: Rarity::Uncommon,
    min_weight: 2.0,
    max_weight: 6.5,
    base_price: 35,
    lore: "Bears a bioluminescent lure that illuminates pitch-black trenches.",
  },
  FishSpecies {
    name: "Shadow Marlin",
    rarity: Rarity::Rare,
    min_weight: 20.0,
    max_weight: 55.0,
    base_price: 120,
    lore: "Cuts through the deep ocean swell like an obsidian blade.",
  },
  FishSpecies {
    name: "Colossal Deep Squid",
    rarity: Rarity::Epic,
    min_weight: 40.0,
    max_weight: 95.0,
    base_price: 260,
    lore: "Its dinner-plate eyes stare calmly into the crushing void of the deep.",
  },
  FishSpecies {
    name: "Abyssal Leviathan",
    rarity: Rarity::Legendary,
    min_weight: 120.0,
    max_weight: 280.0,
    base_price: 850,
    lore: "An ancient sovereign of the sea floor that swallows ocean storms whole.",
  },
];

/// All registered fishing spots.
pub const SPOTS: [Spot; 3] = [
  Spot {
    id: 0,
    name: "Whispering Lilypond",
    description: "Tranquil waters shaded by weeping willows. Ideal for beginners.",
    required_rod: 0,
    species: &POND_SPECIES,
  },
  Spot {
    id: 1,
    name: "Roaring Rapids",
    description: "Fast-flowing alpine torrent with powerful, muscular swimmers.",
    required_rod: 1,
    species: &RIVER_SPECIES,
  },
  Spot {
    id: 2,
    name: "Midnight Abyss",
    description: "Crushing ocean depths concealing glowing predators and leviathans.",
    required_rod: 2,
    species: &ABYSS_SPECIES,
  },
];

/// A fast 64-bit pseudo-random number generator.
#[derive(Clone, Copy, Debug)]
pub struct SplitMix64(u64);

impl SplitMix64 {
  /// Initializes the PRNG with a 64-bit seed.
  pub const fn new(seed: u64) -> Self {
    Self(seed)
  }

  /// Generates the next pseudo-random `u64`.
  pub fn next_u64(&mut self) -> u64 {
    self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = self.0;
    z = (z ^ z >> 30).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ z >> 27).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ z >> 31
  }

  /// Generates a random `f32` in the half-open interval `[0.0, 1.0)`.
  pub fn next_f32(&mut self) -> f32 {
    ((self.next_u64() >> 40) as f32) * (1.0 / (1u32 << 24) as f32)
  }
}

/// Computes a pseudo-random roll using [`SplitMix64`].
#[must_use]
pub fn roll_catch(spot_id: usize, luck_bonus: u32, rng: &mut SplitMix64) -> (FishSpecies, f32, u32) {
  let spot = SPOTS.get(spot_id).unwrap_or(&SPOTS[0]);
  let pool = spot.species;

  let total_weight: u32 = pool
    .iter()
    .map(|s| {
      let w = s.rarity.weight();
      if s.rarity == Rarity::Common { w } else { w.saturating_add(luck_bonus / 2) }
    })
    .sum();

  let mut pick = (rng.next_u64() % u64::from(total_weight.max(1))) as u32;
  let mut selected = pool[0];
  for &species in pool {
    let mut weight = species.rarity.weight();
    if species.rarity != Rarity::Common {
      weight = weight.saturating_add(luck_bonus / 2);
    }
    if pick < weight {
      selected = species;
      break;
    }
    pick -= weight;
  }

  // Roll weight within species range
  let weight = selected.min_weight + (selected.max_weight - selected.min_weight) * rng.next_f32();

  // Calculate final price with weight bonus
  let weight_ratio = (weight / selected.min_weight).max(1.0);
  let price = ((selected.base_price as f32) * (1.0 + (weight_ratio - 1.0) * 0.35)).round() as u32;

  (selected, weight, price)
}
