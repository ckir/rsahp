# rsahp Roadmap

Deferred / planned work items, with enough design detail to resume cleanly.

---

## Deferred: Developers Cockpit enrichment + PowerShell launcher

**Status:** DEFERRED (2026-07-14) — **blocked on the product-packaging work** (see below). Design is complete and approved; implementation is shelved, not cancelled.

**Why deferred (sequencing decision):** The cockpit's *SHIP & RELEASE* tier orchestrates the release/packaging machinery. The upcoming **product packaging** work (Windows Inno Setup installer + Linux Flatpak) will define the real release artifacts, their paths, and whether builds run locally or in CI. Building the cockpit's release tier before those facts exist guarantees rework. The dev-loop/quality/housekeeping tiers are packaging-independent, but splitting the cockpit into two implementation phases fragments a cohesive tool and forces a costly context reload — so the whole cockpit waits. (AGY-FIRST + user-approved: architecture fork → "dual-mode façade", refined by user to "enrich xtask + thin launcher"; sequencing fork → defer entire cockpit, do packaging first.)

**Resume condition:** After product packaging (inno/flatpak) ships and the release mechanics are settled. Then pull this off the shelf, reconcile the SHIP & RELEASE tier against the real packaging outputs, and implement in one sweep.

### Approved design

**Shape:** ONE cockpit (the existing `cargo xtask`, enriched) + a thin `scripts/DevelopersCockpit.ps1` launcher. Single source of truth in Rust (`xtask/src/main.rs`); PowerShell is just the front door. No duplicated logic.

**Model reference:** `C:\Users\user\Development\Node\corelib\DevelopersCockpit.py` (letter-keyed, 4-tier ANSI menu; `Action(key, group, desc, cmd|handler, note)` model; colored banner with version/config; handlers for health-check, release packaging, lockstep version bump).

**`xtask` reorganized into 4 tiers** (menu keys: letters grouped by tier, per the model — number keys are acceptable too). ● = already implemented in xtask this session; ＋ = new work:

| Tier | Actions |
|---|---|
| **INNER LOOP** | ● Quick (build & launch) · ● Format & Lint · ＋ Build workspace (`cargo build --workspace`) |
| **QUALITY GATE** | ● Unit tests (`cargo nextest run`) · ● Coverage (`cargo llvm-cov nextest`) · ● Supply-chain & hygiene (`cargo deny check` + `cargo machete` + `typos`) · ＋ Contract test (`.\verify.ps1`) |
| **SHIP & RELEASE** *(depends on packaging)* | ● Fullscale (commit & push) · ● Version bump (lockstep) · ＋ Tag & push (`v<ver>`) · ＋ Local release package (zip of `target/release`) — **reconcile against inno/flatpak outputs at resume time** |
| **HOUSEKEEPING** | ＋ Health check (tool versions: cargo/rustc/nextest/llvm-cov/deny/machete/typos/gh/lefthook/sea-orm-cli) · ＋ Clean (`cargo clean`) · ● Quit |

**New xtask functionality to add:** build-workspace, contract-test (invoke `verify.ps1`), tag & push, local release-zip, health-check, clean. Reuse the existing `binary_present()` PATH probe and `version_bump()` handler. Keep the menu-alive-on-error pattern (`.run().ok()` / `if let Err`). Missing tools degrade to a note, not a crash.

**`scripts/DevelopersCockpit.ps1` (thin launcher):** (a) `cd` to repo root (works from anywhere / double-click); (b) check `cargo` is on PATH (friendly message if absent); (c) run `cargo xtask`, passing any args straight through so `.\scripts\DevelopersCockpit.ps1 quick` also works as a non-interactive shortcut. No menu logic of its own.

**Testing:** unit-test the new pure-logic helpers in xtask (release-artifact path list, version→tag string) via `cargo nextest run -p xtask`; launcher gets a manual smoke + arg-passthrough check. Menu interaction stays manual (YAGNI for an internal tool).

---

## Next up: Product packaging (Inno Setup + Flatpak)

Windows Inno Setup installer + Linux Flatpak for the egui/axum app. To be brainstormed next (own spec → plan → implementation cycle). This is the blocker for the cockpit's SHIP & RELEASE tier above.
