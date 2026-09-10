#!/usr/bin/env bash
# Collects a complete compiler/static/runtime handoff report without stopping at the first failure.
# Usage: tools/collect-diagnostics.sh [output-file]

set -u

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
out=${1:-"$root/diagnostics.txt"}
mkdir -p "$(dirname "$out")"

failures=0
run_shell() {
  local label=$1
  shift
  printf '\n\n================================================================\n'
  printf '>>> %s\n' "$label"
  printf '================================================================\n'
  "$@"
  status=$?
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
  if [[ ${HANDOFF_CARGO_OFFLINE:-1} != 0 ]]; then
    export CARGO_NET_OFFLINE=true
  fi

  printf '=== ENVIRONMENT ===\n'
  date -Is
  uname -a
  printf 'root=%s\n' "$root"
  printf 'CARGO_NET_OFFLINE=%s\n' "${CARGO_NET_OFFLINE:-false}"

  if have git && [[ -d .git ]]; then
    run_shell 'git status --short --branch' git status --short --branch
    run_shell 'git rev-parse HEAD' git rev-parse HEAD
    run_shell 'git diff --stat' git diff --stat
    run_shell 'git diff --check' git diff --check
  else
    printf '\n=== GIT ===\nNo .git checkout available.\n'
  fi

  printf '\n=== REQUIRED TD SCHEMA SURFACE ===\n'
  for symbol in buttonStylePrimary buttonStyleDanger buttonStyleSuccess inlineButton inputPageBlockButtonRow inputMessageRichMessage sendEphemeralMessage editEphemeralMessage deleteEphemeralMessage dateTimeFormattingTypeRelative richTextDateTime textEntityTypeDateTime; do
    if grep -Eq "^${symbol}( | =|$)" td/td_api.tl; then
      printf 'OK   %s\n' "$symbol"
    else
      printf 'MISS %s\n' "$symbol"
      failures=$((failures + 1))
    fi
  done
  if grep -Eq '^botCommand .*is_ephemeral:Bool' td/td_api.tl; then
    printf 'OK   botCommand.is_ephemeral\n'
  else
    printf 'MISS botCommand.is_ephemeral\n'
    failures=$((failures + 1))
  fi

  printf '\n=== HANDOFF TREE IDENTITY ===\n'
  if [[ -f _handoff_manifest.json ]] && have python3; then
    run_shell 'python3 tools/handoff.py verify-tree' python3 tools/handoff.py verify-tree
  else
    printf 'SKIP tree manifest verification (manifest or python3 unavailable)\n'
  fi

  printf '\n=== SOURCE REGRESSION SCANS ===\n'
  if grep -RIn --exclude-dir=target -E '(definition\("game"|command\.name.*"game"|Use /game|group_hub|world_status)' game/src tdx/src; then
    printf 'Unexpected legacy group UX references found above.\n'
    failures=$((failures + 1))
  else
    printf 'OK   no legacy /game/group_hub/world_status references in active Rust sources\n'
  fi
  if grep -RIn -E '(MVP|prototype|this build|loaded game content|world telemetry|developer-facing|schema mutation)' game/src/telegram/present game/src/telegram/dispatch.rs; then
    printf 'Unexpected developer/prototype wording found in player-facing Telegram sources above.\n'
    failures=$((failures + 1))
  else
    printf 'OK   no known developer/prototype wording in player-facing Telegram sources\n'
  fi
  if grep -Fq 'command::ephemeral_definition("help"' game/src/telegram/mod.rs && grep -Fq 'send::ephemeral_reply' game/src/telegram/present/social.rs; then
    printf 'OK   group /help uses the ephemeral-command path\n'
  else
    printf 'MISS group /help ephemeral-command path\n'
    failures=$((failures + 1))
  fi
  if grep -Fq 'How it works' game/src/telegram/present/social.rs && grep -Fq 'Callback::GroupHelp' game/src/telegram/dispatch.rs; then
    printf 'OK   group help is discoverable from the shared Rich Message\n'
  else
    printf 'MISS group help in-message discovery path\n'
    failures=$((failures + 1))
  fi
  if grep -RIn --include='*.rs' -E '\b(plain|concat|empty)[[:space:]]*\(' tdx/src tdx/examples game/src; then
    printf 'Unexpected removed formatting helper references found above.\n'
    failures=$((failures + 1))
  else
    printf 'OK   removed plain/concat/empty formatting helpers stay absent\n'
  fi
  if grep -RIn --include='*.rs' -E 'table[[:space:]]*\([[:space:]]*\[' tdx/src tdx/examples game/src; then
    printf 'Unexpected old header-taking table constructor references found above.\n'
    failures=$((failures + 1))
  else
    printf 'OK   table() is headerless-by-default at active call sites\n'
  fi
  if grep -RIn --include='*.rs' -E 'std::ops::.*(Add|AddAssign)|impl[^[:cntrl:]]+(Add|AddAssign)' tdx/src/format; then
    printf 'Unexpected formatting composition operator implementations found above.\n'
    failures=$((failures + 1))
  else
    printf 'OK   formatting composition operators stay removed\n'
  fi
  if grep -Fq 'relative_time(format_duration(event.resets_in_ms), event.ends_at_unix)' game/src/telegram/present/social.rs; then
    printf 'OK   group shoal uses Telegram client-updated relative time\n'
  else
    printf 'MISS group shoal native relative-time path\n'
    failures=$((failures + 1))
  fi
  run_shell 'bash syntax: td/fetch' bash -n td/fetch
  run_shell 'bash syntax: tools/collect-diagnostics.sh' bash -n tools/collect-diagnostics.sh

  if [[ -f game/game.sqlite3 ]]; then
    sqlite_tmp=$(mktemp -d)
    cp game/game.sqlite3 "$sqlite_tmp/game.sqlite3"
    for suffix in -wal -shm; do
      if [[ -f "game/game.sqlite3${suffix}" ]]; then
        cp "game/game.sqlite3${suffix}" "$sqlite_tmp/game.sqlite3${suffix}"
      fi
    done
    if have python3; then
      run_shell 'SQLite quick_check + schema version (Python, temporary copy)' python3 -c '
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
      run_shell 'SQLite quick_check + schema version (temporary copy)' bash -c '
output=$(sqlite3 -readonly "$1" "PRAGMA quick_check; PRAGMA user_version;") || exit $?
printf "%s\n" "$output"
first=${output%%$'"'"'\n'"'"'*}
[[ $first == ok ]]
' _ "$sqlite_tmp/game.sqlite3"
    else
      printf '\n=== SQLITE ===\nGame DB exists, but neither sqlite3 nor python3 is available; skipped.\n'
    fi
    rm -rf "$sqlite_tmp"
  else
    printf '\n=== SQLITE ===\nGame DB absent; skipped.\n'
  fi

  printf '\n=== RUST TOOLCHAIN ===\n'
  for tool in rustup rustc cargo rustfmt; do
    if have "$tool"; then
      run_shell "$tool version" "$tool" --version
    else
      printf 'MISSING %s\n' "$tool"
      failures=$((failures + 1))
    fi
  done
  if have cargo; then
    run_shell 'cargo clippy -V' cargo clippy -V

    printf '\n=== CARGO METADATA ===\n'
    run_shell 'cargo metadata --format-version 1 --no-deps' cargo metadata --format-version 1 --no-deps

    printf '\n=== FORMAT ===\n'
    run_shell 'cargo fmt --all -- --check' cargo fmt --all -- --check

    printf '\n=== DEFAULT FEATURE BUILD ===\n'
    run_shell 'cargo check --workspace --all-targets' cargo check --workspace --all-targets

    printf '\n=== DEFAULT FEATURE TESTS ===\n'
    run_shell 'cargo test --workspace' cargo test --workspace

    printf '\n=== DEFAULT FEATURE CLIPPY ===\n'
    run_shell 'cargo clippy --workspace --all-targets -- -D warnings' cargo clippy --workspace --all-targets -- -D warnings

    printf '\n=== ALL FEATURES BUILD ===\n'
    run_shell 'cargo check --workspace --all-targets --all-features' cargo check --workspace --all-targets --all-features

    printf '\n=== ALL FEATURES TESTS ===\n'
    run_shell 'cargo test --workspace --all-features' cargo test --workspace --all-features

    printf '\n=== ALL FEATURES CLIPPY ===\n'
    run_shell 'cargo clippy --workspace --all-targets --all-features -- -D warnings' cargo clippy --workspace --all-targets --all-features -- -D warnings

    printf '\n=== GAME WITHOUT DEFAULT FEATURES ===\n'
    run_shell 'cargo check -p game --all-targets --no-default-features' cargo check -p game --all-targets --no-default-features
    run_shell 'cargo test -p game --no-default-features' cargo test -p game --no-default-features
    run_shell 'cargo clippy -p game --all-targets --no-default-features -- -D warnings -A dead-code' cargo clippy -p game --all-targets --no-default-features -- -D warnings -A dead-code

    printf '\n=== DEPENDENCY FEATURES ===\n'
    run_shell 'cargo tree -e features' cargo tree -e features
  fi

  if have git && [[ -d .git ]]; then
    printf '\n=== FINAL GIT STATUS ===\n'
    run_shell 'git status --short --branch' git status --short --branch
  fi

  printf '\n=== SUMMARY ===\n'
  printf 'Recorded failing/missing checks: %d\n' "$failures"
  printf 'Return this entire file with the next project handoff.\n'
) 2>&1 | tee "$out"

# tee runs in a pipeline/subshell, so use the report itself for a simple final signal.
if grep -Eq 'EXIT STATUS: [1-9]|^MISS |^MISSING |^Unexpected ' "$out"; then
  exit 1
fi
