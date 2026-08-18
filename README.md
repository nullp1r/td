# td

A modular, type-safe Rust toolkit for building Telegram clients, bots, and automation tools on top of [TDLib](https://core.telegram.org/tdlib).

TDLib is Telegram's official library providing full access to the Telegram MTProto protocol—supporting user accounts, bots, secret chats, local database caching, and real-time event updates. This workspace bridges TDLib's native JSON interface into idiomatic Rust, providing a complete pipeline from Type Language (`td_api.tl`) parsing and strictly-typed Serde code generation to low-level C FFI bindings and an ergonomic async client runtime.

## Architecture

![Architecture](assets/architecture.svg)

## Status

- [x] **[`td-parser`](td-parser)**: Parses TDLib's TL schema (`td_api.tl`) into an AST.
  - Supports combinators, constructors, types, documentation comments, and parameter annotations.
  - Handles TDLib-specific TL syntax (vector types, boxed types, and built-in primitives).
- [x] **[`td-gen`](td-gen)**: Codegen engine translating parsed TL AST into idiomatic Rust.
  - Generates strongly-typed structs, tagged enums, doc comments, and default implementations.
  - Emits custom Serde derives for TDLib's JSON wire format (`@type` tags, base64 bytes, 64-bit int string conversions, and boxed recursion).
- [x] **[`td-types`](td-types)**: Generated Rust API definitions for TDLib.
  - Complete, strongly-typed models for all TDLib objects, updates, and functions.
  - `traits::Function` associating each request with its compile-time return type (`type Return = ...`).
- [ ] **[`td-sys`](td-sys)**: Minimal, low-level C FFI bindings to `libtdjson` *(planned)*.
- [ ] **[`td-client`](td-client)**: High-level async client runtime and event dispatcher *(planned)*.

## Quick Look

```rust
use td_types::{enums, functions, traits, types};

fn api<F: traits::Function>(req: &F) -> Result<F::Return, enums::Error> {
  // some code to send `req` as JSON to TDLib and return the result as `F::Return` (or `Error`)
  // `F::Return` is statically associated with `F` via `traits::Function`
}

let user = api(&functions::getUser { user_id: 123456789 })?;
let enums::User::user(types::user { first_name, last_name, id, .. }) = user;
println!("User: {first_name} {last_name} (ID: {id})");
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
3. **Codegen**: [`td-gen`](td-gen) handles dependency graphs, recursive type boxing, and Serde derives.
4. **Build**: [`td-types`](td-types) runs the generator in `build.rs` during compilation.

To emit a standalone reference file (`td/td_api.rs`) for inspection:

```bash
cargo test -p td-gen -- full
```
