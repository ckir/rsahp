# PolyForm Noncommercial License Switch — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace rsahp's custom noncommercial license with the standard **PolyForm Noncommercial License 1.0.0**, declared consistently across `LICENSE`, `README.md`, the Flatpak metainfo, the Cargo manifests, and an SPDX header on every workspace `.rs` file.

**Architecture:** Pure declaration change — no runtime behavior changes. Five edit groups (LICENSE text, README/metainfo blurbs, Cargo `license-file`, a uniform `.rs` SPDX-header sweep, typos-config) followed by the full local CI gate, on a `license-polyform` branch.

**Tech Stack:** Rust 2024 workspace; SPDX `LicenseRef-` expressions; Cargo `[workspace.package]` inheritance; `typos` / `cargo-deny` / `cargo clippy` gates; `appstreamcli` (metainfo, tag-CI only).

**Source of truth for behavioral correctness (oracle):** the full local gate — `cargo fmt --all -- --check`, `cargo clippy --workspace -- -D warnings`, `cargo nextest run`, `cargo deny check`, `typos` — must all pass, exactly as CI runs them (`.github/workflows/ci.yml`). A comment/metadata/text change must not alter any of them.

**Design spec:** `docs/superpowers/specs/2026-07-14-polyform-noncommercial-license-design.md` (user-approved; AGY-FIRST-consulted; AGY-AFTER round-2 GREEN). Binding decisions: F1 `license-file` (not `license =`); F2 SPDX id `LicenseRef-PolyForm-Noncommercial-1.0.0`; F3 header dead files too; F4 header `m0_initial.rs` (drift guard is semantic — confirmed at `backend/src/lib.rs:277`); F5 SPDX at line 1 (confirmed: zero shebangs in the tree).

**Verified pre-conditions (checked against master at authoring time — Step 0s re-confirm):** no `.rs` file starts with a `#!/` shebang; no `.rs` uses `include_str!`/`include_bytes!`; the only generator-marker hit is `migration/src/m0_initial.rs`'s frozen `//!` baseline (not auto-regenerated); root `Cargo.toml` has `[workspace]` + `[workspace.lints.clippy]` but NO `[workspace.package]`; each member crate has `[package]` with `edition = "2024"` and no `license`/`license-file`; `typos.toml` exists with `[files] extend-exclude` + `[default.extend-words]`; CI runs bare `typos` (`.github/workflows/ci.yml:55`).

---

## File Structure

- `LICENSE` — replaced with the copyright/Required-Notice line + verbatim PolyForm text.
- `README.md` (line ~75) — license blurb.
- `packaging/linux/io.github.ckir.rsahp.metainfo.xml` (line 5) — `<project_license>`.
- `Cargo.toml` (root) — new `[workspace.package]` with `license-file`.
- `backend/Cargo.toml`, `frontend/Cargo.toml`, `common/Cargo.toml`, `xtask/Cargo.toml`, `migration/Cargo.toml`, `rsahp-desktop/Cargo.toml` — `license-file.workspace = true`.
- Every tracked `*.rs` (≈46 files, enumerated by `git ls-files '*.rs'`) — one SPDX header line.
- `typos.toml` — exclude the third-party `LICENSE` text from spellcheck.

---

### Task 1: Branch + replace `LICENSE` with PolyForm text

**Files:** Modify `LICENSE`.

- [ ] **Step 0: Verify state.** Run `git status --short` (expect clean tree on `master`) and `git rev-parse --abbrev-ref HEAD`. Open `LICENSE`; confirm it is the current custom MIT-derived text whose line 3 begins `Permission is hereby granted, free of charge, ... for non-commercial use only`. If the tree is dirty or `LICENSE` differs, STOP and report `STATE_MISMATCH: <what>`.

- [ ] **Step 1: Create the branch.**

```bash
git checkout -b license-polyform
```
Expected: `Switched to a new branch 'license-polyform'`.

- [ ] **Step 2: Write the new `LICENSE`.** Replace the ENTIRE file with exactly the following (the first line is the PolyForm "Required Notice"; the rest is the verbatim canonical PolyForm Noncommercial 1.0.0 text — do NOT paraphrase, reorder, or reword any of it):

