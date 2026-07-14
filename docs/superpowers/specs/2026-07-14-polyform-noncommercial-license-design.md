# Switch to PolyForm Noncommercial License 1.0.0 — Design

**Status:** approved (user, 2026-07-14) after an AGY-FIRST design consult.
**Goal:** Replace rsahp's current custom MIT-derived "non-commercial use only" license with the standard **PolyForm Noncommercial License 1.0.0**, and declare it consistently across the repo — including a per-file SPDX header on every workspace source file.

This is a like-for-like change of licensing *instrument* (both are noncommercial); it does not change licensing *intent*.

## Decisions (settled — not open questions)

| # | Decision | Source |
|---|---|---|
| Licensor | `Copyright (c) 2026 Costas Kirgoussios` | user |
| Scope | Declaration files **plus** an SPDX header on **every** workspace `.rs` file (~46) | user |
| F1 — Cargo manifest | `license-file = "LICENSE"` (NOT `license = "LicenseRef-…"`) — Cargo's canonical field for non-OSI licenses; avoids downstream tooling ambiguity | agy + controller |
| F2 — SPDX id | `LicenseRef-PolyForm-Noncommercial-1.0.0` (valid SPDX `LicenseRef` form; PolyForm is not in the standard SPDX list) | agy + controller |
| F3 — dead files | Header the dead `frontend/src/ui/explorer_{old,very_old,full,1151}.rs` variants too (avoid distributed code under ambiguous copyright); do NOT delete them (out of scope) | agy + controller |
| F4 — `m0_initial.rs` | Safe to header (a `//` comment never alters the AST/schema); **verify first** the drift guard is semantic (PRAGMA/AST), not a raw-byte/`include_bytes!` hash | agy + controller |
| F5 — header placement | SPDX header is line 1, above any `//!` doc and `#![…]` inner attribute; **verify no `.rs` file starts with a UNIX shebang** (`#!/…`), which would force the header to line 2 | agy (edge case) |

## Touch-points

**A. `LICENSE`** — replace entirely with the canonical PolyForm Noncommercial 1.0.0 text, fetched verbatim from `https://polyformproject.org/licenses/noncommercial/1.0.0/`, prefixed with the copyright/Licensor line `Copyright (c) 2026 Costas Kirgoussios`.

**B. `README.md`** (line ~75) — replace the "custom Non-Commercial License" blurb with: licensed under the [PolyForm Noncommercial License 1.0.0](LICENSE), free for noncommercial use.

**C. `packaging/linux/io.github.ckir.rsahp.metainfo.xml`** (line 5) — `<project_license>LicenseRef-PolyForm-Noncommercial-1.0.0</project_license>`. `<metadata_license>` stays `CC0-1.0` (it licenses the metadata file, not the project). Note: this element is only validated by `appstreamcli` in the tag-triggered Flatpak CI job, so full validation is deferred to a release tag.

