# Repository guidance

## Development principles

- Start from demonstrated consumer needs. Remove unsupported premises before adding machinery, and add capabilities only when a real caller requires them.
- Choose the smallest coherent design, not merely the smallest patch. Track production line count while designing, record predecessor/draft deltas in rewrite RFCs, and make every added type, helper, state, channel, wrapper, and synchronization primitive earn its lines through correctness or clarity. Use line count as design feedback, never as a reason to obscure invariants or compress expressive control flow.
- Backward compatibility and public-interface stability are not goals. For every engineering task, explore the full solution space, especially designs that change the public interface; prefer the cleanest end state over preserving existing callers.
- Do not add compatibility layers, deprecated bridges, or internal complexity to retain an old API. Update affected callers, examples, and tests together.
- Make ownership and lifecycle visible in APIs. Use Rust ownership and borrowing to prevent invalid states instead of compensating with cloneable handles or public state machines.
- Prefer typed protocol operations and generated TDLib types over hand-built JSON. Model guaranteed wire fields as required fields rather than defensive `Option`s.
- Preserve ordering and errors. Never silently discard updates, serialization failures, TDLib errors, or lifecycle failures.
- Keep policy at the correct layer. The library reports errors; applications decide logging and operational policy. Do not invent generic overflow, retry, operation-deadline, or update-dropping behavior without an explicit requirement. Low-level transport tuning, such as TDLib's receive wait, is mechanism rather than application policy and should remain configurable when real deployments need it.
- Avoid speculative extensibility: no builders, adapters, handles, wrappers, traits, or dependencies for hypothetical consumers. Standard-library and already-used primitives come first.
- Treat efficiency as a first-class design requirement, not a cleanup pass. Evaluate memory layout, cache locality, allocations, copies, pointer indirection, traversal count, and asymptotic behavior while choosing the design.
- Prefer data-oriented design where applicable: flat contiguous storage, compact state, dense identifiers and offsets, and predictable linear passes over pointer-rich object graphs and scattered allocations.
- Actively look for specialized, niche algorithms and data structures that fit the domain better than the obvious general-purpose solution. Prefer compact representations and targeted algorithms when they make invariants sharper, reduce allocation or dependency cost, or improve asymptotic behavior; unfamiliarity alone is not a reason to reject them.
- Treat language and library freshness as a goal when it produces a concrete correctness, clarity, efficiency, or line-count benefit; novelty alone does not justify machinery.
- Keep shared dependency versions in the workspace manifest and enable only the features each crate uses. Track current releases aggressively, but do not add a dependency when the standard library provides a small direct solution.
- Keep reusable crates policy-free and dependency-light. Use explicit library error types; reserve `anyhow`, subscriber setup, and operational logging for binaries and application layers.

## Rust style

- Track the newest Rust compiler and edition available, including nightly when it unlocks a concrete worthwhile feature. Bump toolchain pins and crate `rust-version` values promptly; compatibility with older compilers is not a goal.
- Prefer the newest useful syntax, standard-library APIs, compiler capabilities, and lints. Migrate eagerly when new options become available.
- Follow `.rustfmt.toml`: two-space indentation, 160-column width, and maximized small-item formatting. Let `cargo fmt` decide layout.
- Keep the workspace's strict Clippy configuration clean. Fix the code first; when an exception is intrinsic to an external signature, generated code, or benchmark, use a narrow `#[expect(..., reason = "...")]`, never a broad `allow`.
- Favor pattern-driven control flow: destructuring, slice and range patterns, `@` bindings, or-patterns, irrefutable `let` patterns, `let ... else`, let-chains, match guards, `matches!`/`assert_matches!`, `?`, and early returns. Prefer compact pattern composition such as `let ([foo @ .., _] | foo) = foo;` over a one-use helper that only reshapes or unwraps a value. Do not flatten an expressive structural match into combinators merely to reduce line count; prefer the form that makes the accepted shapes clearest.
- Keep methods small and at one level of abstraction. When an operation has multiple stages, let its entry method orchestrate clearly named private steps; extract even a one-use helper when it isolates a real protocol or lifecycle responsibility. Do not split straight-line mechanics into helpers that merely move lines or force readers to jump around.
- Generally avoid direct indexing and index-driven loops. Prefer pattern matching, slice destructuring, iterators, `get`/`get_mut`, `split_first`/`split_last`, `windows`, or `chunks`; index only when the bounds proof is local and obvious or the algorithm genuinely requires indices.
- Prefer native async language features: async functions and closures, return-position `impl Future` in traits, and borrowed futures. Avoid `async_trait`, boxed futures, and erased dynamic dispatch unless a concrete heterogeneous use case requires them.
- Derive `Default` and use struct-update syntax for wide generated types. Prefer direct enum/struct destructuring over accessor boilerplate.
- Initialize related default-valued locals together with tuple destructuring, such as `let (pending, queued, closed) = Default::default();`, instead of repeating individual default constructors.
- Borrow source data through parsing and transformation where lifetimes stay simple. Accept slices, deserialize from bytes, reuse caller-provided buffers for hot paths, and avoid intermediate `String`s or collections unless ownership is required.
- Formatting is allocation-free by default. Compose output with `format_args!`, `write!`, `fmt::Display`, and `fmt::from_fn`; do not use `format!` or `.to_string()` merely to pass formatted text onward. If a callee forces an allocation, first consider changing its interface to accept `fmt::Arguments`, `impl Display`, or a `fmt::Write` destination. Allocate only when an owned, stored string is the actual required result.
- Choose collections for the actual access pattern: `HashMap` for dynamic keyed routing, `VecDeque` for ordered buffering, and sorted/deduplicated `Vec`s plus binary search for compact build-once/read-many tables. Use unstable sorting when equal-item order is irrelevant.
- Keep visibility narrow and public APIs small. Give public types and operations concise, conventional names with natural pairs such as noun/verb and `send`/`recv`; do not expose transport identifiers or internal synchronization concepts as application API.
- Give private types, fields, and methods descriptive role names. Prefer names that make their one responsibility apparent at the call site over comments that compensate for vague words such as `handle`, `process`, `state`, or `data`.
- Put a precise `// SAFETY:` comment immediately before every `unsafe` block. State the actual FFI invariant, such as pointer validity, NUL termination, object lifetime, or sole-caller ownership.
- At FFI boundaries, prefer C string literals for static inputs and explicitly NUL-terminated byte buffers for dynamic serialized inputs. Stay in bytes until text validation is actually needed.
- Comments should explain contracts, ordering, or non-obvious tradeoffs, not restate code or substitute for precise private names.
- Preserve upstream generated TDLib naming even when its lowercase type and variant names are not idiomatic Rust.

