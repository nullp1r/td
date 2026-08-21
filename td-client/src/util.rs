use std::ffi::{CStr, c_char};
use std::sync::{LockResult, PoisonError};

use serde::{Deserialize, Serialize};

/// Returns the bytes of a NUL-terminated JSON buffer returned by a C API.
///
/// # Safety
///
/// `ptr` must point to a valid, NUL-terminated C string for the duration of
/// the returned slice.
pub unsafe fn c_json<'a>(ptr: *const c_char) -> &'a [u8] {
  // SAFETY: upheld by the caller.
  unsafe { CStr::from_ptr(ptr) }.to_bytes()
}

/// Serializes JSON into the NUL-terminated buffer expected by `TDLib`'s C API.
pub fn to_c_json<T: Serialize>(value: &T) -> serde_json::Result<Vec<u8>> {
  let mut bytes = serde_json::to_vec(value)?;
  bytes.push(0);
  Ok(bytes)
}

/// Deserializes a JSON buffer received through a C API.
pub fn from_c_json<'de, T: Deserialize<'de>>(bytes: &'de [u8]) -> serde_json::Result<T> {
  serde_json::from_slice(bytes)
}

/// Recovers the value from a poisoned synchronization primitive.
pub trait PoisonErrorExt<T> {
  /// Returns the contained value, regardless of whether the lock was poisoned.
  fn into_inner(self) -> T;
}

impl<T> PoisonErrorExt<T> for LockResult<T> {
  fn into_inner(self) -> T {
    self.unwrap_or_else(PoisonError::into_inner)
  }
}