```
Copyright (c) 2026 Costas Kirgoussios

# PolyForm Noncommercial License 1.0.0

<https://polyformproject.org/licenses/noncommercial/1.0.0>

## Acceptance

In order to get any license under these terms, you must agree to them as both strict obligations and conditions to all your licenses.

## Copyright License

The licensor grants you a copyright license for the software to do everything you might do with the software that would otherwise infringe the licensor's copyright in it for any permitted purpose.  However, you may only distribute the software according to [Distribution License](#distribution-license) and make changes or new works based on the software according to [Changes and New Works License](#changes-and-new-works-license).

## Distribution License

The licensor grants you an additional copyright license to distribute copies of the software.  Your license to distribute covers distributing the software with changes and new works permitted by [Changes and New Works License](#changes-and-new-works-license).

## Notices

You must ensure that anyone who gets a copy of any part of the software from you also gets a copy of these terms or the URL for them above, as well as copies of any plain-text lines beginning with `Required Notice:` that the licensor provided with the software.  For example:

> Required Notice: Copyright Yoyodyne, Inc. (http://example.com)

## Changes and New Works License

The licensor grants you an additional copyright license to make changes and new works based on the software for any permitted purpose.

## Patent License

The licensor grants you a patent license for the software that covers patent claims the licensor can license, or becomes able to license, that you would infringe by using the software.

## Noncommercial Purposes

Any noncommercial purpose is a permitted purpose.

## Personal Uses

Personal use for research, experiment, and testing for the benefit of public knowledge, personal study, private entertainment, hobby projects, amateur pursuits, or religious observance, without any anticipated commercial application, is use for a permitted purpose.

## Noncommercial Organizations

Use by any charitable organization, educational institution, public research organization, public safety or health organization, environmental protection organization, or government institution is use for a permitted purpose regardless of the source of funding or obligations resulting from the funding.

## Fair Use

You may have "fair use" rights for the software under the law. These terms do not limit them.

## No Other Rights

These terms do not allow you to sublicense or transfer any of your licenses to anyone else, or prevent the licensor from granting licenses to anyone else.  These terms do not imply any other licenses.

## Patent Defense

If you make any written claim that the software infringes or contributes to infringement of any patent, your patent license for the software granted under these terms ends immediately. If your company makes such a claim, your patent license ends immediately for work on behalf of your company.

## Violations

The first time you are notified in writing that you have violated any of these terms, or done anything with the software not covered by your licenses, your licenses can nonetheless continue if you come into full compliance with these terms, and take practical steps to correct past violations, within 32 days of receiving notice.  Otherwise, all your licenses end immediately.

## No Liability

***As far as the law allows, the software comes as is, without any warranty or condition, and the licensor will not be liable to you for any damages arising out of these terms or the use or nature of the software, under any kind of legal claim.***

## Definitions

The **licensor** is the individual or entity offering these terms, and the **software** is the software the licensor makes available under these terms.

**You** refers to the individual or entity agreeing to these terms.

**Your company** is any legal entity, sole proprietorship, or other kind of organization that you work for, plus all organizations that have control over, are under the control of, or are under common control with that organization.  **Control** means ownership of substantially all the assets of an entity, or the power to direct its management and policies by vote, contract, or otherwise.  Control can be direct or indirect.

**Your licenses** are all the licenses granted to you for the software under these terms.

**Use** means anything you do with the software requiring one of your licenses.
```

- [ ] **Step 3: Verify the text landed intact.** Run:

```bash
grep -c "PolyForm Noncommercial License 1.0.0" LICENSE
grep -c "Costas Kirgoussios" LICENSE
grep -c "## Definitions" LICENSE
```
Expected: each prints `1` (title present, copyright line present, final section present). If any is `0`, the paste was truncated — redo Step 2.

- [ ] **Step 4: Commit.**

```bash
git add LICENSE
git commit -m "docs: switch LICENSE to PolyForm Noncommercial License 1.0.0"
```

### Task 2: README + metainfo declaration

**Files:** Modify `README.md`; modify `packaging/linux/io.github.ckir.rsahp.metainfo.xml`.

- [ ] **Step 0: Verify state.** `README.md:75` reads exactly:
  `This project is licensed under a custom Non-Commercial License (Free for non-commercial use). See the [LICENSE](LICENSE) file for details.`
  `packaging/linux/io.github.ckir.rsahp.metainfo.xml:5` reads exactly:
  `  <project_license>LicenseRef-proprietary</project_license>`
  If either differs, STOP and report `STATE_MISMATCH`.

