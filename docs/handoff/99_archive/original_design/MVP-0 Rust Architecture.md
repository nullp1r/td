# MVP-0 Rust Architecture

## 1. Architectural Goal

MVP-0 should be a single-process Rust application containing:

```text
TDLib / tdx
    ↓
Telegram input
    ↓
Application
    ↓
Game logic
    ↓
SQLite
    ↓
Telegram presentation
```

plus one durable timer driver.

No microservices.

No internal RPC.

No actor framework.

No ORM.

No generic event bus.

No distributed scheduler.

No cache layer.

No connection pool.

No dependency-injection framework.

The architecture should nevertheless avoid decisions that would make the future MMO difficult to build.

The guiding engineering principle remains:

**Design for the infinite game; implement the tiny game.**

This follows the `td` repository philosophy particularly closely: demonstrated consumers before machinery, explicit ownership/lifecycle, minimal public surface, data-oriented representations where useful, and no speculative wrappers.

---

# 2. Repository Shape

Start with **one Rust package**, not an internal workspace of tiny crates.

A likely layout:

```text
game/
├── Cargo.toml
├── build.rs
├── migrations/
├── content/
│   ├── locations.json
│   ├── items.json
│   ├── species.json
│   └── ...
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── app.rs
│   ├── db.rs
│   ├── timer.rs
│   ├── clock.rs
│   ├── content.rs
│   ├── account.rs
│   ├── character.rs
│   ├── fishing.rs
│   ├── travel.rs
│   ├── discovery.rs
│   └── telegram/
│       ├── mod.rs
│       ├── input.rs
│       ├── callback.rs
│       └── present.rs
└── tests/             # only where true integration tests warrant it
```

The module structure should change naturally as code appears.

Do not create crates such as:

```text
game-domain
game-core
game-application
game-infrastructure
game-common
game-types
```

until an actual independent consumer or compilation boundary requires them.

---

# 3. Primary Dependencies

Initial dependencies should be approximately:

```text
tdx
tokio

rusqlite
rusqlite_migration

tracing
tracing-subscriber

serde
serde_json

rand
rand_chacha

thiserror
```

Potentially `anyhow` for the application/startup boundary.

`tempfile` is reasonable as a development dependency for database integration tests.

No ORM or async SQL framework.

No `async_trait`.

No unnecessary boxed-future abstraction.

This follows the repo's preference for native async Rust, current language capabilities, explicit error types in reusable code, and operational/error aggregation in the application layer.

---

# 4. Rust Version Policy

The application targets the same aggressive Rust policy as the `td` repository.

Current baseline:

```text
MSRV = 1.98.1
```

Nightly features may be used when they provide a concrete improvement and are reasonably close to stabilization.

We should aggressively use useful modern:

- language syntax;
- standard-library APIs;
- async language features;
- pattern matching;
- compiler lints;
- const capabilities;
- type-system improvements.

Compatibility with old compilers is not a design requirement.

---

# 5. TDLib Ownership

`Session` remains owned at the application lifecycle boundary.

Conceptually:

```text
run()
│
├── Session
│
├── Client clones
│
├── App
│
├── TimerDriver
│
└── TelegramPresenter
```

`Session` is the lifecycle owner.

`Client` is passed where Telegram requests are needed.

This directly matches `td-client`: `Session` owns the TDLib instance and ordered update consumption, while cloneable `Client` values are non-owning request handles.

The game should not wrap `Client` merely to hide it.

---

# 6. Startup

Startup should roughly be:

```text
initialize tracing
    ↓
load configuration/secrets
    ↓
open SQLite
    ↓
configure SQLite
    ↓
run migrations
    ↓
load + validate content
    ↓
create/authorize TDLib Session
    ↓
construct App
    ↓
start TimerDriver
    ↓
start TelegramPresenter
    ↓
enter Session::recv loop
```

Failures should unwind explicitly.

Shutdown should:

```text
stop accepting input
    ↓
stop timer driver
    ↓
finish/cancel owned application tasks
    ↓
stop presenter
    ↓
close TDLib Session gracefully
    ↓
close database worker
```

