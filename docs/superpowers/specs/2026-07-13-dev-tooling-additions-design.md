# Dev Tooling Additions — Design Spec

**Date:** 2026-07-13
**Status:** Approved (design) — passed adversarial panel review GREEN (5 rounds, solo + agy second-model, ~18 findings folded) — pending implementation plan
**Scope:** Add gap-filling developer tooling to the `rsahp` Rust workspace and wire it into the existing `xtask` cockpit, `lefthook` hooks, and GitHub Actions CI.

## Context

`rsahp` is a full-stack Rust workspace: `frontend` (egui/eframe, native), `backend` (axum + sea-orm/SQLite), `common`, and an `xtask` crate. Existing tooling already in place:

- A custom `cargo xtask` "cockpit" TUI task runner (quick loop, quality gate, tests, fullscale build+commit+push) built on `dialoguer`/`xshell`/`sysinfo`.
- `lefthook` git hooks: pre-commit `fmt` + `clippy` + `test`; pre-push `verify.ps1`.
- Workspace-wide clippy: `all` + `pedantic` + `nursery` + `cargo`, plus `unwrap_used`/`expect_used` = warn.
- GitHub Actions CI on windows/ubuntu/macos: fmt, clippy, test, release build, and a `verify.ps1` contract test.
- `egui_kittest` headless UI tests; `approx` for AHP float assertions.
- `rtk` token-optimizing CLI proxy (global, hook-driven).

This spec **adds** tools that fill genuine gaps. It does **not** add `just` — the pure-Rust, cross-platform `xtask` cockpit already owns the task-runner role, so `just` would be redundant. `trunk`/`wasm-bindgen` are also excluded: there are no wasm/trunk artifacts today and the frontend ships native. `sccache` was considered and deferred (only worth it if CI minutes become a pain point). `cargo-insta` was considered and deferred (`approx` already covers the float-comparison need; insta would be additive, not gap-filling).

Design forks were consulted with the `agy` peer (AGY-FIRST). The tool selection converged. The migration-baseline fork was negotiated: the driver's entity-derived-`m0` proposal was withdrawn after agy demonstrated it breaks the init pipeline (see WS3).

## Goals

- Faster, cleaner test runs and line-coverage reporting tied to `TESTING_STRATEGY.md`.
- Supply-chain / license / hygiene gates in CI.
- Versioned, reproducible, immutable database schema history replacing today's imperative in-code schema build.
- Proactive discoverability of the tools for future sessions.

## Non-Goals

- Adding `just`, `trunk`/`wasm-bindgen`, `sccache`, or `cargo-insta` (rationale above).
- Changing the AHP math, API surface, or frontend behavior.
- Migrating the database engine off SQLite.

## Workstream 1 — Test runner + coverage

**Tools:** `cargo-nextest`, `cargo-llvm-cov`.

- Add `.config/nextest.toml` with a `default` profile and a `ci` profile (JUnit output for CI).
- `xtask` (verified structure: menu item `[3] "QUALITY GATE: Run Unit Tests"` at `xtask/src/main.rs:33` is a **single combined branch** running `cargo fmt` + `cargo clippy` + `cargo test` at lines 58–69; there is **not** a separate standalone "Run Unit Tests" action despite the README listing four items — the plan must reconcile README ↔ code):
  - In that combined branch, replace `cargo test` with `cargo nextest run`.
  - Add a separate "Coverage" menu item running `cargo llvm-cov nextest`. If the `cargo-llvm-cov` binary is absent locally, this item prints an install hint (`cargo install cargo-llvm-cov`) and returns cleanly — it does **not** crash or hide silently.
