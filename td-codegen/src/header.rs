//! Source header formatting for generated Rust files.

use std::fmt;
use std::time::{Duration, UNIX_EPOCH};

/// Formats a source header with schema provenance and generation metrics.
pub fn header(schema: &str, [parse, codegen]: [Duration; 2]) -> impl fmt::Display + '_ {
  fmt::from_fn(move |f| {
    let mut lines = schema.lines();
    let (Some(line1), Some(line2)) = (lines.next(), lines.next()) else { return Ok(()) };
    if !line1.starts_with("// TDLib") || !line2.starts_with("// fetched") {
      return Ok(());
    }

    let total = fmt_dur(parse + codegen);
    let parse = fmt_dur(parse);
    let codegen = fmt_dur(codegen);
    let utc = fmt_utc();

    writeln!(f, "{line1}")?;
    writeln!(f, "{line2} · generated {utc} in {total} ({parse} + {codegen})")?;
    writeln!(f)
  })
}

/// Formats an elapsed duration cleanly (e.g. `14.3ms`, `450µs`, `1.25s`).
fn fmt_dur(d: Duration) -> impl fmt::Display {
  let nanos = d.as_nanos() as u64;
  let i = nanos.max(1).ilog10().min(9) as usize / 3;
  let unit = ["n", "µ", "m", ""][i];
  let time = nanos as f64 / [1e0, 1e3, 1e6, 1e9][i];
  let prec = i.min(1) * ((time < 99.95) as usize + (time < 9.995) as usize);
  fmt::from_fn(move |f| write!(f, "{time:.prec$}{unit}s"))
}

/// Formats the current UTC system time as `YYYY-MM-DD HH:MM:SS UTC`.
fn fmt_utc() -> impl fmt::Display {
  let secs = UNIX_EPOCH.elapsed().map_or(0, |d| d.as_secs());
  let [mins, sec] = [secs / 60, secs % 60];
  let [hours, min] = [mins / 60, mins % 60];
  let [days, hour] = [hours / 24, hours % 24];
  let (year, month, day) = civil_from_days(days);
  fmt::from_fn(move |f| write!(f, "{year:04}-{month:02}-{day:02} {hour:02}:{min:02}:{sec:02} UTC"))
}

/// Converts days since the Unix epoch (1970-01-01) to a Gregorian `(year, month, day)`.
///
/// Implements Howard Hinnant's calendar algorithm for zero-allocation date conversion.
fn civil_from_days(days: u64) -> (i64, u32, u32) {
  let z = days as i64 + 719_468;
  let era = if let 0.. = z { z } else { z - 146_096 } / 146_097;
  let doe = (z - era * 146_097) as u32;
  let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
  let y = yoe as i64 + era * 400;
  let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
  let mp = (5 * doy + 2) / 153;
  let d = doy - (153 * mp + 2) / 5 + 1;
  let (m, y) = if let ..10 = mp { (mp + 3, y) } else { (mp - 9, y + 1) };
  (y, m, d)
}
