//! Process-wide log and unsolicited-error callbacks.
//!
//! Each callback is boxed and called under its own mutex. Calls are serialized,
//! replacement/removal waits for the current call, and mutex poisoning is ignored.
//! Callbacks must return promptly, must not call `TDLib`, and must not reconfigure
//! callbacks. Use an application-owned queue when processing needs those operations.
//!
//! Log callbacks run on native threads; error callbacks run on the receiver thread.
//! Panicking from a log callback aborts at the C boundary. Panicking from an error
//! callback can terminate the receiver. Neither callback should panic.

use std::ffi::{CStr, c_char};
use std::sync::{Mutex, Once, PoisonError};

use td_types::fns;

use crate::{Error, Result, execute};

type LogCallback = Box<dyn FnMut(i32, &CStr) + Send>;
type ErrorCallback = Box<dyn FnMut(Error) + Send>;

static LOG_CALLBACK: Mutex<Option<LogCallback>> = Mutex::new(None);
static ERROR_CALLBACK: Mutex<Option<ErrorCallback>> = Mutex::new(None);
static INSTALL: Once = Once::new();

/// Installs or replaces the native log callback for all sessions.
///
/// Leaves the native output stream and verbosity unchanged. Use [`set_log_level`]
/// to adjust verbosity at any time outside callbacks. `message` borrows the complete
/// native C string, including its prefix and newline.
/// Do not replace the native hook through raw FFI.
///
/// ```no_run
/// td_client::set_log_level(2)?;
/// td_client::set_log_callback(|level, message| {
///   eprintln!("[{level}] {}", message.to_string_lossy());
/// });
/// # Ok::<(), td_client::Error>(())
/// ```
pub fn set_log_callback(callback: impl FnMut(i32, &CStr) + Send + 'static) {
  INSTALL.call_once(|| {
    // SAFETY: The permanent C-ABI trampoline borrows TDLib's message only for
    // the call and serializes access to the owned Rust callback.
    unsafe { td_sys::td_set_log_message_callback(i32::MAX, Some(log_message)) };
  });
  LOG_CALLBACK.lock().unwrap_or_else(PoisonError::into_inner).replace(Box::new(callback));
}

/// Removes the log callback without changing the native output stream or verbosity.
pub fn clear_log_callback() {
  LOG_CALLBACK.lock().unwrap_or_else(PoisonError::into_inner).take();
}

/// Installs or replaces the unsolicited-error callback for all sessions.
///
/// Receives malformed/unroutable native output and native errors with no waiting
/// request recipient. Correlated request errors are returned to their caller instead.
/// Without a callback these diagnostics are unobserved; they do not enter unrelated
/// sessions' update queues. This callback is independent of native logging.
pub fn set_error_callback(callback: impl FnMut(Error) + Send + 'static) {
  ERROR_CALLBACK.lock().unwrap_or_else(PoisonError::into_inner).replace(Box::new(callback));
}

/// Removes the unsolicited-error callback.
pub fn clear_error_callback() {
  ERROR_CALLBACK.lock().unwrap_or_else(PoisonError::into_inner).take();
}

/// Changes native log verbosity for all sessions, before or during their lifetime.
///
/// Levels 0–5 enable progressively more output; levels up to 1024 enable additional
/// diagnostics. Changes apply to the installed callback without reinstalling it.
/// Returns an error if `TDLib` rejects the level or the response cannot be decoded.
pub fn set_log_level(level: i32) -> Result {
  execute(&fns::setLogVerbosityLevel { new_verbosity_level: level })?;
  Ok(())
}

pub(crate) fn report(error: impl Into<Error>) {
  if let Some(callback) = &mut *ERROR_CALLBACK.lock().unwrap_or_else(PoisonError::into_inner) {
    callback(error.into());
  }
}

unsafe extern "C" fn log_message(level: i32, message: *const c_char) {
  if let Some(callback) = &mut *LOG_CALLBACK.lock().unwrap_or_else(PoisonError::into_inner) {
    // SAFETY: TDLib supplies a non-null NUL-terminated message valid for this call.
    callback(level, unsafe { CStr::from_ptr(message) });
  }
}
