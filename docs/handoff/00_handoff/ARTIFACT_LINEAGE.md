# Artifact Lineage and Recoverability

> **Status:** historical

This file exists because long ChatGPT sessions can be rewound/truncated and sandbox download URLs can disappear.

## Known notable artifacts

### Gameplay/social prototypes

Several ZIPs were generated during the feature-building phase, including expanded v3/v4 game slices. Exact availability depends on the user's local downloads and is not guaranteed by this handoff.

### Static tech-debt refactor

An earlier refactored archive was reported with SHA-256:

`566a3e768f413a4769ca05f401f79c246b6985b0843d4cd52bac87d257651631`

That pass had extensive static validation but could not run Rust locally in the artifact environment.

### Diagnostics-fixed refactor

After the user supplied a real diagnostics log, an archive was reported as:

`td_refactored_diagfix1.zip`

SHA-256:

`141490095991c7af42c02cd5b8f0de02aa194fa68df4aa98ad15561d40a0a3a2`

This is the strongest known refactor lineage described by the session because it incorporated actual `rustc 1.100.0-nightly` diagnostics and rustfmt output. A second full diagnostic run was still requested.

### Restored and audited UX artifacts

The formerly lost Rich Message / ephemeral / navigation / command overhaul was reimplemented on 2026-09-09. The user confirmed the restored build worked. A later audit build incorporated the original UX request more precisely.

Known reported artifacts:

- `telegram-mmo-rpg-restored-2026-09-09.zip` — SHA-256 `e4200f6480eaf7506eecfadc4240e4c89adbbf9dad44595976e660a786d9788a`;
- `telegram-mmo-rpg-ux-audited-2026-09-09.zip` — SHA-256 `8878940646f23cd7047db6b6bdd4ddd28275de461b57d5e6ab3ebe615edaade1`.

The current source advances from the audited artifact with the 2026-09-10 `tdx` formatting redesign. That newer source still awaits compiler-equipped diagnostics.

## Rule for future work

Never choose a base tree solely from an artifact name in this file. Inspect the user's actual local repository and run diagnostics. These entries are lineage markers, not source-control commits.

### `tdx` formatting + game maintainability lineage — 2026-09-10

The current source advances from the tuple-formatting fixes with the user's `Styled<T>: IntoRichText` compiler correction and a crate-wide maintainability pass over `game`. The pass is documented in `05_technical/GAME_MAINTAINABILITY_PASS_2026-09-10.md` and remains compiler-unverified in the artifact environment. The packaged handoff name should identify it as the 2026-09-10 maintainability build; use the ZIP's external SHA-256 plus embedded `_handoff_manifest.json` rather than trusting this historical filename alone.

