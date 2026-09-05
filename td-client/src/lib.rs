//! Typed asynchronous access to [`TDLib`](https://core.telegram.org/tdlib).
//!
//! This crate connects generated `td-types` requests to `TDLib`, receives ordered
//! updates, and tracks message sends and downloads. It is a native Telegram client
//! integration for bots and user accounts, not an HTTP Bot API wrapper.
//!
//! # Getting started
//!
//! The workspace crates are not published. Use local path dependencies on
//! `td-client` and `td-types`, plus Tokio with the features your application
//! needs. For example, from another directory beside this checkout:
//!
//! ```toml
//! [dependencies]
//! td-client = { path = "../td/td-client" }
//! td-types = { path = "../td/td-types" }
//! tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
//! ```
//!
//! The Rust API is generated from `td/td_api.tl` at build time. Fetch a matching
//! native library and schema with `./td/fetch` from the repository root before
//! building. The supplied fetch script uses Bash 4+, curl, jq, and tar; on Linux
//! it also uses readelf. It selects Linux glibc or macOS packages for the host
//! architecture. The workspace requires Rust 1.98 or newer. Keep the native
//! library and generated schema in sync; updating only one can cause decoding
//! failures or unsupported requests.
//!
//! ## Native linking and deployment
//!
//! The build helper searches the checkout's `td/` directory and dynamically
//! links `tdjson`. An external application must also make that shared library
//! discoverable at runtime. Cargo does not propagate a dependency's executable
//! runtime-path flags into every downstream binary.
//!
//! For local applications, add `td-sys` as a path build-dependency and call its
//! helper from `main` in your application's `build.rs`:
//!
//! ```no_run
//! td_sys::build::link();
//! ```
//!
//! On Linux/macOS the helper adds runtime search paths for the executable's
//! directory and the local native-library directory. Deployment still requires
//! shipping/installing the matching shared library (including the name expected
//! by the platform loader) and its native dependencies. A loader error is not an
//! authentication error; check library placement before debugging credentials.
//! The fetch script is host-oriented, not a cross-compilation setup. For other
//! targets, supply the appropriate native artifacts and platform linker/loader
//! configuration yourself.
//!
//! Obtain an API ID and hash from [Telegram](https://my.telegram.org/apps).
//! Bots additionally need a token from [BotFather](https://t.me/BotFather).
//! Keep credentials and session directories out of version control.
//!
//! # A request and a clean closure
//!
//! A [`Session`] owns one `TDLib` instance. Obtain a
//! [`Client`] for requests; keep the owner until closure.
//! A request's generated type determines its response type:
//!
//! ```no_run
//! use td_client::types::{enums::User, fns};
//! use td_client::{Session, parameters};
//! use td_client::Result;
//!
//! # async fn example(api_id: i32, api_hash: &str, token: &str) -> Result {
//! let mut session = Session::bot(parameters(api_id, api_hash, "session"), token).await?;
//! let result = session.client().send(&fns::getMe {}).await;
//! let close = session.close().await;
//!
//! // Attempt cleanup even when the application request fails.
//! let User::user(user) = result?;
//! close?;
//! println!("Signed in as {}", user.first_name);
//! # Ok(())
//! # }
//! ```
//!
//! Do not put a fallible application's entire body before `close().await?`
//! using unchecked early `?` returns: they can drop the owner without closing
//! `TDLib`. Save the application result, attempt close, then choose how to
//! report either or both errors. Dropping an unfinished constructor or close
//! future also abandons graceful cleanup.
//!
//! # Choosing an operation
//!
//! | Method | What its result means |
//! | --- | --- |
//! | [`Client::send`](client::Client::send) | The function's direct `TDLib` response |
//! | [`Client::track`](client::Client::track) | One normal send reached its terminal outcome |
//! | [`Client::track_all`](client::Client::track_all) | Ordered individual outcomes for a normal-send batch |
//! | [`Client::download`](client::Client::download) | `TDLib` finished the synchronous download request |
//! | [`execute`] | A synchronously executable function returned |
//!
//! Use direct requests for getters, edits, previews, and other API functions.
//! Only normal sends belong on tracked message methods; the [message] module
//! explains the distinction and cancellation races. The [transfer] module
//! describes measurements, download ranges, and callback requirements.
//!
//! # Requests, updates, and ownership
//!
//! Clients are cloneable and can be moved into independent tasks. They cannot
//! receive updates, close the session, or keep it operational after its owner
//! drops. Requests may run concurrently; response arrival is not submission
//! order. The owner alone consumes updates through
//! [`recv`](session::Session::recv) and authorization through
//! [`recv_auth`](session::Session::recv_auth).
//!
//! One process-wide native receiver routes all clients. It resolves requests and
//! tracked sends independently of application polling. Original application
//! updates remain ordered and unchanged; authorization updates use the separate
//! auth API. The queue is unbounded, so applications must drain it to avoid
//! accumulating memory. Dispatch slow work separately from the receive loop.
//!
//! Do not run this crate alongside another `TDLib` receiver implementation in the
//! same process. Do not call raw receive functions behind its back: the
//! native receive stream has one coordinated owner.
//!
//! # Generated API vocabulary
//!
//! `td-types::fns` contains requests, `td-types::types` concrete payloads,
//! and `td-types::enums` tagged unions. Names preserve `TDLib` spelling.
//! `td-types::traits::Function` associates each request with its return type.
//! Defaults make wide request construction convenient, but do not guarantee that
//! the resulting arguments are valid for `TDLib`.
//!
//! These crates are built locally; references to their items are written as code
//! rather than links to an assumed hosted documentation tree.
//!
//! # Errors and application policy
//!
//! [`error::Error`] distinguishes native errors, JSON failures, terminal message
//! failures, cancellation, and disconnection. Unsolicited diagnostics without a
//! request recipient go only to the optional [`on_error`] callback.
//! A malformed terminal update may leave a tracked send waiting indefinitely.
//!
//! The crate supplies no request deadlines, retries, scheduling, or rate-limit
//! policy. A timeout that drops a request future does not undo its native work;
//! see [`Client::send`](client::Client::send) and the tracked methods before
//! retrying an operation that might already have taken effect.
//!
//! `TDLib` handles its own network/protocol behavior. The wrapper does not promise
//! that every failure is retryable, that cancelling is server-atomic, or that a
//! successful progress callback means an operation is complete.

pub mod client;
pub mod error;
pub mod message;
pub mod runtime;
pub mod session;
pub mod transfer;

mod connection;

pub use td_types as types;

pub use crate::client::*;
pub use crate::error::*;
pub use crate::message::*;
pub use crate::runtime::*;
pub use crate::session::*;
pub use crate::transfer::*;
