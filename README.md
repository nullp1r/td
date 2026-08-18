# td

A modular, type-safe Rust toolkit for building Telegram clients, bots, and automation tools on top of [TDLib](https://core.telegram.org/tdlib).

TDLib is Telegram's official library providing full access to the Telegram MTProto protocol—supporting user accounts, bots, secret chats, local database caching, and real-time event updates. This workspace bridges TDLib's native JSON interface into idiomatic Rust, providing a complete pipeline from Type Language (`td_api.tl`) parsing and strictly-typed Serde code generation to low-level C FFI bindings and an ergonomic async client runtime.

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
use td_types::{enums, functions, traits::Function, types};

// 1. Serialize a typed request to TDLib JSON
let req = functions::getUser { user_id: 123456789 };
let req_json = serde_json::to_string(&req)?; // {"@type":"getUser","user_id":123456789}

// 2. Deserialize the response directly into the statically-associated return type
let res_json = r#"{"@type":"user","id":123456789,"first_name":"Alice","last_name":"Smith"}"#;
let res: <functions::getUser as Function>::Return = serde_json::from_str(res_json)?;
let enums::User::user(user) = res;
println!("User: {} {} (ID: {})", user.first_name, user.last_name, user.id);
```

## Development

```bash
# Check the workspace
cargo check --workspace

# Run all tests
cargo test --workspace

# Fetch latest upstream schema and prebuilt binaries
./td/fetch
```

## Architecture

![Architecture](assets/architecture.svg)
