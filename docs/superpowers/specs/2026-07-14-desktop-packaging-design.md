# rsahp Desktop Packaging — Design Spec

**Date:** 2026-07-14
**Status:** Draft (brainstormed; passed adversarial-panel-review round 1 solo+agy → folded; pending user review → writing-plans)
**Goal:** Ship rsahp as a double-clickable installed desktop app on **Windows (Inno Setup installer)** and **Linux (Flatpak)**. macOS is explicitly deferred.

**Unblocks:** the deferred *Developers Cockpit* "SHIP & RELEASE" tier (see `ROADMAP.md`) — its release actions will wrap this packaging.

**Review status:** Passed adversarial-panel-review **GREEN** (2 rounds, solo + agy). Round 1 (solo + agy escalation) folded 20 findings (agy round-1 verdict was RED); round 2 (Mechanism Gamer / Activation Auditor / Resource Vampire / Boundary Smuggler) landed no live challenge. Fold ledger at the end. Round-2 plan-notes to carry into the implementation plan: (a) pin the CI release **tag pattern** (`v*.*.*`, not bare `v*`); (b) the single-instance guard's **2nd-launch UX** (focus-existing-window is hard cross-platform → at minimum exit with a message); (c) `metainfo.xml` must pass `appstreamcli validate` (flatpak-builder runs it).

---

## Problem: rsahp is a two-process app

- **`backend`** — an axum HTTP server (headless), owns a SQLite DB (sea-orm migrations run at startup via `setup_schema`).
- **`frontend`** — an egui/eframe native GUI that is a **pure HTTP client** (`ehttp`) to the backend's `api_url` (from `config.json`). It does NOT spawn the backend.
- Today a dev script `start.ps1` launches both as independent processes on a static port 4002; `config.json` + `rsahp.db` + `logs/` live in the working directory. No app icon or desktop file exists.

A packaged/installed app has no `start.ps1`, cannot write to its (read-only) install directory, and must present a single clickable entry point.

## Architecture decision: single-binary `rsahp-desktop` wrapper (keep the HTTP boundary)

**Decision (AGY-FIRST consult + user-approved, 2026-07-14):** embed both processes in ONE binary via a thin wrapper crate, **preserving** the localhost HTTP boundary. Chosen over "frontend spawns backend as a child" and "launcher wrapper".

**Why:** no zombie/orphan processes (shared process ⇒ OS tears both down together — a force-killed child otherwise keeps holding the DB lock and port); lowest churn (frontend stays a pure HTTP client, backend stays an axum server, neither is rewritten); trivial packaging (both installers ship exactly one executable, one entry point).

## Core interfaces (resolve the dev-vs-packaged + startup-race + port issues at the seams)

These interface shapes are load-bearing — they were the fixes for the panel's biggest findings:

- `backend::run_server(config: ResolvedConfig, ready_tx: oneshot::Sender<SocketAddr>, shutdown_rx: oneshot::Receiver<()>)` — binds a listener, **sends the actually-bound `SocketAddr` back over `ready_tx`**, then serves via `axum::serve(...).with_graceful_shutdown(async { shutdown_rx.await.ok(); })`.
- `frontend::run_gui(api_base: String, config: ResolvedConfig)` — runs the eframe app on the **calling (main) thread**, using the `api_base` it is handed (no config lookup for the URL).
- `ResolvedConfig` carries the resolved data-dir paths (db, logs) + `use_gpu` etc. — resolution is done ONCE by the caller, not inside these fns.
- The existing `backend`/`frontend` **bins are retained**: their `main.rs` becomes a thin caller that resolves config from `--config <path>` if given, **else defaults to cwd** (dev/`start.ps1`/`verify.ps1` behavior unchanged — a bare `cargo run --bin backend` still uses the local workspace, NOT the global data dir), binds the static port from config, and (for the backend) ignores the ready/shutdown channels (or uses a trivial never-fires shutdown).

## Phase 1 — Data-dir relocation *(app code; mergeable to master independently)*

The read-only-cwd problem is real in both `%LocalAppData%\Programs\rsahp` and the Flatpak sandbox, and must be fixed regardless of the launch model.

