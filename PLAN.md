# Implementation Plan: AHP Group Decision Support System

## Phase 1: Project Skeleton & Configuration
- [ ] Initialize a new Rust Cargo workspace containing `frontend` (egui) and `backend` (axum).
- [ ] Implement rotational logging using `tracing` and `tracing-appender` (10MB size limit, partitioned by date) in both services.
- [ ] Set up the basic `eframe` window structure for the frontend.
- [ ] Set up the basic `axum` routing structure for the backend.

## Phase 2: Core Models & Database (SQLite -> Postgres path)
- [ ] Set up database connection using an ORM (e.g., `SeaORM` or `sqlx`) pointing to a local SQLite file.
- [ ] Define User and UserGroup models.
- [ ] Define Document and N-level Criteria/Node models.
- [ ] Define Pairwise Comparison Result models.

## Phase 3: Frontend Virtual Desktop UI
- [ ] Build the Bottom Taskbar and "Task List" popup.
- [ ] Build the pinned Project Explorer (Tree view, right-click context menus).
- [ ] Build the MDI Window Manager for floating document windows.
- [ ] Build the Document Toolbar (Save, Print, Export PDF, Toggle AIJ/AIP).

## Phase 4: Core AHP Math Engine
- [ ] Implement matrix structures and Eigenvector/Eigenvalue calculations using `nalgebra`.
- [ ] Implement Consistency Ratio (CR) calculation.
- [ ] Implement AIJ (Aggregation of Individual Judgments) logic via Geometric Mean.
- [ ] Implement AIP (Aggregation of Individual Priorities) logic.

## Phase 5: Workflow & UI Integration
- [ ] Implement the Step-by-step (Wizard) and Scrolling views for comparison sliders.
- [ ] Implement the Soft-Warning UI for CR > 0.10.
- [ ] Connect the frontend UI to the backend REST API for saving/loading documents.

## Phase 6: PDF Generation & Batch Import
- [ ] Implement fillable PDF generation with a Name/Email field and Saaty scale inputs.
- [ ] Implement PDF parsing to extract responses from filled forms.
- [ ] Wire up the "Batch Import PDFs" toolbar action to process and deduplicate responses.

## Phase 7: Exhaustive E2E Testing
- [ ] Set up `axum-test` dependency in `backend`.
- [ ] Create `backend/tests/common/` with `TestContext` (in-memory SQLite, migrations, router initialization).
- [ ] Implement Golden Matrices in `tests/common/golden_data.rs`.
- [ ] Implement `test_auth.rs` (Login, Authorization).
- [ ] Implement `test_folders.rs` (Tree manipulation, cycles).
- [ ] Implement `test_math.rs` (AIJ, AIP, CR validation against golden data).
- [ ] Implement `test_documents.rs` (Full E2E creation, distribution, aggregation flow).
- [ ] Implement isolated frontend state tests in `frontend/src/tests/`.
