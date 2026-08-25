//! Raw C bindings for `TDLib`'s JSON interface.
//!
//! These bindings expose `TDLib` API objects as JSON. Object fields keep their
//! `TDLib` names, and `@type` identifies the object type unless context makes it
//! unambiguous. Booleans use JSON booleans; `int32`, `int53`, and `double` use
//! numbers; `int64` and strings use strings; bytes use base64 strings; and
//! arrays use JSON arrays.
//!
//! The main interface is asynchronous. A request may include `@extra`; its
//! response contains the same value. Every received object contains
//! `@client_id`, identifying its client.
//!
//! Create clients with [`td_create_client_id`], send requests with [`td_send`],
//! and receive results with [`td_receive`]. Only one thread may receive at a
//! time, and results must be applied in receive order. Some documented requests
//! may instead be run synchronously with [`td_execute`]. Clients are destroyed
//! after they close; close all clients before process exit.
//!
//! ```no_run
//! use td_sys::{td_create_client_id, td_receive, td_send};
//!
//! // SAFETY: This call has no pointer arguments.
//! let client_id = unsafe { td_create_client_id() };
//! let request = cr#"{"@type":"getOption","name":"version"}"#;
//! // SAFETY: `client_id` came from TDLib; `request` is static and NUL-terminated.
//! unsafe { td_send(client_id, request.as_ptr()) };
//!
//! loop {
//!   // SAFETY: This is the program's only receiver thread.
//!   let response = unsafe { td_receive(10.0) };
//!   if !response.is_null() {
//!     // Parse and apply the response before the next receive or execute call.
//!   }
//! }
//! ```
//!
//! The `td_json_client_*` functions provide the legacy pointer API, which uses
//! the same JSON representation and correlation scheme. `TDLib` plans to remove
//! this API in version 2.0.0. These are raw FFI bindings; callers must uphold
//! the pointer, callback, threading, and buffer-lifetime contracts documented
//! on each item.

use core::ffi::{c_char, c_double, c_int, c_void};

pub mod build;

/// Called when `TDLib` adds a message to its internal log.
///
/// `verbosity_level` ranges from -1 through 1024. At 0, `TDLib` crashes after the
/// callback returns. No `TDLib` function may be called from the callback.
/// `message` points to a NUL-terminated UTF-8 string valid for the call.
pub type TdLogMessageCallback = unsafe extern "C" fn(verbosity_level: c_int, message: *const c_char);

/// Called when `TDLib` encounters a fatal error.
///
/// No `TDLib` function may be called from the callback. `TDLib` crashes after it
/// returns. `error_message` points to a NUL-terminated error description.
pub type TdLogFatalErrorCallback = unsafe extern "C" fn(error_message: *const c_char);

