# td

A modular, type-safe Rust toolkit for building Telegram clients, bots, and automation tools on top of [TDLib](https://core.telegram.org/tdlib).

TDLib is Telegram's official library providing full access to the Telegram MTProto protocol—supporting user accounts, bots, secret chats, local database caching, and real-time event updates. This workspace bridges TDLib's native JSON interface into idiomatic Rust, providing a complete pipeline from Type Language (`td_api.tl`) parsing and strictly-typed Serde code generation to low-level C FFI bindings and an ergonomic async client runtime.

## Architecture

![Architecture](assets/architecture.svg)

## Status

- [x] **[`td-parser`](td-parser)**: Parses TDLib's TL schema (`td_api.tl`) into an AST.
  - Supports combinators, constructors, types, documentation comments, and parameter annotations.
  - Handles TDLib-specific TL syntax (vector types, boxed types, and built-in primitives).
- [x] **[`td-codegen`](td-codegen)**: Codegen engine translating parsed TL AST into idiomatic Rust.
  - Generates strongly-typed structs, tagged enums, doc comments, and default implementations.
  - Emits custom Serde derives for TDLib's JSON wire format (`@type` tags, base64 bytes, 64-bit int string conversions, and boxed recursion).
- [x] **[`td-types`](td-types)**: Generated Rust API definitions for TDLib.
  - Complete, strongly-typed models for all TDLib objects, updates, and functions.
  - `traits::Function` associating each request with its compile-time return type (`type Return = ...`).
- [x] **[`td-sys`](td-sys)**: Minimal, low-level C FFI bindings to `libtdjson`.
  - Modern multi-client ID interface (`td_create_client_id`, `td_send`, `td_receive`, `td_execute`) and legacy pointer interface.
  - Global logging configuration, callback hooks, and build script with automatic `$ORIGIN` / `@loader_path` rpath linkage.
- [x] **[`td-client`](td-client)**: Safe, async client runtime for TDLib.
  - Multiple independently owned clients sharing one process-wide receiver thread.
  - Typed concurrent requests, ordered updates, bot authentication, and graceful shutdown.
- [x] **[`td-app`](td-app)**: Example Telegram bot showcasing `td-client` and `td-types`.
  - Demonstrates bot authentication, handling incoming updates, dispatching commands, and handling inline queries.

## Quick Look

```rust
use std::{fmt, time::Duration};

use td_client::{Client, defaults};
use td_types::enums::{MessageContent, MessageSender, Update, User};
use td_types::{fns, types};

#[tokio::main]
async fn main() -> td_client::Result {
  let api_id = 123456789;
  let api_hash = "abcdefghijklmnopqrstuvwxyz".into();
  let bot_token = "123456789:abcdefghijklmnopqrstuvwxyz";

  tracing_subscriber::fmt().without_time().init();

  td_client::set_log_level(1);
  td_client::set_receive_timeout(Duration::from_millis(100));

  let params = fns::setTdlibParameters { api_id, api_hash, ..defaults() };
  let mut client = Client::bot(params, bot_token).await?;

  if let Err(err) = run(&mut client).await {
    tracing::error!(%err, "failed to run");
  }
  if let Err(err) = client.shutdown().await {
    tracing::error!(%err, "failed to shut down");
  }
  Ok(())
}

async fn run(client: &mut Client) -> td_client::Result {
  let User::user(me) = client.send(&fns::getMe {}).await?;
  let types::user { usernames, first_name, id, .. } = me;
  let me = usernames.iter().find_map(|u| u.active_usernames.first()).map_or("…", |u| u);
  tracing::info!(as=%first_name, "@"=%me, id, "signed in");

  while let Some(update) = tokio::select! {
    r = client.recv() => r?,
    _ = tokio::signal::ctrl_c() => None
  } {
    match update {
      Update::updateNewMessage(upd) if !upd.message.is_outgoing => {
        let types::message { id, chat_id, sender_id, content, .. } = upd.message;
        let content = display(&content);
        let sender_id = match sender_id {
          MessageSender::messageSenderChat(s) => s.chat_id,
          MessageSender::messageSenderUser(s) => s.user_id,
        };
        tracing::info!(sender_id, chat_id, id, %content, "new message");
      }
      _ => {}
    }
  }

  Ok(())
}

fn display(content: &MessageContent) -> impl fmt::Display {
  fmt::from_fn(move |f| match &content {
    MessageContent::messageText(m) =>
      write!(f, "{}", m.text.text),
    MessageContent::messageSticker(m) =>
      write!(f, "<sticker {}>", m.sticker.sticker.remote.id),
    MessageContent::messageAnimation(m) =>
      write!(f, "<gif {}>", m.animation.animation.remote.id),
    MessageContent::messageAudio(m) =>
      write!(f, "<audio {}>", m.audio.audio.remote.id),
    MessageContent::messageDocument(m) =>
      write!(f, "<file {}>", m.document.document.remote.id),
    MessageContent::messagePhoto(m) =>
      write!(f, "<photo {}>", m.photo.sizes.last().map_or("", |s| &s.photo.remote.id)),
    MessageContent::messageVideo(m) =>
      write!(f, "<video {}>", m.video.video.remote.id),
    _ =>
      write!(f, "{content:?}"),
  })
}
```

## Development

### Prerequisites

- **Rust Toolchain**: Rust 2024 edition compatible compiler (e.g. latest stable or nightly).
- **External Tools**: `curl` and `jq` (required by [`td/fetch`](td/fetch) to download upstream schemas and binary releases).

### Setup & Workflow

Upstream artifacts (`td_api.tl`, `libtdjson`) are not committed to git and must be fetched locally via [`td/fetch`](td/fetch):

```bash
td/fetch                                # fetch upstream schema and prebuilt binaries

cargo check --workspace                 # check compilation across all workspace crates
cargo test --workspace                  # run all unit, integration, and roundtrip tests
cargo clippy --workspace --all-targets  # run linter across all targets
cargo fmt --all                         # format codebase according to formatting rules
```

### Code Generation Pipeline

[`td-types`](td-types) compiles the TL schema into Rust definitions in stages:

1. **Schema**: `td_api.tl` provides the upstream definition.
2. **Parse**: [`td-parser`](td-parser) transforms TL syntax into an AST.
3. **Codegen**: [`td-codegen`](td-codegen) handles dependency graphs, recursive type boxing, and Serde derives.
4. **Build**: [`td-types`](td-types) runs the generator in `build.rs` during compilation.

To emit a standalone reference file (`td/td_api.rs`) for inspection:

```bash
cargo test -p td-codegen --test codegen upstream
```
