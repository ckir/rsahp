use eframe::egui;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum DirPosition {
    First,
    Last,
    Before(usize),
    After(usize),
}pub struct CriteriaNode {
    pub id: usize,
    pub name: String,
    pub children: Vec<CriteriaNode>,
}

impl CriteriaNode {
    pub fn remove(&mut self, id: usize) -> Option<CriteriaNode> {
        if let Some(index) = self.children.iter().position(|n| n.id == id) {
            Some(self.children.remove(index))
        } else {
            for node in self.children.iter_mut() {
                if let Some(r) = node.remove(id) {
                    return Some(r);
                }
            }
            None
        }
    }

    pub fn insert(&mut self, parent_id: usize, position: DirPosition, value: CriteriaNode) -> Result<(), CriteriaNode> {
        if self.id == parent_id {
            match position {
                DirPosition::First => self.children.insert(0, value),
                DirPosition::Last => self.children.push(value),
                DirPosition::After(after_id) => {
                    if let Some(index) = self.children.iter().position(|n| n.id == after_id) {
                        self.children.insert(index + 1, value);
                    }
                }
                DirPosition::Before(before_id) => {
                    if let Some(index) = self.children.iter().position(|n| n.id == before_id) {
                        self.children.insert(index, value);
                    }
                }
            }
            Ok(())
        } else {
            let mut value = Err(value);
            for node in self.children.iter_mut() {
                if let Err(v) = value {
                    value = node.insert(parent_id, position.clone(), v);
                }
            }
            value
        }
    }

    pub fn rename(&mut self, id: usize, new_name: String) -> bool {
        if self.id == id {
            self.name = new_name;
            return true;
        }
        for child in &mut self.children {
            if child.rename(id, new_name.clone()) {
                return true;
            }
        }
        false
    }
    
    pub fn find(&self, id: usize) -> Option<&CriteriaNode> {
        if self.id == id {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find(id) {
                return Some(found);
            }
        }
        None
    }
}

#[derive(Clone)]
pub enum CriteriaModalAction {
    AddChild(usize, DirPosition),
    ConfirmDelete(usize),
    Rename(usize),
}

pub struct CriteriaModalState {
    pub action: CriteriaModalAction,
    pub input_name: String,
}

pub struct DocumentState {
    pub id: i32,
    pub title: String,
    pub version: i32,
    pub active_tab: DocumentTab,
    pub aggregation_mode: String, // "AIJ" or "AIP"
    pub input_mode: String, // "Wizard" or "Scrolling"
    pub save_status: Option<String>,
    pub saaty_values: HashMap<(usize, usize), f64>,
    pub wizard_step: usize,
    pub goal: String,
    pub criteria: CriteriaNode,
    pub open_nodes: std::collections::HashSet<usize>,
    pub next_id: usize,
    pub modal_state: Option<CriteriaModalState>,
    pub is_modified: bool,
    pub close_requested: bool,
    pub is_loaded: bool,
    pub load_rx: Option<std::sync::mpsc::Receiver<Result<ExportedDocument, String>>>,
    pub save_rx: Option<std::sync::mpsc::Receiver<bool>>,
    pub duplicated_doc_rx: Option<std::sync::mpsc::Receiver<DocumentModel>>,
}

#[derive(PartialEq)]
pub enum DocumentTab {
    Structure,
    Comparisons,
    Results,
}

