use eframe::egui;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum DirPosition {
    First,
    Last,
    Before(usize),
    After(usize),
}
pub struct CriteriaNode {
    pub id: usize,
    pub name: String,
    pub cost: Option<f64>,
    pub node_type: String,
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

    pub fn insert(
        &mut self,
        parent_id: usize,
        position: DirPosition,
        value: CriteriaNode,
    ) -> Result<(), CriteriaNode> {
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

    pub fn set_cost(&mut self, id: usize, new_cost: Option<f64>) -> bool {
        if self.id == id {
            self.cost = new_cost;
            return true;
        }
        for child in &mut self.children {
            if child.set_cost(id, new_cost) {
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
    ConfirmDelete(usize, String),
    AddChild(usize, DirPosition, String), // parent_id, position, node_type to add
    Rename(usize, String),
    EditCost(usize, String),
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
    pub input_mode: String,       // "Wizard" or "Scrolling"
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
    pub sort_column: SortColumn,
    pub sort_descending: bool,
    pub assignments: Option<DocumentAssignments>,
    pub assignments_rx: Option<std::sync::mpsc::Receiver<Result<DocumentAssignments, String>>>,
    pub assignments_save_rx: Option<std::sync::mpsc::Receiver<bool>>,
    pub new_user_assignment_id: String,
    pub new_group_assignment_id: String,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct DocumentAssignments {
    pub user_ids: Vec<i32>,
    pub group_ids: Vec<i32>,
}

#[derive(PartialEq)]
pub enum SortColumn {
    CandidateName,
    Alignment,
    Cost,
    ValueScore,
}

#[derive(PartialEq)]
pub enum DocumentTab {
    Structure,
    Comparisons,
    Results,
    Assignments,
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
                node_type: "Goal".to_string(),
                cost: None,
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
            sort_column: SortColumn::ValueScore,
            sort_descending: true,
            assignments: None,
            assignments_rx: None,
            assignments_save_rx: None,
            new_user_assignment_id: String::new(),
            new_group_assignment_id: String::new(),
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
    pub cost: Option<f64>,
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

pub fn save_document(
    state: &mut DocumentState,
    api_url: &str,
    ctx: &egui::Context,
    jwt_token: Option<&str>,
) {
    let mut nodes = Vec::new();

    // Add goal node manually as the root
    let goal_id = 0;
    nodes.push(NodeModel {
        id: goal_id,
        document_id: state.id,
        parent_node_id: None,
        name: if state.goal.is_empty() {
            "Goal".to_string()
        } else {
            state.goal.clone()
        },
        node_type: "Goal".to_string(),
        cost: None,
    });

    fn traverse(node: &CriteriaNode, doc_id: i32, parent_id: i32, out: &mut Vec<NodeModel>) {
        out.push(NodeModel {
            id: node.id as i32,
            document_id: doc_id,
            parent_node_id: Some(parent_id),
            name: node.name.clone(),
            node_type: node.node_type.clone(),
            cost: node.cost,
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
        let mut request = ehttp::Request::post(format!("{}/{}/full", api_url, state.id), body);
        request
            .headers
            .headers
            .retain(|(k, _)| k.to_lowercase() != "content-type");
        request
            .headers
            .headers
            .retain(|(k, _)| k.to_lowercase() != "content-type");
        request.headers.insert("Content-Type", "application/json");
        if let Some(token) = jwt_token {
            request
                .headers
                .insert("Authorization", &format!("Bearer {}", token));
        }
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
        state.save_status = Some("Saving...".to_string());
        state.is_modified = false;

        let (tx, rx) = std::sync::mpsc::channel();
        state.save_rx = Some(rx);

        ehttp::fetch(request, move |result| {
            match result {
                Ok(res) => {
                    tracing::info!(
                        "Save Result: Status: {}, Text: {}",
                        res.status,
                        res.text().unwrap_or("")
                    );
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

pub fn render(
    ui: &mut egui::Ui,
    state: &mut DocumentState,
    api_url: &str,
    jwt_token: Option<&str>,
) {
    if !state.is_loaded && state.load_rx.is_none() {
        let (tx, rx) = std::sync::mpsc::channel();
        state.load_rx = Some(rx);
        let url = format!("{}/{}/export", api_url, state.id);
        let mut request = ehttp::Request::get(url);
        if let Some(token) = jwt_token {
            request
                .headers
                .insert("Authorization", &format!("Bearer {}", token));
        }
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

    if let Some(rx) = &state.load_rx
        && let Ok(res) = rx.try_recv()
    {
        state.load_rx = None;
        state.is_loaded = true;
        match res {
            Ok(data) => {
                state.title = data.document.name;
                state.version = data.document.version;
                state.aggregation_mode = data.document.aggregation_method;

                if let Some(goal) = data
                    .nodes
                    .iter()
                    .find(|n| n.node_type == "Goal" || n.parent_node_id.is_none())
                {
                    state.goal = goal.name.clone();

                    fn build_tree(nodes: &[NodeModel], parent_id: i32) -> Vec<CriteriaNode> {
                        let mut children = Vec::new();
                        for n in nodes.iter().filter(|n| n.parent_node_id == Some(parent_id)) {
                            children.push(CriteriaNode {
                                id: n.id as usize,
                                name: n.name.clone(),
                                node_type: n.node_type.clone(),
                                cost: n.cost,
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
                    state.saaty_values.insert(
                        (comp.node_a_id as usize, comp.node_b_id as usize),
                        comp.saaty_value,
                    );
                }
            }
            Err(e) => {
                // Ignore 404s for new documents
                if !e.contains("404")
                    && !e.contains("Not Found")
                    && !e.contains("Document not found")
                {
                    state.save_status = Some(format!("❌ Load Failed: {}", e));
                }
            }
        }
    }

    if let Some(rx) = &state.save_rx
        && let Ok(success) = rx.try_recv()
    {
        if success {
            state.save_status = Some("✅ Saved!".to_string());
        } else {
            state.save_status = Some("❌ Save Failed".to_string());
        }
        state.save_rx = None;
    }

    // Toolbar
    ui.horizontal(|ui| {
        if ui.button("💾 Save").clicked() {
            save_document(state, api_url, ui.ctx(), jwt_token);
        }
        if ui.button("📄 Save as New Version").clicked() {
            // Save first to ensure the original is up to date, though technically optional.
            // But we can just trigger the duplicate endpoint directly.
            let url = format!("{}/{}/duplicate", api_url, state.id);
            let mut request = ehttp::Request::post(url, vec![]);
            if let Some(token) = jwt_token {
                request
                    .headers
                    .insert("Authorization", &format!("Bearer {}", token));
            }
            let ctx = ui.ctx().clone();

            let (tx, rx) = std::sync::mpsc::channel();
            state.duplicated_doc_rx = Some(rx);
            state.save_status = Some("Duplicating...".to_string());

            ehttp::fetch(request, move |result| {
                if let Ok(res) = result
                    && let Ok(new_doc) = serde_json::from_slice::<DocumentModel>(&res.bytes)
                {
                    let _ = tx.send(new_doc);
                }
                ctx.request_repaint();
            });
        }
        if ui.button("📤 Export JSON").clicked()
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("JSON", &["json"])
                .set_file_name(format!("{}.json", state.title))
                .save_file()
        {
            let url = format!("{}/{}/export", api_url, state.id);
            let mut request = ehttp::Request::get(url);
            if let Some(token) = jwt_token {
                request
                    .headers
                    .insert("Authorization", &format!("Bearer {}", token));
            }
            let ctx = ui.ctx().clone();

            ehttp::fetch(request, move |result| {
                if let Ok(res) = result
                    && let Some(json_text) = res.text()
                {
                    if let Err(e) = std::fs::write(&path, json_text) {
                        tracing::error!("Failed to save export: {}", e);
                    } else {
                        tracing::info!("Export saved to {:?}", path);
                    }
                }
                ctx.request_repaint();
            });
        }

        ui.separator();

        egui::ComboBox::from_id_salt(format!("agg_mode_{}", state.id))
            .selected_text(format!("Agg: {}", state.aggregation_mode))
            .show_ui(ui, |ui| {
                if ui
                    .selectable_value(
                        &mut state.aggregation_mode,
                        "AIJ".to_string(),
                        "AIJ (Agg. Judgments)",
                    )
                    .changed()
                {
                    state.is_modified = true;
                }
                if ui
                    .selectable_value(
                        &mut state.aggregation_mode,
                        "AIP".to_string(),
                        "AIP (Agg. Priorities)",
                    )
                    .changed()
                {
                    state.is_modified = true;
                }
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
        ui.selectable_value(
            &mut state.active_tab,
            DocumentTab::Comparisons,
            "Comparisons",
        );
        ui.selectable_value(&mut state.active_tab, DocumentTab::Results, "Results");
        ui.selectable_value(
            &mut state.active_tab,
            DocumentTab::Assignments,
            "Assignments",
        );
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

            ui.horizontal(|ui| {
                if ui.button("➕ Add Top-level Criteria").clicked() {
                    state.modal_state = Some(CriteriaModalState {
                        action: CriteriaModalAction::AddChild(
                            0,
                            DirPosition::Last,
                            "Criteria".to_string(),
                        ),
                        input_name: String::new(),
                    });
                }
                if ui.button("➕ Add Candidate").clicked() {
                    state.modal_state = Some(CriteriaModalState {
                        action: CriteriaModalAction::AddChild(
                            0,
                            DirPosition::Last,
                            "Alternative".to_string(),
                        ),
                        input_name: String::new(),
                    });
                }
            });

            fn show_node(
                node: &CriteriaNode,
                actions: &mut Vec<CriteriaModalAction>,
                open_nodes: &mut std::collections::HashSet<usize>,
                ui: &mut egui::Ui,
            ) {
                let id = ui.make_persistent_id(format!("node_{}", node.id));
                let is_open = open_nodes.contains(&node.id);

                let mut display_name = node.name.clone();

                let mut header = egui::CollapsingHeader::new(&display_name)
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

                if response.header_response.double_clicked() {
                    actions.push(CriteriaModalAction::Rename(node.id, node.node_type.clone()));
                } else if response.header_response.clicked() {
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
                            actions.push(CriteriaModalAction::ConfirmDelete(
                                node.id,
                                node.node_type.clone(),
                            ));
                            ui.close();
                        }
                        ui.separator();
                    }
                    if ui.button("✏ Rename").clicked() {
                        actions.push(CriteriaModalAction::Rename(node.id, node.node_type.clone()));
                        ui.close();
                    }
                    if ui.button("➕ Add Sub-criteria").clicked() {
                        actions.push(CriteriaModalAction::AddChild(
                            node.id,
                            DirPosition::Last,
                            "Criteria".to_string(),
                        ));
                        ui.close();
                    }
                });
            }

            egui::ScrollArea::both().show(ui, |ui| {
                ui.push_id(format!("criteria_tree_scope_{}", state.id), |ui| {
                    ui.heading("▾ CRITERIA");
                    for child in state
                        .criteria
                        .children
                        .iter()
                        .filter(|c| c.node_type == "Criteria")
                    {
                        show_node(child, &mut context_menu_actions, &mut state.open_nodes, ui);
                    }

                    ui.add_space(20.0);
                    ui.heading("▾ CANDIDATES");
                    for child in state
                        .criteria
                        .children
                        .iter()
                        .filter(|c| c.node_type == "Alternative")
                    {
                        ui.horizontal(|ui| {
                            if ui.button("🗑️").clicked() {
                                context_menu_actions.push(CriteriaModalAction::ConfirmDelete(
                                    child.id,
                                    child.node_type.clone(),
                                ));
                            }
                            let label_resp = ui.add(
                                egui::Label::new(format!("• {}", child.name))
                                    .sense(egui::Sense::click()),
                            );
                            if label_resp.double_clicked() {
                                context_menu_actions.push(CriteriaModalAction::Rename(
                                    child.id,
                                    child.node_type.clone(),
                                ));
                            }
                            label_resp.context_menu(|ui| {
                                if ui.button("✏ Rename").clicked() {
                                    context_menu_actions.push(CriteriaModalAction::Rename(
                                        child.id,
                                        child.node_type.clone(),
                                    ));
                                    ui.close();
                                }
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("✏️ Edit Cost").clicked() {
                                        context_menu_actions.push(CriteriaModalAction::EditCost(
                                            child.id,
                                            child.node_type.clone(),
                                        ));
                                    }
                                    if let Some(cost) = child.cost {
                                        ui.label(format!("Cost: ${}", cost));
                                    }
                                },
                            );
                        });
                    }
                });
            });

            for action in context_menu_actions {
                match action {
                    CriteriaModalAction::ConfirmDelete(id, nt) => {
                        state.modal_state = Some(CriteriaModalState {
                            action: CriteriaModalAction::ConfirmDelete(id, nt),
                            input_name: String::new(),
                        });
                    }
                    CriteriaModalAction::AddChild(parent_id, position, ref node_type) => {
                        state.modal_state = Some(CriteriaModalState {
                            action: CriteriaModalAction::AddChild(
                                parent_id,
                                position.clone(),
                                node_type.clone(),
                            ),
                            input_name: String::new(),
                        });
                    }
                    CriteriaModalAction::Rename(id, nt) => {
                        let current_name = state
                            .criteria
                            .find(id)
                            .map(|n| n.name.clone())
                            .unwrap_or_default();
                        state.modal_state = Some(CriteriaModalState {
                            action: CriteriaModalAction::Rename(id, nt),
                            input_name: current_name,
                        });
                    }
                    CriteriaModalAction::EditCost(id, nt) => {
                        let current_cost = state
                            .criteria
                            .find(id)
                            .and_then(|n| n.cost.map(|c| c.to_string()))
                            .unwrap_or_default();
                        state.modal_state = Some(CriteriaModalState {
                            action: CriteriaModalAction::EditCost(id, nt),
                            input_name: current_cost,
                        });
                    }
                }
            }

            if let Some(modal) = &mut state.modal_state {
                let mut is_open = true;
                let mut close_requested = false;
                let mut submitted = false;

                let title = match modal.action {
                    CriteriaModalAction::AddChild(_, _, ref nt) => {
                        if nt == "Alternative" {
                            "Enter Candidate Name"
                        } else {
                            "Enter new Criteria Name"
                        }
                    }
                    CriteriaModalAction::ConfirmDelete(..) => "Confirm Deletion",
                    CriteriaModalAction::Rename(..) => "Rename Criteria",
                    CriteriaModalAction::EditCost(..) => "Edit Cost",
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
                            CriteriaModalAction::ConfirmDelete(..) => {
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
                                    if let CriteriaModalAction::EditCost(..) = modal.action {
                                        ui.label("Cost:");
                                    } else {
                                        ui.label("Name:");
                                    }
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
                        CriteriaModalAction::ConfirmDelete(id, _) => {
                            state.criteria.remove(id);
                            state.is_modified = true;
                            state.modal_state = None;
                        }
                        CriteriaModalAction::AddChild(parent_id, position, ref node_type) => {
                            if !modal.input_name.trim().is_empty() {
                                let name = modal.input_name.trim().to_string();
                                let child = CriteriaNode {
                                    id: state.next_id,
                                    name,
                                    node_type: node_type.clone(),
                                    cost: None,
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
                        CriteriaModalAction::Rename(id, _) => {
                            if !modal.input_name.trim().is_empty() {
                                let name = modal.input_name.trim().to_string();
                                state.criteria.rename(id, name);
                                state.is_modified = true;
                                state.modal_state = None;
                            } else {
                                submitted = false; // keep open
                            }
                        }
                        CriteriaModalAction::EditCost(id, _) => {
                            let input = modal.input_name.trim();
                            if input.is_empty() {
                                state.criteria.set_cost(id, None);
                                state.is_modified = true;
                                state.modal_state = None;
                            } else if let Ok(val) = input.parse::<f64>() {
                                state.criteria.set_cost(id, Some(val));
                                state.is_modified = true;
                                state.modal_state = None;
                            } else {
                                submitted = false; // keep open if invalid number
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
                ui.radio_value(
                    &mut state.input_mode,
                    "Wizard".to_string(),
                    "Step-by-step (Wizard)",
                );
                ui.radio_value(
                    &mut state.input_mode,
                    "Scrolling".to_string(),
                    "Single Scrolling Page",
                );
            });
            ui.separator();
            ui.heading("Pairwise Comparisons");

            fn generate_phase1(
                node: &CriteriaNode,
                comps: &mut Vec<(String, Vec<(String, usize, usize)>)>,
                goal_text: &str,
            ) {
                let criteria_children: Vec<&CriteriaNode> = node
                    .children
                    .iter()
                    .filter(|c| c.node_type == "Criteria")
                    .collect();
                let n = criteria_children.len();
                if n >= 2 {
                    let parent_name = if node.id == 0 {
                        if goal_text.is_empty() {
                            "Goal"
                        } else {
                            goal_text
                        }
                    } else {
                        &node.name
                    };
                    let group_title = format!("With respect to: {}", parent_name);
                    let mut group_comps = Vec::new();
                    for i in 0..n {
                        for j in (i + 1)..n {
                            let title = format!(
                                "{} vs {}",
                                criteria_children[i].name, criteria_children[j].name
                            );
                            group_comps.push((
                                title,
                                criteria_children[i].id,
                                criteria_children[j].id,
                            ));
                        }
                    }
                    comps.push((group_title, group_comps));
                }
                for child in criteria_children {
                    generate_phase1(child, comps, goal_text);
                }
            }

            let mut grouped_comparisons = Vec::new();

            // Phase 1
            let mut phase1_comps = Vec::new();
            generate_phase1(&state.criteria, &mut phase1_comps, &state.goal);
            if !phase1_comps.is_empty() {
                grouped_comparisons.push(("PHASE 1: WEIGHTING THE CRITERIA".to_string(), vec![]));
                grouped_comparisons.extend(phase1_comps);
            }

            // Phase 2
            let candidates: Vec<&CriteriaNode> = state
                .criteria
                .children
                .iter()
                .filter(|c| c.node_type == "Alternative")
                .collect();
            let top_criteria: Vec<&CriteriaNode> = state
                .criteria
                .children
                .iter()
                .filter(|c| c.node_type == "Criteria")
                .collect();

            if !candidates.is_empty() && top_criteria.len() >= 2 {
                grouped_comparisons.push(("PHASE 2: CANDIDATE PROFILES".to_string(), vec![]));
                for cand in candidates {
                    let group_title = format!("With respect to: {}", cand.name);
                    let mut group_comps = Vec::new();
                    let n = top_criteria.len();
                    for i in 0..n {
                        for j in (i + 1)..n {
                            let title =
                                format!("{} vs {}", top_criteria[i].name, top_criteria[j].name);
                            // We offset candidate IDs for the mock values so they are uniquely scoped by candidate
                            let id1 = top_criteria[i].id + cand.id * 10000;
                            let id2 = top_criteria[j].id + cand.id * 10000;
                            group_comps.push((title, id1, id2));
                        }
                    }
                    grouped_comparisons.push((group_title, group_comps));
                }
            }

            let mut flat_comparisons = Vec::new();
            for (g_title, comps) in &grouped_comparisons {
                if comps.is_empty() {
                    continue;
                } // Skip Phase headers for the wizard flat list
                for (title, id1, id2) in comps {
                    flat_comparisons.push((g_title.clone(), title.clone(), *id1, *id2));
                }
            }

            if flat_comparisons.is_empty() {
                ui.label("Add at least two criteria and a candidate to begin comparisons.");
            } else {
                let render_selector =
                    |ui: &mut egui::Ui, g_title: &str, title: &str, val: &mut f64| -> bool {
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

                        let current_text = options
                            .iter()
                            .min_by(|a, b| {
                                (a.0 - *val)
                                    .abs()
                                    .partial_cmp(&(b.0 - *val).abs())
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            })
                            .map(|(_, text)| text.to_string())
                            .unwrap_or_else(|| "Equal importance".to_string());

                        // Use a unique ID source combining the group title and title to prevent collision
                        let id_source = format!("{} - {} - selector", g_title, title);
                        egui::ComboBox::from_id_source(id_source)
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
                        ui.horizontal(|ui| {
                            let parts: Vec<&str> = title.split(" vs ").collect();
                            let name1 = parts.get(0).unwrap_or(&"");
                            let name2 = parts.get(1).unwrap_or(&"");
                            ui.label(*name1);
                            if render_selector(ui, g_title, title, val) {
                                state.is_modified = true;
                            }
                            ui.label(*name2);
                        });

                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(idx > 0, egui::Button::new("Previous"))
                                .clicked()
                            {
                                state.wizard_step -= 1;
                            }
                            if ui
                                .add_enabled(
                                    idx < flat_comparisons.len() - 1,
                                    egui::Button::new("Next"),
                                )
                                .clicked()
                            {
                                state.wizard_step += 1;
                            }
                        });
                    });
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (i, (g_title, comps)) in grouped_comparisons.iter().enumerate() {
                            if comps.is_empty() {
                                // This is a Phase header
                                ui.add_space(20.0);
                                ui.label(
                                    egui::RichText::new(g_title)
                                        .heading()
                                        .color(egui::Color32::from_rgb(100, 150, 255)),
                                );
                                ui.separator();
                                continue;
                            }

                            if i > 0 && !grouped_comparisons[i - 1].1.is_empty() {
                                ui.add_space(10.0);
                                ui.separator();
                            }

                            ui.label(egui::RichText::new(g_title).strong());
                            ui.add_space(5.0);
                            for (title, id1, id2) in comps {
                                let val = state.saaty_values.entry((*id1, *id2)).or_insert(1.0);
                                let parts: Vec<&str> = title.split(" vs ").collect();
                                let name1 = parts.get(0).unwrap_or(&"");
                                let name2 = parts.get(1).unwrap_or(&"");

                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(*name1);
                                        if render_selector(ui, g_title, title, val) {
                                            state.is_modified = true;
                                        }
                                        ui.label(*name2);
                                    });
                                });
                            }
                        }
                    });
                }
            }
        }
        DocumentTab::Results => {
            // ... (keep this unchanged but add Assignments below it, I will use line numbers carefully)
            ui.heading("Results & Alignment");
            ui.label("Detailed breakdown of how each Candidate scores across your Criteria.");

            ui.separator();

            let candidates: Vec<&CriteriaNode> = state
                .criteria
                .children
                .iter()
                .filter(|c| c.node_type == "Alternative")
                .collect();
            let top_criteria: Vec<&CriteriaNode> = state
                .criteria
                .children
                .iter()
                .filter(|c| c.node_type == "Criteria")
                .collect();

            if candidates.is_empty() {
                ui.label("No candidates defined.");
            } else if top_criteria.is_empty() {
                ui.label("No criteria defined.");
            } else {
                struct CandidateResult {
                    name: String,
                    alignment: f64,
                    cost: Option<f64>,
                    value_score: Option<f64>,
                    criteria_scores: std::collections::HashMap<usize, f64>,
                }

                let mut results = Vec::new();

                for cand in candidates {
                    let mut mock_alignment = 0.0;
                    let mut criteria_scores = std::collections::HashMap::new();

                    for (i, crit) in top_criteria.iter().enumerate() {
                        let crit_weight = 1.0 / (top_criteria.len() as f64);
                        let profile_score = ((i + cand.id) as f64 % 3.0 + 1.0) / 5.0; // dummy math
                        mock_alignment += crit_weight * profile_score;
                        criteria_scores.insert(crit.id, profile_score);
                    }

                    let value_score = if let Some(c) = cand.cost {
                        if mock_alignment > 0.0 && c > 0.0 {
                            Some(mock_alignment / c)
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    results.push(CandidateResult {
                        name: cand.name.clone(),
                        alignment: mock_alignment,
                        cost: cand.cost,
                        value_score,
                        criteria_scores,
                    });
                }

                // Sorting Logic: Auto-sort by Value Score Descending left-to-right
                results.sort_by(|a, b| {
                    let val_a = a.value_score.unwrap_or(f64::MIN);
                    let val_b = b.value_score.unwrap_or(f64::MIN);
                    val_b
                        .partial_cmp(&val_a)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                use egui_extras::{Column, TableBuilder};

                let mut table = TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::initial(150.0).at_least(100.0)); // Metric Name

                for _ in 0..results.len() {
                    table = table.column(Column::initial(120.0));
                }

                table
                    .header(25.0, |mut header| {
                        header.col(|ui| {
                            ui.strong("Metric / Criteria");
                        });
                        for (idx, cand) in results.iter().enumerate() {
                            header.col(|ui| {
                                if idx == 0
                                    && cand.value_score.is_some()
                                    && cand.value_score.unwrap() > 0.0
                                {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "⭐ Best Value - {}",
                                            cand.name
                                        ))
                                        .strong()
                                        .color(egui::Color32::from_rgb(200, 160, 0)),
                                    );
                                } else {
                                    ui.strong(&cand.name);
                                }
                            });
                        }
                    })
                    .body(|mut body| {
                        // Criteria rows
                        for crit in &top_criteria {
                            body.row(20.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(&crit.name);
                                });
                                for cand in &results {
                                    row.col(|ui| {
                                        let score =
                                            cand.criteria_scores.get(&crit.id).unwrap_or(&0.0);
                                        ui.label(format!("{:.4}", score));
                                    });
                                }
                            });
                        }

                        // Total Alignment
                        body.row(25.0, |mut row| {
                            row.col(|ui| {
                                ui.label(egui::RichText::new("Total Alignment").strong());
                            });
                            for cand in &results {
                                row.col(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{:.2}%",
                                            cand.alignment * 100.0
                                        ))
                                        .strong(),
                                    );
                                });
                            }
                        });

                        // Cost
                        body.row(25.0, |mut row| {
                            row.col(|ui| {
                                ui.label(egui::RichText::new("Cost").strong());
                            });
                            for cand in &results {
                                row.col(|ui| {
                                    if let Some(c) = cand.cost {
                                        ui.label(egui::RichText::new(format!("{:.2}", c)).strong());
                                    } else {
                                        ui.label(egui::RichText::new("-").strong());
                                    }
                                });
                            }
                        });

                        // Value Score
                        body.row(25.0, |mut row| {
                            row.col(|ui| {
                                ui.label(egui::RichText::new("Value Score").strong());
                            });
                            for cand in &results {
                                row.col(|ui| {
                                    if let Some(v) = cand.value_score {
                                        ui.label(
                                            egui::RichText::new(format!("{:.4}", v))
                                                .color(egui::Color32::from_rgb(0, 200, 100))
                                                .strong(),
                                        );
                                    } else {
                                        ui.label(egui::RichText::new("-").strong());
                                    }
                                });
                            }
                        });
                    });
            }
        }
        DocumentTab::Assignments => {
            ui.heading("Document Assignments");
            ui.label("Manage users and groups assigned to evaluate this document.");

            if state.assignments.is_none() && state.assignments_rx.is_none() {
                let (tx, rx) = std::sync::mpsc::channel();
                state.assignments_rx = Some(rx);
                let url = format!("{}/{}/assignments", api_url, state.id);
                let mut request = ehttp::Request::get(url);
                if let Some(token) = jwt_token {
                    request
                        .headers
                        .insert("Authorization", &format!("Bearer {}", token));
                }
                let ctx_clone = ui.ctx().clone();
                ehttp::fetch(request, move |result| {
                    let res = match result {
                        Ok(response) => {
                            if response.status == 200 {
                                if let Some(text) = response.text() {
                                    serde_json::from_str::<DocumentAssignments>(text)
                                        .map_err(|e| format!("Parse error: {}", e))
                                } else {
                                    Err("No body".to_string())
                                }
                            } else {
                                Err(format!("Error: {}", response.status))
                            }
                        }
                        Err(e) => Err(e),
                    };
                    let _ = tx.send(res);
                    ctx_clone.request_repaint();
                });
            }

            if let Some(rx) = &state.assignments_rx {
                if let Ok(res) = rx.try_recv() {
                    state.assignments_rx = None;
                    if let Ok(assignments) = res {
                        state.assignments = Some(assignments);
                    }
                } else {
                    ui.spinner();
                    ui.label("Loading assignments...");
                }
            }

            if let Some(assignments) = &mut state.assignments {
                ui.separator();
                ui.heading("Users");

                let mut remove_user = None;
                for uid in &assignments.user_ids {
                    ui.horizontal(|ui| {
                        ui.label(format!("User ID: {}", uid));
                        if ui.button("Remove").clicked() {
                            remove_user = Some(*uid);
                        }
                    });
                }
                if let Some(u) = remove_user {
                    assignments.user_ids.retain(|x| *x != u);
                }

                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut state.new_user_assignment_id);
                    if ui.button("Add User ID").clicked() {
                        if let Ok(id) = state.new_user_assignment_id.parse::<i32>() {
                            if !assignments.user_ids.contains(&id) {
                                assignments.user_ids.push(id);
                            }
                            state.new_user_assignment_id.clear();
                        }
                    }
                });

                ui.separator();
                ui.heading("Groups");

                let mut remove_group = None;
                for gid in &assignments.group_ids {
                    ui.horizontal(|ui| {
                        ui.label(format!("Group ID: {}", gid));
                        if ui.button("Remove").clicked() {
                            remove_group = Some(*gid);
                        }
                    });
                }
                if let Some(g) = remove_group {
                    assignments.group_ids.retain(|x| *x != g);
                }

                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut state.new_group_assignment_id);
                    if ui.button("Add Group ID").clicked() {
                        if let Ok(id) = state.new_group_assignment_id.parse::<i32>() {
                            if !assignments.group_ids.contains(&id) {
                                assignments.group_ids.push(id);
                            }
                            state.new_group_assignment_id.clear();
                        }
                    }
                });

                ui.separator();
                if ui.button("Save Assignments").clicked() && state.assignments_save_rx.is_none() {
                    let url = format!("{}/{}/assignments", api_url, state.id);
                    if let Ok(body) = serde_json::to_vec(assignments) {
                        let mut request = ehttp::Request::post(url, body);
                        if let Some(token) = jwt_token {
                            request
                                .headers
                                .insert("Authorization", &format!("Bearer {}", token));
                        }
                        request
                            .headers
                            .headers
                            .retain(|(k, _)| k.to_lowercase() != "content-type");
                        request
                            .headers
                            .headers
                            .retain(|(k, _)| k.to_lowercase() != "content-type");
                        request.headers.insert("Content-Type", "application/json");

                        let (tx, rx) = std::sync::mpsc::channel();
                        state.assignments_save_rx = Some(rx);
                        let ctx_clone = ui.ctx().clone();
                        ehttp::fetch(request, move |result| {
                            let _ = tx.send(result.is_ok());
                            ctx_clone.request_repaint();
                        });
                    }
                }

                if let Some(rx) = &state.assignments_save_rx {
                    if let Ok(success) = rx.try_recv() {
                        state.assignments_save_rx = None;
                        if success {
                            // Done
                        }
                    } else {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Saving...");
                        });
                    }
                }
            }
        }
    }
}
