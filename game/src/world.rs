//! Deterministic accelerated world clock derived only from wall-clock milliseconds.
//! No world tick is persisted: the same timestamp always yields the same day part and weather.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum Weather {
  Clear = 0,
  Rain = 1,
  Fog = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum DayPart {
  Morning = 0,
  Day = 1,
  Evening = 2,
  Night = 3,
}

#[derive(Clone, Copy, Debug)]
pub struct Environment {
  pub weather: Weather,
  pub game_minute: u16,
  pub day_part: DayPart,
}

pub const GAME_DAY_REAL_MS: i64 = 2 * 60 * 60 * 1000;
const WEATHER_SLOT_MS: i64 = 15 * 60 * 1000;

pub fn environment_at(now_ms: i64) -> Environment {
  let within_day = now_ms.rem_euclid(GAME_DAY_REAL_MS);
  let game_minute = ((within_day * 1440) / GAME_DAY_REAL_MS) as u16;
  let day_part = match game_minute {
    300..=659 => DayPart::Morning,
    660..=1019 => DayPart::Day,
    1020..=1259 => DayPart::Evening,
    _ => DayPart::Night,
  };

  let slot = now_ms.div_euclid(WEATHER_SLOT_MS) as u64;
  let mixed = slot.wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(17) ^ 0xD1B5_4A32_D192_ED03;
  let weather = match mixed % 10 {
    0..=1 => Weather::Fog,
    2..=4 => Weather::Rain,
    _ => Weather::Clear,
  };

  Environment { weather, game_minute, day_part }
}

pub fn game_day(now_ms: i64) -> i64 {
  now_ms.div_euclid(GAME_DAY_REAL_MS)
}

pub fn next_weather_change_ms(now_ms: i64) -> i64 {
  (now_ms.div_euclid(WEATHER_SLOT_MS) + 1).saturating_mul(WEATHER_SLOT_MS)
}

pub fn next_day_part_change_ms(now_ms: i64) -> i64 {
  let within_day = now_ms.rem_euclid(GAME_DAY_REAL_MS);
  let game_minute = ((within_day * 1440) / GAME_DAY_REAL_MS) as u16;
  let next_game_minute = match game_minute {
    0..=299 => 300_i64,
    300..=659 => 660,
    660..=1019 => 1020,
    1020..=1259 => 1260,
    _ => 1440,
  };
  let day_start = now_ms.saturating_sub(within_day);
  day_start.saturating_add(next_game_minute.saturating_mul(GAME_DAY_REAL_MS) / 1440)
}

impl Weather {
  pub const fn label(self) -> &'static str {
    match self {
      Self::Clear => "Clear",
      Self::Rain => "Rain",
      Self::Fog => "Fog",
    }
  }
}

impl DayPart {
  pub const fn label(self) -> &'static str {
    match self {
      Self::Morning => "Morning",
      Self::Day => "Day",
      Self::Evening => "Evening",
      Self::Night => "Night",
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn future_boundaries_are_strictly_ahead() {
    for now in [0_i64, 1, 123_456_789, GAME_DAY_REAL_MS - 1, GAME_DAY_REAL_MS] {
      assert!(next_weather_change_ms(now) > now);
      assert!(next_day_part_change_ms(now) > now);
    }
  }

  #[test]
  fn game_day_matches_accelerated_clock() {
    assert_eq!(game_day(0), 0);
    assert_eq!(game_day(GAME_DAY_REAL_MS - 1), 0);
    assert_eq!(game_day(GAME_DAY_REAL_MS), 1);
  }
}
