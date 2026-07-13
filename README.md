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

You will be presented with a terminal UI to select your workflow:
1. **Quick Loop**: Instantly builds the workspace, kills old background processes, and relaunches the frontend and backend.
2. **Quality Gate**: Runs `cargo fmt` and `cargo clippy` over the workspace.
3. **Run Unit Tests**: Executes `cargo test`.
4. **Fullscale Workflow**: Runs tests, formats, builds, launches, commits with an interactive message prompt, and pushes to GitHub (hooking into `lefthook`).

Alternatively, you can bypass the menu by specifying the command directly:
- `cargo xtask quick`
- `cargo xtask fullscale "Your commit message"`

## License

This project is licensed under a custom Non-Commercial License (Free for non-commercial use). See the [LICENSE](LICENSE) file for details.
