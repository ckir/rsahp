# Dev Tooling Additions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add gap-filling developer tooling to the `rsahp` Rust workspace — a faster test runner + coverage (nextest/llvm-cov), supply-chain/hygiene CI gates (deny/machete/typos), and versioned immutable DB migrations (sea-orm-migration) — wired into the existing `xtask` cockpit, `lefthook`, and GitHub Actions CI.

**Architecture:** Three independent workstreams landed in order WS1 → WS2 → WS3, plus a cross-cutting `.claude/recommended-tools.json`. WS1/WS2 are config + wiring. WS3 replaces the imperative in-code `create_table_from_entity` schema build with a hand-written immutable `m0` baseline run via `Migrator::up`, guarded by a PRAGMA-based structural drift test. CI is the authoritative gate; local `xtask`/`lefthook` copies are fast/advisory where noted.

**Tech Stack:** Rust 2024 workspace (`frontend` egui/eframe, `backend` axum + sea-orm 1.1.20/SQLite, `common`, `xtask`), lefthook, GitHub Actions, `dialoguer`/`xshell`/`sysinfo` (xtask).

**Source spec:** `docs/superpowers/specs/2026-07-13-dev-tooling-additions-design.md` (panel-GREEN).

**Review status:** Passed adversarial-panel-review **GREEN** (2 rounds, solo + agy second-model). Round 1 folded 11 findings (agy caught 2 novel ones: the `#[cfg(test)]`-across-integration-test compile break, and the error-swallowing seed defeating the S1-fenced forcing-function premise); round 2 landed clean. Fold ledger below.

<details><summary><b>Panel fold ledger (round 1 → applied)</b></summary>

1. Cutover guard made **unconditional** (was debug-only; release would panic since `m0` has no `IF NOT EXISTS`) — Task 11.
2. Dev seed rewritten raw-SQL `INSERT OR IGNORE` → **ActiveModel existence-checked inserts** (compile-time forcing function; errors propagate, no silent swallow) — Task 11.
3. Drift test **relocated** from `backend/tests/` integration test → inline `#[cfg(test)] mod` in `lib.rs` (integration crates link the lib without `cfg(test)`, stripping the helpers) — Task 12.
4. Test `fresh_db()` pins `max_connections(1)` (`sqlite::memory:` is per-connection) — Task 12.
5. Explicit `async-trait` dep added; hand-wavy fallback note removed — Task 10.
6. Linux coverage uses `NEXTEST_PROFILE=ci` env, not `--profile` (flag collision) — Task 4.
7. `taiki-e/install-action` typos key `typos-cli` → `typos` — Task 9.
8. JUnit dead config dropped from nextest.toml — Task 1.
9. Verification no longer hard-depends on the `sqlite3` CLI — Task 13.
10. CI-trigger note: feature-branch pushes don't run CI; verify on the PR to `master` — Final verification.
11. `entity_schema_db` mirrors the proven `builder.build(...if_not_exists())` pattern — Task 11.

</details>

---

## ⚠️ Spec-vs-code reconciliations (read before starting)

While verifying the spec against the live code, three things diverged from the spec text. **The live code wins**; this plan is authored against the code as it exists now:

1. **The xtask "Run Unit Tests" action is STANDALONE, not combined.** The spec (WS1, line 40) claims menu item `[3] "QUALITY GATE: Run Unit Tests"` is a *single combined branch running fmt+clippy+test* and that there is *no separate standalone Run Unit Tests action*. **This is backwards.** In `xtask/src/main.rs` the match arms are separate: arm `1` (menu `[2] Format & Lint`) runs `cargo fmt` + `cargo clippy` only (L57–66); arm `2` (menu `[3] Run Unit Tests`) runs `cargo test` alone (L67–70). `README.md:37` agrees ("Run Unit Tests: Executes `cargo test`"). So the nextest swap targets the standalone test arm — simpler than the spec assumed.
2. **The 5 `ALTER TABLE ADD COLUMN` shims in `setup_schema` are no-ops on a fresh DB.** All 5 added columns (`document.folder_id`, `node.cost`, `user.is_deleted`, `user_group.parent_id`, `comparison.respondent_id`) already exist in the current entity structs, so `create_table_from_entity` already creates the full modern schema. The ALTERs only mattered for old DBs and are dropped in the WS3 rewrite (old DBs are handled by the reset/cutover guard).
3. **Seed data must survive the cutover (spec gap, now resolved).** `setup_schema` also runs 3 `INSERT OR IGNORE` rows (admin group/user/membership, L119–139) that enable out-of-box dev login. A naive `setup_schema`→`Migrator::up` replacement would drop them. **Decision (AGY-FIRST consult + negotiation, user-approved): S1-fenced** — keep the seed as ordinary raw-SQL `INSERT OR IGNORE` in `setup_schema`, *after* `Migrator::up`, wrapped in `#[cfg(debug_assertions)]` so it is physically absent from release builds. Migration crate stays pure schema history.

Also note: `TESTING_STRATEGY.md:28` documents pre-commit as `cargo test --lib`, but the actual `lefthook.yml` runs `cargo test` (full). This plan swaps the actual `cargo test` → `cargo nextest run`; align the doc in Task 5.

---

## File structure

**Created:**
- `.config/nextest.toml` — nextest `default` + `ci` profiles.
- `deny.toml` — cargo-deny advisories/bans/licenses config.
- `typos.toml` — typos dictionary + `extend-exclude`.
- `.claude/recommended-tools.json` — SessionStart tooling-check registrations.
- `migration/Cargo.toml`, `migration/src/lib.rs`, `migration/src/m0_initial.rs` — new migration crate.

**Modified:**
- `xtask/src/main.rs` — nextest swap + Coverage item + advisory hygiene item + nextest/llvm-cov binary-probe fallbacks.
- `lefthook.yml:9` — pre-commit `test` → `cargo nextest run`.
- `.github/workflows/ci.yml` — llvm-tools-preview component, tool installs, OS-branched coverage steps, gating deny/machete/typos steps, remove unconditional `cargo test`.
- `.gitignore` — coverage artifacts.
- `Cargo.toml:2-7` — add `migration` to workspace members.
- `backend/Cargo.toml` — add `migration` path dep; `sea-orm-migration` dev-dep for the drift test.
- `backend/src/lib.rs` — `setup_schema` rewrite (Migrator::up + fenced seed + cutover guard); `#[cfg(test)]` entity-schema helper + canonical entity list + drift-guard test.
- `README.md:37` + `TESTING_STRATEGY.md:28` + `CONTRIBUTING`/testing docs — nextest, reset command, doctest + registration notes.

---

## WS1 — Test runner + coverage (nextest + llvm-cov)

### Task 1: nextest config + coverage-artifact gitignore

**Files:**
- Create: `.config/nextest.toml`
- Modify: `.gitignore`

- [ ] **Step 1: Create `.config/nextest.toml`**

```toml
# Nextest configuration. Docs: https://nexte.st/book/configuration.html
[profile.default]
# Fail fast is off so a full local run reports every failure at once.
fail-fast = false

[profile.ci]
# CI profile: retry flaky headless UI tests once.
fail-fast = false
retries = 1
```

- [ ] **Step 2: Append coverage artifacts to `.gitignore`**

Add these lines to the end of `.gitignore` (current contents end at `.serena/`):

```gitignore
# Coverage output (cargo-llvm-cov)
lcov.info
*.profraw
/target/llvm-cov-target/
```

- [ ] **Step 3: Verify nextest reads the config**

Run: `cargo nextest list --profile ci`
Expected: nextest lists test binaries with no config-parse error (proves `.config/nextest.toml` is valid). If `cargo-nextest` is not installed, install it first: `cargo install cargo-nextest --locked`.