- [ ] **Step 1: Update the README blurb.** Replace that `README.md` line with:

```
This project is licensed under the [PolyForm Noncommercial License 1.0.0](LICENSE) — free for noncommercial use. See the [LICENSE](LICENSE) file for the full terms.
```

- [ ] **Step 2: Update the metainfo `project_license`.** Replace that metainfo line with (keep the two-space indent):

```
  <project_license>LicenseRef-PolyForm-Noncommercial-1.0.0</project_license>
```
Leave `<metadata_license>CC0-1.0</metadata_license>` (line 4) unchanged — it licenses the metadata file, not the project.

- [ ] **Step 3: Confirm the XML still parses.** Run (shell-agnostic — `yq` is a binary on PATH, works from Bash or pwsh): `yq -p=xml '.' packaging/linux/io.github.ckir.rsahp.metainfo.xml > /dev/null && echo XML_OK`
  Expected: `XML_OK`. (Full `appstreamcli` validation is exercised only by the tag-triggered Flatpak CI job; `LicenseRef-*` is a valid SPDX expression appstream accepts. This element being validated only at tag time is an OWED item, not a blocker.)

- [ ] **Step 4: Commit.**

```bash
git add README.md packaging/linux/io.github.ckir.rsahp.metainfo.xml
git commit -m "docs: declare PolyForm Noncommercial 1.0.0 in README + Flatpak metainfo"
```

### Task 3: Cargo `license-file` metadata (workspace-inherited)

**Files:** Modify `Cargo.toml` (root) + all 6 member crates' `Cargo.toml`.

- [ ] **Step 0: Verify state.** Root `Cargo.toml` has `[workspace]` (members list) and `[workspace.lints.clippy]` but NO `[workspace.package]` section. Each of `backend/Cargo.toml`, `frontend/Cargo.toml`, `common/Cargo.toml`, `xtask/Cargo.toml`, `migration/Cargo.toml`, `rsahp-desktop/Cargo.toml` has a `[package]` section with `edition = "2024"` and no `license`/`license-file` key. If any differs, STOP and report `STATE_MISMATCH`.

- [ ] **Step 1: Add `[workspace.package]` to the root `Cargo.toml`.** Insert this block immediately AFTER the `resolver = "2"` line (before `[workspace.lints.clippy]`):

```toml

[workspace.package]
# Non-OSI license → use license-file (Cargo's canonical field for non-standard licenses).
# Inherited license-file resolves relative to the workspace root, so all crates point here.
license-file = "LICENSE"
```

- [ ] **Step 2: Inherit it in each member crate.** In EACH of the 6 crate `Cargo.toml` files, add this line inside the `[package]` section (e.g. directly after the `edition = "2024"` line):

```toml
license-file.workspace = true
```
The 6 files (do all of them): `backend/Cargo.toml`, `frontend/Cargo.toml`, `common/Cargo.toml`, `xtask/Cargo.toml`, `migration/Cargo.toml`, `rsahp-desktop/Cargo.toml`.

- [ ] **Step 3: Verify Cargo accepts it.** Run:

```bash
cargo metadata --format-version 1 --no-deps 2>&1 | grep -c "license_file"
```
Expected: a count ≥ 6 (each workspace package now reports a `license_file`). Then run `cargo build --workspace 2>&1 | tail -2` — expected: `Finished` (no manifest errors, no new warnings). If cargo errors that `license-file` cannot be inherited, STOP and report the exact error (do NOT switch to `license =` — that is the F1 decision; report the conflict).

- [ ] **Step 4: Confirm cargo-deny is unaffected.** Run: `cargo deny check licenses 2>&1 | tail -3`
  Expected: `licenses ok`. (Baseline passes with no license field, and `[licenses.private] ignore = true` exempts our crates; adding `license-file` must not regress it.)

- [ ] **Step 5: Commit.**

```bash
git add Cargo.toml backend/Cargo.toml frontend/Cargo.toml common/Cargo.toml xtask/Cargo.toml migration/Cargo.toml rsahp-desktop/Cargo.toml
git commit -m "build: declare license-file = LICENSE across the workspace (PolyForm NC)"
```

### Task 4: SPDX header sweep across every workspace `.rs`

