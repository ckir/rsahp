# AHP Group Decision Support System - Design Document

## 1. Understanding Summary
**What is being built:** A full-stack Rust application for Analytic Hierarchy Process (AHP) group decision-making. The frontend uses `egui` to simulate a "Windows Desktop" UI (running natively or via WebAssembly), backed by a Rust server API.
**Why it exists:** To facilitate complex, N-level criteria evaluation by aggregating pairwise comparisons from both internal users (online) and external users (offline via PDF).
**Who it is for:** Registered system users who can create AHP hierarchies, distribute them to specific user groups or individuals, and answer surveys assigned to them.
**Explicit Non-goals:** Real-time synchronous collaboration (users work asynchronously and data is aggregated).

## 2. Assumptions & Constraints
* **Math Backend:** Matrix calculations (Principal Eigenvectors, Consistency Ratios). Group aggregation supports both **AIJ** (Aggregation of Individual Judgments) and **AIP** (Aggregation of Individual Priorities).
* **Versioning:** Modifying a document's criteria increments its version. Submitted responses (online or PDF) must match the document version to ensure mathematical integrity.
* **PDF Technical Limitations:** Generating and parsing fillable AcroForms entirely in Rust may require visual simplicity in the generated PDFs to ensure compatibility across various PDF readers.
* **Database ORM:** An ORM (like `SeaORM`) will be used. The system will start with SQLite for initial development and migrate to PostgreSQL for production.

## 3. Final Design Details
### Frontend UI (Virtual Desktop)
* **Taskbar:** Located at the bottom. Includes a "Start/Menu" equivalent, pinned apps, and minimized document windows.
* **Task List:** A specific button on the taskbar opens a window displaying the user's pending AHP documents (surveys they need to answer).
* **Project Explorer:** A pinned window featuring a tree-view for document management. Users can right-click (New, Duplicate, Delete) and drag-and-drop to organize their owned documents.
* **Document Windows:** Floating MDI windows with Minimize/Maximize/Restore/Close controls.
* **Toolbar (Top of Document):** Contains actions: `Save`, `Print`, `Export PDF`, `Batch Import PDFs`. Also provides a toggle between **AIJ / AIP** aggregation methods. Displays document version.
* **Comparison Input:** Users use sliders on a 1-9 Saaty scale. The UI provides a toggle between a **Wizard (Step-by-step)** view and a **Single Scrolling** page view.
* **Consistency Check:** If a user's Consistency Ratio (CR) is > 0.10, they receive a **soft warning** but can still submit. The result is flagged for the Manager.
* **Global Zoom / UI Scaling:** A zoom button on the taskbar opens a slider (0.5x to 3.0x) that scales the entire application interface globally. The default base scale is set slightly larger (e.g., 1.25x) for better out-of-the-box readability. User preference is persisted across sessions in `config.json`.

### Backend Architecture
* **Server:** Rust-based (e.g., `axum`).
* **User Management:** Full accounts with authentication. An Admin role defines User Groups (e.g., "Management", "Operations").
* **Distribution (Internal):** Users can assign documents to specific users or groups. These appear in the assignees' "Task List".
* **Distribution (External):** Users export a fillable PDF and email it. When returned, the user batch-imports them. Deduplication is handled via a required "Name/Email" field inside the PDF form.

## 4. Decision Log
1. **Desktop-like UI vs Web UI:** Chose a desktop MDI paradigm within `egui` to allow users to manage complex multi-document workflows efficiently without opening dozens of browser tabs.
2. **N-Level Hierarchy vs Fixed 3-Level:** Chose N-level to support deeply complex enterprise decision models, accepting the UI complexity of rendering deep trees.
3. **Full-Stack vs Serverless:** Pivoted from a local-only app to a full-stack app with a database to support internal user accounts, user groups, and online survey completion.
4. **SQLite -> Postgres Path:** Decided to use an ORM to allow rapid prototyping with SQLite while guaranteeing a smooth transition to PostgreSQL.
5. **PDF Deduplication:** Decided to require a manual Name/Email field in the exported PDF rather than trying to track uniquely generated PDFs per user, optimizing for implementation simplicity.
6. **Aggregation Math:** Allowed the Manager to toggle between AIJ and AIP per document, as different evaluation contexts require different aggregation methodologies.
8. **Global UI Scaling:** Opted for a Taskbar-based visual slider over just keyboard shortcuts to maximize discoverability. Chose a default scale of 1.25x rather than 1.0x to immediately address readability issues with default fonts. User preference is persisted locally.