- [ ] **Step 4: Commit**

```bash
git add .config/nextest.toml .gitignore
git commit -m "feat(ws1): add nextest config and ignore coverage artifacts"
```

---

### Task 2: Repoint xtask at nextest + add Coverage action (with binary-probe fallbacks)

**Files:**
- Modify: `xtask/src/main.rs` (menu array L30–37; match arms L50–86; helpers)

**Context (Step 0 — verify before editing):** Open `xtask/src/main.rs`. Confirm: the `selections` array (L30–37) has 6 entries `[1]..[5]` + `[0] Quit`; match arm `2` (L67–70) is `println!("=== Running Tests ==="); cmd!(sh, "cargo test").run().ok();`; the loop `match selection { 0..=5 }` ends with `_ => unreachable!()` at L85. If any differs, STOP and report `STATE_MISMATCH: <what>`.

- [ ] **Step 1: Add a binary-probe helper** (insert after the `use` block, before `fn main`)

`cargo-nextest`/`cargo-llvm-cov` are external subcommands; probe the actual binary name (not the `cargo <sub>` alias) so a fresh clone degrades gracefully instead of erroring.

```rust
/// Returns true if an external command binary is resolvable on PATH.
fn binary_present(bin: &str) -> bool {
    #[cfg(windows)]
    let (probe, arg) = ("where", bin);
    #[cfg(not(windows))]
    let (probe, arg) = ("which", bin);
    Command::new(probe)
        .arg(arg)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
```

- [ ] **Step 2: Add two menu items** — replace the `selections` array (L30–37) with:

```rust
    let selections = &[
        "[1] INNER LOOP: Quick Test (Build & Launch)",
        "[2] QUALITY GATE: Format & Lint Workspace",
        "[3] QUALITY GATE: Run Unit Tests",
        "[4] QUALITY GATE: Coverage Report (llvm-cov)",
        "[5] SHIP & RELEASE: Fullscale Workflow (Commit & Push)",
        "[6] SHIP & RELEASE: Version Bump (lockstep)",
        "[0] Quit",
    ];
```

- [ ] **Step 3: Rewire the match arms** — replace the whole `match selection { ... }` block (L50–86) with the following. Arms 0–2 keep their behavior except arm 2 now uses nextest with a `cargo test` fallback; new arm 3 is Coverage; arms 4/5/6 are the old fullscale/version-bump/quit shifted down by one:

```rust
        match selection {
            0 => {
                if let Err(e) = quick(sh) {
                    println!("Error: {e}");
                }
            }
            1 => {
                println!("=== Formatting ===");
                cmd!(sh, "cargo fmt").run().ok();
                println!("=== Linting ===");
                cmd!(
                    sh,
                    "cargo clippy --workspace --all-targets --all-features -- -D warnings"
                )
                .run()
                .ok();
            }
            2 => {
                println!("=== Running Tests ===");
                if binary_present("cargo-nextest") {
                    cmd!(sh, "cargo nextest run").run().ok();
                } else {
                    println!(
                        "cargo-nextest not found — falling back to `cargo test`. \
                         Install it for faster runs: cargo install cargo-nextest --locked"
                    );
                    cmd!(sh, "cargo test").run().ok();
                }
            }
            3 => {
                println!("=== Coverage (llvm-cov + nextest) ===");
                if binary_present("cargo-llvm-cov") {
                    cmd!(sh, "cargo llvm-cov nextest").run().ok();
                } else {
                    println!(
                        "cargo-llvm-cov not found. Install it to generate coverage: \
                         cargo install cargo-llvm-cov --locked \
                         (also needs the llvm-tools-preview rustup component)"
                    );
                }
            }
            4 => {
                if let Err(e) = fullscale(sh) {
                    println!("Error: {e}");
                }
            }
            5 => {
                if let Err(e) = version_bump(sh) {
                    println!("Error: {e}");
                }
            }
            6 => {
                println!("Exiting Cockpit...");
                break;
            }
            _ => unreachable!(),
        }
```

- [ ] **Step 4: Repoint the `quick()` lefthook-driven test path is unaffected** — no change to `quick()`; it invokes `lefthook run pre-commit` which Task 3 repoints. Nothing to edit here; this step is a confirmation only.

- [ ] **Step 5: Build xtask and smoke-test the menu compiles**

Run: `cargo build --package xtask`
Expected: `Finished` with no warnings (workspace clippy is `-D warnings` in CI, so also run:)
Run: `cargo clippy --package xtask --all-targets -- -D warnings`
Expected: `Finished` / no issues.

- [ ] **Step 6: Commit**

```bash
git add xtask/src/main.rs
git commit -m "feat(ws1): xtask runs tests via nextest, adds coverage action with probes"
```

---

### Task 3: Repoint lefthook pre-commit at nextest

**Files:**
- Modify: `lefthook.yml:8-9`

**Context (Step 0):** Open `lefthook.yml`. Confirm L8–9 is:
```yaml
    test:
      run: cargo test
```
If different, STOP and report `STATE_MISMATCH`.

- [ ] **Step 1: Change the test command**

Replace `run: cargo test` (L9) with:

```yaml
    test:
      run: cargo nextest run
```

- [ ] **Step 2: Verify lefthook runs the hook end-to-end**

Run: `lefthook run pre-commit`
Expected: `fmt`, `clippy`, and `test` all report success; the `test` step invokes `cargo nextest run`. (nextest is a **hard prerequisite** for pre-commit — there is no fallback here by design; documented in Task 5.)

- [ ] **Step 3: Commit**

```bash
git add lefthook.yml
git commit -m "feat(ws1): lefthook pre-commit uses nextest"
```

---

### Task 4: CI — coverage (OS-branched) + toolchain component + tool installs

**Files:**
- Modify: `.github/workflows/ci.yml` (components L30; steps L40–41)

**Context (Step 0):** Open `.github/workflows/ci.yml`. Confirm: `components: rustfmt, clippy` at L30; the test step at L40–41 is `- name: Run All Tests (including Headless UI)` / `run: cargo test`. If different, STOP and report `STATE_MISMATCH`.

- [ ] **Step 1: Add `llvm-tools-preview` to the toolchain components** — replace L27–30:

```yaml
    - name: Set up Rust
      uses: dtolnay/rust-toolchain@stable
      with:
        components: rustfmt, clippy, llvm-tools-preview
```

- [ ] **Step 2: Install nextest + llvm-cov on all runners** — insert a new step immediately after the `Cache dependencies` step (after L33), before `Lint and Format`:

```yaml
    - name: Install test tooling
      uses: taiki-e/install-action@v2
      with:
        tool: cargo-nextest,cargo-llvm-cov
```

- [ ] **Step 3: Replace the unconditional test step** — replace L40–41 (`- name: Run All Tests ...` / `run: cargo test`) with two OS-branched steps:

```yaml
    - name: Run tests with coverage (Linux)
      if: runner.os == 'Linux'
      env:
        # Select the `ci` nextest profile via env — `cargo llvm-cov nextest --profile`
        # would collide with cargo-llvm-cov's own build-profile flag.
        NEXTEST_PROFILE: ci
      run: cargo llvm-cov nextest --lcov --output-path lcov.info

    - name: Run tests (Windows/macOS)
      if: runner.os != 'Linux'
      run: cargo nextest run --profile ci
```

- [ ] **Step 4: Upload the coverage artifact (Linux only)** — insert after the two test steps, before `- name: Build Everything`:

