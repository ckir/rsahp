use eframe::egui;
use super::document_window::DocumentState;
use egui_ltreeview::{TreeView, TreeViewState, NodeBuilder, Action, DirPosition};

pub enum Node {
    Directory(Directory),
    File(File),
}

pub struct Directory {
    pub id: usize,
    pub name: String,
    pub children: Vec<Node>,
}

pub struct File {
    pub id: usize,
    pub name: String,
    pub document_id: Option<usize>,
}

impl Node {
    pub fn id(&self) -> usize {
        match self {
            Node::Directory(d) => d.id,
            Node::File(f) => f.id,
        }
    }
    pub fn name(&self) -> &str {
        match self {
            Node::Directory(d) => &d.name,
            Node::File(f) => &f.name,
        }
    }
    pub fn remove(&mut self, id: usize) -> Option<Node> {
        match self {
            Node::Directory(dir) => {
                if let Some(index) = dir.children.iter().position(|n| n.id() == id) {
                    Some(dir.children.remove(index))
                } else {
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
    pub fn insert(&mut self, parent_id: usize, position: DirPosition<usize>, value: Node) -> Result<(), Node> {
        match self {
            Node::Directory(dir) => {
                if dir.id == parent_id {
                    match position {
                        DirPosition::First => dir.children.insert(0, value),
                        DirPosition::Last => dir.children.push(value),
                        DirPosition::After(after_id) => {
                            if let Some(index) = dir.children.iter().position(|n| n.id() == after_id) {
                                dir.children.insert(index + 1, value);
                            }
                        }
                        DirPosition::Before(before_id) => {
                            if let Some(index) = dir.children.iter().position(|n| n.id() == before_id) {
                                dir.children.insert(index, value);
                            }
                        }
                    }
                    Ok(())
                } else {
                    let mut value = Err(value);
                    for node in dir.children.iter_mut() {
                        if let Err(v) = value {
                            value = node.insert(parent_id, position.clone(), v);
                        }
                    }
                    value
                }
            }
            _ => Err(value),
        }
    }
}

#[derive(Clone)]
pub enum ModalAction {
    AddFile(usize, DirPosition<usize>),
    AddDir(usize, DirPosition<usize>),
    ConfirmDelete(usize),
}

pub struct ModalState {
    pub action: ModalAction,
    pub input_name: String,
}

pub struct ExplorerState {
    pub import_status: Option<String>,
    pub tree: Node,
    pub tree_view_state: TreeViewState<usize>,
    pub next_id: usize,
    pub modal_state: Option<ModalState>,
    pub fetched_initial: bool,
    pub tree_rx: Option<std::sync::mpsc::Receiver<TreeDto>>,
}

#[derive(serde::Deserialize, Clone)]
pub struct DocumentModel {
    pub id: i32,
    pub name: String,
    pub folder_id: Option<i32>,
}

#[derive(serde::Deserialize, Clone)]
pub struct FolderModel {
    pub id: i32,
    pub name: String,
    pub parent_folder_id: Option<i32>,
}

#[derive(serde::Deserialize, Clone)]
pub struct TreeDto {
    pub folders: Vec<FolderModel>,
    pub documents: Vec<DocumentModel>,
}

impl Default for ExplorerState {
    fn default() -> Self {
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
        }
    }
}

enum ContextMenuActions {
    Delete(usize),
    AddLeaf(usize, DirPosition<usize>),
    AddDir(usize, DirPosition<usize>),
}

fn show_node(builder: &mut egui_ltreeview::TreeViewBuilder<usize>, node: &Node, actions: &mut Vec<ContextMenuActions>) {
    match node {
        Node::Directory(dir) => {
            builder.node(NodeBuilder::dir(dir.id).label(&dir.name).default_open(true).context_menu(|ui| {
                ui.set_width(100.0);
                ui.label("dir:");
                ui.label(&dir.name);
                ui.separator();
                if ui.button("delete").clicked() {
                    actions.push(ContextMenuActions::Delete(dir.id));
                    ui.close();
                }
                ui.separator();
                if ui.button("new file").clicked() {
                    actions.push(ContextMenuActions::AddLeaf(dir.id, DirPosition::Last));
                    ui.close();
                }
                if ui.button("new directory").clicked() {
                    actions.push(ContextMenuActions::AddDir(dir.id, DirPosition::Last));
                    ui.close();
                }
            }));
            for child in &dir.children {
                show_node(builder, child, actions);
            }
            builder.close_dir();
        }
        Node::File(file) => {
            let parent_node = builder.parent_id().copied().unwrap_or(0);
            builder.node(NodeBuilder::leaf(file.id).label(&file.name).activatable(true).context_menu(|ui| {
                ui.set_width(100.0);
                ui.label("file:");
                ui.label(&file.name);
                if ui.button("delete").clicked() {
                    actions.push(ContextMenuActions::Delete(file.id));
                    ui.close();
                }
                ui.separator();
                if ui.button("new file").clicked() {
                    actions.push(ContextMenuActions::AddLeaf(parent_node, DirPosition::After(file.id)));
                    ui.close();
                }
                if ui.button("new directory").clicked() {
                    actions.push(ContextMenuActions::AddDir(parent_node, DirPosition::After(file.id)));
                    ui.close();
                }
            }));
        }
    }
}

pub fn render(ctx: &egui::Context, state: &mut ExplorerState, open_documents: &mut Vec<DocumentState>, api_url: &str) {
    if !state.fetched_initial && state.tree_rx.is_none() {
        let (tx, rx) = std::sync::mpsc::channel();
        state.tree_rx = Some(rx);
        state.fetched_initial = true;
        
        let mut tree_url = api_url.to_string();
        if tree_url.ends_with('/') {
            tree_url.pop();
        }
        tree_url.push_str("/tree");
        
        let request = ehttp::Request::get(tree_url);
        let ctx_clone = ctx.clone();
        ehttp::fetch(request, move |result| {
            match result {
                Ok(res) => {
                    if res.status == 200 {
                        match serde_json::from_slice::<TreeDto>(&res.bytes) {
                            Ok(tree_dto) => { let _ = tx.send(tree_dto); }
                            Err(e) => tracing::error!("Failed to parse JSON tree: {}. Body: {}", e, String::from_utf8_lossy(&res.bytes)),
                        }
                    } else {
                        tracing::error!("Fetch tree failed: Status: {}, Body: {:?}", res.status, res.text());
                    }
                }
                Err(e) => tracing::error!("Fetch tree request error: {}", e),
            }
            ctx_clone.request_repaint();
        });
    }

    if let Some(rx) = &state.tree_rx {
        if let Ok(tree_dto) = rx.try_recv() {
            // Rebuild tree
            fn build_node(parent_folder_id: Option<i32>, dto: &TreeDto, next_id: &mut usize) -> Vec<Node> {
                let mut children = Vec::new();
                for folder in &dto.folders {
                    if folder.parent_folder_id == parent_folder_id {
                        let dir_id = folder.id as usize;
                        if dir_id >= *next_id { *next_id = dir_id + 1; }
                        children.push(Node::Directory(Directory {
                            id: dir_id,
                            name: folder.name.clone(),
                            children: build_node(Some(folder.id), dto, next_id),
                        }));
                    }
                }
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
            
            let children = build_node(None, &tree_dto, &mut state.next_id);
            state.tree = Node::Directory(Directory {
                id: 0, // Root ID is 0
                name: "My AHP Documents".to_string(),
                children,
            });
            state.tree_rx = None;
        }
    }

    #[allow(deprecated)]
    egui::SidePanel::left("explorer_panel")
        .resizable(true)
        .default_width(250.0)
        .show(ctx, |ui| {
            ui.heading("Project Explorer");
            ui.separator();
            
            if ui.button("📥 Import JSON Document").clicked() {
                if let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
                    if let Ok(json_text) = std::fs::read_to_string(&path) {
                        let mut request = ehttp::Request::post(&format!("{}/import", api_url), json_text.into_bytes());
                        request.headers.insert("Content-Type", "application/json");
                        let ctx_clone = ctx.clone();
                        state.import_status = Some("Importing...".to_string());
                        ehttp::fetch(request, move |result| {
                            match result {
                                Ok(res) => tracing::info!("Import success: {}", res.text().unwrap_or("")),
                                Err(e) => tracing::error!("Import error: {}", e),
                            }
                            ctx_clone.request_repaint();
                        });
                    }
                }
            }
            if let Some(status) = &state.import_status {
                ui.label(status);
            }
            ui.separator();
            
            egui::ScrollArea::both().show(ui, |ui| {
                let mut context_menu_actions = Vec::<ContextMenuActions>::new();
                let (_, actions) = TreeView::new(ui.make_persistent_id("explorer_tree"))
                    .allow_drag_and_drop(true)
                    .show_state(ui, &mut state.tree_view_state, |mut builder| {
                        show_node(&mut builder, &state.tree, &mut context_menu_actions);
                    });
                    
                let mut docs_to_open = Vec::new();
                for action in actions {
                    match action {
                        Action::Move(dnd) => {
                            for source_node in &dnd.source {
                                if let Some(source) = state.tree.remove(*source_node) {
                                    // Send move API
                                    let target_folder_id = if dnd.target == 0 { "null".to_string() } else { dnd.target.to_string() };
                                    let mut url = api_url.to_string();
                                    if url.ends_with('/') { url.pop(); }
                                    
                                    if let Node::File(ref f) = source {
                                        if let Some(did) = f.document_id {
                                            let move_url = format!("{}/{}/move", url, did);
                                            let payload = format!(r#"{{"folder_id":{}}}"#, target_folder_id);
                                            let mut req = ehttp::Request::post(move_url, payload.into_bytes());
                                            ehttp::fetch(req, |_| {});
                                        }
                                    } else if let Node::Directory(ref d) = source {
                                        let update_url = format!("{}/folders/{}", url, d.id);
                                        let payload = format!(r#"{{"name":"{}","owner_id":1,"parent_folder_id":{}}}"#, d.name, target_folder_id);
                                        let mut req = ehttp::Request::post(update_url, payload.into_bytes());
                                        ehttp::fetch(req, |_| {});
                                    }
                                    
                                    let _ = state.tree.insert(dnd.target, dnd.position.clone(), source);
                                }
                            }
                        }
                        Action::Activate(activate) => {
                            for &id in &activate.selected {
                                fn find_doc(node: &Node, target: usize) -> Option<(usize, String)> {
                                    match node {
                                        Node::File(f) if f.id == target => f.document_id.map(|did| (did, f.name.clone())),
                                        Node::Directory(d) => d.children.iter().find_map(|c| find_doc(c, target)),
                                        _ => None
                                    }
                                }
                                if let Some((doc_id, name)) = find_doc(&state.tree, id) {
                                    docs_to_open.push(DocumentState::new(doc_id as i32, &name));
                                }
                            }
                        }
                        _ => {}
                    }
                }
                open_documents.extend(docs_to_open);
                
                for action in context_menu_actions {
                    match action {
                        ContextMenuActions::Delete(id) => {
                            state.modal_state = Some(ModalState {
                                action: ModalAction::ConfirmDelete(id),
                                input_name: String::new(),
                            });
                        }
                        ContextMenuActions::AddLeaf(parent_id, position) => {
                            state.modal_state = Some(ModalState {
                                action: ModalAction::AddFile(parent_id, position),
                                input_name: String::new(),
                            });
                        }
                        ContextMenuActions::AddDir(parent_id, position) => {
                            state.modal_state = Some(ModalState {
                                action: ModalAction::AddDir(parent_id, position),
                                input_name: String::new(),
                            });
                        }
                    }
                }
            });
        });

    if let Some(modal) = &mut state.modal_state {
        let mut is_open = true;
        let mut close_requested = false;
        let mut submitted = false;
        
        let title = match modal.action {
            ModalAction::AddFile(..) => "New File Name",
            ModalAction::AddDir(..) => "New Directory Name",
            ModalAction::ConfirmDelete(..) => "Confirm Deletion",
        };

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .open(&mut is_open)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
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

        if submitted {
            match modal.action.clone() {
                ModalAction::ConfirmDelete(id) => {
                    state.tree.remove(id);
                    state.modal_state = None;
                }
                ModalAction::AddFile(parent_id, position) => {
                    if !modal.input_name.trim().is_empty() {
                        let name = modal.input_name.trim().to_string();
                        let doc_id = state.next_id;
                        let leaf = Node::File(File {
                            id: state.next_id,
                            name: name.clone(),
                            document_id: Some(doc_id),
                        });
                        let id = state.next_id;
                        state.next_id += 1;
                        let _ = state.tree.insert(parent_id, position, leaf);
                        state.tree_view_state.set_selected(vec![id]);
                        open_documents.push(DocumentState::new(doc_id as i32, &name));
                        state.modal_state = None;
                    } else {
                        submitted = false; // keep open
                    }
                }
                ModalAction::AddDir(parent_id, position) => {
                    if !modal.input_name.trim().is_empty() {
                        let name = modal.input_name.trim().to_string();
                        let dir = Node::Directory(Directory {
                            id: state.next_id,
                            name: name.clone(),
                            children: vec![],
                        });
                        let id = state.next_id;
                        state.next_id += 1;
                        let _ = state.tree.insert(parent_id, position, dir);
                        state.tree_view_state.set_selected(vec![id]);
                        
                        // Fire API request
                        let real_parent_id = if parent_id == 0 { None } else { Some(parent_id as i32) };
                        let payload = format!(r#"{{"name":"{}","owner_id":1,"parent_folder_id":{}}}"#, name, match real_parent_id { Some(pid) => pid.to_string(), None => "null".to_string() });
                        let mut url = api_url.to_string();
                        if url.ends_with('/') { url.pop(); }
                        url.push_str("/folders");
                        
                        let request = ehttp::Request::post(url, payload.into_bytes());
                        let ctx_clone = ctx.clone();
                        ehttp::fetch(request, move |_| {
                            ctx_clone.request_repaint();
                        });
                        
                        state.modal_state = None;
                    } else {
                        submitted = false; // keep open
                    }
                }
            }
        }
        
        if (!is_open || close_requested) && !submitted {
            state.modal_state = None;
        }
    }
}