**Files:** Modify every tracked `*.rs` file (≈46; enumerated by `git ls-files '*.rs'`, which excludes the gitignored `target/`). Includes the dead `frontend/src/ui/explorer_{old,very_old,full,1151}.rs` (F3) and `migration/src/m0_initial.rs` (F4 — confirmed safe: semantic drift guard, no `include_str!`).

- [ ] **Step 0: Verify state (the two pre-conditions this sweep relies on).** Run:

```bash
git ls-files '*.rs' | xargs grep -l '^#!/' ; echo "shebang-count above (expect none)"
git ls-files '*.rs' | xargs grep -l 'include_str!\|include_bytes!' ; echo "include-count above (expect none)"
```
Expected: BOTH produce no file paths. If any `.rs` prints (a shebang, or an `include_str!`/`include_bytes!`), STOP and report `STATE_MISMATCH: <file>` — the uniform line-1 sweep below assumes neither exists (a shebang would need the header on line 2; an included source file would embed the header into a runtime string).

- [ ] **Step 1: Apply the header idempotently to all tracked `.rs`.** Run this exact script (Git Bash). It prepends the SPDX line as line 1 only when it is not already there, preserving the rest of each file byte-for-byte:

```bash
HEADER='// SPDX-License-Identifier: LicenseRef-PolyForm-Noncommercial-1.0.0'
git ls-files '*.rs' | while IFS= read -r f; do
  # tr -d '\r': the .rs files are CRLF, so head keeps a trailing \r; strip it before
  # comparing or the check never matches and the header gets prepended again on re-runs.
  first=$(head -n 1 "$f" | tr -d '\r')
  if [ "$first" != "$HEADER" ]; then
    { printf '%s\n' "$HEADER"; cat "$f"; } > "$f.spdxtmp" && mv "$f.spdxtmp" "$f"
    echo "headered: $f"
  else
    echo "already:  $f"
  fi
done
```
(The header line is written with `\n` while the files are CRLF — that mixed ending is normalized by `cargo fmt --all` in Step 3, whose `newline_style = "Auto"` rewrites the file to its predominant CRLF.)

- [ ] **Step 2: Verify every file now carries the header exactly once.** Run:

```bash
# NOTE: no trailing `$` anchor — the .rs files are CRLF, so the line ends with `\r\n`
# and `...$` would never match (the `\r` sits before end-of-line). NOTE: `grep -c -H`
# forces the `filename:` prefix even when xargs dispatches a single file, so the awk
# `-F:` parse is reliable.
total=$(git ls-files '*.rs' | wc -l)
with=$(git ls-files '*.rs' | xargs grep -l '^// SPDX-License-Identifier: LicenseRef-PolyForm-Noncommercial-1.0.0' | wc -l)
dupes=$(git ls-files '*.rs' | xargs grep -c -H '^// SPDX-License-Identifier:' | awk -F: '$2>1{print $1}')
echo "total=$total with_header=$with"; echo "files_with_dupes=[$dupes]"
```
Expected: `total` == `with_header` (every file headered) and `files_with_dupes=[]` (none headered twice). If they differ, re-run Step 1 (it is idempotent) or fix the offending file, and STOP if a file resists.

- [ ] **Step 3: Normalize formatting + confirm the header didn't break compilation.** The prepended line uses `\n`; run rustfmt to normalize line endings/style, then confirm the check passes and the crates still build:

```bash
cargo fmt --all
cargo fmt --all -- --check && echo "FMT_OK"
cargo build --workspace 2>&1 | tail -2
```
Expected: `FMT_OK`; build `Finished`. (rustfmt preserves leading `//` comments and will not move the SPDX line; a `//` comment above a `//!` doc or `#![…]` inner attribute is valid Rust.)

- [ ] **Step 4: Confirm tests still pass (headers are inert comments).** Run: `cargo nextest run 2>&1 | tail -3`
  Expected: `15 tests run: 15 passed, 0 skipped` (unchanged from the current suite).

- [ ] **Step 5: Commit.** (Targeted add — do NOT `git add -A`: untracked scratch like `.clavity/` must not be staged.)

```bash
git add $(git ls-files -m '*.rs')
git commit -m "chore: add PolyForm Noncommercial SPDX header to all workspace .rs files"
```

### Task 5: typos config + clippy gate

**Files:** Modify `typos.toml`.

- [ ] **Step 0: Verify state.** `typos.toml` has a `[files] extend-exclude = [ … ]` array and a `[default.extend-words]` table. `LICENSE` is NOT currently in `extend-exclude`. If it differs, STOP and report `STATE_MISMATCH`.