unsafe extern "C" {
  // --- Modern Client-ID JSON Interface ---

  /// Returns an opaque identifier for a new `TDLib` instance.
  ///
  /// The instance sends no updates until its first request. `TDLib` destroys it
  /// automatically after it is closed. Close every instance before exit.
  pub fn td_create_client_id() -> c_int;

  /// Sends a request to a `TDLib` client. May be called from any thread.
  ///
  /// `client_id` must identify a `TDLib` client. `request` must point to a
  /// NUL-terminated JSON request valid for the call.
  pub fn td_send(client_id: c_int, request: *const c_char);

  /// Receives updates and request responses for any client.
  ///
  /// May be called from any thread after the first request is sent, but never
  /// concurrently from two threads. Results must be handled in receive order.
  ///
  /// Waits for at most `timeout` seconds and returns null on timeout. Otherwise,
  /// the returned NUL-terminated JSON string remains valid until the next
  /// `td_receive` or `td_execute` call.
  pub fn td_receive(timeout: c_double) -> *const c_char;

  /// Synchronously executes a `TDLib` request. May be called from any thread.
  ///
  /// Only requests documented as “Can be called synchronously” are supported.
  /// `request` must point to a NUL-terminated JSON request valid for the call.
  /// The returned NUL-terminated JSON string remains valid until the next
  /// `td_receive` or `td_execute` call.
  pub fn td_execute(request: *const c_char) -> *const c_char;

  // --- Legacy Pointer-Based JSON Interface (scheduled for removal in TDLib 2.0.0) ---

  /// Creates a `TDLib` instance through the legacy pointer API.
  pub fn td_json_client_create() -> *mut c_void;

  /// Sends a request through the legacy pointer API. May be called from any thread.
  ///
  /// `client` must point to a live `TDLib` instance. `request` must point to a
  /// NUL-terminated JSON request valid for the call.
  pub fn td_json_client_send(client: *mut c_void, request: *const c_char);

  /// Receives updates and request responses through the legacy pointer API.
  ///
  /// May be called from any thread, but never concurrently from two threads.
  /// Results must be handled in receive order.
  ///
  /// `client` must point to a live `TDLib` instance. The function waits for at
  /// most `timeout` seconds and returns null on timeout. Otherwise, the returned
  /// NUL-terminated JSON string remains valid until the next legacy receive or
  /// execute call on the same thread.
  pub fn td_json_client_receive(client: *mut c_void, timeout: c_double) -> *const c_char;

  /// Synchronously executes a request through the legacy pointer API.
  ///
  /// May be called from any thread, but only a few requests support synchronous
  /// execution. `client` is currently ignored and may be null. `request` must
  /// point to a NUL-terminated JSON request valid for the call. The returned
  /// string remains valid until the next legacy receive or execute call on the
  /// same thread.
  pub fn td_json_client_execute(client: *mut c_void, request: *const c_char) -> *const c_char;

  /// Destroys a legacy `TDLib` instance.
  ///
  /// `client` must point to a live instance and must not be used after this call.
  pub fn td_json_client_destroy(client: *mut c_void);

  // --- Logging & Global Configuration ---

  /// Redirects internal logs from the default stream to a file.
  ///
  /// `file_path` must point to a NUL-terminated path. An empty path restores the
  /// default stream. Returns 1 on success or 0 if the file cannot be opened.
  /// Deprecated upstream in favor of the synchronous `setLogStream` request.
  pub fn td_set_log_file_path(file_path: *const c_char) -> c_int;

  /// Sets the internal log file size at which `TDLib` rotates the file.
  ///
  /// The size must be positive. It defaults to 10 MB and is unused when logging
  /// to the default stream. Deprecated upstream in favor of `setLogStream`.
  pub fn td_set_log_max_file_size(max_file_size: i64);

  /// Sets the internal log verbosity level, which defaults to 5.
  ///
  /// Levels 0 through 5 select fatal, error, warning/debug-warning,
  /// informational, debug, and verbose-debug logging. Levels through 1024
  /// enable more logging. Deprecated upstream in favor of the synchronous
  /// `setLogVerbosityLevel` request.
  pub fn td_set_log_verbosity_level(new_verbosity_level: c_int);

  /// Sets the callback for messages up to `max_verbosity_level`.
  ///
  /// No `TDLib` function may be called from the callback. Pass `None` to remove it.
  pub fn td_set_log_message_callback(max_verbosity_level: c_int, callback: Option<TdLogMessageCallback>);

  /// Sets the callback for fatal errors. Pass `None` to remove it.
  ///
  /// No `TDLib` function may be called from the callback. `TDLib` crashes after the
  /// callback returns. Deprecated upstream in favor of
  /// `td_set_log_message_callback`.
  pub fn td_set_log_fatal_error_callback(callback: Option<TdLogFatalErrorCallback>);
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::ffi::CStr;
  use std::sync::{Mutex, Once};

  static CALLS: Mutex<()> = Mutex::new(());
  static SILENCE: Once = Once::new();

  fn silence_logs() {
    // SAFETY: The call passes no pointers or borrowed storage.
    SILENCE.call_once(|| unsafe { td_set_log_verbosity_level(0) });
  }

  #[test]
  fn stateless_execute() {
    silence_logs();
    let _calls = CALLS.lock().expect("lock poisoned");

    let req = cr#"{"@type":"getTextEntities","text":"@telegram https://t.me"}"#;
    // SAFETY: `req` is a static, NUL-terminated C string.
    let res_ptr = unsafe { td_execute(req.as_ptr()) };
    assert!(!res_ptr.is_null(), "td_execute response should not be null");

    // SAFETY: `res_ptr` is a non-null, NUL-terminated TDLib response.
    // `CALLS` keeps its buffer valid while borrowed.
    let res_str = unsafe { CStr::from_ptr(res_ptr) }.to_str().expect("valid utf-8 response");
    assert!(res_str.contains(r#""offset":0,"length":9,"type":{"@type":"textEntityTypeMention"}"#));
    assert!(res_str.contains(r#""offset":10,"length":12,"type":{"@type":"textEntityTypeUrl"}"#));
  }

  #[test]
  fn client_id_lifecycle() {
    silence_logs();

    // SAFETY: The call takes no arguments and returns an opaque ID by value.
    let client_id = unsafe { td_create_client_id() };
    assert!(client_id > 0, "client ID should be positive");

    let _calls = CALLS.lock().expect("lock poisoned");
    let req = cr#"{"@type":"getOption","name":"version"}"#;
    // SAFETY: `client_id` came from TDLib; `req` is static and NUL-terminated.
    unsafe { td_send(client_id, req.as_ptr()) }

    // SAFETY: `CALLS` serializes all `td_receive` and `td_execute` calls.
    let res_ptr = unsafe { td_receive(1.0) };
    if !res_ptr.is_null() {
      // SAFETY: `res_ptr` is a non-null, NUL-terminated TDLib response.
      // `CALLS` keeps its buffer valid while borrowed.
      let res_str = unsafe { CStr::from_ptr(res_ptr) }.to_str().expect("valid utf-8 response");
      assert!(!res_str.is_empty());
    }
  }

  #[test]
  fn legacy_pointer_lifecycle() {
    silence_logs();

    // SAFETY: The call takes no arguments; Rust treats the returned pointer as opaque.
    let client = unsafe { td_json_client_create() };
    assert!(!client.is_null(), "client pointer should not be null");

    // SAFETY: `client` is non-null, live, and not used after this call.
    unsafe { td_json_client_destroy(client) }
  }
}
