//! Small deterministic SplitMix64-based RNG used for reproducible gameplay rolls.

#[derive(Clone, Copy, Debug)]
pub struct Rng(u64);

impl Rng {
  pub const fn new(seed: u64) -> Self {
    Self(seed)
  }

  pub fn next_u64(&mut self) -> u64 {
    self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut value = self.0;
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
  }

  pub fn next_u32(&mut self) -> u32 {
    self.next_u64() as u32
  }

  /// Rejection sampling avoids modulo bias for bounds that do not divide `u64::MAX + 1`.
  pub fn below(&mut self, upper: u64) -> u64 {
    assert!(upper > 0, "upper bound must be non-zero");
    let threshold = upper.wrapping_neg() % upper;
    loop {
      let value = self.next_u64();
      if value >= threshold {
        return value % upper;
      }
    }
  }

  pub fn range_inclusive(&mut self, start: u64, end: u64) -> u64 {
    assert!(start <= end, "invalid inclusive range");
    let width = end.wrapping_sub(start).wrapping_add(1);
    if width == 0 {
      return self.next_u64();
    }
    start + self.below(width)
  }
}

#[cfg(test)]
mod tests {
  use super::Rng;

  #[test]
  fn seeded_sequence_is_stable() {
    let mut first = Rng::new(42);
    let mut second = Rng::new(42);
    for _ in 0..32 {
      assert_eq!(first.next_u64(), second.next_u64());
    }
  }

  #[test]
  fn range_stays_inside_bounds() {
    let mut rng = Rng::new(7);
    for _ in 0..1_000 {
      let value = rng.range_inclusive(10, 15);
      assert!((10..=15).contains(&value));
    }
  }
}
