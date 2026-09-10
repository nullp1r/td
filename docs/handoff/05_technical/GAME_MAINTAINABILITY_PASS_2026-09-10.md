# Game Maintainability Pass — 2026-09-10

> **Status:** implemented; first real compiler/rustfmt/Clippy diagnostics repaired; one clean rerun is still required.

## Why this pass happened

After the Telegram UX and `tdx` formatting work, the user identified a different form of debt in `game`: production code had almost no explanatory comments, several application methods mixed SQL, state transitions, and projection in long straight-line bodies, and some modules were difficult to navigate even when the behavior was correct.

The request was explicitly **not** to add commentary on every statement or split functions mechanically. The target was a smaller, easier-to-follow game crate: remove redundant machinery, make important invariants visible, isolate real responsibilities, use typed/domain values earlier, and prefer declarative data where the gameplay set is small and closed.

## Quantitative result

Production Rust here means `game/src/**/*.rs` excluding `app/tests.rs`.

| Metric | Before | After | Delta |
|---|---:|---:|---:|
| Production lines | 5,285 | 5,166 | **-119** |
| Comment/doc-comment lines | 7 | 74 | **+67** |

The crate therefore became smaller despite adding substantially more documentation. LOC was used as design feedback, not as a target for dense one-liners: `angling.rs` and `social.rs` intentionally grew where named state/transaction boundaries made persisted behavior easier to audit, while larger reductions came from duplicated database/application/presenter mechanics.

## Comment policy

Every game source module now has a short responsibility-level `//!` module document.

Ordinary comments are reserved for facts that are hard or unsafe to infer from the next statement alone, especially:

- transaction and persistence invariants;
- durable timer semantics;
- reaction-window timing relative to Telegram delivery;
- persisted numeric IDs/tags and ordering constraints;
- callback wire compatibility with old Telegram messages;
- deterministic RNG-domain separation;
- non-obvious game policy such as which stored specimen a contract consumes;
- deliberately different social-vs-private equipment semantics.

Comments that merely translate Rust or SQL into English were not added.

## Application/database boundary

`App` now owns application error policy through a single `run_db` boundary. The database worker's `job` method simply transports an arbitrary `Send` result from the dedicated SQLite thread; it no longer knows that callers happen to return nested domain `Result`s.

The old intermediary transaction-error layer was removed. SQL errors convert into the application error type at the application boundary instead of being wrapped and unwrapped through parallel enums.

SQLite integers are decoded into domain ID newtypes (`LocationId`, `SpeciesId`, `BaitId`, `RodId`, and similar) as early as practical. Raw integers are recovered only where SQLite/protocol fields actually store integers. This removes repeated wrapping/unwrapping and makes state-machine code operate in domain terms.

No ORM, repository trait, generic query framework, actor layer, or new dependency was introduced.

## Angling state machine

Fishing remains an explicit persisted state machine rather than being hidden behind a generic framework.

The pass:

- moved fishing phase/timer/special tags next to the code that interprets them;
- documented that those numeric tags are persisted state;
- named encounter loading/validation and reaction calculation steps;
- kept bait consumption, encounter insertion, and first durable timer in one transaction;
- moved SQL seed/ID representation conversion to the persistence boundary;
- documented the critical reaction invariant: the player's reaction clock begins only after Telegram successfully presents the actionable state;
- separated due-timer loading from the state transition that advances a timer;
- removed view/state fields that diagnostics had shown were not consumed.

The callback/timer `step` guard remains explicit because duplicate/stale delivery is a real application invariant.

## Economy and progression

Small closed gameplay sets now use declarative definitions where that makes behavior and metadata auditable in one place:

- bait-preparation recipes use `RecipeDefinition`/`RECIPES`;
- Harbor Board milestones use `ObjectiveDefinition`/`OBJECTIVES`;
- persisted IDs remain explicit and stable rather than being inferred from array position.

Crafting availability now obtains stored-catch counts with one grouped query instead of performing a query per recipe.

Buying bait, crafting bait, and emergency forage share one atomic bait-stack mutation rule, including the subtle selection policy: newly acquired bait only replaces the selected bait when the selected stack is exhausted.

