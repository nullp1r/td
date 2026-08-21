//! Small utilities shared by the client internals.

use std::sync::{LockResult, PoisonError};

/// Recovers the value from a poisoned synchronization primitive.
///
/// The client treats lock poisoning as recoverable and continues with the
/// contained guard or value. Callers should use this only where that policy
/// is appropriate for the protected state.
pub(crate) trait PoisonErrorExt<T> {
  /// Returns the contained value, regardless of whether the lock was poisoned.
  fn into_inner(self) -> T;
}

impl<T> PoisonErrorExt<T> for LockResult<T> {
  fn into_inner(self) -> T {
    self.unwrap_or_else(PoisonError::into_inner)
  }
}
