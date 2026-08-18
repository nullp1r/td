# td

A modular, type-safe Rust toolkit for building Telegram clients, bots, and automation tools on top of [TDLib](https://core.telegram.org/tdlib).

TDLib is Telegram's official library providing full access to the Telegram MTProto protocol—supporting user accounts, bots, secret chats, local database caching, and real-time event updates. This workspace bridges TDLib's native JSON interface into idiomatic Rust, providing a complete pipeline from Type Language (`td_api.tl`) parsing and strictly-typed Serde code generation to low-level C FFI bindings and an ergonomic async client runtime.

## Architecture

![Architecture](assets/architecture.svg)

## Status

- [x] **`td-parser`**: Parses TDLib's TL schema (`td_api.tl`) into an AST.
  - Supports combinators, constructors, types, documentation comments, and parameter annotations.
  - Handles TDLib-specific TL syntax (vector types, boxed types, and built-in primitives).
- [x] **`td-gen`**: Codegen engine translating parsed TL AST into idiomatic Rust.
  - Generates strongly-typed structs, tagged enums, doc comments, and default implementations.
  - Emits custom Serde derives for TDLib's JSON wire format (`@type` tags, base64 bytes, 64-bit int string conversions, and boxed recursion).
- [x] **`td-types`**: Generated Rust API definitions for TDLib.
  - Complete, strongly-typed models for all TDLib objects, updates, and functions.
  - `traits::Function` associating each request with its compile-time return type (`type Return = ...`).
- [ ] **`td-sys`**: Minimal, low-level C FFI bindings to `libtdjson` *(planned)*.
- [ ] **`td-client`**: High-level async client runtime and event dispatcher *(planned)*.

## Quick Look

```rust
use td_types::{enums, functions, traits, types};

fn api<F: traits::Function>(req: &F) -> Result<F::Return, enums::Error> {
  // some code to send `req` as JSON to TDLib and parse the result into `F::Return` (or `Error`)
  // `F::Return` is statically associated with `F` via `traits::Function`
}

let user = api(&functions::getUser { user_id: 123456789 })?;
let enums::User::user(types::user { first_name, last_name, id, .. }) = user;
println!("User: {first_name} {last_name} (ID: {id})");
```

## Development

### Prerequisites

- **Rust Toolchain**: Rust 2024 edition compatible compiler (e.g. latest stable or nightly).
- **External Tools**: `curl` and `jq` (required by `td/fetch` to download upstream schemas and binary releases).

### Setup & Workflow

The upstream schema (`td/td_api.tl`) and native libraries (`td/libtdjson.*`) are gitignored and downloaded locally via `td/fetch`:

```bash
td/fetch                                # fetch upstream schema and prebuilt binaries

cargo check --workspace                 # check compilation across all workspace crates
cargo test --workspace                  # run all unit, integration, and roundtrip tests
cargo clippy --workspace --all-targets  # run linter across all targets
cargo fmt --all                         # format codebase according to formatting rules
```

### Code Generation Pipeline

The strongly-typed definitions in `td-types` are generated directly from the TDLib schema:

1. **Schema Source**: `td/fetch` downloads `td_api.tl` into `td/`.
2. **Parsing & AST**: [`td-parser`](td-parser) parses the TL grammar into an abstract syntax tree.
3. **Rust Codegen**: [`td-gen`](td-gen) analyzes type dependencies, calculates strongly connected components for recursive type boxing, and emits Serde-annotated Rust models.
4. **Compile-Time Build**: [`td-types`](td-types) runs this pipeline in its `build.rs` to generate the complete API surface directly into `OUT_DIR`.

To generate and inspect the standalone reference file (`td/td_api.rs`, also gitignored) for exploration or debugging, run the `td-gen` integration test:

```bash
cargo test -p td-gen full
```