Do not rely on process destruction as successful lifecycle completion.

---

# 7. SQLite

Use:

```text
rusqlite
```

with bundled SQLite.

The database is the authoritative persistent game state.

Do not create an in-memory character cache initially.

Do not mirror the world in a giant mutable object graph.

Immutable game content can live in memory; mutable player/world state lives in SQLite.

---

# 8. SQLite Ownership

Use **one dedicated database thread** owning exactly one:

```rust
rusqlite::Connection
```

The async application talks to it through a small bounded job channel.

Conceptually:

```text
Tokio tasks
    │
    │ Db::call(...)
    ▼
bounded channel
    ▼
DB thread
    ▼
rusqlite::Connection
```

Responses return through Tokio oneshots.

`Db::call` can accept a closure approximately conceptually equivalent to:

```rust
FnOnce(&mut Connection) -> Result<T>
```

This introduces one small dynamic job allocation but gives us:

- explicit connection ownership;
- no blocking SQLite work on Tokio workers;
- ordinary synchronous transactions;
- deterministic serialization;
- no pool;
- no `Arc<Mutex<Connection>>`;
- no async-SQL adapter dependency.

This abstraction is small enough to own ourselves.

---

# 9. SQLite Configuration

Initial database configuration should favor correctness.

Likely:

```text
foreign_keys = ON
journal_mode = WAL
synchronous = FULL
```

Use SQLite `STRICT` tables where appropriate.

Use `WITHOUT ROWID` for suitable compact relation tables with natural composite primary keys.

Persist timestamps as integer UTC units, probably milliseconds.

Use SQLite integer primary keys for game-owned entities.

We do not need UUIDs merely because the project is an MMO.

If an entity later needs an externally portable identifier, add one for that demonstrated requirement.

---

# 10. Transactions Are the Concurrency Authority

Every meaningful game action should have an explicit transactional boundary.

For example, reeling a fish may atomically:

```text
validate encounter
    ↓
resolve action
    ↓
create catch
    ↓
modify inventory
    ↓
create discovery
    ↓
award XP
    ↓
update character level
    ↓
delete/update timers
    ↓
advance/finish encounter
    ↓
COMMIT
```

Telegram output occurs **after** this transaction commits.

Therefore:

> Telegram is never the source of truth for whether an action happened.

If rendering or sending fails after commit, the application repairs presentation.

It does not roll back game reality.

---

# 11. No Repository Abstraction

Do not introduce:

```rust
trait CharacterRepository
trait FishingRepository
trait DiscoveryRepository
```

with one SQLite implementation.

There is only one persistence implementation.

Concrete SQL helpers are acceptable.

For example:

```text
db
├── account queries
├── character queries
├── fishing queries
└── timer queries
```

Pure game logic should remain testable independently where that is genuinely useful, but persistence does not require an enterprise repository pattern.

---

# 12. Accounts and Characters

Do not identify a Telegram user directly with a game character.

Persist:

```text
Telegram user
    ↓
Account
    ↓
Character
```

MVP-0 exposes one character per account.

The schema permits more later.

Conceptually:

```text
accounts
--------
id
telegram_user_id

characters
----------
id
account_id
name
xp
level
coins
location_id
...
```

This makes future alts an application rule rather than a schema rewrite.

---

# 13. Game IDs

Use strongly typed Rust ID newtypes.

For example:

```rust
struct AccountId(i64);
struct CharacterId(i64);
struct EncounterId(i64);
struct ItemId(i64);
```

Do not pass anonymous `i64`s throughout game logic.

Content IDs should also be typed:

```rust
struct SpeciesId(u32);
struct LocationId(u32);
struct ItemDefId(u32);
```

Content definitions assign stable IDs explicitly.

Runtime storage may compile these into dense indexes for fast lookup.

---

# 14. Content

Content is loaded once during startup.

Examples:

```text
species
locations
bait definitions
equipment definitions
observation text
shop inventories
```

