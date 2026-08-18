# td

A modular, type-safe Rust toolkit for building Telegram clients, bots, and tools on top of [TDLib](https://core.telegram.org/tdlib).

The workspace provides the complete pipeline: parsing TDLib's Type Language (TL) schema, generating typed Rust representations, binding to the underlying C library, and exposing a high-level asynchronous client interface.

---

## Architecture Overview

![Architecture](assets/architecture.svg)

The stack is separated into distinct layers to allow using components independently (for example, utilizing generated types without the client runtime, or using code generation tools with custom schemas).

---

## Crates

### `td-parser`
Parses TDLib's TL schema (`td_api.tl`) into an Abstract Syntax Tree (AST).
- Parses combinators, constructors, types, documentation comments, and parameter annotations.
- Handles TDLib-specific TL syntax rules, including vector types, boxed types, and built-in primitives.

### `td-gen`
Translates parsed TL AST definitions into idiomatic Rust code.
- Generates typed structs, tagged enums, and doc comments.
- Emits Serde serialization and deserialization implementations matching TDLib's JSON interface conventions.
- Handles recursive types automatically using `Box<T>`.

### `td-types`
The resulting Rust API definitions generated from `td_api.tl`.
- Contains all strongly-typed TDLib objects, updates, and functions.
- Provides the `traits::Function` trait, connecting each request struct to its corresponding return type at compile time (`type Return = ...`).

### `td-sys` *(Stub / Planned)*
Minimal, low-level FFI bindings to `libtdjson`.
- Unsafe C function declarations for client creation, request dispatching, synchronous execution, and event polling (`td_json_client_*`).
- Supports static linking, dynamic linking, and runtime dynamic loading.

### `td-client` *(Stub / Planned)*
High-level, safe, and ergonomic async client for building applications and bots.
- RAII management of TDLib client lifecycles.
- Background worker thread for polling incoming updates and responses via `td_json_client_receive`.
- Request-response correlation using the `@extra` metadata field for asynchronous function calls.
- Event stream dispatching, filter pipelines, and authentication flow helpers (phone, bot token, password / 2FA, QR code).

---

## Assets and Utilities (`td/`)

The `td/` directory contains runtime assets and schema sources:
- `td_api.tl`: Upstream TDLib Type Language definition.
- `libtdjson.so`: Pre-built shared library for local development and testing.
- `fetch`: Script to retrieve the latest schema definitions and precompiled binaries from upstream TDLib releases.

---

## Project Status

- [x] **Schema Parser (`td-parser`)**: Full TL syntax parsing and AST generation.
- [x] **Code Generator (`td-gen`)**: AST-to-Rust code generation with recursive type resolution and Serde derive support.
- [x] **API Types (`td-types`)**: Generated Rust types and `Function` request-to-return-type bindings.
- [ ] **FFI Bindings (`td-sys`)**: Raw C FFI signatures and linking configurations.
- [ ] **Client Runtime (`td-client`)**: Async worker loop, `@extra` correlation, typed function execution, and update handlers.

---

## Building

Ensure you are using a recent stable Rust toolchain (edition 2024).

```bash
# Check the entire workspace
cargo check --workspace

# Run tests across all crates
cargo test --workspace
```