- Add the `directories` crate. Resolve a per-user **local** (non-roaming) data dir via `ProjectDirs::…data_local_dir()` → `%LocalAppData%\…\rsahp\` (Windows) / `~/.local/share/rsahp/` (Linux; inside Flatpak this maps under `~/.var/app/io.github.ckir.rsahp/`). **Local, not `data_dir()`/Roaming** — a SQLite DB must not roam (file-locking, size, sync). The plan pins the exact `ProjectDirs::from(...)` args and asserts the resolved path in a test (the args determine whether the folder nests under an org segment).
- The resolved config carries **absolute, URI-safe** paths: `database_url` = `sqlite://<abs>/rsahp.db?mode=rwc` with **forward slashes** (a raw Windows path with backslashes + drive colon breaks the sqlite URL) — build it by URL-encoding / slash-normalizing the absolute path.
- Seed a default `config.json` on first run containing **every key both binaries read**: `port`, `api_url`, `database_url`, `use_gpu` (do not omit `use_gpu`). Create the data dir (+ `logs/`) if missing.
- **Both** the backend AND the frontend resolve `config.json` from the data dir (the frontend reads `api_url`), preserving the `--config <path>` override for dev.
- **Acceptance:** launching from an arbitrary read-only cwd creates/uses the per-user **local** data dir, resolves an absolute URI-safe db path, and boots cleanly (migrations + fenced seed run). Unit-test path resolution + the config seed (all keys present) + the Windows-path→sqlite-URL conversion.

## Phase 2 — `rsahp-desktop` single-binary wrapper *(new workspace crate)*

- Add workspace member `rsahp-desktop` depending on `backend` + `frontend`.
- `rsahp-desktop/src/main.rs`:
  1. **Resolve + seed the data dir ONCE** (Phase 1) — done here, before any thread spawns, so the background server thread and the main GUI thread never race to create/seed files.
  2. Create `ready` (oneshot `SocketAddr`) and `shutdown` (oneshot `()`) channels.
  3. `std::thread::spawn` a multi-thread tokio runtime running `backend::run_server(cfg, ready_tx, shutdown_rx)`. It binds `127.0.0.1:0` (**ephemeral port** — no static 4002, so a second app or an orphan can never cause a port-in-use failure) and sends the bound addr back.
  4. **Block on `ready_rx`** to get the actual `SocketAddr` (deterministic — no blind retry / sleep race). If the server thread instead fails/panics (bind error, migration error), the ready channel drops → surface a clear error dialog / stderr + non-zero exit, NOT a silent broken GUI.
  5. Run `frontend::run_gui(format!("http://{addr}/api/documents"), cfg)` on the **main thread** (eframe requires it).
  6. When `run_gui()` returns (window closed), fire `shutdown_tx` so axum closes connections cleanly, then exit.
- **Single-instance:** ephemeral ports remove the port conflict, but two instances still open the same SQLite file (→ `database is locked`). Add a single-instance guard (named mutex on Windows / advisory lock or lockfile under the data dir on Linux); on a second launch, focus/notify rather than starting a second backend. *(Design decision — recommend a lightweight lockfile; the plan sizes it.)*
- **Acceptance:** launch → GUI appears → backend reachable on the reported ephemeral port → window-close exits cleanly (graceful shutdown fired); a **force-kill leaves NO orphan / held resource**; a **second launch** does not spawn a second backend or corrupt the DB; a **backend bind/migration failure** surfaces a visible error, not a frozen GUI.

## Phase 3 — Windows Inno Setup installer