Use `serde` + JSON initially.

Why JSON:

- ubiquitous;
- simple tooling;
- trivial parsing;
- straightforward diffs;
- no custom language;
- sufficient for MVP content.

The loader should validate all cross-references at startup.

Invalid content should prevent startup.

Examples:

```text
duplicate species ID
unknown bait tag
location references missing species
negative encounter weight
invalid size distribution
unknown item definition
```

Once loaded, convert authoring data into a compact runtime representation.

---

# 15. Content Runtime Representation

Avoid repeatedly interpreting raw JSON structures during gameplay.

Startup may transform:

```text
human-oriented definitions
```

into:

```text
compact immutable tables
```

For example, a location may contain a precomputed contiguous array of possible encounters.

The runtime representation should favor:

- `Vec`;
- compact enums;
- typed IDs;
- dense indexes;
- prevalidated references.

This matches the data-oriented preference in `AGENTS.md`.

---

# 16. No Generic Condition DSL Yet

Do not invent:

```text
condition:
  all:
    - any:
      - ...
```

or a scripting language.

Represent actual MVP requirements directly.

For example:

```rust
enum Requirement {
  Weather(Weather),
  Time(TimeRange),
  Location(LocationId),
  BaitTag(BaitTag),
}
```

Only add variants when content needs them.

If this eventually becomes sufficiently expressive that a DSL is justified, we will have real examples from which to design it.

---

# 17. Randomness

Randomness must be injectable/testable.

The fishing selection code should operate on an explicit RNG rather than hidden global randomness.

Production generates a specimen seed.

Individual persistent catches may store:

```text
seed
generator_version
```

Future procedural characteristics can then be deterministically generated from that combination.

The generation algorithm is versioned so changing procedural logic does not silently reinterpret old items.

---

# 18. Application Layer

Use one concrete application object:

```text
App
├── Db
├── Content
├── Clock
└── output sender
```

The public application vocabulary should consist of real operations.

For example:

```text
start_character(...)
cast(...)
fish_action(...)
travel(...)
buy(...)
sell(...)
explore(...)
timer_fired(...)
```

Avoid generic APIs like:

```text
dispatch(Command)
execute(Action)
handle(Event)
```

until there is evidence that such an abstraction materially improves the design.

---

# 19. Telegram Input

`Session::recv()` should remain a hot ingestion boundary.

The receiving code should:

```text
receive TDLib update
    ↓
classify relevant update
    ↓
extract typed Telegram/game input
    ↓
dispatch application work
```

It should not perform long gameplay logic, database work, or rich rendering inline.

`td-client` intentionally preserves ordered application updates and keeps its application queue unbounded rather than silently discarding updates, so slow consumers would otherwise accumulate backlog.

---

# 20. Callback Protocol

Callback payloads should be compact binary data.

Do not encode verbose JSON or colon-delimited strings.

A fishing callback can conceptually contain:

```text
protocol version
action
encounter ID
step
```

Potentially around a dozen bytes.

Example conceptual type:

```rust
struct FishingCallback {
  encounter: EncounterId,
  step: u32,
  action: FishingAction,
}
```

The Telegram-supplied identity is never trusted to authorize the action.

The server verifies:

- callback sender owns/controls the character;
- encounter belongs to that character;
- encounter is currently actionable;
- step matches;
- action is valid during this step.

---

# 21. Encounter Step Instead of Generic Revision

Fishing encounters should have a monotonic:

```text
step
```

A step represents a gameplay state for which a particular set of actions is valid.

Example:

```text
step 3:
  fish is diving
  valid actions = Pull | GiveLine
```

Buttons carry `step = 3`.

If the encounter has advanced to step 4, a callback from step 3 is stale and cannot mutate state.

This directly solves:

- double clicks;
- delayed callbacks;
- duplicated delivery;
- buttons on old Telegram messages;
- timer/action races.

No generic distributed concurrency framework is needed.

---

# 22. Important Reaction-Time Rule