```yaml
    - name: Upload coverage
      if: runner.os == 'Linux'
      uses: actions/upload-artifact@v4
      with:
        name: coverage-lcov
        path: lcov.info
        if-no-files-found: warn
```

- [ ] **Step 5: Validate the workflow YAML**

Run: `cargo xtask` is not for this; instead confirm YAML parses. If `yq` is available: `yq '.jobs.build_and_verify.steps | length' .github/workflows/ci.yml` (expected: a number, no parse error). Otherwise visually confirm indentation matches the surrounding steps.

- [ ] **Step 6: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "feat(ws1): CI installs nextest+llvm-cov, runs OS-branched coverage"
```

---

### Task 5: Docs — nextest as required tool, cockpit menu, doctest note

**Files:**
- Modify: `README.md:34-38`
- Modify: `TESTING_STRATEGY.md:28`

- [ ] **Step 1: Update the README cockpit menu list** — replace L35–38 with the reordered/updated list:

```markdown
1. **Quick Loop**: Instantly builds the workspace, kills old background processes, and relaunches the frontend and backend.
2. **Quality Gate**: Runs `cargo fmt` and `cargo clippy` over the workspace.
3. **Run Unit Tests**: Executes `cargo nextest run` (falls back to `cargo test` if `cargo-nextest` is not installed).
4. **Coverage Report**: Runs `cargo llvm-cov nextest` (prints an install hint if `cargo-llvm-cov` is absent).
5. **Fullscale Workflow**: Runs tests, formats, builds, launches, commits with an interactive message prompt, and pushes to GitHub (hooking into `lefthook`).
6. **Version Bump**: Bumps the workspace version in lockstep across all crates.
```

- [ ] **Step 2: Add a "Required dev tools" note** — insert after L32 (after the ```` ``` ```` closing the `cargo xtask` code block, before L34 "You will be presented..."):

```markdown
> **Required dev tools:** `cargo-nextest` is a hard prerequisite for the `lefthook` pre-commit hook (`cargo install cargo-nextest --locked`). `cargo-llvm-cov` is optional (coverage only). The SessionStart hook (`.claude/recommended-tools.json`) surfaces any missing tools. Note: nextest does **not** run doctests — there are 0 doctests today, but if you add one, also add a `cargo test --doc` step so it is not silently skipped.
```

- [ ] **Step 3: Fix the pre-commit doc drift in `TESTING_STRATEGY.md`** — replace L28:

```markdown
- **Pre-Commit Hook**: Runs lightning-fast checks (`cargo fmt`, `cargo clippy`, and `cargo nextest run`). Commits are blocked if these fail.
```

- [ ] **Step 4: Commit**

```bash
git add README.md TESTING_STRATEGY.md
git commit -m "docs(ws1): document nextest/coverage cockpit items and prerequisites"
```

---

## WS2 — Supply-chain / hygiene gates (deny + machete + typos)

### Task 6: `deny.toml` derived from a real run

**Files:**
- Create: `deny.toml`

- [ ] **Step 1: Generate the baseline license list** — run cargo-deny against the current tree to see exactly which licenses the dep graph uses:

Run: `cargo deny init` then `cargo deny check licenses 2>&1 | tee scratch-deny.txt` (install first if needed: `cargo install cargo-deny --locked`). Read the output for every license the tree actually contains.

- [ ] **Step 2: Write `deny.toml`** — start from this template and extend the `licenses.allow` list with **every** license the Step 1 output reported (do not ship a shorter list — an incomplete allowlist fails the build):

```toml
# cargo-deny configuration. Docs: https://embarkstudios.github.io/cargo-deny/
[advisories]
version = 2
# Deny any crate with a known security advisory.
yanked = "deny"

[bans]
# Warn (not deny) on duplicate crate versions — informational only.
multiple-versions = "warn"

[licenses]
version = 2
# Allowlist derived from `cargo deny check licenses` on the current tree.
# EXTEND this list with the actual licenses from Step 1 output before committing.
allow = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Unicode-3.0",
    "Unicode-DFS-2016",
    "Zlib",
]
# Workspace member crates carry no SPDX license expression; treat them as private.
[licenses.private]
ignore = true
```

- [ ] **Step 3: Verify deny passes and fails correctly**

Run: `cargo deny check`
Expected: `advisories ok`, `bans ok`, `licenses ok` (exit 0). If a license is reported as not-allowed, add it to `allow` and re-run. Confirm non-zero exit on findings is the default (do not add `--allow` flags).

- [ ] **Step 4: Clean up + commit**

```bash
rm -f scratch-deny.txt
git add deny.toml
git commit -m "feat(ws2): add cargo-deny config with derived license allowlist"
```

---

### Task 7: `typos.toml` derived from a real run

**Files:**
- Create: `typos.toml`

- [ ] **Step 1: Run typos to discover real false-positives**

Run: `typos 2>&1 | tee scratch-typos.txt` (install first if needed: `cargo install typos-cli --locked` — note crate is `typos-cli`, binary is `typos`). Read every reported "typo" — the domain words (`rsahp`, `ahp`, `eframe`, `nalgebra`, `eigenvector`, etc.) and any binary/lockfile noise.

- [ ] **Step 2: Write `typos.toml`** — seed `default.extend-words` with the real domain words from Step 1, and `files.extend-exclude` with the noise files:

```toml
# typos configuration. Docs: https://github.com/crate-ci/typos
[files]
# Exclude generated/binary files that produce base64/hash false-positives.
extend-exclude = [
    "Cargo.lock",
    "rsahp.db",
    "output.mp4",
    "*.log",
    "target/",
    "lcov.info",
    "*.profraw",
]

[default.extend-words]
# Domain vocabulary — EXTEND from the Step 1 output. Each is `word = "word"` (accept as-is).
rsahp = "rsahp"
ahp = "ahp"
egui = "egui"
eframe = "eframe"
axum = "axum"
nalgebra = "nalgebra"
saaty = "saaty"
eigenvector = "eigenvector"
```

- [ ] **Step 3: Verify typos passes**

Run: `typos`
Expected: exit 0, no reported typos. If any real domain word is still flagged, add it to `extend-words` and re-run.

- [ ] **Step 4: Clean up + commit**

```bash
rm -f scratch-typos.txt
git add typos.toml
git commit -m "feat(ws2): add typos config with derived dictionary and excludes"
```

---

### Task 8: xtask advisory hygiene action

**Files:**
- Modify: `xtask/src/main.rs` (menu array + match arms from Task 2)

**Context (Step 0):** This builds on Task 2's edits. Confirm the `selections` array now has the 7 entries ending in `[0] Quit` and match arms `0..=6`. If Task 2 is not yet applied, STOP and report `STATE_MISMATCH`.

- [ ] **Step 1: Add a hygiene menu item** — insert `"[7] QUALITY GATE: Supply-chain & Hygiene (advisory)"` into the `selections` array, before `"[0] Quit"`, and renumber Quit's match arm. Replace the array with:

```rust
    let selections = &[
        "[1] INNER LOOP: Quick Test (Build & Launch)",
        "[2] QUALITY GATE: Format & Lint Workspace",
        "[3] QUALITY GATE: Run Unit Tests",
        "[4] QUALITY GATE: Coverage Report (llvm-cov)",
        "[5] QUALITY GATE: Supply-chain & Hygiene (advisory)",
        "[6] SHIP & RELEASE: Fullscale Workflow (Commit & Push)",
        "[7] SHIP & RELEASE: Version Bump (lockstep)",
        "[0] Quit",
    ];
```

- [ ] **Step 2: Insert the hygiene arm and shift the tail** — the match block must become arms `0..=7`. Replace arms `4,5,6` (fullscale/version/quit from Task 2) with `5,6,7`, and insert new arm `4`:

