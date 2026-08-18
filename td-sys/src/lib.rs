//! # `td-sys`
//!
//! Low-level unsafe FFI bindings to Telegram's `libtdjson`.
//!
//! ## Plan & Responsibilities
//!
//! - Provide raw C FFI function declarations and type definitions for TDLib's JSON interface:
//!   - `td_json_client_create`: Create a new TDLib JSON client instance.
//!   - `td_json_client_send`: Send an asynchronous request to TDLib.
//!   - `td_json_client_receive`: Fetch incoming responses and updates with a timeout.
//!   - `td_json_client_execute`: Synchronously execute a stateless TDLib request.
//!   - `td_json_client_destroy`: Clean up and destroy a TDLib client instance.
//!   - `td_set_log_message_callback`: Register a custom logging callback.
//!   - `td_set_log_verbosity_level`: Configure internal TDLib log verbosity.
//!   - `td_set_log_fatal_error_callback`: Register callback for unrecoverable errors.
//! - Support static and dynamic linking to `libtdjson` as well as runtime dynamic loading (e.g. via `libloading`).
//! - Keep this layer strictly minimal, zero-cost, and un-opinionated.