**Reaction timing must not start when the game internally decides that a bite occurred.**

Suppose:

```text
bite generated
    ↓
Telegram output queue waits 2 seconds
    ↓
message edit takes another second
    ↓
player finally sees button
```

Starting the reaction clock at the top would unfairly penalize the player by three seconds.

Instead:

```text
game advances encounter
    ↓
actionable message is rendered
    ↓
Telegram edit/send succeeds
    ↓
reaction opportunity starts
```

This distinction should be designed into the architecture from the beginning.

---

# 23. Actionable Encounter State

An actionable step should conceptually contain:

```text
Encounter {
  id
  character_id
  step
  phase
  opened_at: Option<Timestamp>
  ...
}
```

When the domain decides that a reaction opportunity exists:

```text
phase = AwaitingAction
opened_at = NULL
```

The Telegram presenter renders the actionable buttons.

After the edit/send successfully completes:

```text
App::presented(encounter_id, step, now)
```

sets:

```text
opened_at = now
```

and schedules the reaction deadline.

If a callback somehow arrives while `opened_at` is still `NULL`, it should be treated conservatively in the player's favor rather than rejected for impossible timing.

This also means application-side rate limiting does not corrupt the reaction mechanic.

---

# 24. Reaction Deadline

Once presentation has succeeded:

```text
opened_at = now
deadline = now + reaction_window
```

A durable timer is inserted.

When a callback arrives:

```text
reaction_time =
    callback_received_at - opened_at
```

and the species/encounter maps that duration into broad categories:

```text
Excellent
Good
Late
Missed
```

No assumption is made that this represents perfect physical human reaction time.

It represents observed Telegram interaction response.

Actual tester distributions should determine balance.

---

# 25. Fishing Cast Flow

A cast can work as follows:

```text
CAST TRANSACTION

validate character
validate location
validate equipment/bait
build FishingContext
select encounter outcome
generate specimen seed
determine bite delay

if valid bite:
    consume bait

create FishingEncounter {
    phase = Waiting,
    selected encounter,
    environment snapshot,
    equipment/bait snapshot,
    seed,
    step = 0
}

insert durable Bite timer
commit
```

Selecting the encounter at cast time is preferable for MVP-0.

It makes the entire cast context deterministic and avoids needing to reserve bait or equipment while waiting.

The actual bite is merely revealed later.

---

# 26. Environment Snapshot

Persist enough context with the encounter that its outcome cannot change because somebody edits equipment while the timer is running.

For example:

```text
location
game time
weather
bait definition
relevant equipment
active modifiers
selection seed
```

Do not necessarily serialize the entire `FishingContext`.

Persist only the fields required to preserve the encounter's meaning.

---

# 27. Game Clock

The fictional game clock should be derived rather than ticked continuously.

Store something like:

```text
world real epoch
world game epoch
game-time multiplier
```

Then:

```text
game_time(now)
```

is calculated.

No task needs to increment a “current minute” database value.

Weather can similarly be derived from:

```text
world seed
weather period
region
```

for MVP-0.

This makes game time and weather naturally restart-safe.

Administrative overrides can be added separately.

---

# 28. Durable Timers

Use one concrete timer table.

Conceptually:

```text
timers
------
id
due_at
kind
entity_id
step
```

`kind` initially might include:

```text
FishingBite
FishingDeadline
TravelArrival
```

No arbitrary JSON payload.

No generic job execution framework.

A timer identifies the persistent game state it expects to advance.

---

# 29. Timer Semantics

Timers operate **at least once**.

The TimerDriver:

```text
query next timer
    ↓
sleep until due
    ↓
load due timers
    ↓
App::timer_fired(timer)
```

The application transaction checks:

```text
does timer still exist?
does entity still exist?
does step still match?
is state appropriate?
```

Then it either:

- applies the transition and deletes the timer;
- recognizes it as stale and deletes it;
- fails, leaving it available for retry.

Thus duplicate timer delivery cannot duplicate catches or travel arrivals.

---

# 30. TimerDriver

One Tokio task is sufficient.

