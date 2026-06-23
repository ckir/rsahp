//! This module renders the project explorer, displaying the user's folders and documents in a tree view.

use super::document_window::DocumentState;
use common::{DocumentDto, TreeDto, FolderDto};
use eframe::egui;
use egui_ltreeview::{Action, DirPosition, NodeBuilder, TreeView, TreeViewState};

/// Represents a node in the file system tree.
pub enum Node {
    /// A directory node containing other nodes.
    Directory(Directory),
    /// A file node representing a document.
    File(File),
}

/// Represents a directory in the tree.
pub struct Directory {
    /// The unique identifier for this directory.
    pub id: usize,
    /// The name of the directory.
    pub name: String,
    /// The children nodes contained within this directory.
    pub children: Vec<Node>,
}

/// Represents a file in the tree.
pub struct File {
    /// The unique identifier for this file node.
    pub id: usize,
    /// The name of the file.
    pub name: String,
    /// The associated document ID in the database, if any.
    pub document_id: Option<usize>,
}

impl Node {
    /// Returns the unique ID of the node.
    pub fn id(&self) -> usize {
        match self {
            Node::Directory(d) => d.id,
            Node::File(f) => f.id,
        }
    }

    /// Returns the name of the node.
    pub fn name(&self) -> &str {
        match self {
            Node::Directory(d) => &d.name,
            Node::File(f) => &f.name,
        }
    }

    /// Recursively attempts to remove a node by its ID.
    pub fn remove(&mut self, id: usize) -> Option<Node> {
        match self {
            Node::Directory(dir) => {
                // If the child is directly within this directory, remove it.
                if let Some(index) = dir.children.iter().position(|n| n.id() == id) {
                    Some(dir.children.remove(index))
                } else {
                    // Otherwise, search recursively.
                    for node in dir.children.iter_mut() {
                        if let Some(r) = node.remove(id) {
                            return Some(r);
                        }
                    }
                    None
                }
            }
            Node::File(_) => None,
        }
    }

    /// Recursively attempts to insert a node at a specific position under a parent ID.
    pub fn insert(
        &mut self,
        parent_id: usize,
        position: DirPosition<usize>,
        value: Node,
    ) -> Result<(), Node> {
        match self {
            Node::Directory(dir) => {
                if dir.id == parent_id {
                    // Insert the node based on the requested position.
                    match position {
                        DirPosition::First => dir.children.insert(0, value),
                        DirPosition::Last => dir.children.push(value),
                        DirPosition::After(after_id) => {
                            if let Some(index) =
                                dir.children.iter().position(|n| n.id() == after_id)
                            {
                                dir.children.insert(index + 1, value);
                            }
                        }
                        DirPosition::Before(before_id) => {
                            if let Some(index) =
                                dir.children.iter().position(|n| n.id() == before_id)
                            {
                                dir.children.insert(index, value);
                            }
                        }
                    }
                    Ok(())
                } else {
                    // Recursively attempt to insert into child directories.
                    let mut value = Err(value);
                    for node in dir.children.iter_mut() {
                        if let Err(v) = value {
                            value = node.insert(parent_id, position, v);
                        }
                    }
                    value
                }
            }
            _ => Err(value),
        }
    }
}

/// Action to perform when the modal is closed.
#[derive(Clone)]
pub enum ModalAction {
    /// Add a file under the specified parent at the specified position.
    AddFile(usize, DirPosition<usize>),
    /// Add a directory under the specified parent at the specified position.
    AddDir(usize, DirPosition<usize>),
    /// Confirm deletion of the node with the specified ID.
    ConfirmDelete(usize),
}

/// State for the explorer modal dialog.
pub struct ModalState {
    /// The action the modal represents.
    pub action: ModalAction,
    /// The current text input.
    pub input_name: String,
}