- `lefthook.yml`: pre-commit `test` command → `cargo nextest run`.
- `.github/workflows/ci.yml` — because matrix jobs share step definitions, coverage must be **two OS-branched steps**, not a single replacement (a bare replacement would force Windows/macOS to run `llvm-cov` without `llvm-tools-preview` and fail):
  - Add `llvm-tools-preview` to the toolchain `components` (currently only `rustfmt, clippy` at `ci.yml:30`) — **`cargo llvm-cov` fails without it.**
  - Install `cargo-nextest` on all runners and `cargo-llvm-cov` (e.g. via `taiki-e/install-action`).
  - Linux step (`if: runner.os == 'Linux'`): `cargo llvm-cov nextest --lcov --output-path lcov.info`, then upload `lcov.info` as an artifact.
  - Windows/macOS step (`if: runner.os != 'Linux'`): plain `cargo nextest run` (no coverage).
  - The old unconditional `cargo test` step is removed on all three OSes.

**Notes / risks:**
- `egui_kittest` headless UI tests run fine under nextest — **verified**: `backend/tests/common/mod.rs:22` and `frontend/tests/headless_e2e.rs:17,31` use `sqlite::memory:` + ephemeral `127.0.0.1:0`, so nextest's parallel process-per-test isolation causes no DB/port contention.
- **Hard prerequisite:** repointing xtask + lefthook pre-commit at `cargo nextest run` breaks a fresh clone that lacks `cargo-nextest` (the recommended-tools hook surfaces but never auto-installs). Mitigation: document nextest as a required dev tool in the README **and** have the xtask branch fall back to `cargo test` if the `cargo-nextest` binary is absent (probe once, warn, degrade). Lefthook has no clean fallback — it is documented as a hard prerequisite.
- nextest does **not** run doctests. **Verified: 0 doctests exist today**, so no coverage is lost now. Add a `cargo test --doc` step only if doctests are introduced later (note this in CONTRIBUTING/TESTING docs so a future doctest is not silently skipped).

## Workstream 2 — Supply-chain / hygiene gates

**Tools:** `cargo-deny`, `cargo-machete`, `typos`.

- `deny.toml` at workspace root:
  - `advisories`: deny known vulnerabilities.
  - `bans`: warn on duplicate crate versions.
  - `licenses`: allowlist permissive licenses (MIT, Apache-2.0, BSD-2/3-Clause, Unicode, ISC, and any others the current dep tree legitimately needs — enumerated during implementation from `cargo deny check licenses` output). Workspace member crates are treated as private (not license-checked).
- `typos.toml`: seed the dictionary with domain words (`rsahp`, `ahp`, `egui`, `eframe`, `axum`, `nalgebra`, `sea-orm`, `eigenvector`, `AcroForm`, etc.) discovered during a first `typos` run. **Also set `extend-exclude`** to skip files that will otherwise false-positive CI: `Cargo.lock` (base64 checksums, transitive-crate and author names), `rsahp.db`, `output.mp4`, `*.log`, `target/`, and coverage output (`lcov.info`, `*.profraw`). The coverage artifacts (`lcov.info`, `*.profraw`, `target/llvm-cov-target/`) must **also be added to `.gitignore`** — a `--lcov` run locally or a CI checkout leaving `lcov.info` at root would otherwise both pollute git status and drown the local `typos`/quality-gate in base64 false-positives.
- `cargo-machete`: run at workspace root; add `[package.metadata.cargo-machete] ignored = [...]` per-crate only if a macro-only dependency false-positives.

**Wiring:**
- All three run as separate **CI steps**, each of which must **fail the build on findings** (`cargo deny check`, `typos`, and `cargo machete` all exit non-zero by default — the plan asserts this rather than assuming it). **CI is the authoritative gate.**
- They are also folded into the xtask combined quality-gate branch (`xtask/src/main.rs:58-69`), but note that branch invokes commands with `.run().ok()` (failures are swallowed locally, e.g. `cargo test` at line 69) — so the local copy is **advisory**, not gating. This is acceptable because CI gates; the plan should not rely on the xtask gate to block anything.
- Kept **out** of pre-commit so local commits stay fast.
- The license allowlist and typos dictionary must be **derived from a first real run** of each tool and committed with the actual values — not shipped empty (an empty license allowlist denies everything).

## Workstream 3 — Versioned DB migrations

**Tools:** `sea-orm-migration` (crate), `sea-orm-cli` (dev tool for generating future migration skeletons).