```rust
            4 => {
                println!("=== Supply-chain & Hygiene (advisory — CI is the gate) ===");
                if binary_present("cargo-deny") {
                    cmd!(sh, "cargo deny check").run().ok();
                } else {
                    println!("cargo-deny not found: cargo install cargo-deny --locked");
                }
                if binary_present("cargo-machete") {
                    cmd!(sh, "cargo machete").run().ok();
                } else {
                    println!("cargo-machete not found: cargo install cargo-machete --locked");
                }
                if binary_present("typos") {
                    cmd!(sh, "typos").run().ok();
                } else {
                    println!("typos not found: cargo install typos-cli --locked");
                }
            }
            5 => {
                if let Err(e) = fullscale(sh) {
                    println!("Error: {e}");
                }
            }
            6 => {
                if let Err(e) = version_bump(sh) {
                    println!("Error: {e}");
                }
            }
            7 => {
                println!("Exiting Cockpit...");
                break;
            }
            _ => unreachable!(),
```

Note: all commands use `.run().ok()` — locally **advisory** (never blocks); CI (Task 9) is the authoritative gate.

- [ ] **Step 3: Build + clippy**

Run: `cargo clippy --package xtask --all-targets -- -D warnings`
Expected: no issues.

- [ ] **Step 4: Commit**

```bash
git add xtask/src/main.rs
git commit -m "feat(ws2): add advisory supply-chain/hygiene action to xtask cockpit"
```

---

### Task 9: CI — three gating hygiene steps

**Files:**
- Modify: `.github/workflows/ci.yml`

**Context (Step 0):** Confirm Task 4's `Install test tooling` step exists. If not, STOP.

- [ ] **Step 1: Install the hygiene tools** — extend the `tool:` list in the `Install test tooling` step (from Task 4) to include all five:

```yaml
    - name: Install test tooling
      uses: taiki-e/install-action@v2
      with:
        # NOTE: taiki-e/install-action's tool key for typos is `typos` (NOT the crate
        # name `typos-cli` — that key is unrecognized). The `cargo install` form still
        # uses `typos-cli`.
        tool: cargo-nextest,cargo-llvm-cov,cargo-deny,cargo-machete,typos
```

- [ ] **Step 2: Add three gating steps** — insert after the `Lint and Format` step (after L39 in the original numbering, i.e. before the test steps):

```yaml
    - name: Supply-chain audit (cargo-deny)
      run: cargo deny check

    - name: Unused-dependency check (cargo-machete)
      run: cargo machete

    - name: Spellcheck (typos)
      run: typos
```

Each exits non-zero on findings by default → fails the build. **CI is the authoritative gate.**

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "feat(ws2): CI gates on cargo-deny, cargo-machete, and typos"
```

---

## WS3 — Versioned DB migrations (sea-orm-migration)

> **Load-bearing constraint:** `m0` must reproduce **exactly** what `create_table_from_entity` emits for the 9 entities on a fresh DB. Because it cannot be hand-verified by eye, the **oracle is the PRAGMA-based structural comparison** in Task 14 — build both schemas, compare, iterate `m0` until they match, *then* declare `m0` correct. Do NOT edit the comparison to make a mismatched `m0` pass; the comparison wins.

### Task 10: Create the `migration` crate

**Files:**
- Create: `migration/Cargo.toml`, `migration/src/lib.rs`, `migration/src/m0_initial.rs`
- Modify: `Cargo.toml:2-7` (workspace members)

- [ ] **Step 1: Add the crate to the workspace** — replace `Cargo.toml` members (L2–7):

```toml
members = [
    "frontend",
    "backend",
    "common",
    "xtask",
    "migration"
]
```

- [ ] **Step 2: Create `migration/Cargo.toml`** — version-lock `sea-orm-migration` to `sea-orm`'s `1.1.20` (lockstep releases; a mismatched minor breaks the build):

```toml
[package]
name = "migration"
version = "0.1.0"
edition = "2024"
publish = false

[lib]
name = "migration"
path = "src/lib.rs"

[dependencies]
sea-orm-migration = { version = "1.1.20", features = ["sqlx-sqlite", "runtime-tokio-rustls"] }
async-trait = "0.1"

[lints]
workspace = true
```

- [ ] **Step 3: Create `migration/src/lib.rs`**

```rust
//! Versioned, immutable database migrations for rsahp.
//!
//! `m0_initial` is the immutable baseline snapshot of the schema that the old
//! `create_table_from_entity` startup loop used to build imperatively. Future
//! schema changes are added as new migrations — never by editing `m0`.

pub use sea_orm_migration::prelude::*;

mod m0_initial;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m0_initial::Migration)]
    }
}
```

Note: `#[async_trait::async_trait]` resolves via the explicit `async-trait` dependency added to `migration/Cargo.toml` in Step 2 — it compiles unconditionally, no fallback path needed.

- [ ] **Step 4: Create `migration/src/m0_initial.rs`** — the hand-written baseline. This is the best-effort derivation from the entity structs (columns/types/FKs mapped 1:1). Task 14 will correct any PRAGMA-level mismatch.