/// UI state for the project explorer.
pub struct ExplorerState {
    /// Status message for file imports.
    pub import_status: Option<String>,
    /// The root node of the file system tree.
    pub tree: Node,
    /// State of the egui_ltreeview.
    pub tree_view_state: TreeViewState<usize>,
    /// The next available unique ID for a node.
    pub next_id: usize,
    /// The state of the modal dialog, if open.
    pub modal_state: Option<ModalState>,
    /// Indicates whether the initial tree data has been fetched.
    pub fetched_initial: bool,
    /// Receiver channel for the asynchronous tree data response.
    pub tree_rx: Option<std::sync::mpsc::Receiver<TreeDto>>,
    /// Receiver channel for new document creation.
    pub new_doc_rx: Option<std::sync::mpsc::Receiver<(usize, DocumentDto)>>,
}

/// Data transfer object for a document.
/// Data transfer object for a folder.
#[derive(serde::Deserialize, Clone)]
pub struct FolderModel {
    /// The folder's ID.
    pub id: i32,
    /// The folder's name.
    pub name: String,
    /// The ID of the parent folder, if any.
    pub parent_folder_id: Option<i32>,
}

/// Data transfer object for the file system tree.
/// Default implementation for `ExplorerState`.
impl Default for ExplorerState {
    fn default() -> Self {
        // Create a default root directory.
        let tree = Node::Directory(Directory {
            id: 0,
            name: "My AHP Documents".to_string(),
            children: vec![],
        });

        Self {
            import_status: None,
            tree,
            tree_view_state: TreeViewState::default(),
            next_id: 3,
            modal_state: None,
            fetched_initial: false,
            tree_rx: None,
            new_doc_rx: None,
        }
    }
}

/// Actions available from the context menu.
enum ContextMenuActions {
    /// Delete the specified node.
    Delete(usize),
    /// Add a leaf (file) node at the specified position.
    AddLeaf(usize, DirPosition<usize>),
    /// Add a directory node at the specified position.
    AddDir(usize, DirPosition<usize>),
}

/// Recursively renders a node in the tree view.
fn show_node(
    builder: &mut egui_ltreeview::TreeViewBuilder<usize>,
    node: &Node,
    actions: &mut Vec<ContextMenuActions>,
) {
    match node {
        Node::Directory(dir) => {
            // Render a directory node.
            builder.node(
                NodeBuilder::dir(dir.id)
                    .label(&dir.name)
                    .default_open(true)
                    .context_menu(|ui| {
                        ui.set_width(100.0);
                        ui.label("dir:");
                        ui.label(&dir.name);
                        ui.separator();
                        // Delete action
                        if ui.button("delete").clicked() {
                            actions.push(ContextMenuActions::Delete(dir.id));
                            ui.close();
                        }
                        ui.separator();
                        // New file action
                        if ui.button("new file").clicked() {
                            actions.push(ContextMenuActions::AddLeaf(dir.id, DirPosition::Last));
                            ui.close();
                        }
                        // New directory action
                        if ui.button("new directory").clicked() {
                            actions.push(ContextMenuActions::AddDir(dir.id, DirPosition::Last));
                            ui.close();
                        }
                    }),
            );
            // Render children.
            for child in &dir.children {
                show_node(builder, child, actions);
            }
            builder.close_dir();
        }
        Node::File(file) => {
            // Render a file node.
            let parent_node = builder.parent_id().copied().unwrap_or(0);
            builder.node(
                NodeBuilder::leaf(file.id)
                    .label(&file.name)
                    .activatable(true)
                    .context_menu(|ui| {
                        ui.set_width(100.0);
                        ui.label("file:");
                        ui.label(&file.name);
                        // Delete action
                        if ui.button("delete").clicked() {
                            actions.push(ContextMenuActions::Delete(file.id));
                            ui.close();
                        }
                        ui.separator();
                        // New file action
                        if ui.button("new file").clicked() {
                            actions.push(ContextMenuActions::AddLeaf(
                                parent_node,
                                DirPosition::After(file.id),
                            ));
                            ui.close();
                        }
                        // New directory action
                        if ui.button("new directory").clicked() {
                            actions.push(ContextMenuActions::AddDir(
                                parent_node,
                                DirPosition::After(file.id),
                            ));
                            ui.close();
                        }
                    }),
            );
        }
    }
}

