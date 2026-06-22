# AHP Group Decision Support System - Testing Strategy

## 1. Understanding Summary
*   **What is being built:** A hybrid E2E and integration testing suite for the `rsahp` application.
*   **Why it exists:** To guarantee the correctness of the backend endpoints, the accuracy of the complex AHP math engine, and the state management of the frontend, preventing regressions as the application scales.
*   **Who it is for:** Developers and CI systems working on the `rsahp` repository.
*   **Key constraints:** Since the frontend is `egui` (canvas-based), we are avoiding brittle layout/UI automation and instead testing the frontend's internal state logic combined with exhaustive API tests. 
*   **Explicit non-goals:** We are *not* testing native pixel rendering or driving a headless browser to click on canvas coordinates.

## 2. Assumptions
*   **Testing Framework:** We use Rust's native `cargo test` framework.
*   **Database Isolation:** Tests use an in-memory SQLite database (`sqlite::memory:`) initialized fresh per test. This guarantees a clean slate, eliminates cross-test pollution, and safely enables parallelism.
*   **Math Engine Veracity:** The AHP math (Eigenvectors, CR, AIJ, AIP) is tested with known "golden matrices" representing expected real-world valid and invalid data.
*   **Execution Behavior:** Standard `cargo test` runs in parallel by default. Code supports parallel execution (for CI), but developers can opt to run sequentially locally.

## 3. Decision Log
1. **Testing Boundary & Approach**
   *   **Decided:** A hybrid approach using direct Axum `Router` testing for the backend and isolated state unit tests for the frontend.
   *   **Alternatives Considered:** Full UI automation (driving the egui canvas) and Black-box HTTP server spawning.
   *   **Why:** Egui canvas testing is notoriously brittle and complex. Directly testing the backend via the Axum router bypasses network overhead, ensuring maximum speed and stability.
2. **Database Isolation Strategy**
   *   **Decided:** Use an in-memory SQLite database (`sqlite::memory:`) initialized fresh per test case.
   *   **Alternatives Considered:** A persistent file-based test database (`test.db`) or completely mocking the database layer.
   *   **Why:** In-memory SQLite provides the truest reflection of our production database logic without the disk I/O overhead. Initializing it fresh per test guarantees complete isolation.
3. **Math Engine Testing Strategy**
   *   **Decided:** Use pre-calculated "Golden Matrices" combined with E2E API aggregation tests.
   *   **Alternatives Considered:** Strict property-based testing.
   *   **Why:** Generating random numbers rarely yields valid AHP matrices. Hardcoding known, mathematically proven matrices ensures our engine is accurate and handles edge cases (like high CR) deterministically.

## 4. Final Design

### Architecture & File Structure
We utilize direct HTTP service testing (e.g., `axum-test` or `tower::ServiceExt`) to send requests directly to our backend `Router` in memory.

**Test Context Lifecycle:**
1. Spin up a new in-memory SQLite database.
2. Run SeaORM migrations automatically.
3. Seed the database with default users and mock data.
4. Construct the Axum `Router` with this isolated database pool.
5. Provide helper methods for generating authenticated sessions.

**Directory Structure:**
```text
backend/
├── src/
├── tests/
│   ├── common/           # TestContext, db setup, auth helpers, golden matrices
│   ├── test_auth.rs      # Login, session validation, access control
│   ├── test_folders.rs   # Creating, moving, and tree validation
│   ├── test_math.rs      # AHP logic: AIJ, AIP, Eigenvectors, CR
│   └── test_documents.rs # E2E flows of survey creation -> distribution -> aggregation
```
Frontend logic is tested via `#[cfg(test)]` modules inside `frontend/src/` focusing on state machine logic.

### Math Engine Testing & Golden Matrices
Inside `tests/common/golden_data.rs`, we define:
1. **Perfect Consistency Model:** A matrix where CR = 0.0.
2. **Acceptable Consistency Model:** A matrix where 0.0 < CR <= 0.10.
3. **Invalid Consistency Model:** A matrix where CR > 0.10 (asserting the API returns a soft warning flag).

**Data Flow:** Tests will authenticate, create a document, submit pairwise comparisons from the golden matrices, fetch aggregation results via AIJ/AIP, and assert the output matches expected weights.

### E2E Workflows, Error Handling & Edge Cases
*   **Creation & Distribution:** Complete lifecycle of an admin creating a document, placing it in a folder, assigning it, and a standard user answering it.
*   **Authentication & Authorization:** Asserting `401 Unauthorized` and `403 Forbidden` for invalid access attempts.
*   **Missing or Malformed Data:** Asserting graceful `400 Bad Request` responses for invalid inputs.
*   **Cyclic Folder Structures:** Asserting the API rejects moving a folder inside of its own child folder.
*   **Version Mismatches:** Asserting rejections when submitting a response to an outdated document version.
