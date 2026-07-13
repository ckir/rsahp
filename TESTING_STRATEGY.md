# Testing Strategy & Regression Prevention

## Understanding Summary
- **What is being built**: An enhanced verification and safety-net strategy for the `rsahp` codebase.
- **Why it exists**: To eliminate regressions (e.g., environmental issues, config path mismatches) before they affect runtime.
- **Who it is for**: Developers and AI agents iterating on the rsahp project.
- **Key constraints**: Local tests must be deterministic, fast, and avoid dragging down constrained local hardware (e.g., dual-core CPUs).
- **Explicit non-goals**: We are not rewriting the existing unit tests; we are augmenting them with structured pipeline checks.

## Assumptions (Non-Functional Requirements)
- **Performance**: The local verification suite must run in under 2 minutes.
- **Scale**: The GitHub Actions runner will handle heavy loads since it's a public repo with generous free tiers.
- **Reliability**: Tests must clean up after themselves and not leave lingering DB artifacts.
- **Maintenance**: Low-overhead maintenance; the heavy lifting is isolated to standard GitHub VMs.

## Decision Log
1. **Decision**: Use a Dual-Tier Testing Strategy (Fast Local + Heavy Remote).
   - *Alternatives considered*: Heavy snapshot testing locally, formal database migrations.
   - *Why chosen*: The developer's local hardware (Core i3) is highly constrained. We must keep local checks minimal to maintain development velocity, while leveraging powerful GitHub runners for the intense UI/E2E tasks.
2. **Decision**: Enforce local checks using `lefthook`.
   - *Alternatives considered*: Relying on human/AI discipline to run a `.ps1` script before committing.
   - *Why chosen*: Automating enforcement guarantees that simple syntax or config-path errors never make it into a commit or trigger a remote workflow, saving time and preventing sloppy regressions.

## Final Design

### Tier 1: Local Enforcement (Using Lefthook)
`lefthook` acts as a strict gatekeeper on the local machine:
- **Pre-Commit Hook**: Runs lightning-fast checks (`cargo fmt`, `cargo clippy`, and `cargo nextest run`). Commits are blocked if these fail.
- **Pre-Push Hook**: Runs a `verify.ps1` "Configuration Contract Test". It temporarily boots the backend using the physical `config.json` to ensure the app can actually bind to the port and locate the correct `rsahp.db` file. 

### Tier 2: Remote Exhaustive Suite (GitHub Actions)
A GitHub Actions workflow handles the deep verification on push/PR:
- **Compilation**: Checks out code and compiles both backend and frontend in an isolated Ubuntu/Windows VM.
- **Headless E2E Execution**: Boots the system and runs an exhaustive headless test suite against the `egui` native app to simulate real clicks, folder drags, and mathematical engine evaluations.
- **Artifact Capture**: On failure, automatically archives `rsahp.db`, logs, and crash dumps for remote debugging without local reproduction.
