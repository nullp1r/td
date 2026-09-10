# Implementation Status and Confidence

> **Status:** historical confidence matrix

| Component | Confidence | Meaning |
|---|---|---|
| Core v4 gameplay systems | High historical | Implemented in downloadable prototypes during session. |
| SQLite v4 schema/provenance | High historical | Repeatedly validated/used. |
| Refactor module split | High historical | Implemented and then compiler diagnostics were run. |
| Diagnostics fix 1 | High historical | Based on actual user-supplied `rustc 1.100.0-nightly` output. |
| Full green post-fix diagnostics | Unknown | A second complete diagnostic run was requested; not preserved in current chat context. |
| Rich in-message `tdx` buttons | Reimplemented, compile-unverified | Restored in the 2026-09-09 source artifact; requires local Rust diagnostics. |
| `tdx` ephemeral + ephemeral-command helpers | Reimplemented + audited, compile-unverified | Restored against the supplied schema, including `botCommand.is_ephemeral`; requires local Rust diagnostics. |
| `tdx` tuple formatting / headerless tables / relative time | Implemented, compile-unverified | 2026-09-10 source rewrite removes composition operators/`plain`/`concat`, adds tuple/array/`Vec` composition, optional table headers, and native relative date rendering; local Rust diagnostics are the next gate. |
| Redesigned group `/fish` + `/help` | Reimplemented + audited, compile-unverified | `/game` removed; trust-first public shoal card, native Rich Message action, ephemeral `/help`, personal catch result, Journal and Records stay available in-group. |
| Redesigned private screens/navigation | Reimplemented + audited, compile-unverified | Main player surfaces/help/command reduction, rich hierarchy, restrained emoji, semantic parent/Home navigation and contextual actions restored; compiler/rustfmt/Clippy pass pending. |
| Mini App/web | Future | No implementation expected yet. |
| Monetization/NFT integration | Future | Design direction only. |

## How to use this table

“Historical high” does not mean the current checkout definitely contains the code; it means this session reached that implementation milestone at some point.

Always inspect the local tree and run diagnostics before deciding what must be reimplemented.