```rust
//! m0 — immutable initial baseline. Reproduces the 9-table schema that
//! `create_table_from_entity` produced. DO NOT EDIT after it ships; add new
//! migrations for changes.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // user_group
        manager
            .create_table(
                Table::create()
                    .table(UserGroup::Table)
                    .col(ColumnDef::new(UserGroup::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(UserGroup::Name).string().not_null())
                    .col(ColumnDef::new(UserGroup::ParentId).integer().null())
                    .foreign_key(ForeignKey::create().from(UserGroup::Table, UserGroup::ParentId).to(UserGroup::Table, UserGroup::Id))
                    .to_owned(),
            )
            .await?;

        // user
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .col(ColumnDef::new(User::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(User::Username).string().not_null())
                    .col(ColumnDef::new(User::PasswordHash).string().not_null())
                    .col(ColumnDef::new(User::IsAdmin).boolean().not_null())
                    .col(ColumnDef::new(User::IsDeleted).boolean().not_null())
                    .to_owned(),
            )
            .await?;

        // folder
        manager
            .create_table(
                Table::create()
                    .table(Folder::Table)
                    .col(ColumnDef::new(Folder::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(Folder::Name).string().not_null())
                    .col(ColumnDef::new(Folder::OwnerId).integer().not_null())
                    .col(ColumnDef::new(Folder::ParentFolderId).integer().null())
                    .foreign_key(ForeignKey::create().from(Folder::Table, Folder::OwnerId).to(User::Table, User::Id))
                    .foreign_key(ForeignKey::create().from(Folder::Table, Folder::ParentFolderId).to(Folder::Table, Folder::Id))
                    .to_owned(),
            )
            .await?;

        // document
        manager
            .create_table(
                Table::create()
                    .table(Document::Table)
                    .col(ColumnDef::new(Document::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(Document::Name).string().not_null())
                    .col(ColumnDef::new(Document::OwnerId).integer().not_null())
                    .col(ColumnDef::new(Document::Version).integer().not_null())
                    .col(ColumnDef::new(Document::AggregationMethod).string().not_null())
                    .col(ColumnDef::new(Document::FolderId).integer().null())
                    .col(ColumnDef::new(Document::CreatedAt).timestamp_with_time_zone().not_null())
                    .foreign_key(ForeignKey::create().from(Document::Table, Document::OwnerId).to(User::Table, User::Id))
                    .foreign_key(ForeignKey::create().from(Document::Table, Document::FolderId).to(Folder::Table, Folder::Id))
                    .to_owned(),
            )
            .await?;

        // node
        manager
            .create_table(
                Table::create()
                    .table(Node::Table)
                    .col(ColumnDef::new(Node::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(Node::DocumentId).integer().not_null())
                    .col(ColumnDef::new(Node::ParentNodeId).integer().null())
                    .col(ColumnDef::new(Node::Name).string().not_null())
                    .col(ColumnDef::new(Node::NodeType).string().not_null())
                    .col(ColumnDef::new(Node::Cost).double().null())
                    .foreign_key(ForeignKey::create().from(Node::Table, Node::DocumentId).to(Document::Table, Document::Id))
                    .foreign_key(ForeignKey::create().from(Node::Table, Node::ParentNodeId).to(Node::Table, Node::Id))
                    .to_owned(),
            )
            .await?;

        // comparison (note: node_a_id / node_b_id have NO FK in the entity Relation enum)
        manager
            .create_table(
                Table::create()
                    .table(Comparison::Table)
                    .col(ColumnDef::new(Comparison::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(Comparison::DocumentId).integer().not_null())
                    .col(ColumnDef::new(Comparison::RespondentId).integer().not_null())
                    .col(ColumnDef::new(Comparison::ParentNodeId).integer().not_null())
                    .col(ColumnDef::new(Comparison::NodeAId).integer().not_null())
                    .col(ColumnDef::new(Comparison::NodeBId).integer().not_null())
                    .col(ColumnDef::new(Comparison::SaatyValue).double().not_null())
                    .foreign_key(ForeignKey::create().from(Comparison::Table, Comparison::DocumentId).to(Document::Table, Document::Id))
                    .foreign_key(ForeignKey::create().from(Comparison::Table, Comparison::ParentNodeId).to(Node::Table, Node::Id))
                    .foreign_key(ForeignKey::create().from(Comparison::Table, Comparison::RespondentId).to(User::Table, User::Id))
                    .to_owned(),
            )
            .await?;

        // user_group_membership
        manager
            .create_table(
                Table::create()
                    .table(UserGroupMembership::Table)
                    .col(ColumnDef::new(UserGroupMembership::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(UserGroupMembership::UserId).integer().not_null())
                    .col(ColumnDef::new(UserGroupMembership::GroupId).integer().not_null())
                    .foreign_key(ForeignKey::create().from(UserGroupMembership::Table, UserGroupMembership::UserId).to(User::Table, User::Id))
                    .foreign_key(ForeignKey::create().from(UserGroupMembership::Table, UserGroupMembership::GroupId).to(UserGroup::Table, UserGroup::Id))
                    .to_owned(),
            )
            .await?;

        // document_user_assignment
        manager
            .create_table(
                Table::create()
                    .table(DocumentUserAssignment::Table)
                    .col(ColumnDef::new(DocumentUserAssignment::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(DocumentUserAssignment::DocumentId).integer().not_null())
                    .col(ColumnDef::new(DocumentUserAssignment::UserId).integer().not_null())
                    .foreign_key(ForeignKey::create().from(DocumentUserAssignment::Table, DocumentUserAssignment::DocumentId).to(Document::Table, Document::Id))
                    .foreign_key(ForeignKey::create().from(DocumentUserAssignment::Table, DocumentUserAssignment::UserId).to(User::Table, User::Id))
                    .to_owned(),
            )
            .await?;

        // document_group_assignment
        manager
            .create_table(
                Table::create()
                    .table(DocumentGroupAssignment::Table)
                    .col(ColumnDef::new(DocumentGroupAssignment::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(DocumentGroupAssignment::DocumentId).integer().not_null())
                    .col(ColumnDef::new(DocumentGroupAssignment::GroupId).integer().not_null())
                    .foreign_key(ForeignKey::create().from(DocumentGroupAssignment::Table, DocumentGroupAssignment::DocumentId).to(Document::Table, Document::Id))
                    .foreign_key(ForeignKey::create().from(DocumentGroupAssignment::Table, DocumentGroupAssignment::GroupId).to(UserGroup::Table, UserGroup::Id))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop in reverse dependency order.
        for table in [
            DocumentGroupAssignment::Table.into_iden(),
            DocumentUserAssignment::Table.into_iden(),
            UserGroupMembership::Table.into_iden(),
            Comparison::Table.into_iden(),
            Node::Table.into_iden(),
            Document::Table.into_iden(),
            Folder::Table.into_iden(),
            User::Table.into_iden(),
            UserGroup::Table.into_iden(),
        ] {
            manager.drop_table(Table::drop().table(table).to_owned()).await?;
        }
        Ok(())
    }
}

// --- Iden definitions: table + column identifiers, names matching the entity `table_name`/field snake_case. ---

#[derive(DeriveIden)]
enum UserGroup {
    Table,
    Id,
    Name,
    ParentId,
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
    Username,
    PasswordHash,
    IsAdmin,
    IsDeleted,
}

#[derive(DeriveIden)]
enum Folder {
    Table,
    Id,
    Name,
    OwnerId,
    ParentFolderId,
}

#[derive(DeriveIden)]
enum Document {
    Table,
    Id,
    Name,
    OwnerId,
    Version,
    AggregationMethod,
    FolderId,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Node {
    Table,
    Id,
    DocumentId,
    ParentNodeId,
    Name,
    NodeType,
    Cost,
}

#[derive(DeriveIden)]
enum Comparison {
    Table,
    Id,
    DocumentId,
    RespondentId,
    ParentNodeId,
    NodeAId,
    NodeBId,
    SaatyValue,
}

#[derive(DeriveIden)]
enum UserGroupMembership {
    Table,
    Id,
    UserId,
    GroupId,
}

#[derive(DeriveIden)]
enum DocumentUserAssignment {
    Table,
    Id,
    DocumentId,
    UserId,
}

#[derive(DeriveIden)]
enum DocumentGroupAssignment {
    Table,
    Id,
    DocumentId,
    GroupId,
}
```

> **`DeriveIden` naming caveat:** `DeriveIden` renders `Table` as the enum name in snake_case (`UserGroup::Table` → `user_group`) and each variant in snake_case (`PasswordHash` → `password_hash`). Verify the rendered table name matches the entity `#[sea_orm(table_name = ...)]` exactly. For the multi-word table enums (`UserGroup`, `UserGroupMembership`, `DocumentUserAssignment`, `DocumentGroupAssignment`) confirm the snake_case rendering equals `user_group`, `user_group_membership`, `document_user_assignment`, `document_group_assignment` in Step 5 / Task 14; if `DeriveIden` mangles any, override with `#[sea_orm(iden = "user_group")]` on the `Table` variant.

- [ ] **Step 5: Build the migration crate**

Run: `cargo build --package migration`
Expected: `Finished`. Then:
Run: `cargo clippy --package migration --all-targets -- -D warnings`
Expected: no issues.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock migration/
git commit -m "feat(ws3): add migration crate with hand-written m0 baseline"
```

---

### Task 11: Backend — Migrator::up + fenced seed + cutover guard + test helper

**Files:**
- Modify: `backend/Cargo.toml` (deps + dev-deps)
- Modify: `backend/src/lib.rs` (`setup_schema` rewrite; add `#[cfg(test)]` helper + canonical entity list)

**Context (Step 0):** Open `backend/src/lib.rs`. Confirm `setup_schema` (L16–144) builds `stmts` via `create_table_from_entity` for the 9 entities (L23–69), runs 5 `ALTER TABLE` blocks (L76–117), and 3 `INSERT OR IGNORE` seeds (L119–139). Confirm `backend/Cargo.toml` has `sea-orm = { version = "1.1.20", ... }` at L15. If different, STOP and report `STATE_MISMATCH`.