impl DocumentState {
    pub fn new(id: i32, title: &str) -> Self {
        Self {
            id,
            title: title.to_string(),
            version: 1,
            active_tab: DocumentTab::Structure,
            aggregation_mode: "AIJ".to_string(),
            input_mode: "Scrolling".to_string(),
            save_status: None,
            saaty_values: HashMap::new(),
            wizard_step: 0,
            goal: String::new(),
            criteria: CriteriaNode {
                id: 0,
                name: "ROOT".to_string(),
                children: vec![],
            },
            open_nodes: std::collections::HashSet::new(),
            next_id: 1,
            modal_state: None,
            is_modified: false,
            close_requested: false,
            is_loaded: false,
            load_rx: None,
            save_rx: None,
            duplicated_doc_rx: None,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ExportedDocument {
    pub document: DocumentModel,
    pub nodes: Vec<NodeModel>,
    pub comparisons: Vec<ComparisonModel>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct DocumentModel {
    pub id: i32,
    pub name: String,
    pub owner_id: i32,
    pub version: i32,
    pub aggregation_method: String,
    pub created_at: String,
    pub folder_id: Option<i32>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct NodeModel {
    pub id: i32,
    pub document_id: i32,
    pub parent_node_id: Option<i32>,
    pub name: String,
    pub node_type: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ComparisonModel {
    pub id: i32,
    pub document_id: i32,
    pub respondent_email: String,
    pub parent_node_id: i32,
    pub node_a_id: i32,
    pub node_b_id: i32,
    pub saaty_value: f64,
}

#[derive(Serialize)]
pub struct DocumentDto {
    pub name: String,
    pub owner_id: i32,
    pub aggregation_method: String,
}

pub fn save_document(state: &mut DocumentState, api_url: &str, ctx: &egui::Context) {
    let mut nodes = Vec::new();
    
    // Add goal node manually as the root
    let goal_id = 0;
    nodes.push(NodeModel {
        id: goal_id,
        document_id: state.id,
        parent_node_id: None,
        name: if state.goal.is_empty() { "Goal".to_string() } else { state.goal.clone() },
        node_type: "Goal".to_string(),
    });

    fn traverse(node: &CriteriaNode, doc_id: i32, parent_id: i32, out: &mut Vec<NodeModel>) {
        out.push(NodeModel {
            id: node.id as i32,
            document_id: doc_id,
            parent_node_id: Some(parent_id),
            name: node.name.clone(),
            node_type: "Criteria".to_string(),
        });
        for child in &node.children {
            traverse(child, doc_id, node.id as i32, out);
        }
    }
    
    for child in &state.criteria.children {
        traverse(child, state.id, goal_id, &mut nodes);
    }

    let mut comparisons = Vec::new();
    for (&(a, b), &val) in &state.saaty_values {
        fn find_parent(node: &CriteriaNode, target: usize) -> Option<usize> {
            if node.children.iter().any(|c| c.id == target) {
                return Some(node.id);
            }
            for child in &node.children {
                if let Some(p) = find_parent(child, target) {
                    return Some(p);
                }
            }
            None
        }
        let parent_id = find_parent(&state.criteria, a).unwrap_or(goal_id as usize);

        comparisons.push(ComparisonModel {
            id: 0,
            document_id: state.id,
            respondent_email: "test@example.com".to_string(),
            parent_node_id: parent_id as i32,
            node_a_id: a as i32,
            node_b_id: b as i32,
            saaty_value: val,
        });
    }

    let export = ExportedDocument {
        document: DocumentModel {
            id: state.id,
            name: state.title.clone(),
            owner_id: 1,
            version: state.version,
            aggregation_method: state.aggregation_mode.clone(),
            created_at: "2026-06-21T00:00:00Z".to_string(),
            folder_id: None,
        },
        nodes,
        comparisons,
    };

    if let Ok(body) = serde_json::to_vec(&export) {
        let mut request = ehttp::Request::post(&format!("{}/{}/full", api_url, state.id), body);
        request.headers.headers.clear();
        request.headers.insert("Content-Type", "application/json");
        let ctx_clone = ctx.clone();
        state.save_status = Some("Saving...".to_string());
        state.is_modified = false;
        
        let (tx, rx) = std::sync::mpsc::channel();
        state.save_rx = Some(rx);

        ehttp::fetch(request, move |result| {
            match result {
                Ok(res) => {
                    tracing::info!("Save Result: Status: {}, Text: {}", res.status, res.text().unwrap_or(""));
                    let text = res.text().unwrap_or("");
                    if res.status >= 200 && res.status < 300 && !text.contains("\"ok\":false") {
                        let _ = tx.send(true);
                    } else {
                        let _ = tx.send(false);
                    }
                }
                Err(e) => {
                    tracing::error!("Save Error: {}", e);
                    let _ = tx.send(false);
                }
            }
            ctx_clone.request_repaint();
        });
    }
}

pub fn render(ui: &mut egui::Ui, state: &mut DocumentState, api_url: &str) {
    if !state.is_loaded && state.load_rx.is_none() {
        let (tx, rx) = std::sync::mpsc::channel();
        state.load_rx = Some(rx);
        let url = format!("{}/{}/export", api_url, state.id);
        let request = ehttp::Request::get(&url);
        let ctx = ui.ctx().clone();
        
        ehttp::fetch(request, move |result| {
            if let Ok(res) = result {
                if res.status >= 200 && res.status < 300 {
                    match serde_json::from_slice::<ExportedDocument>(&res.bytes) {
                        Ok(data) => {
                            let _ = tx.send(Ok(data));
                        }
                        Err(e) => {
                            let msg = format!("Invalid JSON: {}", e);
                            tracing::error!("{}", msg);
                            let _ = tx.send(Err(msg));
                        }
                    }
                } else {
                    let _ = tx.send(Err(res.text().unwrap_or("").to_string()));
                }
            } else {
                let _ = tx.send(Err("Network error".to_string()));
            }
            ctx.request_repaint();
        });
    }

    if let Some(rx) = &state.load_rx {
        if let Ok(res) = rx.try_recv() {
            state.load_rx = None;
            state.is_loaded = true;
            match res {
                Ok(data) => {
                    state.title = data.document.name;
                    state.version = data.document.version;
                    state.aggregation_mode = data.document.aggregation_method;
                    
                    if let Some(goal) = data.nodes.iter().find(|n| n.node_type == "Goal" || n.parent_node_id.is_none()) {
                        state.goal = goal.name.clone();
                        
                        fn build_tree(nodes: &[NodeModel], parent_id: i32) -> Vec<CriteriaNode> {
                            let mut children = Vec::new();
                            for n in nodes.iter().filter(|n| n.parent_node_id == Some(parent_id)) {
                                children.push(CriteriaNode {
                                    id: n.id as usize,
                                    name: n.name.clone(),
                                    children: build_tree(nodes, n.id),
                                });
                            }
                            children
                        }
                        
                        state.criteria.children = build_tree(&data.nodes, goal.id);
                        
                        let max_id = data.nodes.iter().map(|n| n.id).max().unwrap_or(0);
                        state.next_id = (max_id as usize) + 1;
                    }
                    
                    state.saaty_values.clear();
                    for comp in data.comparisons {
                        state.saaty_values.insert((comp.node_a_id as usize, comp.node_b_id as usize), comp.saaty_value);
                    }
                }
                Err(e) => {
                    // Ignore 404s for new documents
                    if !e.contains("404") && !e.contains("Not Found") && !e.contains("Document not found") {
                        state.save_status = Some(format!("❌ Load Failed: {}", e));
                    }
                }
            }
        }
    }

    if let Some(rx) = &state.save_rx {
        if let Ok(success) = rx.try_recv() {
            if success {
                state.save_status = Some("✅ Saved!".to_string());
            } else {
                state.save_status = Some("❌ Save Failed".to_string());
            }
            state.save_rx = None;
        }
    }



    // Toolbar
    ui.horizontal(|ui| {
        if ui.button("💾 Save").clicked() { 
            save_document(state, api_url, ui.ctx());
        }
        if ui.button("📄 Save as New Version").clicked() {
            // Save first to ensure the original is up to date, though technically optional.
            // But we can just trigger the duplicate endpoint directly.
            let url = format!("{}/{}/duplicate", api_url, state.id);
            let request = ehttp::Request::post(&url, vec![]);
            let ctx = ui.ctx().clone();
            
            let (tx, rx) = std::sync::mpsc::channel();
            state.duplicated_doc_rx = Some(rx);
            state.save_status = Some("Duplicating...".to_string());
            
            ehttp::fetch(request, move |result| {
                if let Ok(res) = result {
                    if let Ok(new_doc) = serde_json::from_slice::<DocumentModel>(&res.bytes) {
                        let _ = tx.send(new_doc);
                    }
                }
                ctx.request_repaint();
            });
        }
        if ui.button("📤 Export JSON").clicked() { 
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("JSON", &["json"])
                .set_file_name(format!("{}.json", state.title))
                .save_file()
            {
                let url = format!("{}/{}/export", api_url, state.id);
                let request = ehttp::Request::get(&url);
                let ctx = ui.ctx().clone();
                
                ehttp::fetch(request, move |result| {
                    if let Ok(res) = result {
                        if let Some(json_text) = res.text() {
                            if let Err(e) = std::fs::write(&path, json_text) {
                                tracing::error!("Failed to save export: {}", e);
                            } else {
                                tracing::info!("Export saved to {:?}", path);
                            }
                        }
                    }
                    ctx.request_repaint();
                });
            }
        }
        
        ui.separator();
        
        egui::ComboBox::from_id_salt(format!("agg_mode_{}", state.id))
            .selected_text(format!("Agg: {}", state.aggregation_mode))
            .show_ui(ui, |ui| {
                if ui.selectable_value(&mut state.aggregation_mode, "AIJ".to_string(), "AIJ (Agg. Judgments)").changed() { state.is_modified = true; }
                if ui.selectable_value(&mut state.aggregation_mode, "AIP".to_string(), "AIP (Agg. Priorities)").changed() { state.is_modified = true; }
            });
            
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if state.is_modified {
                ui.label("⚫ Modified");
            } else if let Some(status) = &state.save_status {
                ui.label(status);
            }
            ui.label(format!("v{}.0", state.version));
        });
    });
    
    ui.separator();

    // Tabs
    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.active_tab, DocumentTab::Structure, "Structure");
        ui.selectable_value(&mut state.active_tab, DocumentTab::Comparisons, "Comparisons");
        ui.selectable_value(&mut state.active_tab, DocumentTab::Results, "Results");
    });
    
    ui.separator();

    // Tab Content
    match state.active_tab {
        DocumentTab::Structure => {
            ui.heading("Criteria Hierarchy");
            ui.horizontal(|ui| {
                ui.label("Goal:");
                if ui.text_edit_singleline(&mut state.goal).changed() {
                    state.is_modified = true;
                }
            });
            ui.separator();
            
            let mut context_menu_actions = Vec::<CriteriaModalAction>::new();
            
            if ui.button("➕ Add Top-level Criteria").clicked() {
                state.modal_state = Some(CriteriaModalState {
                    action: CriteriaModalAction::AddChild(0, DirPosition::Last),
                    input_name: String::new(),
                });
            }

            fn show_node(node: &CriteriaNode, actions: &mut Vec<CriteriaModalAction>, open_nodes: &mut std::collections::HashSet<usize>, ui: &mut egui::Ui) {
                let id = ui.make_persistent_id(format!("node_{}", node.id));
                let is_open = open_nodes.contains(&node.id);
                
                let mut header = egui::CollapsingHeader::new(&node.name)
                    .id_salt(id)
                    .open(Some(is_open));
                
                if node.children.is_empty() {
                    header = header.icon(|_ui, _open, _rect| {});
                }
                
                let response = header.show(ui, |ui| {
                    for child in &node.children {
                        show_node(child, actions, open_nodes, ui);
                    }
                });

                if response.header_response.clicked() {
                    if is_open {
                        open_nodes.remove(&node.id);
                    } else {
                        open_nodes.insert(node.id);
                    }
                }
                
                response.header_response.context_menu(|ui| {
                    ui.set_width(120.0);
                    ui.label(&node.name);
                    ui.separator();
                    if node.id != 0 {
                        if ui.button("🗑 Delete").clicked() {
                            actions.push(CriteriaModalAction::ConfirmDelete(node.id));
                            ui.close();
                        }
                        ui.separator();
                    }
                    if ui.button("✏ Rename").clicked() {
                        actions.push(CriteriaModalAction::Rename(node.id));
                        ui.close();
                    }
                    if ui.button("➕ Add Sub-criteria").clicked() {
                        actions.push(CriteriaModalAction::AddChild(node.id, DirPosition::Last));
                        ui.close();
                    }
                });
            }

            egui::ScrollArea::both().show(ui, |ui| {
                ui.push_id(format!("criteria_tree_scope_{}", state.id), |ui| {
                    // Show Root Node children (since Goal is a single top level virtual node, we iterate children)
                    for child in &state.criteria.children {
                        show_node(child, &mut context_menu_actions, &mut state.open_nodes, ui);
                    }
                });
            });

            for action in context_menu_actions {
                match action {
                    CriteriaModalAction::ConfirmDelete(id) => {
                        state.modal_state = Some(CriteriaModalState {
                            action: CriteriaModalAction::ConfirmDelete(id),
                            input_name: String::new(),
                        });
                    }
                    CriteriaModalAction::AddChild(parent_id, position) => {
                        state.modal_state = Some(CriteriaModalState {
                            action: CriteriaModalAction::AddChild(parent_id, position),
                            input_name: String::new(),
                        });
                    }
                    CriteriaModalAction::Rename(id) => {
                        let current_name = state.criteria.find(id).map(|n| n.name.clone()).unwrap_or_default();
                        state.modal_state = Some(CriteriaModalState {
                            action: CriteriaModalAction::Rename(id),
                            input_name: current_name,
                        });
                    }
                }
            }

            if let Some(modal) = &mut state.modal_state {
                let mut is_open = true;
                let mut close_requested = false;
                let mut submitted = false;
                
                let title = match modal.action {
                    CriteriaModalAction::AddChild(..) => "New Criteria Name",
                    CriteriaModalAction::ConfirmDelete(..) => "Confirm Deletion",
                    CriteriaModalAction::Rename(..) => "Rename Criteria",
                };

                egui::Window::new(title)
                    .id(egui::Id::new("criteria_modal").with(state.id))
                    .collapsible(false)
                    .resizable(false)
                    .open(&mut is_open)
                    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                    .show(ui.ctx(), |ui| {
                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            close_requested = true;
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            submitted = true;
                        }

                        match modal.action {
                            CriteriaModalAction::ConfirmDelete(_) => {
                                ui.label("Are you sure you want to delete this criteria?");
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
                        CriteriaModalAction::ConfirmDelete(id) => {
                            state.criteria.remove(id);
                            state.is_modified = true;
                            state.modal_state = None;
                        }
                        CriteriaModalAction::AddChild(parent_id, position) => {
                            if !modal.input_name.trim().is_empty() {
                                let name = modal.input_name.trim().to_string();
                                let child = CriteriaNode {
                                    id: state.next_id,
                                    name,
                                    children: vec![],
                                };
                                let id = state.next_id;
                                state.next_id += 1;
                                let _ = state.criteria.insert(parent_id, position, child);
                                state.open_nodes.insert(parent_id);
                                state.open_nodes.insert(id);
                                state.is_modified = true;
                                state.modal_state = None;
                            } else {
                                submitted = false; // keep open
                            }
                        }
                        CriteriaModalAction::Rename(id) => {
                            if !modal.input_name.trim().is_empty() {
                                let name = modal.input_name.trim().to_string();
                                state.criteria.rename(id, name);
                                state.is_modified = true;
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
        DocumentTab::Comparisons => {
            ui.horizontal(|ui| {
                ui.label("View:");
                ui.radio_value(&mut state.input_mode, "Wizard".to_string(), "Step-by-step (Wizard)");
                ui.radio_value(&mut state.input_mode, "Scrolling".to_string(), "Single Scrolling Page");
            });
            ui.separator();
            ui.heading("Pairwise Comparisons");
            
            fn generate_comparisons(node: &CriteriaNode, comps: &mut Vec<(String, Vec<(String, usize, usize)>)>, goal_text: &str) {
                let n = node.children.len();
                if n >= 2 {
                    let parent_name = if node.id == 0 {
                        if goal_text.is_empty() { "Goal" } else { goal_text }
                    } else { 
                        &node.name 
                    };
                    let group_title = format!("With respect to: {}", parent_name);
                    let mut group_comps = Vec::new();
                    for i in 0..n {
                        for j in (i+1)..n {
                            let title = format!("{} vs {}", node.children[i].name, node.children[j].name);
                            group_comps.push((title, node.children[i].id, node.children[j].id));
                        }
                    }
                    comps.push((group_title, group_comps));
                }
                for child in &node.children {
                    generate_comparisons(child, comps, goal_text);
                }
            }

            let mut grouped_comparisons = Vec::new();
            generate_comparisons(&state.criteria, &mut grouped_comparisons, &state.goal);

            let mut flat_comparisons = Vec::new();
            for (g_title, comps) in &grouped_comparisons {
                for (title, id1, id2) in comps {
                    flat_comparisons.push((g_title.clone(), title.clone(), *id1, *id2));
                }
            }

            if flat_comparisons.is_empty() {
                ui.label("Add at least two criteria under the same parent to begin comparisons.");
            } else {
                let render_selector = |ui: &mut egui::Ui, title: &str, val: &mut f64| -> bool {
                    let mut changed = false;
                    
                    if (*val - 0.0).abs() < 0.001 {
                        *val = 1.0;
                    }

                    let options = [
                        (9.0, "Extreme importance"),
                        (7.0, "Very strong importance"),
                        (5.0, "Strong importance"),
                        (3.0, "Moderate importance"),
                        (1.0, "Equal importance"),
                        (1.0 / 3.0, "Moderate less importance"),
                        (1.0 / 5.0, "Strong less importance"),
                        (1.0 / 7.0, "Very strong less importance"),
                        (1.0 / 9.0, "Extreme less importance"),
                    ];
                    
                    let current_text = options.iter()
                        .min_by(|a, b| (a.0 - *val).abs().partial_cmp(&(b.0 - *val).abs()).unwrap_or(std::cmp::Ordering::Equal))
                        .map(|(_, text)| text.to_string())
                        .unwrap_or_else(|| "Equal importance".to_string());
                    
                    egui::ComboBox::from_id_source(title)
                        .width(250.0)
                        .selected_text(current_text)
                        .show_ui(ui, |ui| {
                            for (v, text) in options.iter() {
                                if ui.selectable_value(val, *v, text.to_string()).changed() {
                                    changed = true;
                                }
                            }
                        });
                    changed
                };

                if state.input_mode == "Wizard" {
                    if state.wizard_step >= flat_comparisons.len() {
                        state.wizard_step = flat_comparisons.len() - 1;
                    }
                    let idx = state.wizard_step;
                    let (g_title, title, id1, id2) = &flat_comparisons[idx];
                    let val = state.saaty_values.entry((*id1, *id2)).or_insert(1.0);
                    
                    ui.group(|ui| {
                        ui.label(egui::RichText::new(g_title).strong());
                        ui.label(format!("Compare: {}", title));
                        if render_selector(ui, title, val) {
                            state.is_modified = true;
                        }
                        
                        ui.horizontal(|ui| {
                            if ui.add_enabled(idx > 0, egui::Button::new("Previous")).clicked() {
                                state.wizard_step -= 1;
                            }
                            if ui.add_enabled(idx < flat_comparisons.len() - 1, egui::Button::new("Next")).clicked() {
                                state.wizard_step += 1;
                            }
                        });
                    });
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (i, (g_title, comps)) in grouped_comparisons.iter().enumerate() {
                            if i > 0 {
                                ui.separator();
                            }
                            ui.label(egui::RichText::new(g_title).heading().strong());
                            ui.add_space(5.0);
                            for (title, id1, id2) in comps {
                                let val = state.saaty_values.entry((*id1, *id2)).or_insert(1.0);
                                ui.group(|ui| {
                                    ui.label(title);
                                    if render_selector(ui, title, val) {
                                        state.is_modified = true;
                                    }
                                });
                            }
                            ui.add_space(10.0);
                        }
                    });
                }
            }
        }
        DocumentTab::Results => {
            ui.heading("Results & Consensus");
            ui.label("Priority Vectors and Consistency Ratio (CR):");
            
            // Mock CR logic based on sliders to show the soft warning
            let mut mock_cr = 0.05;
            if state.saaty_values.values().any(|&v| v.abs() > 5.0) {
                // Introduce inconsistency for demonstration
                mock_cr = 0.15; 
            }
            
            ui.label(format!("Consistency Ratio (CR): {:.3}", mock_cr));
            
            if mock_cr > 0.10 {
                ui.colored_label(egui::Color32::from_rgb(200, 100, 0), "⚠️ Warning: CR > 0.10. Judgments may be inconsistent. Please review your comparisons.");
            } else {
                ui.colored_label(egui::Color32::from_rgb(0, 200, 0), "✅ CR is within acceptable limits (< 0.10).");
            }
            
            ui.separator();

            fn collect_criteria(node: &CriteriaNode, list: &mut Vec<String>) {
                if node.id != 0 {
                    list.push(node.name.clone());
                }
                for child in &node.children {
                    collect_criteria(child, list);
                }
            }
            let mut all_criteria = Vec::new();
            collect_criteria(&state.criteria, &mut all_criteria);
            
            if all_criteria.is_empty() {
                ui.label("No criteria defined.");
            } else {
                let mock_weight = 1.0 / (all_criteria.len() as f64);
                for c in all_criteria {
                    ui.label(format!("{}: {:.3}", c, mock_weight));
                }
            }
        }
    }
}
