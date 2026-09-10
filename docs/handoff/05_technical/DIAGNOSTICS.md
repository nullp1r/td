# Diagnostics and Handoff Gate

> **Status:** operational

Run from repository root. This captures environment, formatting, checks, tests, Clippy, feature configurations, and final Git state while continuing after failures.

```sh
(
set +e

run() {
    printf '\n\n================================================================\n'
    printf '>>> %s\n' "$*"
    printf '================================================================\n'
    "$@"
    status=$?
    printf '\n>>> EXIT STATUS: %d\n' "$status"
}

printf '=== ENVIRONMENT ===\n'
date -Is
uname -a
pwd

run git status --short --branch
run git rev-parse HEAD
run git diff --stat

run rustup show
run rustc -Vv
run cargo -V
run rustfmt -V
run cargo clippy -V

printf '\n\n=== CARGO METADATA ===\n'
run cargo metadata --format-version 1 --no-deps

printf '\n\n=== FORMAT ===\n'
run cargo fmt --all -- --check

printf '\n\n=== DEFAULT FEATURE BUILD ===\n'
run cargo check --workspace --all-targets

printf '\n\n=== DEFAULT FEATURE TESTS ===\n'
run cargo test --workspace

printf '\n\n=== DEFAULT FEATURE CLIPPY ===\n'
run cargo clippy --workspace --all-targets -- -D warnings

printf '\n\n=== ALL FEATURES BUILD ===\n'
run cargo check --workspace --all-targets --all-features

printf '\n\n=== ALL FEATURES TESTS ===\n'
run cargo test --workspace --all-features

printf '\n\n=== ALL FEATURES CLIPPY ===\n'
run cargo clippy --workspace --all-targets --all-features -- -D warnings

printf '\n\n=== GAME WITHOUT DEFAULT FEATURES ===\n'
run cargo check -p game --all-targets --no-default-features
run cargo test -p game --no-default-features
run cargo clippy -p game --all-targets --no-default-features -- -D warnings

printf '\n\n=== DEPENDENCY FEATURES ===\n'
run cargo tree -e features

printf '\n\n=== FINAL GIT STATUS ===\n'
run git status --short --branch

) 2>&1 | tee diagnostics.txt
```

## Why this matrix exists

- Records exact compiler/toolchain versions.
- A failing command does not hide later diagnostics.
- Tests default/all-feature/no-default-feature configurations.
- Runs strict Clippy with `-D warnings`.
- Captures dependency feature activation.
- Detects rustfmt differences.
- Final Git status reveals generated/modified files.

## Historical diagnostic environment

The real refactor diagnostic supplied during the session ran on `rustc 1.100.0-nightly` dated 2026-09-03.

After fixing early compiler errors, always run the full matrix again because later Clippy/type failures may have been masked.

Handoff identity is intentionally bidirectional: extracting a new handoff over an older checkout must not silently leave extra stale source/config/doc files that `verify-tree` would overlook. Mutable diagnostics, TDLib session state, and game SQLite runtime files are excluded from this identity check.
