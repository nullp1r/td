//! Shared string and UTF-16 calculation utilities.

/// Extension trait for UTF-16 measurement and splitting on string slices.
pub trait Utf16 {
  /// Computes the length of the string in UTF-16 code units.
  fn len_utf16(&self) -> i32;

  /// Splits the string at a UTF-16 code unit boundary.
  ///
  /// Returns `None` if `mid` falls inside a surrogate pair or exceeds the string length.
  fn split_at_utf16(&self, mid: usize) -> Option<(&str, &str)>;
}

impl Utf16 for str {
  fn len_utf16(&self) -> i32 {
    let mut len = 0;
    for &b in self.as_bytes() {
      match b {
        0x80..=0xBF => {}
        0xF0..=0xFF => len += 2,
        _ => len += 1,
      }
    }
    len
  }

  fn split_at_utf16(&self, mid: usize) -> Option<(&str, &str)> {
    let mut utf16 = 0;
    for (idx, &b) in self.as_bytes().iter().enumerate() {
      match b {
        0x80..=0xBF => {}
        _ if utf16 == mid => return self.split_at_checked(idx),
        _ if utf16 > mid => return None,
        0xF0..=0xFF => utf16 += 2,
        _ => utf16 += 1,
      }
    }
    (utf16 == mid).then_some((self, ""))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn utf16_utilities() {
    let s = "Hello 🦀 Привет";
    assert_eq!(s.len_utf16(), 15);
    assert_eq!(s.split_at_utf16(0), Some(("", "Hello 🦀 Привет")));
    assert_eq!(s.split_at_utf16(6), Some(("Hello ", "🦀 Привет")));
    assert_eq!(s.split_at_utf16(8), Some(("Hello 🦀", " Привет")));
    assert_eq!(s.split_at_utf16(7), None);
    assert_eq!(s.split_at_utf16(15), Some(("Hello 🦀 Привет", "")));
    assert_eq!(s.split_at_utf16(16), None);
  }
}