/// Renders the file explorer panel.
pub fn render(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut ExplorerState,
    open_documents: &mut Vec<DocumentState>,
    api_url: &str,
    jwt_token: Option<&str>,
) {
    // Check if initial fetch is needed.
    if !state.fetched_initial && state.tree_rx.is_none() {
        let (tx, rx) = std::sync::mpsc::channel();
        state.tree_rx = Some(rx);
        state.fetched_initial = true;

        // Construct the URL for the tree endpoint.
        let mut tree_url = api_url.to_string();
        if tree_url.ends_with('/') {
            tree_url.pop();
        }
        tree_url.push_str("/tree");

        // Prepare the GET request.
        let mut request = ehttp::Request::get(tree_url);
        if let Some(token) = jwt_token {
            request
                .headers
                .insert("Authorization", &format!("Bearer {}", token));
        }

        let ctx_clone = ctx.clone();
        // Execute the background fetch.
        ehttp::fetch(request, move |result| {
            match result {
                Ok(res) => {
                    if res.status == 200 {
                        // Attempt to parse the response.
                        match serde_json::from_slice::<TreeDto>(&res.bytes) {
                            Ok(tree_dto) => {
                                let _ = tx.send(tree_dto);
                            }
                            Err(e) => tracing::error!(
                                "Failed to parse JSON tree: {}. Body: {}",
                                e,
                                String::from_utf8_lossy(&res.bytes)
                            ),
                        }
                    } else {
                        tracing::error!(
                            "Fetch tree failed: Status: {}, Body: {:?}",
                            res.status,
                            res.text()
                        );
                    }
                }
                Err(e) => tracing::error!("Fetch tree request error: {}", e),
            }
            // Request UI repaint.
            ctx_clone.request_repaint();
        });
    }

    // Process the fetch results.
    if let Some(rx) = &state.tree_rx
        && let Ok(tree_dto) = rx.try_recv()
    {
        // Helper function to build nodes recursively from the DTO.
        fn build_node(
            parent_folder_id: Option<i32>,
            dto: &TreeDto,
            next_id: &mut usize,
        ) -> Vec<Node> {
            let mut children = Vec::new();

            // Build subfolders.
            for folder in &dto.folders {
                if folder.parent_folder_id == parent_folder_id {
                    let dir_id = folder.id as usize;
                    if dir_id >= *next_id {
                        *next_id = dir_id + 1;
                    }
                    children.push(Node::Directory(Directory {
                        id: dir_id,
                        name: folder.name.clone(),
                        children: build_node(Some(folder.id), dto, next_id),
                    }));
                }
            }

            // Build documents within the folder.
            for doc in &dto.documents {
                if doc.folder_id == parent_folder_id {
                    let file_id = *next_id;
                    *next_id += 1;
                    children.push(Node::File(File {
                        id: file_id,
                        name: doc.name.clone(),
                        document_id: Some(doc.id as usize),
                    }));
                }
            }
            children
        }

        // Build the root children list and update the tree.
        let children = build_node(None, &tree_dto, &mut state.next_id);
        state.tree = Node::Directory(Directory {
            id: 0, // Root ID is 0
            name: "My AHP Documents".to_string(),
            children,
        });
        // Clear the receiver as fetching is complete.
        state.tree_rx = None;
    }

    // Process new document creation.
    if let Some(rx) = &state.new_doc_rx
        && let Ok((ui_id, doc)) = rx.try_recv()
    {
        // Recursively find the file node and update its document_id.
        fn update_doc_id(node: &mut Node, target_id: usize, doc_id: usize) {
            match node {
                Node::File(f) if f.id == target_id => {
                    f.document_id = Some(doc_id);
                }
                Node::Directory(d) => {
                    for child in &mut d.children {
                        update_doc_id(child, target_id, doc_id);
                    }
                }
                _ => {}
            }
        }
        update_doc_id(&mut state.tree, ui_id, doc.id as usize);
        open_documents.push(DocumentState::new(doc.id, &doc.name));
    }

    // Render the explorer tree inside the given ui.
    ui.vertical(|ui| {
            ui.heading("Project Explorer");
            ui.separator();

            // Import Document Button
            if ui.button("📥 Import JSON Document").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .pick_file()
                && let Ok(json_text) = std::fs::read_to_string(&path)
            {
                // Construct and send the import POST request.
                let mut request =
                    ehttp::Request::post(format!("{}/import", api_url), json_text.into_bytes());
                if let Some(token) = jwt_token {
                    request
                        .headers
                        .insert("Authorization", &format!("Bearer {}", token));
                }

                // Clear and set content-type headers.
                request
                    .headers
                    .headers
                    .retain(|(k, _)| k.to_lowercase() != "content-type");
                request
                    .headers
                    .headers
                    .retain(|(k, _)| k.to_lowercase() != "content-type");
                request.headers.insert("Content-Type", "application/json");

                let ctx_clone = ctx.clone();
                state.import_status = Some("Importing...".to_string());

                // Execute the import request.
                ehttp::fetch(request, move |result| {
                    match result {
                        Ok(res) => {
                            tracing::info!("Import success: {}", res.text().unwrap_or(""))
                        }
                        Err(e) => tracing::error!("Import error: {}", e),
                    }
                    ctx_clone.request_repaint();
                });
            }

            // Display any import status message.
            if let Some(status) = &state.import_status {
                ui.label(status);
            }
            ui.separator();

            // Render the tree view.
            egui::ScrollArea::both().show(ui, |ui| {
                let mut context_menu_actions = Vec::<ContextMenuActions>::new();

                // Construct and display the tree using egui_ltreeview.
                let (_, actions) = TreeView::new(ui.make_persistent_id("explorer_tree"))
                    .allow_drag_and_drop(true)
                    .show_state(ui, &mut state.tree_view_state, |mut builder| {
                        show_node(builder, &state.tree, &mut context_menu_actions);
                    });

                let mut docs_to_open = Vec::new();

                // Handle tree view interactions (moves and activations).
                for action in actions {
                    match action {
                        Action::Move(dnd) => {
                            // Handle drag-and-drop moves.
                            for source_node in &dnd.source {
                                if let Some(source) = state.tree.remove(*source_node) {
                                    // Prepare the move API payload.
                                    let target_folder_id = if dnd.target == 0 {
                                        "null".to_string()
                                    } else {
                                        dnd.target.to_string()
                                    };
                                    let mut url = api_url.to_string();
                                    if url.ends_with('/') {
                                        url.pop();
                                    }

                                    // Fire API requests based on node type.
                                    if let Node::File(ref f) = source {
                                        if let Some(did) = f.document_id {
                                            let move_url = format!("{}/{}/move", url, did);
                                            let payload =
                                                format!(r#"{{"folder_id":{}}}"#, target_folder_id);
                                            let mut request = ehttp::Request::post(
                                                move_url,
                                                payload.into_bytes(),
                                            );
                                            if let Some(token) = jwt_token {
                                                request.headers.insert(
                                                    "Authorization",
                                                    &format!("Bearer {}", token),
                                                );
                                            }
                                            ehttp::fetch(request, |_| {});
                                        }
                                    } else if let Node::Directory(ref d) = source {
                                        let update_url = format!("{}/folders/{}", url, d.id);
                                        let payload = format!(
                                            r#"{{"name":"{}","owner_id":1,"parent_folder_id":{}}}"#,
                                            d.name, target_folder_id
                                        );
                                        let mut req =
                                            ehttp::Request::post(update_url, payload.into_bytes());
                                        if let Some(token) = jwt_token {
                                            req.headers.insert(
                                                "Authorization",
                                                &format!("Bearer {}", token),
                                            );
                                        }
                                        ehttp::fetch(req, |_| {});
                                    }

                                    // Re-insert the node at its new local position.
                                    let _ = state.tree.insert(dnd.target, dnd.position, source);
                                }
                            }
                        }
                        Action::Activate(activate) => {
                            // Handle node activation (e.g., double-clicks).
                            for &id in &activate.selected {
                                // Recursive search for the document.
                                fn find_doc(node: &Node, target: usize) -> Option<(usize, String)> {
                                    match node {
                                        Node::File(f) if f.id == target => {
                                            f.document_id.map(|did| (did, f.name.clone()))
                                        }
                                        Node::Directory(d) => {
                                            d.children.iter().find_map(|c| find_doc(c, target))
                                        }
                                        _ => None,
                                    }
                                }
                                // If found, prepare to open it.
                                if let Some((doc_id, name)) = find_doc(&state.tree, id) {
                                    docs_to_open.push(DocumentState::new(doc_id as i32, &name));
                                }
                            }
                        }
                        _ => {}
                    }
                }

                // Add activated documents to the global list.
                open_documents.extend(docs_to_open);

                // Process queued context menu actions.
                for action in context_menu_actions {
                    match action {
                        ContextMenuActions::Delete(id) => {
                            // Prompt for delete confirmation.
                            state.modal_state = Some(ModalState {
                                action: ModalAction::ConfirmDelete(id),
                                input_name: String::new(),
                            });
                        }
                        ContextMenuActions::AddLeaf(parent_id, position) => {
                            // Prompt for new file name.
                            state.modal_state = Some(ModalState {
                                action: ModalAction::AddFile(parent_id, position),
                                input_name: String::new(),
                            });
                        }
                        ContextMenuActions::AddDir(parent_id, position) => {
                            // Prompt for new directory name.
                            state.modal_state = Some(ModalState {
                                action: ModalAction::AddDir(parent_id, position),
                                input_name: String::new(),
                            });
                        }
                    }
                }
            });
        });

    // Render the modal dialog if needed.
    if let Some(modal) = &mut state.modal_state {
        let mut is_open = true;
        let mut close_requested = false;
        let mut submitted = false;

        // Determine modal title based on action.
        let title = match modal.action {
            ModalAction::AddFile(..) => "New File Name",
            ModalAction::AddDir(..) => "New Directory Name",
            ModalAction::ConfirmDelete(..) => "Confirm Deletion",
        };

        // Display the modal window.
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .open(&mut is_open)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                // Support keyboard shortcuts.
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close_requested = true;
                }
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    submitted = true;
                }

                match modal.action {
                    ModalAction::ConfirmDelete(_) => {
                        ui.label("Are you sure you want to delete this item?");
                        ui.horizontal(|ui| {
                            if ui.button("Yes").clicked() {
                                submitted = true;
                            }
                            if ui.button("No").clicked() {
                                close_requested = true;
                            }
                        });
                    }
                    _ => {
                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            let response = ui.text_edit_singleline(&mut modal.input_name);
                            response.request_focus();
                        });
                        ui.horizontal(|ui| {
                            if ui.button("OK").clicked() {
                                submitted = true;
                            }
                            if ui.button("Cancel").clicked() {
                                close_requested = true;
                            }
                        });
                    }
                }
            });

        // Handle modal submission.
        if submitted {
            match modal.action.clone() {
                ModalAction::ConfirmDelete(id) => {
                    // Remove the item from the tree.
                    state.tree.remove(id);
                    state.modal_state = None;
                }
                ModalAction::AddFile(parent_id, position) => {
                    // Ensure name is not empty.
                    if !modal.input_name.trim().is_empty() {
                        let name = modal.input_name.trim().to_string();
                        // Initially we don't have the real doc ID, use None.
                        let leaf = Node::File(File {
                            id: state.next_id,
                            name: name.clone(),
                            document_id: None,
                        });
                        let id = state.next_id;
                        state.next_id += 1;

                        // Insert the node locally.
                        let _ = state.tree.insert(parent_id, position, leaf);

                        // Select the newly created file.
                        state.tree_view_state.set_selected(vec![id]);

                        // Send an API request to persist the document creation.
                        let real_parent_id = if parent_id == 0 {
                            None
                        } else {
                            Some(parent_id as i32)
                        };
                        let payload = format!(
                            r#"{{"name":"{}","owner_id":1,"aggregation_method":"AIJ","folder_id":{}}}"#,
                            name,
                            match real_parent_id {
                                Some(pid) => pid.to_string(),
                                None => "null".to_string(),
                            }
                        );
                        let mut url = api_url.to_string();
                        if url.ends_with('/') {
                            url.pop();
                        }
                        
                        let mut request = ehttp::Request::post(url, payload.into_bytes());
                        if let Some(token) = jwt_token {
                            request
                                .headers
                                .insert("Authorization", &format!("Bearer {}", token));
                        }
                        // Set proper headers
                        request
                            .headers
                            .headers
                            .retain(|(k, _)| k.to_lowercase() != "content-type");
                        request.headers.insert("Content-Type", "application/json");

                        let (tx, rx) = std::sync::mpsc::channel();
                        state.new_doc_rx = Some(rx);
                        let ctx_clone = ctx.clone();
                        
                        ehttp::fetch(request, move |result| {
                            if let Ok(res) = result {
                                if res.status == 200 {
                                    if let Ok(doc) = serde_json::from_slice::<DocumentDto>(&res.bytes) {
                                        let _ = tx.send((id, doc));
                                    }
                                } else {
                                    tracing::error!("Create doc failed: Status: {}, Body: {:?}", res.status, res.text());
                                }
                            }
                            ctx_clone.request_repaint();
                        });

                        state.modal_state = None;
                    } else {
                        // Reject submission if empty.
                        submitted = false;
                    }
                }
                ModalAction::AddDir(parent_id, position) => {
                    // Ensure name is not empty.
                    if !modal.input_name.trim().is_empty() {
                        let name = modal.input_name.trim().to_string();
                        // Create the new directory node.
                        let dir = Node::Directory(Directory {
                            id: state.next_id,
                            name: name.clone(),
                            children: vec![],
                        });
                        let id = state.next_id;
                        state.next_id += 1;

                        // Insert the directory locally.
                        let _ = state.tree.insert(parent_id, position, dir);
                        state.tree_view_state.set_selected(vec![id]);

                        // Send an API request to persist the directory creation.
                        let real_parent_id = if parent_id == 0 {
                            None
                        } else {
                            Some(parent_id as i32)
                        };
                        let payload = format!(
                            r#"{{"name":"{}","owner_id":1,"parent_folder_id":{}}}"#,
                            name,
                            match real_parent_id {
                                Some(pid) => pid.to_string(),
                                None => "null".to_string(),
                            }
                        );
                        let mut url = api_url.to_string();
                        if url.ends_with('/') {
                            url.pop();
                        }
                        url.push_str("/folders");

                        let mut request = ehttp::Request::post(url, payload.into_bytes());
                        if let Some(token) = jwt_token {
                            request
                                .headers
                                .insert("Authorization", &format!("Bearer {}", token));
                        }
                        let ctx_clone = ctx.clone();
                        ehttp::fetch(request, move |_| {
                            ctx_clone.request_repaint();
                        });

                        state.modal_state = None;
                    } else {
                        // Reject submission if empty.
                        submitted = false;
                    }
                }
            }
        }

        // Handle closure due to user pressing Cancel or Esc.
        if (!is_open || close_requested) && !submitted {
            state.modal_state = None;
        }
    }
}
