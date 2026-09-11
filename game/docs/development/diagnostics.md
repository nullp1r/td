# Diagnostics and verification

`tools/collect-diagnostics.sh` is the project-wide reproducible verification collector. It captures enough environment, documentation, database, formatting, build, test and lint evidence to diagnose the current checkout in one report.

The default output is `diagnostics.txt` at the project root. That file is transient and ignored by Git.

## What it collects

The script should continue after individual failures so one run captures the useful matrix:

- environment/time/kernel/root path;
- Git status/revision/diff summary/diff whitespace check when `.git` exists;
- `tools/check-docs.py` integrity check;
- SQLite `quick_check` + schema version using a temporary copy when a development DB is present;
- Rust/Rustfmt/Clippy tool versions;
- Cargo metadata;
- `cargo fmt --all -- --check`;
- workspace default-feature check/test/strict Clippy;
- workspace all-features check/test/strict Clippy;
- `game` no-default-features check/test/strict Clippy;
- Cargo feature tree;
- final Git status.

Keep the collector generic: it should orchestrate project verification, not encode product-specific regression expectations. Focused behavior regressions belong in Rust tests, while durable project knowledge belongs in `game/docs/`.

## Offline behavior

Diagnostics default to `CARGO_NET_OFFLINE=true` so collecting a report does not unexpectedly fetch dependencies.

Override when intentional:

```sh
DIAGNOSTICS_CARGO_OFFLINE=0 ./tools/collect-diagnostics.sh
```

An explicitly supplied `CARGO_NET_OFFLINE` environment variable takes precedence.

## Normal use

From the workspace root:

```sh
./tools/collect-diagnostics.sh
```

Or choose an output path:

```sh
./tools/collect-diagnostics.sh /tmp/rustwater-diagnostics.txt
```

The script returns nonzero when a collected check fails or a required Rust tool is missing. Inspect/share the generated report when debugging, then discard it; it is not part of the durable docs.

## Minimum manual matrix

If the collector cannot be used:

```sh
python3 tools/check-docs.py
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check -p game --all-targets --no-default-features
cargo test -p game --no-default-features
cargo clippy -p game --all-targets --no-default-features -- -D warnings -A dead-code
```

The no-default-feature `dead_code` allowance is intentional because Telegram-facing consumers are conditionally absent in that build; do not widen it without a specific reason.

## SQLite safety

Diagnostics must never run integrity checks directly against a live DB/WAL set in a way that mutates it. Copy the DB plus WAL/SHM to a temporary directory, then open the copy read-only where possible.

Current schema version is 5.

## Documentation check

`tools/check-docs.py` verifies durable documentation structure rather than project-transfer history:

- required entry/canonical documents exist;
- every Markdown document starts with an H1 title;
- internal Markdown links resolve and stay inside the project tree;
- every document is linked directly from the package map in `game/docs/README.md`, preventing orphaned knowledge.

It is intentionally small and standard-library-only.