**Current state:** `backend/src/lib.rs` builds the schema imperatively at startup via `schema.create_table_from_entity(entity::X::Entity)` for 9 entities: `user_group`, `user`, `folder`, `document`, `node`, `comparison`, `user_group_membership`, `document_user_assignment`, `document_group_assignment`. A populated dev DB (`rsahp.db`) exists.

**Approach (decided — option A, hand-written baseline):**

- Add a new `migration/` workspace crate depending on `sea-orm-migration`.
- Write a single initial migration `m0` that creates all 9 tables using the migration schema builder (`Table::create`), reproducing the current schema exactly — including primary keys, column types, nullability, and every foreign key with its `on_delete` behavior.
- Replace the `create_table_from_entity` startup loop in `backend/src/lib.rs` with `Migrator::up(&db, None)`.

**Why hand-written, not entity-derived:** an entity-derived `m0` (`manager.create_table(Schema::new(..).create_table_from_entity(E))`) reads the *live* entity crate at run time, making the baseline mutable. When the first real change ships (e.g. add `email` to `User` + write `m1 = ALTER TABLE ADD email`), a fresh clone runs `m0` against the *updated* entity — creating the table already containing `email` — then `m1` re-adds it → hard `SQLite: duplicate column` crash. Migrations must be immutable snapshots of history; the hand-written baseline is decoupled from the entity crate and cannot drift this way.

**Existing-DB transition (required — this is a startup-crash risk):** an existing `rsahp.db` already contains the 9 tables (built by the old `create_table_from_entity` loop) but has **no `seaql_migrations` tracking table**. Running `Migrator::up` against it will re-issue `CREATE TABLE` and fail with `table already exists`, panicking at startup. Documentation alone is insufficient — a developer who runs `git pull && cargo run` never reads the plan and hits a cryptic `DbErr`. Therefore the backend init must **detect this specific condition** (tables present, `seaql_migrations` absent) and, in debug builds, emit a loud, human-readable terminal message explaining the migration cutover and instructing the developer to delete `rsahp.db` (or provide a `cargo xtask` reset action that does it). The plan must still document the reset step and the data-loss warning (see Blindspot note), but the in-code guard is what actually prevents the cryptic crash.

**Safety nets (both required) — comparison must be SEMANTIC, not textual:**

`create_table_from_entity` and hand-written `Table::create` emit equivalent-but-textually-different `CREATE TABLE` DDL (column ordering, quoting, auto-generated constraint names), and SQLite preserves the original text in `sqlite_master`. A raw SQL/text diff therefore produces **false mismatches** even when the schemas are identical. Both safety nets below must compare **normalized structural metadata**, not DDL text: for each table, compare `PRAGMA table_info` (name, type, notnull, dflt_value, pk), `PRAGMA foreign_key_list` (columns, referenced table/column, on_delete/on_update), and index metadata — as order-insensitive sets.

**The comparison MUST exclude the `seaql_migrations` table.** The migrations-built DB contains sea-orm's `seaql_migrations` tracking table; the entity-built DB does not. A naive full-table-set comparison would therefore report a mismatch on *every* run, making the drift-guard useless. Restrict the compared table set to the 9 application tables (or explicitly filter out `seaql_migrations`).

