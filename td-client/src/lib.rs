//! # `td-client`
//!
//! High-level, safe, and idiomatic Rust interface for TDLib to build Telegram bots, userbots, and client applications.
//!
//! ## Plan & Responsibilities
//!
//! - **Client Lifecycle & Safety**:
//!   - Safe RAII encapsulation around `td-sys` client handles.
//!   - Dedicated background worker / event loop thread for `td_json_client_receive`.
//! - **Async Request / Response Handling**:
//!   - Correlation between outbound requests and incoming responses using the `@extra` metadata field.
//!   - Strongly typed request execution leveraging `td_types::traits::Function` to ensure response types match requests at compile-time.
//! - **Event & Update Streaming**:
//!   - Broadcast channels / async streams for incoming Telegram updates (`Update`).
//!   - Flexible dispatchers, filters, and middleware handlers for bot command processing.
//! - **Authentication Helpers**:
//!   - Automated authentication state machine (handling phone code, bot token, password / 2FA, QR code authentication).
//! - **Robust Error Handling**:
//!   - Rich Rust error types mapping TDLib error codes and JSON serialization failures.
