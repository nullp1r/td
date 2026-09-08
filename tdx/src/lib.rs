//! Helpers for building and inspecting Telegram messages with generated `TDLib` types.
//!
//! Constructors produce editable requests and payloads. They perform no network
//! operations: execute requests through [`Client`], while [`Session`] owns the
//! connection and receives updates. [`api`] and [`client`] re-export `td-types`
//! and `td-client`; [`types`], [`enums`] and [`fns`] expose the generated API.
//!
//! # Send and edit
//!
//! Import [`prelude`] for helper namespaces, formatting functions and extension
//! traits. Normal sends use [`Client::track`] to await delivery; edits use
//! [`Client::send`] because their direct response represents completion.
//!
//! ```no_run
//! use tdx::prelude::*;
//!
//! async fn greet(client: &Client, incoming: &types::message) -> tdx::client::Result<()> {
//!   let request = send::reply(incoming, line("Hello, ") + bold("world") + "!");
//!   let sent = client.track(&request, None, None).await?;
//!   client.send(&edit::text(&sent, "Welcome back.")).await?;
//!   Ok(())
//! }
//! ```
//!
//! [`Client::track_all`] returns individual delivery outcomes for albums and
//! forwards. Use [`Client::send`] for other generated operations. Chat IDs,
//! topic coordinates and message references identify destinations; [`target`]
//! documents the accepted shapes for sends, replies and edits.
//!
//! # Compose content
//!
//! [`mod@format`] supports ordinary entity text and structured rich messages.
//! [`compose`] supplies media, file, keyboard and reply constructors. Captions
//! use `Option<types::formattedText>`; converting a [`Text`](format::Text)
//! produces a present caption, while `None` omits it.
//!
//! ```
//! use tdx::prelude::*;
//!
//! let caption = bold("Lake report") + " — " + italic("three catches");
//! let photo = content::photo(file::local("lake.jpg"), [800, 600], caption.into());
//! let mut request = send::message(123, photo);
//! request.reply_markup = Some(markup::inline([
//!   [markup::callback("Refresh", b"refresh")],
//! ]));
//!
//! let report = rich([
//!   heading("Catch log", 1),
//!   table(["Species", "Weight"]).row(["Trout", "1.5 kg"]).into(),
//! ]);
//! let _request = send::message(123, report);
//! ```
//!
//! For mixed media albums, convert payloads into a common enum explicitly:
//! `send::album(chat_id, [photo.into(), video.into()])`. Generated payloads and
//! requests remain available for options and operations without a helper.
//!
//! # Inspect updates and manage sessions
//!
//! Traits in [`ext`] borrow text, captions, senders and other message fields.
//! [`command`] recognizes command entities in messages and slash-prefixed tokens
//! in strings, including optional bot-name matching.
//!
//! ```
//! use tdx::prelude::*;
//!
//! fn inspect(message: &types::message) {
//!   if let Some(command) = message.command() {
//!     println!("/{} {}", command.name, command.args);
//!   }
//!   if let Some(text) = message.text() {
//!     println!("{}", text.text);
//!   }
//! }
//! ```
//!
//! [`session::parameters`] prepares editable connection/storage settings and
//! [`session::bot`] performs bot authorization. Other flows use [`Session::open`]
//! and [`Session::recv_auth`]. Keep the session alive while using its clients,
//! consume updates, and call [`Session::close`] for fallible graceful shutdown.
//! [`client::diagnostics`] provides process-wide native logging and error hooks;
//! [`transfer`] constructs synchronous download requests for [`Client::download`].

pub use td_client as client;
pub use td_types as api;

pub use api::{enums, fns, types};
pub use client::{Client, Session};

pub mod command;
pub mod session;
pub mod target;

mod util;

/// Borrowed inspection traits and text conversions, also exported by [`prelude`].
pub mod ext {
  mod content;
  mod message;
  mod sender;
  mod user;

  pub use content::ContentExt;
  pub use message::MessageExt;
  pub use sender::MessageSenderExt;
  pub use user::UserExt;

  pub use super::command::CommandExt;
  pub use super::format::TextExt;
}

/// Entity text, inline styles, markup parsing and structured rich documents.
pub mod format;

/// Outgoing content, files, keyboards and reply references.
///
/// Constructors return generated values. File I/O happens when `TDLib` executes
/// the request, so callers can finish editing metadata before sending.
pub mod compose {
  pub mod content;
  pub mod file;
  pub mod markup;
  pub mod reply;
}

/// Request constructors for sending, editing, forwarding and deleting messages.
///
/// Execute the returned values with [`Client`]; request fields remain editable.
pub mod action {
  pub mod callback;
  pub mod delete;
  pub mod edit;
  pub mod forward;
  pub mod send;
}

/// Download requests configured for native completion tracking.
pub mod transfer;

/// Imports for `use tdx::prelude::*`: composition, actions, inspection and API types.
///
/// Session setup, diagnostics and error types are accessed through their modules.
pub mod prelude {
  pub use super::{Client, Session};
  pub use super::{enums, fns, types};

  pub use super::action::*;
  pub use super::command::*;
  pub use super::compose::*;
  pub use super::ext::*;
  pub use super::format::*;
  pub use super::target::*;
  pub use super::transfer;
}
