# Project handoff loop

These helpers require only Bash and Python for packaging. Rust diagnostics run when a Rust toolchain is present and are recorded as missing when it is not.

## On the compiler-equipped development machine

Run the complete report from the repository root:

```sh
./tools/collect-diagnostics.sh
```

It writes `diagnostics.txt`, continues after individual failures, verifies the exact immutable source/config/docs file set and hashes against the extracted handoff manifest when present, checks the required TD schema, key UX regression invariants (including both group-help discovery paths), and SQLite database, then runs the workspace format/check/test/Clippy matrix when Cargo is installed. Cargo is offline by default so a handoff does not unexpectedly fetch dependencies. If dependency/network access is intentionally available:

```sh
HANDOFF_CARGO_OFFLINE=0 ./tools/collect-diagnostics.sh
```

The SQLite check uses a temporary copy of `game.sqlite3` plus its WAL/SHM sidecars, so diagnostics do not mutate the handed-off database files.

## Package the next handoff

Recommended one-command path:

```sh
./tools/make-handoff.sh ../rustwater-handoff.zip
```

This runs diagnostics first and packages the source even if diagnostics fail, so `diagnostics.txt` travels with the failing checkout. The script exits according to packaging success rather than compiler success.

By default the packer excludes `.git`, Cargo `target/`, `.tdx-session`, and common `.env` secret files. Include TDLib authorization/session state only when deliberately needed:

```sh
./tools/make-handoff.sh ../rustwater-handoff.zip --include-session
```

Likewise, common `.env` files require an explicit `--include-secrets`.

## Pack / verify / unpack directly

```sh
python3 tools/handoff.py pack ../rustwater-handoff.zip
python3 tools/handoff.py verify ../rustwater-handoff.zip
python3 tools/handoff.py unpack ../rustwater-handoff.zip ../rustwater-unpacked
cd ../rustwater-unpacked && python3 tools/handoff.py verify-tree
```

Every generated ZIP contains `_handoff_manifest.json` with file sizes and SHA-256 hashes. The packer always ignores any previously extracted copy of that embedded manifest, so repacking a handoff cannot create duplicate/stale manifest members. `verify` checks the archive against the fresh manifest. `unpack` rejects path traversal and restores executable permission bits. `verify-tree` checks extracted immutable files against that manifest while intentionally ignoring mutable diagnostics, SQLite runtime files, and TDLib session state.

## What to return after a failed build

The most useful next message is the new handoff ZIP produced by `tools/make-handoff.sh`. If the source itself did not change, `diagnostics.txt` alone is enough for a focused compiler-fix pass.

`verify-tree` is bidirectional: missing, changed, **and extra stale immutable files** fail verification. Mutable diagnostics, TDLib session state, and the game SQLite runtime files remain excluded from tree identity by design.
