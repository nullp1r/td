# Code Quality Diagnostics Report: `game` Crate

> **Target Audience**: An LLM agent operating exclusively inside the `game/` folder without access to the Rust compiler (`rustc`/`cargo`) or the internet.  
> **Goal**: Fix all errors, clippy lints, formatting violations, and warnings so that `game` passes:
> 1. `cargo check --all-targets`
> 2. `cargo clippy --all-targets`
> 3. `cargo test`
> 4. `cargo fmt --all --check`
>
> **Repository Rules Reminder (`AGENTS.md`)**:
> - MSRV: 1.98+ / Rust 2024 edition.
> - Do not introduce speculative frameworks or compatibility shims.
> - Fix code first; when an exception is genuinely required, use narrow `#[expect(..., reason = "...")]`, never broad `allow`.
> - SQLite is authoritative; presentation is deferred until transaction commit.
> - Two-space indentation, max 160 columns (preferred 80–120 columns).

---

## Summary of Diagnostic Findings

| Category | Count | Status | Key Files |
| :--- | :--- | :--- | :--- |
| **Compiler Errors (`rustc`)** | 5 | **FAIL (Blocking)** | `src/telegram/present.rs`, `src/telegram/mod.rs` |
| **Clippy Deny Lints** | 3 | **FAIL (Blocking)** | `src/app.rs` |
| **Unfulfilled Lint Expectations** | 1 | **WARN** | `src/app.rs` |
| **Rustfmt Violations** | 5 files | **FAIL** | `src/app.rs`, `src/fishing.rs`, `src/telegram/mod.rs`, `src/telegram/present.rs`, `src/world.rs` |
| **Manifest Dependency Warnings** | 3 | **INFO** | `Cargo.toml` (only when built without `telegram` feature) |
| **Unit Tests** | 25 tests | **24 PASS / 1 BLOCKED** | 24 core tests pass; 1 Telegram test blocked by compilation errors |

---

## Phase 1: Fatal Compilation Errors (Blocking Build)

These 5 compiler errors halt compilation when building with the default feature (`--features telegram`).

### 1. Mismatched Array Dimensions in Inline Keyboard
- **File**: `game/src/telegram/present.rs`
- **Location**: Line 102–108 (inside `pub async fn struggle(...)`)
- **Error Code**: `E0308` (mismatched types)
- **Current Code**:
  ```rust
  request.reply_markup = Some(markup::inline([
    [
      markup::callback("Pull", Callback::Pull { encounter_id: struggle.encounter_id, step: struggle.step }.encode()),
      markup::callback("Give line", Callback::GiveLine { encounter_id: struggle.encounter_id, step: struggle.step }.encode()),
    ],
    [markup::callback("Cut line", Callback::CancelFishing.encode())],
  ]));
  ```
- **Root Cause**:
  In Rust, array literals `[row0, row1]` require every element to have the exact same type `[T; N]`.
  - `row0` is `[button, button]` $\rightarrow$ type `[inlineKeyboardButton; 2]`.
  - `row1` is `[button]` $\rightarrow$ type `[inlineKeyboardButton; 1]`.
  Because `[T; 2]` and `[T; 1]` are distinct types, the outer array `[row0, row1]` fails type checking.
- **Fix**:
  `tdx::markup::inline` accepts any `IntoIterator<Item = IntoIterator<Item = inlineKeyboardButton>>`.
  Throughout `present.rs` (e.g. lines 84, 129, 149), multi-row keyboards with differing row lengths use `vec![...]`.
  Change the outer and inner arrays to `vec!`:
  ```rust
  request.reply_markup = Some(markup::inline(vec![
    vec![
      markup::callback("Pull", Callback::Pull { encounter_id: struggle.encounter_id, step: struggle.step }.encode()),
      markup::callback("Give line", Callback::GiveLine { encounter_id: struggle.encounter_id, step: struggle.step }.encode()),
    ],
    vec![markup::callback("Cut line", Callback::CancelFishing.encode())],
  ]));
  ```

---

### 2. Missing `Option` Wrapper on `BotCommandScope` (Private Chats)
- **File**: `game/src/telegram/mod.rs`
- **Location**: Line 91–97 (inside `async fn register_commands(...)`)
- **Error Code**: `E0308` (mismatched types)
- **Current Code**:
  ```rust
  client
    .send(&fns::setCommands {
      scope: BotCommandScope::botCommandScopeAllPrivateChats,
      language_code: String::new(),
      commands: private,
    })
    .await?;
  ```
- **Root Cause**:
  In TDLib / `td-types`, the `scope` field of `fns::setCommands` is typed as `Option<enums::BotCommandScope>`, not bare `enums::BotCommandScope`.
- **Fix**:
  Wrap the enum variant in `Some(...)`:
  ```rust
  client
    .send(&fns::setCommands {
      scope: Some(BotCommandScope::botCommandScopeAllPrivateChats),
      language_code: String::new(),
      commands: private,
    })
    .await?;
  ```

---

