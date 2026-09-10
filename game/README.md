# Rustwater

Rustwater is the playable vertical slice of a much larger **Telegram-native persistent sandbox MMO RPG**. Fishing is the first deep activity and the current mechanical spine, not the finished game's identity or ceiling.

The durable product/design/technical knowledge base lives in [`docs/README.md`](docs/README.md). Start there before making game changes.

## Current playable slice

The current build already connects several persistent systems in one small coastal region:

- five discoverable/traversable locations around Old Harbor;
- deterministic accelerated time and weather;
- 26 fish/creature species, seven baits and four rods;
- durable fishing encounters with reaction timing, struggle phases and deterministic individual specimens;
- inventory, selling, crafting/preparation, repairs, records and non-power titles;
- explicit and hidden discovery, including the Rusted Key → lighthouse → Lighthouse Cove thread;
- Mara Reed, Harbor Warden—the first concrete NPC and the game's recurring visual face/mascot;
- Harbor Board milestones and rotating contracts;
- Telegram-native group shoals with shared public state and per-player ephemeral results;
- Rich Message-first navigation rather than a slash-command sitemap.

Exact implemented behavior and content counts are maintained in [`docs/current-state.md`](docs/current-state.md). The long-term game vision is in [`docs/vision.md`](docs/vision.md).

## Running

Set Telegram credentials through the environment; do not commit them.

```sh
export TELEGRAM_API_ID=...
export TELEGRAM_API_HASH=...
export TELEGRAM_BOT_TOKEN=...

# Optional overrides:
export GAME_DB=game.sqlite3
export GAME_CONTENT=content/game.json
export TDLIB_SESSION=.tdx-session
```

Then run from `game/`:

```sh
cargo run --release
```

or from the workspace root:

```sh
cargo run -p game --release
```

## Architecture in one paragraph

Rustwater runs as one Rust process. TDLib/`tdx` owns Telegram transport; `game::telegram` handles dispatch/presentation; `App` owns transactional game operations; SQLite is authoritative mutable state; validated JSON is immutable content; important delays are durable SQLite timers rather than sleeping tasks. Telegram messages are presentation/input, never game truth. See [`docs/architecture/overview.md`](docs/architecture/overview.md).

## Verification

From the workspace root, documentation integrity:

```sh
python3 tools/check-docs.py
```

Full project diagnostics:

```sh
./tools/collect-diagnostics.sh
```

The collector runs formatting, build, test and strict-Clippy matrices when the Rust toolchain is present and writes a transient `diagnostics.txt`. See [`docs/development/diagnostics.md`](docs/development/diagnostics.md).

## Documentation rule

A change is not complete when it makes durable product/design/behavior/architecture knowledge stale. Update the relevant file under [`docs/`](docs/README.md) in the same change; do not create chronological session notes as a substitute.