- [ ] **Step 1: Add the migration dependency** — in `backend/Cargo.toml`, add to `[dependencies]` (after L15 `sea-orm`):

```toml
migration = { version = "0.1.0", path = "../migration" }
```

and add to `[dev-dependencies]` (after L25 `axum-test`):

```toml
sea-orm-migration = { version = "1.1.20", features = ["sqlx-sqlite", "runtime-tokio-rustls"] }
```

- [ ] **Step 2: Rewrite `setup_schema`** — replace the entire body of `setup_schema` (L16–144, everything between the function signature and its closing `}`) with:

```rust
    use migration::{Migrator, MigratorTrait};
    use sea_orm::{ConnectionTrait, Statement};

    // Cutover guard (UNCONDITIONAL — runs in release AND debug): an existing
    // pre-migration rsahp.db has the 9 app tables but no `seaql_migrations` tracking
    // table. `Migrator::up` would re-issue CREATE TABLE and fail with "table already
    // exists". `m0` uses plain `create_table` (no IF NOT EXISTS), so without this
    // guard a release build would panic with a cryptic DbErr. Detect the exact
    // condition and return a loud, human-readable error on every build profile.
    {
        let backend = db.get_database_backend();
        let has_user_table = db
            .query_one(Statement::from_string(
                backend,
                "SELECT name FROM sqlite_master WHERE type='table' AND name='user';".to_owned(),
            ))
            .await?
            .is_some();
        let has_migrations_table = db
            .query_one(Statement::from_string(
                backend,
                "SELECT name FROM sqlite_master WHERE type='table' AND name='seaql_migrations';"
                    .to_owned(),
            ))
            .await?
            .is_some();
        if has_user_table && !has_migrations_table {
            eprintln!(
                "\n============================================================\n\
                 MIGRATION CUTOVER: this database predates versioned migrations.\n\
                 It has the application tables but no `seaql_migrations` tracking\n\
                 table, so `Migrator::up` cannot run against it.\n\n\
                 This is dev-only, disposable data. BACK UP or EXPECT DATA LOSS,\n\
                 then delete the database file and restart:\n\n\
                 \x20   rm rsahp.db   (or delete rsahp.db in the project root)\n\n\
                 A fresh DB will be created and migrated automatically.\n\
                 ============================================================\n"
            );
            return Err(DbErr::Custom(
                "pre-migration database detected; delete rsahp.db and restart".to_owned(),
            ));
        }
    }

    // Apply all pending migrations (creates the schema on a fresh DB).
    Migrator::up(db, None)
        .await
        .map_err(|e| DbErr::Custom(format!("migration failed: {e}")))?;

    // Dev-only seed (admin group/user/membership) for out-of-box login. Fenced to
    // debug builds so it is physically absent from release binaries. Uses ActiveModel
    // existence-checked inserts: every column MUST be set explicitly, so a future
    // schema change (a new required column) breaks THIS at compile time — the
    // forcing function the S1-fenced decision relied on. Real insert errors propagate
    // (no silent swallow); an already-seeded DB is a clean no-op via the find check.
    #[cfg(debug_assertions)]
    {
        use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
        if entity::user_group::Entity::find_by_id(1).one(db).await?.is_none() {
            entity::user_group::ActiveModel {
                id: Set(1),
                name: Set("Admin Group".to_owned()),
                parent_id: Set(None),
            }
            .insert(db)
            .await?;
        }
        if entity::user::Entity::find_by_id(1).one(db).await?.is_none() {
            entity::user::ActiveModel {
                id: Set(1),
                username: Set("admin".to_owned()),
                password_hash: Set("hash".to_owned()),
                is_admin: Set(true),
                is_deleted: Set(false),
            }
            .insert(db)
            .await?;
        }
        if entity::user_group_membership::Entity::find_by_id(1)
            .one(db)
            .await?
            .is_none()
        {
            entity::user_group_membership::ActiveModel {
                id: Set(1),
                user_id: Set(1),
                group_id: Set(1),
            }
            .insert(db)
            .await?;
        }
    }

    tracing::info!("Database schema initialized via migrations.");
    Ok(())
```

Also fix the `use` line at the top of the file: replace the old top-level `use sea_orm::{ConnectionTrait, DbErr, Schema};` (L13) with just `use sea_orm::DbErr;` — `DbErr` is used in the signature; `ConnectionTrait`/`Statement` are imported locally in `setup_schema`, and `Schema`/`ConnectionTrait` locally in the test helper (Step 3). Confirm no unused-import warnings via clippy in Step 4.

> **ActiveModel struct-literal note:** listing every field explicitly (no `..Default::default()`) is deliberate — it is what makes a future added column a *compile* error here. Do not "fix" a future missing-field error by adding `..Default::default()`; update the seed to set the new field.

- [ ] **Step 3: Add the canonical entity list + `#[cfg(test)]` schema helper** — append at the end of `backend/src/lib.rs` (after `create_router`). This keeps `create_table_from_entity` alive as the drift-guard's entity side and defines the single canonical list reused by the test (no second copy to drift):

```rust
/// The canonical list of application tables, in creation order. This is the
/// single source of truth reused by the drift-guard test's entity-side builder
/// and its table-name-set assertion. A new entity MUST be added here AND given a
/// migration (see CONTRIBUTING).
#[cfg(test)]
pub const APP_TABLES: &[&str] = &[
    "user_group",
    "user",
    "folder",
    "document",
    "node",
    "comparison",
    "user_group_membership",
    "document_user_assignment",
    "document_group_assignment",
];

/// Builds the schema from the LIVE entities via `create_table_from_entity`.
/// Test-only: production startup uses `Migrator::up`. This is the entity side of
/// the drift-guard comparison (consumed by the inline `#[cfg(test)] mod` in Task 12)
/// — do NOT delete as "dead code". Mirrors the exact `builder.build(...if_not_exists())`
/// pattern the original `setup_schema` used (proven to compile).
#[cfg(test)]
pub async fn entity_schema_db(db: &sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    use sea_orm::{ConnectionTrait, Schema};
    let builder = db.get_database_backend();
    let schema = Schema::new(builder);
    let stmts = vec![
        builder.build(schema.create_table_from_entity(entity::user_group::Entity).if_not_exists()),
        builder.build(schema.create_table_from_entity(entity::user::Entity).if_not_exists()),
        builder.build(schema.create_table_from_entity(entity::folder::Entity).if_not_exists()),
        builder.build(schema.create_table_from_entity(entity::document::Entity).if_not_exists()),
        builder.build(schema.create_table_from_entity(entity::node::Entity).if_not_exists()),
        builder.build(schema.create_table_from_entity(entity::comparison::Entity).if_not_exists()),
        builder.build(schema.create_table_from_entity(entity::user_group_membership::Entity).if_not_exists()),
        builder.build(schema.create_table_from_entity(entity::document_user_assignment::Entity).if_not_exists()),
        builder.build(schema.create_table_from_entity(entity::document_group_assignment::Entity).if_not_exists()),
    ];
    for stmt in stmts {
        db.execute(stmt).await?;
    }
    Ok(())
}
```

> **Note (from panel round 1):** `APP_TABLES` and `entity_schema_db` are `#[cfg(test)]`, so they exist **only** when the crate is compiled with `--test`. A separate integration test under `backend/tests/` links the library compiled *without* `cfg(test)` and would fail with `unresolved import backend::APP_TABLES`. That is why the drift test in Task 12 is an **inline `#[cfg(test)] mod` inside `lib.rs`**, not a `tests/` file.

- [ ] **Step 4: Build backend + clippy**