- [ ] **Step 1: Exclude the third-party LICENSE text from spellcheck (defensive).** Note (measured): `typos` is CI-only (NOT a `lefthook` hook — `lefthook.yml` runs only `fmt`/`clippy` pre-commit, `nextest`/`verify` pre-push), and running `typos` on the PolyForm text (including the `Yoyodyne` Notices example) exits 0 today — so no intermediate commit is blocked and CI only ever sees the final tree. This exclusion is therefore defensive hygiene (a verbatim third-party legal text shouldn't be spellchecked, and it guards future edits), not a fix for a live break. Add `"LICENSE"` to the `extend-exclude` array in `typos.toml` (append inside the array, before its closing `]`):

```toml
    # PolyForm license is verbatim third-party legal text — not authored source to spellcheck.
    "LICENSE",
```

- [ ] **Step 2: Run typos over the whole change (the CI-authoritative command).** Run: `typos`
  Expected: no output / exit 0. If typos flags any token we DID author (e.g. a token inside a `.rs` SPDX header, the README blurb, or the metainfo — realistically none, since `PolyForm`/`LicenseRef`/`Noncommercial` split into non-typo subwords), add that exact token to `[default.extend-words]` in `typos.toml` as `token = "token"` and re-run until clean. Do NOT exclude source files to silence a real typo — only allow the specific PolyForm-related token.

- [ ] **Step 3: Clippy (the repo gate, exactly as CI runs it).** Run: `cargo clippy --workspace -- -D warnings 2>&1 | tail -3`
  Expected: `No issues found` / exit 0. (A leading comment and `license-file` metadata trigger no clippy lint; `cargo_common_metadata` is `allow` in `[workspace.lints.clippy]`.)

- [ ] **Step 4: Commit.**

```bash
git add typos.toml
git commit -m "ci: exclude verbatim PolyForm LICENSE text from typos spellcheck"
```

### Task 6: Full local gate (the pre-push discipline)

**No file changes — this is the gate that the packaging CI rounds taught us to run in full BEFORE pushing.**

- [ ] **Step 1: Run the complete CI-equivalent gate in one shot.** Run:

```bash
cargo fmt --all -- --check && \
cargo clippy --workspace -- -D warnings && \
cargo nextest run && \
cargo deny check && \
typos && \
echo "ALL_GATES_GREEN"
```
Expected: ends with `ALL_GATES_GREEN`; nextest `15 tests run: 15 passed`. If ANY gate fails, fix it in the relevant task above and re-run — do NOT proceed to merge on a partial gate. (`cargo deny check` runs advisories+bans+licenses; expect `advisories ok`, `bans ok` (warn-only dupes are fine), `licenses ok`.)

- [ ] **Step 2: Sanity-scan the diff.** Run: `git diff master --stat` and confirm the change set is exactly: `LICENSE`, `README.md`, the metainfo XML, `typos.toml`, the 7 `Cargo.toml` files, and ≈46 `.rs` files (headers only). No unexpected files, no code-logic diffs (SPDX lines only in `.rs`). If anything else changed, investigate before merging.

- [ ] **Step 3: Hand off to finishing.** Per `superpowers:finishing-a-development-branch`: merge `license-polyform` to `master` (fast-forward), re-verify tests on the merged result, delete the branch, and — only when the user asks — push. The merge-gate seam will run a final driving-agy review before the merge.

---

## Notes for the executor

- **Verbatim license text is mandatory** — Task 1's block is the canonical PolyForm Noncommercial 1.0.0 text (fetched from `raw.githubusercontent.com/polyformproject/polyform-licenses/1.0.0/PolyForm-Noncommercial-1.0.0.md`). Do not edit a word of it; only the top `Copyright (c) 2026 Costas Kirgoussios` line is ours (it is PolyForm's "Required Notice").
- **F1/F2/F3/F4/F5 are settled** (see spec). Do not substitute `license =` for `license-file`, do not skip the dead `explorer_*` files, do not skip `m0_initial.rs`.
- **No functional change** — every edit is a comment, a manifest metadata key, or human-readable text. If any test/clippy/build outcome changes, something is wrong; investigate rather than suppress.
- **Push is user-gated** — do not push the branch or the merged master without explicit user instruction.
