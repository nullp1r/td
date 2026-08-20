//! # `td-sys`
//!
//! Low-level FFI bindings to Telegram's `libtdjson`.

use core::ffi::{c_char, c_double, c_int, c_void};

/// Callback invoked when `TDLib` emits a log message.
pub type TdLogMessageCallback = unsafe extern "C" fn(verbosity_level: c_int, message: *const c_char);

/// Callback invoked when `TDLib` encounters a fatal, unrecoverable error.
pub type TdLogFatalErrorCallback = unsafe extern "C" fn(error_message: *const c_char);

unsafe extern "C" {
  // --- Modern Client-ID JSON Interface ---

  /// Creates a new client instance and returns its non-zero integer identifier.
  pub fn td_create_client_id() -> c_int;

  /// Sends an asynchronous JSON-encoded request to the client with the given ID.
  pub fn td_send(client_id: c_int, request: *const c_char);

  /// Receives incoming JSON-encoded updates and request responses from any client instance.
  ///
  /// Blocks for up to `timeout` seconds waiting for events. Returns `null` if no event arrives.
  pub fn td_receive(timeout: c_double) -> *const c_char;

  /// Synchronously executes a stateless `TDLib` request and returns a JSON-encoded response.
  ///
  /// Can be called from any thread without creating a client instance.
  pub fn td_execute(request: *const c_char) -> *const c_char;

  // --- Legacy Pointer-Based JSON Interface ---

  /// Creates a new opaque `TDLib` JSON client instance.
  pub fn td_json_client_create() -> *mut c_void;

  /// Sends an asynchronous JSON-encoded request to a specific `TDLib` client instance.
  pub fn td_json_client_send(client: *mut c_void, request: *const c_char);

  /// Receives incoming JSON-encoded updates and request responses for a specific client instance.
  ///
  /// Blocks for up to `timeout` seconds waiting for events. Returns `null` if no event arrives.
  pub fn td_json_client_receive(client: *mut c_void, timeout: c_double) -> *const c_char;

  /// Synchronously executes a stateless request on a specific `TDLib` client instance.
  pub fn td_json_client_execute(client: *mut c_void, request: *const c_char) -> *const c_char;

  /// Destroys a `TDLib` client instance and releases all associated resources.
  pub fn td_json_client_destroy(client: *mut c_void);

  // --- Logging & Global Configuration ---

  /// Sets the path to the file where `TDLib` internal logs will be written.
  ///
  /// Returns `0` on failure, non-zero on success.
  pub fn td_set_log_file_path(file_path: *const c_char) -> c_int;

  /// Sets the maximum size of the file to where `TDLib` internal logs are written before rotation.
  pub fn td_set_log_max_file_size(max_file_size: i64);

  /// Sets the verbosity level of `TDLib` internal logs.
  pub fn td_set_log_verbosity_level(new_verbosity_level: c_int);

  /// Registers a custom callback for handling log messages up to `max_verbosity_level`.
  pub fn td_set_log_message_callback(max_verbosity_level: c_int, callback: Option<TdLogMessageCallback>);

  /// Registers a custom callback for handling fatal error messages.
  pub fn td_set_log_fatal_error_callback(callback: Option<TdLogFatalErrorCallback>);
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::ffi::CStr;
  use std::sync::Once;

  static SILENCE: Once = Once::new();

  fn silence_logs() {
    // SAFETY: Setting log verbosity level to 0 suppresses all TDLib log output.
    SILENCE.call_once(|| unsafe { td_set_log_verbosity_level(0) });
  }

  #[test]
  fn stateless_execute() {
    silence_logs();

    let req = cr#"{"@type":"getTextEntities","text":"@telegram https://t.me"}"#;
    // SAFETY: Passing a valid null-terminated C string to stateless td_execute.
    let res_ptr = unsafe { td_execute(req.as_ptr()) };
    assert!(!res_ptr.is_null(), "td_execute response should not be null");

    // SAFETY: res_ptr is guaranteed by TDLib to point to a valid null-terminated JSON C string.
    let res_str = unsafe { CStr::from_ptr(res_ptr) }.to_str().expect("valid utf-8 response");
    assert!(res_str.contains(r#""offset":0,"length":9,"type":{"@type":"textEntityTypeMention"}"#));
    assert!(res_str.contains(r#""offset":10,"length":12,"type":{"@type":"textEntityTypeUrl"}"#));
  }

  #[test]
  fn client_id_lifecycle() {
    silence_logs();

    // SAFETY: Creating a client ID allocates a new TDLib instance.
    let client_id = unsafe { td_create_client_id() };
    assert!(client_id > 0, "client ID should be positive");

    let req = cr#"{"@type":"getOption","name":"version"}"#;
    // SAFETY: Sending a valid request to an active client ID.
    unsafe { td_send(client_id, req.as_ptr()) }

    // SAFETY: td_receive blocks for up to 1s and returns null or a valid C string.
    let res_ptr = unsafe { td_receive(1.0) };
    if !res_ptr.is_null() {
      // SAFETY: res_ptr is a valid null-terminated C string when non-null.
      let res_str = unsafe { CStr::from_ptr(res_ptr) }.to_str().expect("valid utf-8 response");
      assert!(!res_str.is_empty());
    }
  }

  #[test]
  fn legacy_pointer_lifecycle() {
    silence_logs();

    // SAFETY: Creating an opaque JSON client instance.
    let client = unsafe { td_json_client_create() };
    assert!(!client.is_null(), "client pointer should not be null");

    // SAFETY: Destroying the previously created client pointer.
    unsafe { td_json_client_destroy(client) }
  }
}