Contract turn-in deliberately consumes the smallest matching stored specimen; this policy is documented next to the ordering query instead of being left as an unexplained `ORDER BY`.

Title-unlock positional state is documented because its array indexes correspond to persistent title IDs.

## Character, content, world, and RNG

Exploration findings use a named `Finding` value rather than opaque tuples. Match order is documented as progression order because the first unmet discovery wins.

Content validation is split by responsibility (locations, equipment, species) while retaining startup validation as the single correctness gate. Build-once content collections remain sorted for binary search.

Fishing's independent deterministic random domains are named and documented so adding/removing one random draw cannot silently perturb unrelated persisted specimen behavior.

The durable timer driver explicitly documents that SQLite is authoritative: an in-memory wake notification only invalidates a sleep and cannot lose a timer. Timer draining stays bounded so a backlog cannot monopolize the runtime.

## Telegram boundary

Presentation remains separate from game/domain state. Repeated TDLib edit/ephemeral-send plumbing is centralized at the presenter transport boundary, while individual screens remain direct descriptions of player-visible state.

Presenter tables/actions were migrated toward iterator/tuple-driven construction using the redesigned `tdx` API instead of repeated manual block mutation.

The callback codec was **not** replaced by a macro or generic serializer. Its numeric callback tags are a persistent wire protocol embedded in already-sent Telegram messages; keeping the mapping explicit makes compatibility and auditing easier.

## Things deliberately not done

This pass rejected several superficially attractive ways to reduce file/function size:

- no generic repository/service abstraction around SQLite;
- no state-machine framework around fishing;
- no callback-code-generation macro hiding persistent byte assignments;
- no splitting straight-line functions into one-use helpers unless the helper names a real state, SQL projection, or domain decision;
- no compatibility aliases for removed APIs;
- no comments on self-evident statements;
- no production fields or widened visibility solely to make tests convenient.

Tests that depended on removed production-only fields/constants were changed to assert player/domain behavior or define their fixture IDs locally rather than expanding the production API again.

## `tdx::Styled` compiler correction

During playtesting the user found that `Styled<T>: IntoRichText` needed to implement the trait's scalar conversion method explicitly. The working tree includes the corrected shape:

```rust
impl<T: IntoRichText> IntoRichText for Styled<'_, T> {
  fn into_rich_text(self) -> RichText {
    self.kind.into_rich_text(self.content)
  }

  fn append_to(self, texts: &mut Vec<RichText>) {
    texts.push(self.into_rich_text());
  }
}
```

This is part of the current formatting lineage and must not be lost in later handoffs.

## First real diagnostics follow-up

The user ran `tools/collect-diagnostics.sh` against the exact packaged maintainability tree; `verify-tree` passed before the compiler matrix. That run exposed a small set of integration errors rather than a structural problem with the refactor. The follow-up source fixes are:

- preserve the user's required scalar `Styled<T>: IntoRichText::into_rich_text` implementation;
- derive `Clone` alongside `Copy` for the three intentionally copyable scalar view records;
- make Telegram dispatcher branches explicitly discard TDLib response values when the surrounding match is side-effect-only;
- construct formatted Rich Message/request values before `.await` where `fmt::Arguments` temporaries would otherwise make a spawned callback future non-`Send`;
- make the tuple-composition ownership test borrow the original `Text` and move a clone, rather than borrowing and moving the same binding in one tuple;
- apply the complete rustfmt diff emitted by the real toolchain and remove the newly introduced extra blank lines at EOF.

No gameplay rules, schema, migrations, content, or player-facing behavior were changed by this diagnostics repair.

## Verification status

The artifact environment has no Rust compiler, Cargo, rustfmt, or Clippy. The first real diagnostics identified the issues above, and this tree repairs them by source inspection, but a **fresh clean diagnostic rerun is still required** because compiler blockers can mask later Clippy/test failures. Static source scans, TDLib schema checks, SQLite integrity checks, handoff tooling, and runtime-state hash checks are performed before packaging.

After that rerun, further work should be driven by playtesting rather than another speculative structural rewrite.
