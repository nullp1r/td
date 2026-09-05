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
//! # A request and a clean shutdown
//!
//! A [`Client`](client::Client) owns one `TDLib` instance. Obtain a
//! [`Sender`](client::Sender) for requests; keep the owner until shutdown.
//! A request's generated type determines its response type:
//!
//! ```no_run
//! use td_client::client::{Client, params};
//! use td_client::error::Result;
//! use td_types::{enums::User, fns};
//!
//! # async fn example(api_id: i32, api_hash: &str, token: &str) -> Result {
//! let client = Client::bot(params(api_id, api_hash, "session"), token).await?;
//! let result = client.sender().send(&fns::getMe {}).await;
//! let shutdown = client.shutdown().await;
//!
//! // Attempt cleanup even when the application request fails.
//! let User::user(user) = result?;
//! shutdown?;
//! println!("Signed in as {}", user.first_name);
//! # Ok(())
//! # }
//! ```
//!
//! Do not put a fallible application's entire body before `shutdown().await?`
//! using unchecked early `?` returns: they can drop the owner without closing
//! `TDLib`. Save the application result, attempt shutdown, then choose how to
//! report either or both errors. Dropping an unfinished constructor or shutdown
//! future also abandons graceful cleanup.
//!
//! # Choosing an operation
//!
//! | Method | What its result means |
//! | --- | --- |
//! | [`Sender::send`](client::Sender::send) | The function's direct `TDLib` response |
//! | [`Sender::send_message`](client::Sender::send_message) | One normal send reached its terminal outcome |
//! | [`Sender::send_messages`](client::Sender::send_messages) | Ordered individual outcomes for a normal-send batch |
//! | [`Sender::download`](client::Sender::download) | `TDLib` finished the synchronous download request |
//! | [`native::execute`] | A synchronously executable function returned |
//!
//! Use direct requests for getters, edits, previews, and other API functions.
//! Only normal sends belong on tracked message methods; the [message] module
//! explains the distinction and cancellation races. The [transfer] module
//! describes measurements, download ranges, and callback requirements.
//!
//! # Requests, updates, and ownership
//!
//! Senders are cloneable and can be moved into independent tasks. They cannot
//! receive updates, close the client, or keep it operational after its owner
//! drops. Requests may run concurrently; response arrival is not submission
//! order. The owner alone consumes updates through
//! [`recv`](client::Client::recv) and authorization through
//! [`recv_auth`](client::Client::recv_auth).
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
//! request recipient go only to the optional [`native::on_error`] callback.
//! A malformed terminal update may leave a tracked send waiting indefinitely.
//!
//! The crate supplies no request deadlines, retries, scheduling, or rate-limit
//! policy. A timeout that drops a request future does not undo its native work;
//! see [`Sender::send`](client::Sender::send) and the tracked methods before
//! retrying an operation that might already have taken effect.
//!
//! `TDLib` handles its own network/protocol behavior. The wrapper does not promise
//! that every failure is retryable, that cancelling is server-atomic, or that a
//! successful progress callback means an operation is complete.

pub mod client;
pub mod error;
pub mod message;
pub mod native;
pub mod transfer;

mod connection;
