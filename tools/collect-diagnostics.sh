#!/usr/bin/env bash
# Collect reproducible project diagnostics without stopping at the first failure.
# Usage: tools/collect-diagnostics.sh [output-file]

set -u

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
out=${1:-"$root/diagnostics.txt"}
mkdir -p "$(dirname "$out")"

failures=0
run() {
  local label=$1
  shift
  printf '\n\n================================================================\n'
  printf '>>> %s\n' "$label"
  printf '================================================================\n'
  "$@"
  local status=$?
  printf '\n>>> EXIT STATUS: %d\n' "$status"
  if (( status != 0 )); then
    failures=$((failures + 1))
  fi
  return 0
}

have() { command -v "$1" >/dev/null 2>&1; }

(
  cd "$root" || exit 1
  export CARGO_TERM_COLOR=never
  if [[ -z ${CARGO_NET_OFFLINE+x} && ${DIAGNOSTICS_CARGO_OFFLINE:-1} != 0 ]]; then
    export CARGO_NET_OFFLINE=true
  fi

  printf '=== ENVIRONMENT ===\n'
  date -Is
  uname -a
  printf 'root=%s\n' "$root"
  printf 'CARGO_NET_OFFLINE=%s\n' "${CARGO_NET_OFFLINE:-false}"

  if have git && [[ -d .git ]]; then
    printf '\n=== GIT ===\n'
    run 'git status --short --branch' git status --short --branch
    run 'git rev-parse HEAD' git rev-parse HEAD
    run 'git diff --stat' git diff --stat
    run 'git diff --check' git diff --check
  else
    printf '\n=== GIT ===\nNo .git checkout available.\n'
  fi

  printf '\n=== DOCUMENTATION / SCRIPT HYGIENE ===\n'
  if have python3; then
    run 'python3 tools/check-docs.py' python3 tools/check-docs.py
  else
    printf 'MISSING python3 (documentation check unavailable)\n'
    failures=$((failures + 1))
  fi
  run 'bash syntax: td/fetch' bash -n td/fetch
  run 'bash syntax: tools/collect-diagnostics.sh' bash -n tools/collect-diagnostics.sh

  if [[ -f game/game.sqlite3 ]]; then
    sqlite_tmp=$(mktemp -d)
    cp game/game.sqlite3 "$sqlite_tmp/game.sqlite3"
    for suffix in -wal -shm; do
      [[ ! -f "game/game.sqlite3${suffix}" ]] || cp "game/game.sqlite3${suffix}" "$sqlite_tmp/game.sqlite3${suffix}"
    done
    if have python3; then
      run 'SQLite quick_check + schema version (temporary copy)' python3 -c '
import sqlite3, sys
con = sqlite3.connect("file:" + sys.argv[1] + "?mode=ro", uri=True)
check = str(con.execute("PRAGMA quick_check").fetchone()[0])
version = con.execute("PRAGMA user_version").fetchone()[0]
con.close()
print("quick_check=" + check)
print("user_version=" + str(version))
raise SystemExit(check != "ok")
' "$sqlite_tmp/game.sqlite3"
    elif have sqlite3; then
      run 'SQLite quick_check + schema version (temporary copy)' bash -c '
output=$(sqlite3 -readonly "$1" "PRAGMA quick_check; PRAGMA user_version;") || exit $?
printf "%s\n" "$output"
first=${output%%$'"'"'\n'"'"'*}
[[ $first == ok ]]
' _ "$sqlite_tmp/game.sqlite3"
    else
      printf '\n=== SQLITE ===\nGame DB exists, but neither python3 nor sqlite3 is available; skipped.\n'
    fi
    rm -rf "$sqlite_tmp"
  else
    printf '\n=== SQLITE ===\nGame DB absent; skipped.\n'
  fi

  printf '\n=== RUST TOOLCHAIN ===\n'
  for tool in rustup rustc cargo rustfmt; do
    if have "$tool"; then
      run "$tool version" "$tool" --version
    else
      printf 'MISSING %s\n' "$tool"
      failures=$((failures + 1))
    fi
  done

  if have cargo; then
    run 'cargo clippy -V' cargo clippy -V

    printf '\n=== CARGO METADATA ===\n'
    run 'cargo metadata --format-version 1 --no-deps' cargo metadata --format-version 1 --no-deps

    printf '\n=== FORMAT ===\n'
    run 'cargo fmt --all -- --check' cargo fmt --all -- --check

    printf '\n=== DEFAULT FEATURE BUILD ===\n'
    run 'cargo check --workspace --all-targets' cargo check --workspace --all-targets

    printf '\n=== DEFAULT FEATURE TESTS ===\n'
    run 'cargo test --workspace' cargo test --workspace

    printf '\n=== DEFAULT FEATURE CLIPPY ===\n'
    run 'cargo clippy --workspace --all-targets -- -D warnings' cargo clippy --workspace --all-targets -- -D warnings

    printf '\n=== ALL FEATURES BUILD ===\n'
    run 'cargo check --workspace --all-targets --all-features' cargo check --workspace --all-targets --all-features

    printf '\n=== ALL FEATURES TESTS ===\n'
    run 'cargo test --workspace --all-features' cargo test --workspace --all-features

    printf '\n=== ALL FEATURES CLIPPY ===\n'
    run 'cargo clippy --workspace --all-targets --all-features -- -D warnings' cargo clippy --workspace --all-targets --all-features -- -D warnings

    printf '\n=== GAME WITHOUT DEFAULT FEATURES ===\n'
    run 'cargo check -p game --all-targets --no-default-features' cargo check -p game --all-targets --no-default-features
    run 'cargo test -p game --no-default-features' cargo test -p game --no-default-features
    run 'cargo clippy -p game --all-targets --no-default-features -- -D warnings -A dead-code' cargo clippy -p game --all-targets --no-default-features -- -D warnings -A dead-code

    printf '\n=== DEPENDENCY FEATURES ===\n'
    run 'cargo tree -e features' cargo tree -e features
  fi

  if have git && [[ -d .git ]]; then
    printf '\n=== FINAL GIT STATUS ===\n'
    run 'git status --short --branch' git status --short --branch
  fi

  printf '\n=== SUMMARY ===\n'
  printf 'Recorded failing/missing checks: %d\n' "$failures"
) 2>&1 | tee "$out"

if grep -Eq 'EXIT STATUS: [1-9]|^MISSING ' "$out"; then
  exit 1
fi