Conceptually:

```text
loop {
    next = db.next_timer()

    select:
        sleep until next
        OR timer-notify
        OR shutdown

    process bounded batch of overdue timers
}
```

Whenever an application transaction inserts a timer that may be earlier than the currently sleeping one, notify the driver.

The timer system survives process restarts because the schedule itself lives in SQLite.

No `tokio::spawn(sleep(...))` is authoritative gameplay state.

---

# 31. Inventory Representation

Distinguish immediately between:

### Fungible items

Examples:

```text
Worm ×17
Bread ×4
```

Store compactly as stacks.

### Individual items

Examples:

```text
1.14 kg Silver Perch
Rusted Key
unique equipment
procedural specimen
```

These receive persistent item IDs.

A likely structure:

```text
inventory_stacks
---------------
character_id
item_def_id
quantity

items
-----
id
owner_character_id
item_def_id
...
```

with catch-specific properties stored separately.

---

# 32. Catch Provenance

A caught specimen should persist enough provenance for the future game.

For example:

```text
catches
-------
item_id
species_id
size
weight
seed
generator_version
caught_at
location_id
bait_id
weather
game_time
```

Do not store pre-rendered item descriptions.

Descriptions are generated from structured state.

This leaves room for future:

- trading;
- records;
- trophies;
- world firsts;
- mutations;
- ownership history;
- externally represented assets.

---

# 33. Discoveries

Personal discovery should use a compact uniqueness constraint.

Conceptually:

```text
discoveries
-----------
character_id
kind
subject_id
discovered_at

PRIMARY KEY(character_id, kind, subject_id)
```

This is a good candidate for `WITHOUT ROWID`.

The application can attempt the insert transactionally.

Whether it succeeds tells us whether this is a new personal discovery.

---

# 34. Global Firsts

Use a separate uniqueness constraint:

```text
global_discoveries
------------------
kind
subject_id
character_id
discovered_at

PRIMARY KEY(kind, subject_id)
```

During a catch transaction:

```text
INSERT ... ON CONFLICT DO NOTHING
```

determines the single winner.

This means two simultaneous catches cannot both become the canonical first.

No distributed locking is required.

---

# 35. Telegram Presentation Boundary

Game logic never constructs TDLib messages.

Instead:

```text
domain state
    ↓
view/query
    ↓
telegram::present
    ↓
tdx rich-message structures
```

The Telegram renderer is allowed to use `tdx` fully and directly.

No additional generic Telegram UI framework is needed.

`tdx` already exposes the generated TDLib surface while providing ergonomic application-facing helpers, so game-specific presentation should sit above it rather than wrapping it defensively.

---

# 36. Persist Telegram Panel Bindings Separately

Telegram message identity is presentation state, not game state.

For example:

```text
telegram_panels
---------------
character_id
kind
chat_id
message_id
```

The fishing domain does not store Telegram message IDs.

The presenter does.

If a panel message disappears or becomes uneditable:

```text
edit fails
    ↓
send replacement
    ↓
update binding
```

Game state remains unaffected.

---

# 37. Presenter

Use one concrete `TelegramPresenter` task for stateful game panels.

Its primary operation is something like:

```text
Refresh(PanelKey)
```

rather than:

```text
Edit this exact old snapshot to this exact string
```

The presenter queries the latest committed state, renders it, and updates the Telegram panel.

This is important because several internal state changes may occur faster than we should send Telegram edits.

---

# 38. Coalescing

Panel refresh requests should be coalescible.

If the application asks:

```text
refresh fishing panel
refresh fishing panel
refresh fishing panel
```

before Telegram can process the first one, the presenter should generally render the **newest state once**.

This directly turns Telegram rate limits into an architectural property rather than scattered sleeps inside gameplay code.

Persistent significant events are different.

Examples:

```text
NEW SPECIES
WORLD FIRST
LEVEL UP
```

may deserve explicit conversation messages and should not automatically disappear through state coalescing.

---

# 39. Callback Answers