### 3. Missing `Option` Wrapper on `BotCommandScope` (Group Chats)
- **File**: `game/src/telegram/mod.rs`
- **Location**: Line 100–106 (inside `async fn register_commands(...)`)
- **Error Code**: `E0308` (mismatched types)
- **Current Code**:
  ```rust
  client
    .send(&fns::setCommands {
      scope: BotCommandScope::botCommandScopeAllGroupChats,
      language_code: String::new(),
      commands: groups,
    })
    .await?;
  ```
- **Root Cause**:
  Same as issue #2: `fns::setCommands.scope` requires `Option<enums::BotCommandScope>`.
- **Fix**:
  Wrap in `Some(...)`:
  ```rust
  client
    .send(&fns::setCommands {
      scope: Some(BotCommandScope::botCommandScopeAllGroupChats),
      language_code: String::new(),
      commands: groups,
    })
    .await?;
  ```

---

### 4. Unstable Feature `str_as_str` on Command Name (Group Message Handler)
- **File**: `game/src/telegram/mod.rs`
- **Location**: Line 154–156 (inside `async fn on_message(...)`)
- **Error Code**: `E0658` (use of unstable library feature `str_as_str`, tracking issue #130366)
- **Current Code**:
  ```rust
  if message.chat_id < 0 {
    match command.name.as_str() {
      "start" | "game" | "help" | "fish" | "explore" => {
  ```
- **Root Cause**:
  `message.command()` returns `Option<tdx::command::Command<'a>>`. The `name` field of `Command<'a>` is already `&'a str`.
  Calling `.as_str()` on a value that is already a `&str` resolves to the experimental `str::as_str` method, which is unstable.
  Because `command.name` is already `&str`, calling `.as_str()` is completely redundant.
- **Fix**:
  Remove `.as_str()` and match `command.name` directly:
  ```rust
  if message.chat_id < 0 {
    match command.name {
      "start" | "game" | "help" | "fish" | "explore" => {
  ```

---

### 5. Unstable Feature `str_as_str` on Command Name (Private Message Handler)
- **File**: `game/src/telegram/mod.rs`
- **Location**: Line 165 (inside `async fn on_message(...)`)
- **Error Code**: `E0658` (use of unstable library feature `str_as_str`)
- **Current Code**:
  ```rust
  match command.name.as_str() {
    "start" | "home" | "fish" => {
  ```
- **Root Cause**:
  Same as issue #4. `command.name` is already `&str`.
- **Fix**:
  Remove `.as_str()`:
  ```rust
  match command.name {
    "start" | "home" | "fish" => {
  ```

---

## Phase 2: Clippy Deny Lints & Attributes (Blocking `cargo clippy`)

The workspace enables strict clippy pedantic/correctness checks, including `panic = "deny"`. The following issues cause `cargo clippy --all-targets` to fail:

### 6. `clippy::manual_is_multiple_of`
- **File**: `game/src/app.rs`
- **Location**: Line 1825
- **Lint**: `clippy::manual_is_multiple_of`
- **Current Code**:
  ```rust
  let special_id = if location_id == 2 && !has_rusted_key && (breakwater_catches >= 4 || seed % 8 == 0) {
  ```
- **Root Cause**:
  Checking divisibility using `% N == 0` is flagged by clippy in favor of the standard `.is_multiple_of(N)` integer method.
- **Fix**:
  Replace `seed % 8 == 0` with `seed.is_multiple_of(8)`:
  ```rust
  let special_id = if location_id == 2 && !has_rusted_key && (breakwater_catches >= 4 || seed.is_multiple_of(8)) {
  ```

---

### 7. `clippy::bool_to_int_with_if`
- **File**: `game/src/app.rs`
- **Location**: Line 2201
- **Lint**: `clippy::bool_to_int_with_if`
- **Current Code**:
  ```rust
  Ok(((if complete { 1 } else { 0 }), 1))
  ```
- **Root Cause**:
  Clippy flags using `if <bool> { 1 } else { 0 }` to convert a boolean to an integer.
- **Fix**:
  Use `u32::from(complete)`:
  ```rust
  Ok((u32::from(complete), 1))
  ```

---

### 8. `clippy::panic` in Test Target
- **File**: `game/src/app.rs`
- **Location**: Line 3049 (inside `async fn catches_wear_rods_and_shop_repairs_them()`)
- **Lint**: `clippy::panic` (configured as `deny` in `Cargo.toml`)
- **Current Code**:
  ```rust
  let TimerOutcome::Bite(bite) = outcome else { panic!("expected bite") };
  ```
- **Root Cause**:
  Because `panic = "deny"` is set workspace-wide, raw `panic!(...)` calls are denied even in test modules when checking `--all-targets`.
- **Fix**:
  Use `unreachable!("expected bite")` (which expresses the invariant without violating `clippy::panic`), or place `#[expect(clippy::panic, reason = "test setup invariant")]` immediately preceding the statement:
  ```rust
  let TimerOutcome::Bite(bite) = outcome else { unreachable!("expected bite") };
  ```

---

### 9. Unfulfilled Lint Expectation on `CastStarted::due_at_ms`
- **File**: `game/src/app.rs`
- **Location**: Line 443–444
- **Warning**: `this lint expectation is unfulfilled` (`#[warn(unfulfilled_lint_expectations)]`)
- **Current Code**:
  ```rust
  #[derive(Clone, Copy, Debug)]
  pub struct CastStarted {
    #[cfg_attr(not(test), expect(dead_code, reason = "used by deterministic timer tests"))]
    pub due_at_ms: i64,
  }
  ```
- **Root Cause**:
  `CastStarted` is a `pub` struct with a `pub` field in a library crate. In Rust, public fields of public items are visible to external callers, so `dead_code` is never emitted for `due_at_ms` in non-test builds. Thus, the `expect(dead_code)` attribute is unfulfilled.
- **Fix**:
  Remove the `#[cfg_attr(not(test), expect(dead_code, ...))]` attribute:
  ```rust
  #[derive(Clone, Copy, Debug)]
  pub struct CastStarted {
    pub due_at_ms: i64,
  }
  ```

---

## Phase 3: Rustfmt Formatting Violations

`cargo fmt --all --check` reports style diffs across 5 files:

1. `game/src/world.rs`
   - **Line 46–48**: Extra blank line between `environment_at` and `game_day`. Remove the blank line.
2. `game/src/telegram/present.rs`
   - **Line 687, 697, 703**: `rows.push(...)` callbacks should be formatted according to line-length rules (wrapping long `format!` calls cleanly).
   - **Line 781**: `format_args!` split across multiple lines.
   - **Line 808**: Table row cell additions collapsed into clean indentation.
   - **Line 824**: `let status = if ... else if ... else ...` multi-line layout.
   - **Line 916**: `group_markup` inline row formatting.
   - **Line 926**: `help_content` multi-line paragraphs.
   - **Line 982**: `format_weight_u64` single-line `if/else`.
3. `game/src/telegram/mod.rs`
   - **Line 19**: Imports spacing.
   - **Line 88**: Command list array indentation and line breaks.
   - **Line 216**: `match app.npc` match-arm formatting.
4. `game/src/fishing.rs`
   - **Lines 95 & 139**: Formatting of arithmetic expressions and match guards.
5. `game/src/app.rs`
   - Spacing, trailing commas, and multiline expressions across database query helpers.

> **Instruction for the LLM**: Ensure all files use 2-space indentation and lines stay comfortably within 80–120 columns (never exceeding 160). Avoid extraneous blank lines.

---

## Phase 4: Manifest Dependency Notes (`game/Cargo.toml`)

When building without default features (`cargo check --no-default-features`), cargo emits warnings for unused dependencies:
```
warning: unused dependency `anyhow`
warning: unused dependency `getrandom`
warning: unused dependency `tracing-subscriber`
```
- **Context**: These three crates are used in `src/telegram/mod.rs` and `src/main.rs`. Because `telegram` is an optional feature (`telegram = ["dep:tdx"]`) that is on by default, these crates are fully utilized during normal operation.
- **Recommendation**: If keeping `--no-default-features` completely free of manifest warnings is desired, mark them as optional dependencies tied to `telegram`:
  ```toml
  anyhow = { workspace = true, optional = true }
  getrandom = { workspace = true, optional = true }
  tracing-subscriber = { workspace = true, optional = true }
  ```
  And in `[features]`:
  ```toml
  telegram = ["dep:tdx", "dep:anyhow", "dep:getrandom", "dep:tracing-subscriber"]
  ```
  Otherwise, leaving them as-is is standard if `default = ["telegram"]` is always intended for this crate.

---

## Phase 5: Verification Checklist

Once the edits are applied, verify the following:

- [ ] `telegram/present.rs:107`: `vec![vec![Pull, GiveLine], vec![CutLine]]` used for ragged button matrix.
- [ ] `telegram/mod.rs:93`: `scope: Some(BotCommandScope::botCommandScopeAllPrivateChats)`.
- [ ] `telegram/mod.rs:102`: `scope: Some(BotCommandScope::botCommandScopeAllGroupChats)`.
- [ ] `telegram/mod.rs:155`: `match command.name { ... }`.
- [ ] `telegram/mod.rs:165`: `match command.name { ... }`.
- [ ] `app.rs:1825`: `seed.is_multiple_of(8)` used instead of `seed % 8 == 0`.
- [ ] `app.rs:2201`: `u32::from(complete)` used instead of `(if complete { 1 } else { 0 })`.
- [ ] `app.rs:3049`: `unreachable!("expected bite")` used instead of `panic!(...)`.
- [ ] `app.rs:443`: `#[cfg_attr(not(test), expect(dead_code, ...))]` removed.
- [ ] Formatting aligned to 2 spaces and max 120/160 columns without redundant blank lines.
- [ ] All 25 unit tests pass cleanly.
