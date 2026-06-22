# Document Versioning Design

## Understanding Summary
*   **What is being built:** A "Save as new version..." feature for the AHP document editor.
*   **Why it exists:** To allow users to create safe snapshots or branches of their current work to experiment with different structures or comparisons without losing their original data.
*   **Who it is for:** End-users building complex AHP models who need iteration checkpoints.
*   **Key constraints:** The new version will act as a completely separate document and will instantly appear as its own entry in the Project Explorer sidebar.
*   **Explicit non-goals:** We are *not* building a Git-like revision history system or a timeline view. We are *not* modifying the database schema to track complex parent/child lineage between documents.

## Assumptions
*   When a user clicks "Save as new version...", the system will deep-copy the entire document (metadata, all nodes, and all pairwise comparisons).
*   The new document will simply take the current name and append a version or copy indicator (e.g., changing "Vendor Selection" to "Vendor Selection (v2)").
*   If the user has unsaved modifications when clicking "Save as new version...", the frontend will first auto-save the current document, and then trigger the duplication.

## Decision Log
*   **Decision 1: Storage Approach**
    *   *What was decided:* We will duplicate documents entirely as separate, standalone records in the database rather than building a complex single-document revision history table.
    *   *Alternatives considered:* Keeping a full history of changes under a single document ID; grouping versions in the sidebar automatically.
    *   *Why this option was chosen:* Simpler schema, highly flexible for the user (they can branch and test variants easily), and prevents massive sidebar clutter by ensuring branching is an explicit user action ("Save as new version") rather than happening automatically on every save.
*   **Decision 2: Execution Location**
    *   *What was decided:* The deep copy will be executed entirely on the backend via a single SQLite transaction (`POST /api/documents/{id}/duplicate`).
    *   *Alternatives considered:* Exporting and auto-importing JSON via the frontend; having the frontend strip IDs from its memory state and forcing a "create from scratch" payload.
    *   *Why this option was chosen:* It guarantees data integrity, handles ID re-mapping safely within the database, and keeps the frontend thin and fast.

## Final Design

### Part 1: Backend Architecture
We will expose a new route: `POST /api/documents/{id}/duplicate`.
The transaction will perform the following steps:
1.  **Fetch Original:** Read the document metadata, nodes, and comparisons for `{id}`.
2.  **Create New Document:** Insert a new document record, appending "(v2)" or "(Copy)" to the name. Get the `new_doc_id`.
3.  **Map Nodes:** Insert all nodes linked to `new_doc_id`. Maintain a `HashMap<old_node_id, new_node_id>` to remap parent-child relationships as we insert.
4.  **Map Comparisons:** Insert all comparisons linked to `new_doc_id`, rewriting `node_a_id`, `node_b_id`, and `parent_node_id` using the lookup map.
5.  **Return:** Commit the transaction and return the new document metadata.

### Part 2: Frontend Integration
1.  **UI Addition:** Add a `📄 Save as New Version` button next to the standard `💾 Save` button in the document window header.
2.  **Triggering:** When clicked, ensure any pending changes are saved, then trigger the `POST /duplicate` endpoint. Show a "Duplicating..." loading state.
3.  **Sidebar Update:** Upon success, receive the new document metadata and immediately inject it as a new `Node::File` into the `ExplorerState` tree so it appears in the sidebar.