Callback acknowledgement has tighter latency expectations than normal game presentation.

Do not allow it to sit behind a large backlog of cosmetic panel refreshes.

Initially this can simply be handled directly during callback processing.

If group-scale traffic eventually requires more policy, that policy belongs in the application/presentation layer rather than `tdx`, consistent with `td-client`'s separation of mechanism from retry/rate-limit/application policy.

---

# 40. Message History Principle

Use:

**panels for state**

and:

**messages for events**

Examples:

### Update existing panel

```text
weather changed
bait selection changed
fishing tension changed
travel countdown/state
inventory view
```

### Send persistent message

```text
new species discovered
exceptional catch
new location discovered
level reached
major world event
```

This should reduce traffic while making conversation history meaningful.

---

# 41. Application Result

Do not create a generic event-sourcing architecture.

An application operation may simply return a concrete result.

Example:

```text
FishActionResult {
  panel_changed
  catch
  discovery
  level_up
}
```

The caller then requests the necessary presentation.

If repeated result patterns eventually justify a common representation, extract it then.

---

# 42. No Event Sourcing

SQLite stores current authoritative state.

Important historical facts get explicit historical tables when they actually matter.

Do not store every gameplay mutation as an append-only generic event stream and rebuild state from it.

Telemetry/logging is separate from game-state persistence.

---

# 43. Observability

Use structured `tracing`.

Useful span fields:

```text
character_id
chat_id
encounter_id
timer_id
action
```

Avoid logging secrets.

Avoid logging complete Telegram message contents by default.

Useful spans:

```text
telegram.update
game.cast
game.fish_action
db.transaction
timer.fire
telegram.render
telegram.send
```

The purpose is to make timing and state-transition problems observable during playtesting.

---

# 44. Fishing Telemetry

Early testing should record:

```text
cast time
bite delay
reaction opportunity presentation time
callback receipt time
reaction category
species
location
bait
equipment
success/failure
failure reason
```

This allows us to answer the most important timing question empirically:

> What does Telegram interaction latency actually look like for real players?

Do not balance reaction windows entirely from intuition.

---

# 45. Error Policy

Separate errors by responsibility.

Examples:

```text
ContentError
DbError
FishingError
CallbackError
PresentationError
```

Expected gameplay failures are not operational errors.

Example:

```text
fish escaped
```

is a game result.

Example:

```text
SQLite constraint unexpectedly violated
```

is an application failure.

At the top-level binary, aggregation with `anyhow` is reasonable.

Reusable modules should retain explicit errors where they communicate meaningful contracts.

---

# 46. Testing

Prioritize a small number of strong tests.

### Pure fishing tests

Given:

```text
content
FishingContext
seeded RNG
```

assert:

```text
eligible encounters
selected encounter
modifier behavior
specimen generation
```

### Database integration tests

Prove complete transactions such as:

```text
catch fish
→ item inserted
→ discovery inserted
→ XP updated
→ encounter resolved
```

### Concurrency/idempotency tests

Examples:

```text
same Reel callback twice
old encounter step
timer races Reel
timer delivered twice
```

Exactly one valid transition should occur.

### Restart tests

Persist:

```text
travel timer
fishing bite timer
```

reopen the application database, run timer processing, and verify correct completion.

### Telegram live tests

Keep credentialed integration tests narrow and ignored, following the existing `tdx` live-test philosophy rather than mocking TDLib when the actual boundary is what needs verification.

---

# 47. What We Deliberately Do Not Cache

Initially:

```text
accounts
characters
inventory
encounters
travel
discoveries
```

are queried from SQLite as needed.

Content is cached because it is immutable and read constantly.

Do not add Redis.

Do not build an entity cache.

Do not maintain a second authoritative in-memory world.

If profiling later proves SQLite reads material, optimize that demonstrated access pattern.

---

# 48. What “Scalable” Means at MVP-0

Scalable currently means:

- state transitions are transactional;
- Telegram is not the source of truth;
- timers are durable;
- callbacks are idempotent/stale-safe;
- content is separate from code where useful;
- mutable state is not duplicated across caches;
- Telegram rendering is separate from game logic;
- accounts are separate from characters;
- game activities are not structurally defined as “fish”;
- procedural items have stable identities/seeds;
- historical uniqueness is database-enforced.

It does **not** mean:

```text
Kubernetes
distributed actors
Kafka
Redis
PostgreSQL
sharded databases
multiple game servers
service mesh
```

Those would solve problems we do not currently possess.

---

# 49. First Implementation Sequence

I would implement in this order.

### 1. Skeleton

```text
startup
tracing
TDLib lifecycle
SQLite worker
migrations
clean shutdown
```

### 2. Identity

```text
Telegram user → account
account → character
/start
basic character panel
```

### 3. Content

```text
locations
items
species
startup validation
```

### 4. Minimal fishing

```text
Cast
select fish
schedule bite
show bite
Reel
create persistent catch
```

At this point, **the game is playable**.

### 5. Correct interaction timing

```text
presented callback
reaction measurement
deadlines
stale buttons
timer recovery
```

### 6. Progression

```text
inventory
discovery
XP
levels
money
selling
shop
```

### 7. Depth

```text
bait modifiers
equipment
weather/time
intermediate encounters
exceptional encounter
```

### 8. World thread

```text
travel
exploration
Broken Breakwater
Rusted Key
Lighthouse
hidden fishing location
```

### 9. Testing and tuning

Only after real play begins should we substantially expand content.

---

# 50. Architecture Invariants

These should be treated as hard rules unless a demonstrated requirement disproves one.

1. **SQLite is authoritative for mutable game state.**
2. **TDLib/Telegram is presentation and input, never game truth.**
3. **Game-state mutations happen transactionally.**
4. **Telegram output happens after commit.**
5. **Reaction windows begin after successful presentation, not internal state generation.**
6. **Durable gameplay delays live in SQLite, not sleeping tasks.**
7. **Timer processing and callbacks are safe under duplicate/stale delivery.**
8. **Content definitions are validated once and compacted for runtime use.**
9. **Randomness is explicit and reproducible where required.**
10. **Telegram-specific identifiers do not leak into the fishing/world domain.**
11. **The game depends directly on `tdx`; it does not build another generic Telegram framework.**
12. **If `tdx` is missing a generally useful capability, improve `tdx` instead of compensating awkwardly in the game.**
13. **No abstraction exists solely because the hypothetical final MMO might need it.**
14. **No dependency exists merely to avoid writing a small, clearer implementation ourselves.**
15. **When measurements contradict an architectural assumption, change the architecture rather than preserving compatibility with the prototype.**

---

# 51. Core Runtime Shape

The resulting MVP runtime is deliberately small:

```text
                         ┌───────────────────────┐
                         │      TDLib Session    │
                         │       Session::recv   │
                         └───────────┬───────────┘
                                     │
                                     ▼
                           ┌──────────────────┐
                           │ Telegram Input   │
                           └────────┬─────────┘
                                    │
                                    ▼
              ┌────────────────────────────────────────┐
              │                  App                   │
              │                                        │
              │ account / fishing / travel / discovery │
              └───────────────┬───────────────┬────────┘
                              │               │
                              ▼               │
                       ┌────────────┐          │
                       │     Db     │          │
                       │ async API  │          │
                       └─────┬──────┘          │
                             │                 │
                             ▼                 │
                       ┌────────────┐          │
                       │ SQLite     │          │
                       │ DB thread  │          │
                       └─────▲──────┘          │
                             │                 │
                       durable timers          │
                             │                 │
                       ┌─────┴──────┐          │
                       │ TimerDriver│──────────┘
                       └────────────┘

                                    │ committed result
                                    ▼
                         ┌────────────────────┐
                         │ TelegramPresenter  │
                         │ coalesce / render  │
                         └─────────┬──────────┘
                                   │
                                   ▼
                              tdx::Client
```

That is enough architecture to build the first game without pretending we have already built the MMO.