**D. Cargo manifests** — add `license-file = "LICENSE"` once under `[workspace.package]` in the root `Cargo.toml`, and `license-file.workspace = true` under `[package]` in each member crate (`backend`, `frontend`, `common`, `rsahp-desktop`, `xtask`, `migration`). `license-file` under `[workspace.package]` resolves relative to the workspace root, so all crates reference the single root `LICENSE`. `deny.toml` needs no change (`[licenses.private] ignore = true` exempts our own crates from cargo-deny's license check).

**E. Per-file SPDX headers** — prepend, as the first line of every workspace `.rs` file:
```
// SPDX-License-Identifier: LicenseRef-PolyForm-Noncommercial-1.0.0
```
Files in scope (~46): all `.rs` under `backend/src`, `backend/tests`, `frontend/src`, `frontend/tests`, `common/src`, `rsahp-desktop/src`, `xtask/src`, `migration/src` — including the dead `explorer_*` variants and `migration/src/m0_initial.rs` (after the F4 guard verification). **Excluded:** everything under `target/` (build artifacts / generated code — never edited). The header goes above any existing `//!` module doc or `#![…]` inner attribute; a blank line separates it from a following `//!` doc block for readability.

## Verification & risks

- **F4 guard check (blocking for `m0_initial.rs`):** inspect the migration drift mechanism (`backend`'s `migration_drift` test + WS3 cutover guard) to confirm it compares schema **semantically** (PRAGMA / entity-vs-migration), not by hashing the raw bytes of `m0_initial.rs`. If (unexpectedly) it hashes file bytes, exempt that one file from the header and record why. Expected: semantic → safe to header.
- **F5 shebang check (blocking for line-1 placement):** grep every in-scope `.rs` for a leading `#!/` shebang (distinct from `#![` inner attributes). Expected: none → header at line 1 everywhere. Any shebang file → header at line 2.
- **Full local gate before any push** (the lesson from the packaging CI rounds — do not push multi-OS/CI-touching changes on a partial gate): `cargo fmt --all -- --check` (a leading comment is fmt-stable), `cargo clippy --workspace -- -D warnings`, `cargo nextest run`, `cargo deny check`, and `typos`. A `//` comment affects none of build/lint/test/typos, but running the whole gate is the discipline.
- **`typos` proper-noun risk (CI landmine — panel finding):** the words **`PolyForm`** (camelCase) and **`polyformproject`** now appear in `LICENSE` and `README.md`, and `typos` checks all files. This is the same failure class that broke the packaging CI (`mis-parsed`). The plan MUST, after the text edits, run `typos` and — if it flags `PolyForm`/`polyformproject`/any PolyForm-license term — add them to the repo's typos config allow-list (`_typos.toml` `[default.extend-words]` or `[default.extend-identifiers]`) as part of the SAME change, so CI stays green. Do not push until `typos` is clean locally.
- **License text must be VERBATIM (panel finding):** `LICENSE` is a legal instrument. The plan fetches the canonical PolyForm Noncommercial 1.0.0 text from the official source and copies it exactly — it MUST NOT paraphrase, summarize, or reconstruct it from memory. If the fetch is unavailable, STOP and obtain the exact text through another channel rather than approximating; only the copyright/Licensor line is authored by us.
- **No functional risk:** SPDX headers are comments; `license-file` is inert metadata; the README/metainfo/LICENSE changes are text. No code behavior changes. (Panel note: `license-file` in Cargo.toml and the `LicenseRef-…` SPDX id in headers/metainfo are complementary representations of the same license — intentional per F1, not a conflict.)

## Panel round 2 (agy escalation) — GREEN, with three facts measured

The AGY-AFTER escalation raised three findings; **all three were refuted by measurement** (agy conceded → PANEL VERDICT GREEN). Recorded here so the implementer does not re-litigate them:

- **PolyForm text has NO fill-in placeholder.** The canonical PolyForm Noncommercial 1.0.0 text uses the *defined lowercase term* "licensor" ("the individual or entity offering these terms") throughout — there is **no** `<Licensor>`/angle-bracket/`{{}}` token to substitute. Its Notices section shows an example "Required Notice: Copyright Yoyodyne, Inc.": the `Copyright (c) 2026 Costas Kirgoussios` line we prepend **is** exactly that PolyForm "required notice." So: copy the text verbatim, prepend the copyright line, substitute nothing inside the body.
- **No auto-generated `.rs` files exist.** A workspace grep for generator markers (`@generated`, "Generated by", "Code generated", "DO NOT EDIT") returns a single hit — `migration/src/m0_initial.rs`'s `//!` "DO NOT EDIT after it ships" — which is the one-time-produced, then-frozen immutable baseline (the F4 case), NOT an auto-*regenerated* file. So the header sweep has no overwrite risk. (If any `@generated`-marked, truly regenerated file is ever added, skip it.)
- **No `include_str!`/`include_bytes!` anywhere.** A workspace grep returns zero matches, so no source file's bytes are embedded into a runtime value — headering `m0_initial.rs` (or any file) cannot leak the comment into program behavior. This makes F4 definitively safe (beyond just the semantic-drift-guard check).

## Process / discipline

The change touches CI-validated files (metainfo) and manifests (Cargo.toml), so it does NOT qualify for the docs→master fast path. Flow: create a short branch (e.g. `license-polyform`) → apply A–E → run the full local gate → merge to master (fast-forward) → push → confirm CI green. The header sweep (E) across ~46 files is mechanical and suited to a delegated implementer; the LICENSE text fetch (A) and the F4/F5 verifications need judgment.

## Out of scope

- Deleting the dead `explorer_*` files (headered, not removed).
- Adding license headers to non-`.rs` files (`.toml`, `.yml`, `.ps1`, `.md`) beyond the four declaration touch-points above.
- Any change to the noncommercial *terms* — this is an instrument swap, not a relicensing of intent.