## Parsing and code generation

- Treat schemas and generators as the source of generated Rust. Fix parser or generator logic rather than patching generated output.
- Keep parsing zero-copy where practical: AST nodes borrow from the input and allocate only structural containers or recursive indirection.
- Keep generation deterministic. Sort and deduplicate unstable input order, snapshot small exact fixtures, and syntax-parse large generated output to validate it structurally.
- Escape Rust keywords with raw identifiers and add `Box` only where recursive layout analysis proves indirection is required.

## `td-client` invariants

- Each live TDLib client has one owning, non-`Clone` `Client`. Ordinary concurrent requests borrow `&Client`; detached tasks use a non-owning, request-only `Sender`; ordered update/auth consumption borrows `&mut Client`; graceful shutdown consumes `Client`.
- `Sender` stores `Weak<ClientState>`, is not a lifecycle owner, and cannot receive updates or initiate shutdown. Dropping `Client` revokes new detached requests even if `Sender` values remain.
- Keep the public vocabulary compact and paired. V5 currently uses `Client`, `Sender`, `sender`, `send`, `recv`, `recv_auth`, `shutdown`, `parameters`, `set_log_level`, and `set_receive_timeout`; replace names outright if a later redesign finds a cleaner vocabulary rather than adding aliases or compatibility names.
- Exactly one process-wide receiver thread may call `td_receive`. Route by required `@client_id`, correlate requests by `@extra`, keep one pending map and one ordered event queue per client, and store router entries as `Weak<ClientState>` so routing never extends client lifetime.
- Match synchronization primitives to their semantics: short non-async critical sections use `std::sync::Mutex`, request replies use `oneshot`, ordered events use `mpsc`, and state transitions use `watch`. Never hold a synchronous lock across `.await`.
- Authentication helpers may buffer non-auth updates, but `recv()` must later return them in order. Authorization transitions do not leak through the application-update API.
- Graceful shutdown is explicit and fallible: send the generated `fns::close {}` through the correlated request path, propagate its response, observe `authorizationStateClosed`, then wait for the receiver's safe idle/ownership transition.
- `Drop` performs no native TDLib work and never blocks, sleeps, joins, or claims successful shutdown. Dropping without `shutdown()` is misuse.
- Construction and bot-authentication failures attempt graceful shutdown before returning the original failure.
- Keep TDLib receive-timeout tuning process-wide like `td_receive`, accept `Duration` rather than unchecked floating-point seconds, and retain a sensible default. The setting controls the next native receive wait; it is not a request timeout, retry policy, or license to drop updates.
- Do not add `execute_sync` while the receiver is active without first proving safe process-wide synchronization around TDLib's shared response buffer.
- Keep the event queue unbounded unless the application supplies an explicit backpressure or spill policy; a synchronous native receiver cannot await capacity, and silently dropping ordered updates is incorrect.

## Verification

- Run `cargo fmt --all --check`, `cargo check --workspace`, `cargo test --workspace`, and `cargo clippy --workspace --all-targets` for repository-wide changes.
- Prefer one focused integration test that proves a complete boundary over a framework of shallow unit tests. For lifecycle/concurrency changes, cover relevant failure paths, ordering, concurrent request correlation, cross-client routing, already-closed and simultaneous shutdown, shutdown/registration races, and worker restart.
- Test protocol and serialization code with exact wire representations, invalid inputs, round trips, and pattern assertions over structured values. Bind calls, awaits, and other nontrivial expressions before passing the result to `assert_matches!`; keep the macro focused on the value and expected pattern. Prefer `assert_matches!(value, pattern)` (with a guard when useful) over `assert!(matches!(...))`; use `assert_eq!` when full equality is the contract. Use real FFI integration tests where practical instead of mocks of the boundary being tested.
- `unwrap` and `expect` are acceptable in tests for conditions that make the test meaningless, with messages at non-obvious failure points. Production fallible boundaries return errors and use `?`.
- Keep microbenchmarks as ignored release-mode tests using `black_box` until a dedicated benchmark harness provides clear value.
- Put a deadline around concurrency/lifecycle tests so deadlocks fail visibly. Repeat fresh-process tests when validating native teardown or process-exit behavior.
