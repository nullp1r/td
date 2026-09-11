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

## Patch-first collaboration

For chat-assisted development, the preferred transfer pattern is:

1. start a new session with a complete project archive representing the exact current checkout;
2. include fresh diagnostics when the problem is build/test/runtime related;
3. inspect the supplied project as the source of truth;
4. return durable source changes as one or more small unified `.patch` files;
5. apply each patch locally only after `git apply --check` succeeds;
6. run formatting/verification locally and use the resulting diagnostics to drive the next incremental patch.

The full archive is the **bootstrap snapshot**. Patches are the **change transport**. This keeps ownership of the real checkout with the developer, makes every AI-generated change reviewable, and avoids repeatedly replacing an entire project tree.

Patch files themselves are transient transport artifacts, like diagnostics and project ZIPs. Do not commit them merely because they were used during collaboration. If a patch describes a durable product/engineering decision, the resulting source/docs should record that decision after application.

### Applying a patch

From the workspace root, inspect and apply a patch with:

```sh
git apply --stat /path/to/change.patch
git apply --check /path/to/change.patch
git apply /path/to/change.patch
```

Then run the appropriate formatter/checks, usually beginning with:

```sh
cargo fmt --all
```

Use `git diff` immediately afterwards when the change deserves manual review. `git apply --check` is not optional for generated patches: it catches path/context drift before modifying the checkout.

Useful recovery/inspection commands:

```sh
# Was this patch already applied?
git apply --reverse --check /path/to/change.patch

# Undo an applied patch when the working tree still matches its result.
git apply -R /path/to/change.patch

# Inspect affected paths without applying.
git apply --stat /path/to/change.patch
git apply --numstat /path/to/change.patch
```

Do not use `--reject`, `--3way`, fuzzy ad-hoc rewriting, or manual conflict surgery as the default response to a generated patch that no longer applies. A failed `--check` usually means the patch was produced against a different source state. Prefer supplying the current tree (and, when relevant, diagnostics) and generating a new incremental patch against that exact state.

### Patch sizing and sequencing

Prefer patches that are:

- **incremental** — relative to the developer's current checkout, not an old archive;
- **narrow** — only the files needed for the stated change;
- **reviewable** — avoid unrelated formatting churn or regenerated artifacts;
- **ordered** — when several patches are required, state their application order;
- **self-consistent** — source, migrations, tests and durable docs travel together when one change requires all of them.

A follow-up compile/lint fix should normally be another small patch on top of the previous one rather than a replacement project archive. A complete replacement archive is appropriate only when establishing a fresh baseline or when patch context has become too uncertain to recover safely.

## Maintained collaboration tools

The patch-first workflow has three small maintained helpers. They automate the
workflow around standard formats; they do **not** replace Git patches with a
project-specific change format.

### `tools/package-project.sh`

Create a Git-aware working-tree snapshot:

```sh
tools/package-project.sh
# or
tools/package-project.sh ~/rustwater-current.tar.gz
```

The archive contains tracked files using their current working-tree contents
plus untracked files that are not ignored by Git. It excludes `.git/`, ignored
build/runtime state, and tracked files deleted from the working tree. This is
the low-level snapshot primitive used by `make-handoff.sh`.

### `tools/apply-change.sh`

For normal generated changes, keep the incoming patch outside the repository
and let the helper own the complete local cycle:

```sh
tools/apply-change.sh ~/Downloads/rustwater-change.patch \
  --message "feat(game): improve harbor interaction"
```

The helper:

1. requires a clean Git working tree;
2. prints the patch stat and runs `git apply --check`;
3. applies the patch;
4. runs `cargo fmt --all`;
5. captures `tools/collect-diagnostics.sh` into an external diagnostics file;
6. commits the applied state.

If validation passes, the supplied message is used as the commit subject. If
validation fails, the state is still preserved in a clearly marked
`WIP: ... (validation failed)` commit and the helper exits non-zero. This is
intentional: a failing but reproducible checkpoint is often exactly what the
next patch needs.

A preflight/application failure is different: because no trusted change was
successfully applied, no commit is created.

Use `--no-commit` only when deliberately investigating without creating the
normal checkpoint.

The clean-tree requirement is a safety boundary. It prevents workspace-wide
formatting or `git add -A` from absorbing unrelated edits. Keep downloaded
patches outside the checkout. For local transport artifacts that must live in
the checkout temporarily, prefer `.git/info/exclude` over project-wide ignore
rules.

### `tools/make-handoff.sh`

Create the next chat bootstrap as one ordinary `tar.gz`:

```sh
tools/make-handoff.sh
# or
tools/make-handoff.sh ~/rustwater-handoff.tar.gz
```

The bundle contains:

```text
project.tar.gz    complete Git-aware project snapshot
diagnostics.txt   fresh full diagnostics, including failing checks
manifest.txt      HEAD, branch, status, checksums and failure count
```

`make-handoff.sh` still produces the bundle when diagnostics fail. A broken
state is often the state that needs to be handed off.

This is the preferred session boundary: attach the single handoff archive and
describe the desired change. The receiving session should treat the contained
project snapshot as authoritative and the diagnostics as evidence about that
exact state.

### Why not invent a custom patch format?

Unified diffs remain the durable change transport because they are inspectable
and supported directly by Git. The maintained scripts add orchestration around
them: safety checks, formatting, diagnostics, checkpoint commits, provenance,
and session packaging.

If that eventually becomes insufficient, add a thin metadata sidecar or a
standard archive containing a patch plus metadata. Do not replace the diff
itself unless a concrete limitation requires it.


## Disposable investigation scripts

Disposable Bash/Python/etc. scripts are encouraged for investigations where a temporary tool gives better evidence than speculative reasoning. Examples include:

- Telegram/TDLib capability probes and UI labs;
- schema/migration experiments against copied databases;
- source-tree consistency checks;
- one-off data/content inspection;
- reproducer generation;
- patch generation/application checks;
- diagnostic extraction or comparison.

A disposable script should be easy to understand, easy to delete, and explicit about any mutation it performs. Prefer these properties:

- operate on copies or temporary directories when examining mutable/runtime data;
- support `apply`/`restore` (or otherwise be reversible) when temporarily modifying source;
- refuse unexpected source drift rather than silently overwriting it;
- print enough context that failures can be copied into a follow-up session;
- avoid adding production dependencies for a one-off investigation;
- live outside the project tree when practical, or be clearly transient/ignored if placed inside it.

Do not automatically promote an investigation script into permanent tooling. Promote it only when the workflow is expected to recur and the script has become part of normal project verification/development. At that point, clean it up, document it, and test it like any other maintained tool.

### Recommended chat handoff

For the maintained one-file workflow, prefer:

```sh
tools/make-handoff.sh
```

The manual equivalent remains useful when only selected artifacts are needed.

A high-signal development handoff is therefore:

```text
project.zip            # complete current snapshot
diagnostics.txt        # optional; include when investigating failures
problem / desired change
```

and the expected return can be:

```text
change.patch           # durable project modifications
probe.py / probe.sh    # optional disposable investigation helper
application commands
verification commands
known remaining risks
```

This convention does not make chat history authoritative. Each new session should still recover context from the supplied tree and durable docs.

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
