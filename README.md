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
  - Multi-client routing and async request correlation over background receiver thread.
  - Interactive authentication flows (bot token, user phone + SMS code, 2FA cloud password).
  - Built-in device presets (`DESKTOP`, `ANDROID`, `IOS`, `MACOS`, `WEB_Z`, etc.).
- [x] **[`td-app`](td-app)**: Example Telegram bot showcasing `td-client` and `td-types`.
  - Demonstrates bot authentication, handling incoming updates, dispatching commands, and handling inline queries.

## Quick Look

```rust
use std::error::Error;
use td_client::{Client, Config};
use td_types::{enums, fns, types};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let config = Config {
    td: fns::setTdlibParameters {
      api_id: 123456789,
      api_hash: "abcdefghijklmnopqrstuvwxyz".into(),
      ..Config::default().td
    },
    ..Config::default()
  };

  let (handle, mut updates) = Client::new(config)
    .auth_bot("123456789:abcdefghijklmnopqrstuvwxyz")
    .await?;

  let me = handle.execute(&fns::getMe {}).await?;
  let enums::User::user(types::user { first_name, username, id, .. }) = me;
  println!("Authenticated as @{username:?} ({first_name}, ID: {id})");

  while let Some(update) = updates.recv().await {
    if let enums::Update::updateNewMessage(types::updateNewMessage { message, .. }) = update {
      println!("New message received: ID {}", message.id);
    }
  }

  Ok(())
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
cargo test -p td-codegen -- upstream
```
