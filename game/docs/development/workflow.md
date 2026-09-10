# Development workflow

## Canonical project truth

The project tree is the working source of truth:

- source/config/content define what executes;
- `game/docs/` records durable product/design/technical context;
- runtime DB/TDLib session files are mutable local state;
- generated diagnostics are transient evidence.

Do not treat a chat transcript, archive filename or previously generated report as more authoritative than the actual project tree.

## Project transfer between environments

The project may be moved between development environments as a manually created compressed archive. This is a transport constraint, not a project concept.

Treat transfer archives as disposable transport: extract into a clean directory when practical, inspect the actual tree, preserve intentionally included SQLite/TDLib runtime state, and never use archive filenames or hashes as development milestones. No custom packaging workflow is required.

Credentials/secrets are not ordinary project state and must not be copied into source or docs.

## Memoryless developer bootstrap

A developer/AI with zero conversational memory should:

1. read root `AGENTS.md` and `game/AGENTS.md`;
2. read [`../README.md`](../README.md);
3. read [`../vision.md`](../vision.md), [`../current-state.md`](../current-state.md), [`../decisions.md`](../decisions.md), and [`../open-questions.md`](../open-questions.md);
4. read subsystem docs relevant to the requested task;
5. inspect the current source before assuming docs describe every implementation detail;
6. run diagnostics when verification status matters;
7. continue from the current request/evidence—not from a stale prediction of “what comes next.”

There is intentionally no next-iteration forecast in the docs.

## Decision-making style

When enough product context already exists, make the best coherent decision instead of repeatedly reopening broad questions. Present alternatives when the choice has meaningful consequences, evidence is genuinely insufficient, or cheap playtesting can distinguish them.

Conversely, do not treat an explicit **Open** question as missing work that must be resolved. Uncertainty is allowed to remain documented until a real need provides evidence.

## Documentation is definition of done

If a code/design change alters durable knowledge, update the relevant document in the same work.

Examples:

- changing reaction timing → `gameplay/fishing.md` + persistence docs if invariant changes;
- adding an NPC relationship mechanic → narrative/progression docs;
- changing Telegram interaction surfaces → UX/platform docs;
- changing schema/invariants → architecture docs;
- changing Mara's canonical appearance → Mara/art docs;
- settling an open question → remove it from `open-questions.md` and update `decisions.md`/the subsystem doc.

Do **not** satisfy this rule by appending a session log. The goal is current knowledge, not proof that work happened.

Avoid documentation noise such as:

- transient archive filenames/hashes;
- session/chat-limit stories;
- superseded implementation incident reports;
- long chronological decision histories when only current rationale matters;
- “the next developer should build X” predictions;
- duplicated source code listings that immediately go stale.

## Conflict resolution

If docs and code disagree:

- code is authoritative for what currently executes;
- durable documented decisions/vision explain what it is *supposed* to mean;
- investigate the mismatch rather than silently choosing one;
- reconcile both before considering the work done.

If a platform fact may have changed, re-check official documentation and update the verification date/source.

## Technical quality rules

Also follow root/game `AGENTS.md`:

- modern idiomatic Rust;
- solve root cause, not symptom;
- no speculative frameworks/compatibility layers;
- remove superseded code;
- comments explain invariants/intent rather than syntax;
- compile/test/Clippy/rustfmt with the repository's real toolchain;
- use `tdx`/TDLib capabilities rather than inventing game-local transport abstractions.

## Ending a work session

Before returning the project:

- ensure code/content/docs agree;
- remove temporary/generated work that should not live in source;
- run [`../../tools/check-docs.py`](../../../tools/check-docs.py);
- run [`../../tools/collect-diagnostics.sh`](../../../tools/collect-diagnostics.sh) when a Rust toolchain is available or diagnostics are requested;
- make sure a memoryless reader can understand any new durable decision without the chat transcript.