1. **Baseline verification (one-time, during implementation):** build one DB from the current `create_table_from_entity` path and one from `m0`, then compare the two via the PRAGMA-based structural comparison above before declaring `m0` correct. This catches any missed column type, length, nullability, or `on_delete` rule (agy's flagged gotcha).
2. **Drift-guard test (ongoing, in CI):** an automated test that builds a fresh in-memory DB from the live entities (`create_table_from_entity`) and another from the migrations, then asserts structural equality via the same PRAGMA-based comparison. This catches "future entity edit without a corresponding migration" automatically — giving immutability *plus* a regression net. **This test is load-bearing:** it is the entire reason the immutable hand-written baseline was chosen over entity-derived, so a naive text-diff implementation (perpetually red → ignored, or lenient → blind) would forfeit WS3's justification.

**Keep the entity-schema builder alive as a test-only helper.** Once startup switches to `Migrator::up`, the `create_table_from_entity` code path is dead in production but is still required by the drift-guard test to build the entity-side schema. Given this repo's clippy/dead-code-cleanup culture, wrap it in a clearly-named, documented test helper (e.g. `#[cfg(test)] fn entity_schema_db()`) so a future "remove unused code" pass does not silently delete the drift-guard's foundation.

**Exhaustive-registration limitation (documented, partially mitigated).** The entity side is built from an explicit list of entities. A brand-new 10th entity that a developer forgets to add a migration for *and* forgets to add to that list would leave the test comparing an incomplete-but-matching set (both sides have the same 9 tables) and pass — a false green. Full closure isn't possible because sea-orm entities have no runtime registry to enumerate. Mitigations required by the plan: (a) define the entity list in **one canonical constant** reused by the test helper (no second copy to drift); (b) the drift-guard additionally asserts the **table-name set** of the entity-built DB equals the migration-built DB's set (excluding `seaql_migrations`), so an entity added to the list without a migration — or a migration added without the list — is caught. The residual gap (a new entity omitted from *both* the migration and the canonical list) is explicitly a code-review responsibility, noted in CONTRIBUTING/TESTING.

**Notes / risks:**
- **Version-lock `sea-orm-migration` to `sea-orm` (1.1.20)** — they release in lockstep; a mismatched major breaks the build.
- Migration failures at startup must surface a clear error rather than being swallowed (the current bind path already uses `unwrap`/`expect`; keep migration errors loud and fatal, not silent).
- Existing dev `rsahp.db` is disposable dev data. **No production data migration is in scope**, but the reset is destructive — the plan must include an explicit "back up or expect data loss" warning and the exact reset command.
- SQLite's limited `ALTER TABLE` support constrains future migrations (documented for later, not a blocker for `m0`).

## Cross-cutting — Tool discoverability

Add `.claude/recommended-tools.json` registering each tool as `{name, why, install, in_path}` so the SessionStart tooling-check hook proactively surfaces any that are missing. The `in_path` probe must use the **exact installed binary name**, not the cargo-subcommand alias — otherwise the check never matches:

| Tool | `in_path` probe binary | `install` |
|---|---|---|
| cargo-nextest | `cargo-nextest` | `cargo install cargo-nextest --locked` |
| cargo-llvm-cov | `cargo-llvm-cov` | `cargo install cargo-llvm-cov --locked` |
| cargo-deny | `cargo-deny` | `cargo install cargo-deny --locked` |
| cargo-machete | `cargo-machete` | `cargo install cargo-machete --locked` |
| typos | `typos` | `cargo install typos-cli --locked` |
| sea-orm-cli | `sea-orm-cli` | `cargo install sea-orm-cli --locked` |

Note the crate/binary name mismatch for typos (crate `typos-cli`, binary `typos`).

## Verification / acceptance

- `cargo xtask` cockpit runs tests via nextest and exposes a working Coverage action.
- `cargo llvm-cov nextest` produces `lcov.info`.
- `cargo deny check`, `cargo machete`, and `typos` all pass at workspace root and gate CI.
- `cargo run --bin backend` initializes the schema via `Migrator::up` against a fresh DB; the **PRAGMA-based structural comparison** (table_info + foreign_key_list + indexes, excluding `seaql_migrations`) between the entity-built schema and the migrated schema reports no differences across all 9 application tables.
- On an existing pre-migration `rsahp.db`, backend startup emits the human-readable cutover message instead of a cryptic `DbErr` panic.
- The drift-guard test passes and would fail if an entity is edited without a migration.
- CI is green on all three OSes.
- `.claude/recommended-tools.json` present and valid.

## Implementation ordering

1. WS1 (nextest + llvm-cov) — lowest risk, immediate everyday value.
2. WS2 (deny + machete + typos) — config + CI wiring; fix any real findings surfaced.
3. WS3 (migrations) — the meatier refactor; do last so the improved test/coverage tooling is already available to validate it.
4. Cross-cutting `recommended-tools.json` alongside each tool as it lands.