- **Install scope:** per-user (`%LocalAppData%\Programs\rsahp`) — **no UAC/admin prompt.**
- Add a simple placeholder app icon (`packaging/windows/rsahp.ico`, multi-resolution), swappable later.
- `packaging/windows/rsahp.iss` (Inno script): installs `rsahp-desktop.exe` + icon + `LICENSE`; Start-menu shortcut (Desktop shortcut opt-in); uninstaller; **app version injected explicitly** — the build step runs `cargo metadata` (or an `xtask`) to extract the version string and passes it to `iscc.exe` via `/DMyAppVersion=<x.y.z>` (Inno cannot parse TOML). No bundled DB/config — created in the per-user local data dir at first run.
- **Output:** `iscc` writes to its `OutputDir` (set explicitly, e.g. `dist/`) as `rsahp-setup-<version>.exe` — this exact path/name is what CI uploads.
- **Acceptance:** install (no admin) → Start-menu launch → app works → uninstall removes program files (user data under `%LocalAppData%\…\rsahp\` is preserved unless a "remove data" checkbox is opted).

## Phase 4 — Linux Flatpak

- **App-id:** `io.github.ckir.rsahp` (reverse-DNS). The `.desktop`, icon, and AppStream file are **named exactly** `io.github.ckir.rsahp.desktop` / `io.github.ckir.rsahp.png` (+ hicolor sizes) / `io.github.ckir.rsahp.metainfo.xml` (Flatpak requires the app-id prefix).
- **Pinned runtime:** `org.freedesktop.Platform` + `Sdk` at a **fixed version** (e.g. `//24.08` — the plan pins the current stable, not "latest").
- **Offline build:** flatpak-builder sandboxes the network, so the Rust build cannot reach crates.io. Generate a `cargo-sources.json` via `flatpak-cargo-generator.py` (from `Cargo.lock`) and reference it in the manifest to pre-fetch all crates into the build cache. Add the `rust-stable` SDK extension (or vendored toolchain) for `cargo` inside the sandbox.
- **finish-args:** `--share=network` (localhost socket), `--socket=wayland` + `--socket=fallback-x11` + `--device=dri` (egui/eframe GL — respects the existing `use_gpu` software fallback), default writable app-data (SQLite under `~/.var/app/io.github.ckir.rsahp/`).
- **Bundle:** `flatpak-builder` outputs a repo tree; producing the single `.flatpak` requires `flatpak build-export <repo> <builddir>` then `flatpak build-bundle <repo> rsahp-<ver>.flatpak io.github.ckir.rsahp` — these exact steps + output path are specified for CI upload.
- **Output:** `rsahp-<version>.flatpak` for local install (`flatpak install --user`). Flathub submission is out of scope (later).
- **Acceptance:** `flatpak-builder` (with pre-fetched sources) builds; install + launch shows the GUI, backend reachable inside the sandbox, DB persists under the app data dir across restarts.

## Build pipeline

- **Primary — CI-driven release** (GitHub Actions, on a version tag): builds `rsahp-desktop` per-OS and produces both installers. The job **must install the toolchains** (not preinstalled on runners): Windows → Inno Setup (e.g. `choco install innosetup`); Ubuntu → `flatpak` + `flatpak-builder` + `flatpak remote-add flathub` + `flatpak install` the pinned runtime/SDK, plus the cargo-sources generator. It then uploads `dist/rsahp-setup-<ver>.exe` and `rsahp-<ver>.flatpak` (exact paths from Phases 3/4) to the GitHub Release. This is what the deferred cockpit's SHIP & RELEASE tier will trigger/watch.
- **Secondary — local Windows dev build:** an `xtask` action (or `scripts/` helper) that builds `rsahp-desktop`, extracts the version via `cargo metadata`, and runs `iscc /DMyAppVersion=…`, so the Windows installer can be produced/tested on-machine without a CI round-trip. (Local Flatpak build is documented but not required.)

## Testing

- Phase 1: unit-test data-dir path resolution (assert local/non-roaming), first-run config seed (all keys), Windows-path→sqlite-URL conversion.
- Phase 2: smoke-test the wrapper (launch → GUI up → `GET <ephemeral>/…` OK → clean exit via graceful shutdown; force-kill → no orphan; second launch → single-instance guard fires; simulated backend bind-failure → visible error not a frozen GUI).
- Phase 3/4: manual install → launch → uninstall on each OS; a CI job that at minimum *builds* both installers (catches manifest/script/toolchain breakage) even before a tagged release.

## Explicitly out of scope / deferred

macOS packaging; Windows code signing / Authenticode; Flatpak/Flathub submission + signing; auto-update; collapsing the HTTP boundary into direct in-process calls (a possible future optimization only if warranted).

## Phasing & decomposition

Four sequential increments, each independently testable: **P1 data-dir** (mergeable first) → **P2 wrapper** (produces the shippable binary; owns the ephemeral-port / ready-signal / graceful-shutdown / single-instance logic) → **P3 Inno** → **P4 Flatpak**. The build pipeline is wired incrementally as P3/P4 land. Each phase becomes a section of the implementation plan.

---

<details><summary><b>Panel fold ledger — round 1 (solo + agy), 20 findings applied</b></summary>

Solo panel: (1) ProjectDirs stated-path vs API-call contradiction; (2) frontend config resolution must also move; (3) embedded-backend-thread failure was silent → surfaced error path; (4) second-instance port+SQLite conflict → single-instance guard; (5) database_url must be absolute + URI-safe (Windows slashes/colon); (6) seeded config must include use_gpu; (7) Flatpak artifacts must be app-id-named; (8) log location surfaced; (9) Flatpak runtime unpinned → pinned; (10) CI missing Inno/flatpak-builder/runtime installs + GL finish-args; (11) data-dir init location (reconciled below).

agy escalation: (12) dev-mutation footgun — standalone bins default to cwd, wrapper owns the data-dir default (reconciles #11); (13) **ephemeral port** `127.0.0.1:0` + in-memory port passing (dissolves the static-port fragility); (14) graceful shutdown via `with_graceful_shutdown` + oneshot (agy's "SQLite corruption" framing corrected — SQLite is durable across hard kill; folded as clean-close robustness); (15) data-dir init FS race → init ONCE in the wrapper before spawn; (16) `data_local_dir()` (%LocalAppData%) not roaming `data_dir()`; (17) CI output paths (iscc `Output/`, bundle path) surfaced for upload; (18) Flatpak offline build needs `cargo-sources.json` (flatpak-cargo-generator); (19) `.flatpak` needs `build-export` + `build-bundle`, not just flatpak-builder; (20) version injection mechanism (`cargo metadata` → `iscc /DMyAppVersion`). Plus a determinism fold: blind startup retry → deterministic ready-signal channel.

</details>