Run: `cargo build --package backend`
Expected: `Finished`.
Run: `cargo clippy --package backend --all-targets -- -D warnings`
Expected: no issues (fix any unused-import warning from the `use` reshuffle in Step 2).

- [ ] **Step 5: Commit**

```bash
git add backend/Cargo.toml backend/src/lib.rs Cargo.lock
git commit -m "feat(ws3): backend schema via Migrator::up, fenced seed, cutover guard"
```

---

### Task 12: Drift-guard test (PRAGMA structural comparison) + baseline verification

**Files:**
- Modify: `backend/src/lib.rs` (append an inline `#[cfg(test)] mod migration_drift`)

This is both the one-time baseline verification (safety net #1) and the ongoing CI drift-guard (safety net #2) — same PRAGMA comparison, run automatically. **It is an inline module, not a `tests/` file** — see the Task 11 note: the `#[cfg(test)]` helpers it consumes exist only under `--test`, which a separate integration-test crate does not get.

- [ ] **Step 1: Write the failing test** — append this module to the end of `backend/src/lib.rs` (after the `entity_schema_db` helper):

```rust
//! Drift-guard: asserts the schema built from the LIVE entities
//! (`create_table_from_entity`) is structurally identical to the schema built by
//! the migrations (`Migrator::up`), across all 9 application tables — proving the
//! immutable `m0` baseline still matches the entities. Fails if an entity is
//! edited without a corresponding migration.
//!
//! Comparison is SEMANTIC (normalized PRAGMA metadata as order-insensitive sets),
//! NOT textual DDL — the two paths emit equivalent-but-different CREATE TABLE text.
//! The `seaql_migrations` tracking table is excluded (present only on the migrated DB).
#[cfg(test)]
mod migration_drift {
use crate::{entity_schema_db, APP_TABLES};
use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};
use std::collections::BTreeSet;

/// One column's structural metadata from PRAGMA table_info.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ColInfo {
    name: String,
    col_type: String,
    notnull: bool,
    dflt: Option<String>,
    pk: i32,
}

/// One foreign key's structural metadata from PRAGMA foreign_key_list.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct FkInfo {
    from: String,
    table: String,
    to: String,
    on_delete: String,
    on_update: String,
}

async fn table_names(db: &DatabaseConnection) -> BTreeSet<String> {
    let rows = db
        .query_all(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != 'seaql_migrations';".to_owned(),
        ))
        .await
        .unwrap();
    rows.into_iter()
        .map(|r| r.try_get::<String>("", "name").unwrap())
        .collect()
}

async fn columns(db: &DatabaseConnection, table: &str) -> BTreeSet<ColInfo> {
    let rows = db
        .query_all(Statement::from_string(
            DbBackend::Sqlite,
            format!("PRAGMA table_info(\"{table}\");"),
        ))
        .await
        .unwrap();
    rows.into_iter()
        .map(|r| ColInfo {
            name: r.try_get::<String>("", "name").unwrap(),
            col_type: r.try_get::<String>("", "type").unwrap().to_uppercase(),
            notnull: r.try_get::<i32>("", "notnull").unwrap() != 0,
            dflt: r.try_get::<Option<String>>("", "dflt_value").unwrap(),
            pk: r.try_get::<i32>("", "pk").unwrap(),
        })
        .collect()
}

async fn foreign_keys(db: &DatabaseConnection, table: &str) -> BTreeSet<FkInfo> {
    let rows = db
        .query_all(Statement::from_string(
            DbBackend::Sqlite,
            format!("PRAGMA foreign_key_list(\"{table}\");"),
        ))
        .await
        .unwrap();
    rows.into_iter()
        .map(|r| FkInfo {
            from: r.try_get::<String>("", "from").unwrap(),
            table: r.try_get::<String>("", "table").unwrap(),
            to: r.try_get::<String>("", "to").unwrap(),
            on_delete: r.try_get::<String>("", "on_delete").unwrap(),
            on_update: r.try_get::<String>("", "on_update").unwrap(),
        })
        .collect()
}

async fn fresh_db() -> DatabaseConnection {
    // max_connections(1) is REQUIRED: `sqlite::memory:` gives each pooled connection
    // its OWN empty database, so a multi-connection pool would run Migrator::up on one
    // connection and PRAGMA queries on another (empty) one — a false/empty comparison.
    let mut opt = ConnectOptions::new("sqlite::memory:");
    opt.max_connections(1);
    Database::connect(opt).await.unwrap()
}

#[tokio::test]
async fn entity_schema_matches_migrations() {
    let entity_db = fresh_db().await;
    entity_schema_db(&entity_db).await.unwrap();

    let migrated_db = fresh_db().await;
    Migrator::up(&migrated_db, None).await.unwrap();

    // Table-name sets must match exactly (catches entity-added-without-migration
    // AND migration-added-without-entity), excluding seaql_migrations.
    let entity_tables = table_names(&entity_db).await;
    let migrated_tables = table_names(&migrated_db).await;
    assert_eq!(
        entity_tables, migrated_tables,
        "table-name sets differ: entity={entity_tables:?} migrated={migrated_tables:?}"
    );
    let expected: BTreeSet<String> = APP_TABLES.iter().map(|s| (*s).to_owned()).collect();
    assert_eq!(entity_tables, expected, "entity tables != canonical APP_TABLES");

    // Per-table column + FK structural equality.
    for table in APP_TABLES {
        let ec = columns(&entity_db, table).await;
        let mc = columns(&migrated_db, table).await;
        assert_eq!(ec, mc, "column mismatch in `{table}`:\n entity={ec:#?}\n migrated={mc:#?}");

        let ef = foreign_keys(&entity_db, table).await;
        let mf = foreign_keys(&migrated_db, table).await;
        assert_eq!(ef, mf, "FK mismatch in `{table}`:\n entity={ef:#?}\n migrated={mf:#?}");
    }
}
} // mod migration_drift
```

- [ ] **Step 2: Run the test — this IS the baseline verification (expect it to guide m0 fixes)**

Run: `cargo nextest run --package backend -E 'test(entity_schema_matches_migrations)'`
Expected on first run: **may FAIL** with a specific column/FK mismatch (e.g. a `col_type` like `VARCHAR` vs `TEXT`, a `notnull`/`pk` difference, an FK `on_delete` value, or a table-name-render mismatch). Each assertion message names the exact table + diff.

- [ ] **Step 3: Fix `m0` (NOT the test) until it passes** — for every mismatch the oracle reports, adjust `migration/src/m0_initial.rs` to match the entity side:
  - Column type mismatch → change the builder method (e.g. `.string()` vs `.text()`, `.double()` vs `.float()`, `.timestamp_with_time_zone()` vs `.timestamp()`) to whatever `create_table_from_entity` emitted.
  - `pk`/`auto_increment` mismatch → adjust `.auto_increment()`/`.primary_key()`.
  - FK `on_delete`/`on_update` mismatch → add `.on_delete(...)`/`.on_update(...)` to match (entity default is `NO ACTION`).
  - Table-name render mismatch → add `#[sea_orm(iden = "...")]` overrides per the DeriveIden caveat in Task 10.
  
  Re-run Step 2 after each fix. Iterate until the test passes. **Do not weaken the assertions to force a pass** — the entity side is the oracle.

- [ ] **Step 4: Confirm the guard actually catches drift (one-time sanity check, then revert)** — temporarily add a dummy column to any entity struct (e.g. `pub scratch: Option<i32>` on `node::Model`), re-run Step 2, confirm the test FAILS with a `node` column mismatch, then **revert the entity change**. This proves the guard is live, not vacuously green.

- [ ] **Step 5: Commit**

```bash
git add backend/src/lib.rs migration/src/m0_initial.rs
git commit -m "test(ws3): PRAGMA-based drift-guard proving m0 matches entities"
```

---

### Task 13: WS3 docs + full-stack verification

**Files:**
- Modify: `README.md` and/or `CONTRIBUTING.md` (create if absent), `TESTING_STRATEGY.md`

- [ ] **Step 1: Document the migration workflow + reset + residual gap** — add a "Database migrations" section (in `README.md` after the Cockpit section, or a new `CONTRIBUTING.md`):

```markdown
## Database migrations

Schema is managed by the `migration` crate (sea-orm-migration) and applied at
backend startup via `Migrator::up`. `m0_initial` is the **immutable** baseline —
never edit it; add a new migration for any schema change.

**Adding an entity:** (1) add the entity module; (2) add its table to the `m0`
migration ONLY if pre-release, otherwise write a NEW migration; (3) add its table
name to the `APP_TABLES` canonical list in `backend/src/lib.rs`. The drift-guard
test asserts entity-built and migration-built schemas match. **Residual gap:** a
new entity omitted from BOTH its migration AND `APP_TABLES` is not caught
automatically (sea-orm has no runtime entity registry) — this is a code-review
responsibility.

**Upgrading from a pre-migration database:** an old `rsahp.db` (built before
migrations) has no `seaql_migrations` table; startup detects this and prints a
cutover message. The dev DB is disposable — **back up or expect data loss**, then:

    rm rsahp.db

and restart; a fresh migrated DB is created automatically.

**Doctests:** `cargo nextest run` does not run doctests. There are 0 today; if you
add one, also add a `cargo test --doc` CI step.
```

- [ ] **Step 2: Full-stack fresh-DB verification**

Run: `rm -f rsahp.db && cargo run --bin backend` (let it boot, confirm `Database schema initialized via migrations.` logs, Ctrl+C).
Expected: a fresh `rsahp.db` is created and the drift-guard test (Task 12) — which already proves the 9-table structure semantically — is green. No `sqlite3` CLI is required for the structural check; the drift test is the authoritative structural oracle.

**Optional cutover-guard smoke test (requires the `sqlite3` CLI, which is NOT a project prerequisite — skip if absent):** remove the tracking table to simulate a pre-migration DB — `sqlite3 rsahp.db "DROP TABLE seaql_migrations;"` — then re-run `cargo run --bin backend`; expect the human-readable cutover message and a clean non-zero exit, not a panic. Delete `rsahp.db` afterward. (If `sqlite3` is unavailable, the guard logic is still exercised by reasoning + the unconditional guard code; a dedicated guard unit test may be added later.)

- [ ] **Step 3: Commit**

```bash
git add README.md TESTING_STRATEGY.md CONTRIBUTING.md
git commit -m "docs(ws3): document migrations, reset, doctest and registration notes"
```

---

## Cross-cutting — Tool discoverability

### Task 14: `.claude/recommended-tools.json`

**Files:**
- Create: `.claude/recommended-tools.json`

**Context (Step 0):** The SessionStart hook (`recommended-tooling-check.sh`) reads entries of shape `{name, why, install, in_path?, file_exists?}`. Before writing, confirm the exact top-level shape the hook expects — check the hook script or any existing `recommended-tools.json` in a sibling project. If the hook expects a bare array vs a `{"tools": [...]}` wrapper, match it. If unable to confirm, use the `{"tools": [...]}` wrapper below and note it for review.

- [ ] **Step 1: Write the file** — `in_path` uses the **exact installed binary name** (not the `cargo <sub>` alias); typos' crate is `typos-cli` but its binary is `typos`:

```json
{
  "tools": [
    {
      "name": "cargo-nextest",
      "why": "Faster test runner; hard prerequisite for the lefthook pre-commit hook and CI.",
      "install": "cargo install cargo-nextest --locked",
      "in_path": "cargo-nextest"
    },
    {
      "name": "cargo-llvm-cov",
      "why": "Line coverage via the xtask Coverage action and CI (Linux). Needs the llvm-tools-preview rustup component.",
      "install": "cargo install cargo-llvm-cov --locked",
      "in_path": "cargo-llvm-cov"
    },
    {
      "name": "cargo-deny",
      "why": "Supply-chain / license / advisory gate (CI-authoritative, xtask-advisory).",
      "install": "cargo install cargo-deny --locked",
      "in_path": "cargo-deny"
    },
    {
      "name": "cargo-machete",
      "why": "Detects unused dependencies (CI-gating).",
      "install": "cargo install cargo-machete --locked",
      "in_path": "cargo-machete"
    },
    {
      "name": "typos",
      "why": "Source spellcheck (CI-gating). Crate is typos-cli; binary is typos.",
      "install": "cargo install typos-cli --locked",
      "in_path": "typos"
    },
    {
      "name": "sea-orm-cli",
      "why": "Generates future migration skeletons for the migration crate.",
      "install": "cargo install sea-orm-cli --locked",
      "in_path": "sea-orm-cli"
    }
  ]
}
```

- [ ] **Step 2: Verify the hook accepts it** — start a fresh Claude Code session (or re-trigger SessionStart) and confirm the tooling-check hook parses the file with no error and surfaces missing tools. If it errors on shape, adjust the wrapper to match Step 0's finding.

- [ ] **Step 3: Commit**

```bash
git add .claude/recommended-tools.json
git commit -m "chore: register dev tools in recommended-tools.json"
```

---

## Final verification (whole-plan acceptance)

- [ ] `cargo xtask` cockpit: item 3 runs nextest, item 4 (Coverage) runs llvm-cov or prints the install hint, item 5 (hygiene) runs deny/machete/typos advisory.
- [ ] `cargo llvm-cov nextest --lcov --output-path lcov.info` produces `lcov.info`.
- [ ] `cargo deny check`, `cargo machete`, `typos` all exit 0 at workspace root.
- [ ] `rm -f rsahp.db && cargo run --bin backend` initializes via `Migrator::up`; the drift-guard test (`cargo nextest run --package backend -E 'test(entity_schema_matches_migrations)'`) is green and the PRAGMA comparison reports no differences across all 9 tables.
- [ ] (Optional, needs `sqlite3`) Dropping `seaql_migrations` from an existing DB then booting yields the human-readable cutover message, not a panic.
- [ ] `lefthook run pre-commit` passes end-to-end using nextest.
- [ ] `.claude/recommended-tools.json` present, valid, and accepted by the SessionStart hook.
- [ ] CI green on windows/ubuntu/macos (coverage on Linux only). **Note:** CI triggers on `push`/`pull_request` to `master` only — pushes to the `dev-tooling-additions` feature branch do **not** run CI. The CI steps are first exercised when a PR to `master` is opened; verify green there before merge.

## Self-review notes (author)

- **Spec coverage:** WS1 → Tasks 1–5; WS2 → Tasks 6–9; WS3 → Tasks 10–13; cross-cutting → Task 14. Every spec acceptance bullet maps to a Final-verification checkbox.
- **Known iteration point:** `m0` (Task 10) is a best-effort derivation; Task 12's PRAGMA oracle is the mechanism that makes it exact — this is by design, not a placeholder.
- **Type-consistency:** `entity_schema_db`, `APP_TABLES`, `Migrator`, `binary_present`, and the `ColInfo`/`FkInfo` structs are the only cross-task symbols; names are used identically everywhere they appear.
- **Open risks flagged for review:** (a) `recommended-tools.json` top-level shape (Task 14 Step 0); (b) `async_trait` re-export path (Task 10 Step 3); (c) `DeriveIden` snake_case rendering of multi-word table names (Task 10 caveat).
