# rsahp (Rust AHP)

**rsahp** is a full-stack Rust application for Analytic Hierarchy Process (AHP) group decision-making. It enables complex, N-level criteria evaluation by aggregating pairwise comparisons from users.

## Features

- **Virtual Desktop UI**: A sleek, Windows Desktop-like interface built with `egui`, completely avoiding browser-tab fatigue. Includes a taskbar, project explorer, and MDI floating windows.
- **Robust AHP Math Engine**: N-level criteria hierarchy modeling with mathematical matrix calculation using `nalgebra`. Automatically computes Principal Eigenvectors and Consistency Ratios (CR).
- **Group Aggregation**: Toggles between AIJ (Aggregation of Individual Judgments) and AIP (Aggregation of Individual Priorities) for robust multi-user analytics.
- **Drag-and-Drop Explorer**: Organize documents dynamically with persistent folder hierarchies.
- **Offline Support (PDF integration)**: Generate and distribute AHP fillable AcroForms (PDFs) directly to external evaluators and batch-import their offline feedback back into the mathematical ecosystem.

## Architecture

- **Frontend**: Rust `egui` framework (Native OS application with WebAssembly support).
- **Backend**: Rust `axum` server providing reliable REST APIs.
- **Database**: Local SQLite via `sea-orm` (Easily scalable to PostgreSQL for production environments).

## Getting Started

### Prerequisites

- [Rust Toolchain](https://rustup.rs/) (Cargo)

### Developer Cockpit (`cargo xtask`)

This project uses an interactive Rust-native task runner (Cockpit) to manage the build and deployment lifecycles.

To start the Cockpit menu, simply run:
```bash
cargo xtask
```

> **Required dev tools:** `cargo-nextest` is a hard prerequisite for the `lefthook` pre-commit hook (`cargo install cargo-nextest --locked`). `cargo-llvm-cov` is optional (coverage only). The SessionStart hook (`.claude/recommended-tools.json`) surfaces any missing tools. Note: nextest does **not** run doctests — there are 0 doctests today, but if you add one, also add a `cargo test --doc` step so it is not silently skipped.

You will be presented with a terminal UI to select your workflow:
1. **Quick Loop**: Instantly builds the workspace, kills old background processes, and relaunches the frontend and backend.
2. **Quality Gate**: Runs `cargo fmt` and `cargo clippy` over the workspace.
3. **Run Unit Tests**: Executes `cargo nextest run` (falls back to `cargo test` if `cargo-nextest` is not installed).
4. **Coverage Report**: Runs `cargo llvm-cov nextest` (prints an install hint if `cargo-llvm-cov` is absent).
5. **Fullscale Workflow**: Runs tests, formats, builds, launches, commits with an interactive message prompt, and pushes to GitHub (hooking into `lefthook`).
6. **Version Bump**: Bumps the workspace version in lockstep across all crates.

Alternatively, you can bypass the menu by specifying the command directly:
- `cargo xtask quick`
- `cargo xtask fullscale "Your commit message"`

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

## License

This project is licensed under the [PolyForm Noncommercial License 1.0.0](LICENSE) — free for noncommercial use. See the [LICENSE](LICENSE) file for the full terms.
