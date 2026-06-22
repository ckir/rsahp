use eframe::egui;
use serde::{Deserialize, Serialize};
use egui_ltreeview::{Action, DirPosition, NodeBuilder, TreeView, TreeViewState};
use egui_extras::{Column, TableBuilder};

#[derive(Clone)]
pub enum ModalAction {
    AddGroup(Option<i32>, DirPosition<i32>),
    ConfirmDelete(i32),
    AssignUser(i32), // pass user id
    AddUser, // placeholder for add user
    RenameGroup(i32),
}

pub struct ModalState {
    pub action: ModalAction,
    pub input_name: String,
    pub selected_groups: std::collections::HashSet<i32>,
}

pub struct AdminState {
    pub is_open: bool,
    pub users: Vec<UserAdminDto>,
    pub groups: Vec<GroupDto>,
    pub fetch_rx: Option<std::sync::mpsc::Receiver<Result<(Vec<UserAdminDto>, Vec<GroupDto>), String>>>,
    pub action_rx: Option<std::sync::mpsc::Receiver<bool>>,
    pub tree_view_state: TreeViewState<i32>,
    pub modal_state: Option<ModalState>,
    pub selected_user_id: Option<i32>,
}

impl Default for AdminState {
    fn default() -> Self {
        Self {
            is_open: false,
            users: Vec::new(),
            groups: Vec::new(),
            fetch_rx: None,
            action_rx: None,
            tree_view_state: TreeViewState::default(),
            modal_state: None,
            selected_user_id: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UserAdminDto {
    pub id: i32,
    pub email: String,
    pub is_admin: bool,
    pub is_deleted: bool,
    pub groups: Vec<i32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GroupDto {
    pub id: Option<i32>,
    pub name: String,
    pub parent_id: Option<i32>,
}

pub struct GroupNode {
    pub id: i32,
    pub name: String,
    pub children: Vec<GroupNode>,
}

fn build_group_tree(groups: &[GroupDto], parent_id: Option<i32>) -> Vec<GroupNode> {
    let mut children = Vec::new();
    for g in groups {
        if g.parent_id == parent_id {
            if let Some(id) = g.id {
                children.push(GroupNode {
                    id,
                    name: g.name.clone(),
                    children: build_group_tree(groups, Some(id)),
                });
            }
        }
    }
    children
}

enum ContextMenuActions {
    Delete(i32),
    AddSubGroup(i32, DirPosition<i32>),
    Rename(i32),
}

fn show_group_node(
    builder: &mut egui_ltreeview::TreeViewBuilder<i32>,
    node: &GroupNode,
    actions: &mut Vec<ContextMenuActions>,
) {
    builder.node(
        NodeBuilder::dir(node.id)
            .label(&node.name)
            .default_open(true)
            .context_menu(|ui| {
                ui.set_width(100.0);
                ui.label("group:");
                ui.label(&node.name);
                ui.separator();
                if ui.button("rename").clicked() {
                    actions.push(ContextMenuActions::Rename(node.id));
                    ui.close();
                }
                if ui.button("delete").clicked() {
                    actions.push(ContextMenuActions::Delete(node.id));
                    ui.close();
                }
                ui.separator();
                if ui.button("new sub-group").clicked() {
                    actions.push(ContextMenuActions::AddSubGroup(node.id, DirPosition::Last));
                    ui.close();
                }
            }),
    );
    for child in &node.children {
        show_group_node(builder, child, actions);
    }
    builder.close_dir();
}

#[derive(Serialize)]
struct SetGroupsDto {
    group_ids: Vec<i32>,
}

pub fn render(ctx: &egui::Context, state: &mut AdminState, api_url: &str, jwt_token: Option<&str>) {
    if !state.is_open {
        return;
    }

    // Refresh data
    if state.users.is_empty() && state.groups.is_empty() && state.fetch_rx.is_none() {
        let (tx, rx) = std::sync::mpsc::channel();
        state.fetch_rx = Some(rx);

        let mut req1 = ehttp::Request::get(format!("{}/admin/users", api_url.replace("/documents", "")));
        let mut req2 = ehttp::Request::get(format!("{}/admin/groups", api_url.replace("/documents", "")));
        if let Some(token) = jwt_token {
            req1.headers.insert("Authorization", &format!("Bearer {}", token));
            req2.headers.insert("Authorization", &format!("Bearer {}", token));
        }

        let ctx_clone = ctx.clone();
        ehttp::fetch(req1, move |res1| {
            let u = res1.and_then(|r| {
                if let Some(txt) = r.text() {
                    serde_json::from_str::<Vec<UserAdminDto>>(txt).map_err(|e| e.to_string())
                } else {
                    Err("No body".to_string())
                }
            });
            ehttp::fetch(req2, move |res2| {
                let g = res2.and_then(|r| {
                    if let Some(txt) = r.text() {
                        serde_json::from_str::<Vec<GroupDto>>(txt).map_err(|e| e.to_string())
                    } else {
                        Err("No body".to_string())
                    }
                });
                
                let combined = match (u, g) {
                    (Ok(users), Ok(groups)) => Ok((users, groups)),
                    (Err(e), _) | (_, Err(e)) => Err(e),
                };
                let _ = tx.send(combined);
                ctx_clone.request_repaint();
            });
        });
    }

    if let Some(rx) = &state.fetch_rx {
        if let Ok(res) = rx.try_recv() {
            state.fetch_rx = None;
            if let Ok((users, groups)) = res {
                state.users = users;
                state.groups = groups;
            }
        }
    }

    if let Some(rx) = &state.action_rx {
        if let Ok(_) = rx.try_recv() {
            state.action_rx = None;
            state.users.clear(); // trigger refresh
            state.groups.clear();
        }
    }

    egui::SidePanel::left("admin_group_tree_panel")
        .resizable(true)
        .default_width(250.0)
        .show(ctx, |ui| {
            ui.heading("User Groups");
            ui.separator();

            egui::ScrollArea::both().id_source("groups").show(ui, |ui| {
                let root_children = build_group_tree(&state.groups, None);
                let root = GroupNode {
                    id: -1,
                    name: "All Groups (Root)".to_string(),
                    children: root_children,
                };

                let mut context_menu_actions = Vec::<ContextMenuActions>::new();

                fn show_custom_group_node(
                    node: &GroupNode,
                    actions: &mut Vec<ContextMenuActions>,
                    ui: &mut egui::Ui,
                ) {
                    let id = ui.make_persistent_id(format!("group_node_{}", node.id));
                    
                    let mut header = egui::CollapsingHeader::new(&node.name)
                        .id_salt(id)
                        .default_open(true);

                    if node.children.is_empty() {
                        header = header.icon(|_ui, _open, _rect| {});
                    }

                    let response = header.show(ui, |ui| {
                        for child in &node.children {
                            show_custom_group_node(child, actions, ui);
                        }
                    });

                    if response.header_response.double_clicked() {
                        if node.id != -1 {
                            actions.push(ContextMenuActions::Rename(node.id));
                        }
                    }

                    response.header_response.context_menu(|ui| {
                        ui.set_width(100.0);
                        if node.id == -1 {
                            ui.label("Root:");
                            ui.separator();
                            if ui.button("new sub-group").clicked() {
                                actions.push(ContextMenuActions::AddSubGroup(node.id, DirPosition::Last));
                                ui.close();
                            }
                        } else {
                            ui.label("group:");
                            ui.label(&node.name);
                            ui.separator();
                            if ui.button("rename").clicked() {
                                actions.push(ContextMenuActions::Rename(node.id));
                                ui.close();
                            }
                            if ui.button("delete").clicked() {
                                actions.push(ContextMenuActions::Delete(node.id));
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("new sub-group").clicked() {
                                actions.push(ContextMenuActions::AddSubGroup(node.id, DirPosition::Last));
                                ui.close();
                            }
                        }
                    });
                }

                show_custom_group_node(&root, &mut context_menu_actions, ui);

                for action in context_menu_actions {
                    match action {
                        ContextMenuActions::Delete(id) => {
                            state.modal_state = Some(ModalState {
                                action: ModalAction::ConfirmDelete(id),
                                input_name: String::new(),
                                selected_groups: std::collections::HashSet::new(),
                            });
                        }
                        ContextMenuActions::AddSubGroup(parent_id, position) => {
                            state.modal_state = Some(ModalState {
                                action: ModalAction::AddGroup(if parent_id == -1 { None } else { Some(parent_id) }, position),
                                input_name: String::new(),
                                selected_groups: std::collections::HashSet::new(),
                            });
                        }
                        ContextMenuActions::Rename(id) => {
                            let current_name = state.groups.iter().find(|g| g.id == Some(id)).map(|g| g.name.clone()).unwrap_or_default();
                            state.modal_state = Some(ModalState {
                                action: ModalAction::RenameGroup(id),
                                input_name: current_name,
                                selected_groups: std::collections::HashSet::new(),
                            });
                        }
                    }
                }
            });
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading("Admin User Management");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Assign to Group").clicked() {
                    if let Some(uid) = state.selected_user_id {
                        let mut selected = std::collections::HashSet::new();
                        if let Some(user) = state.users.iter().find(|u| u.id == uid) {
                            for g in &user.groups {
                                selected.insert(*g);
                            }
                        }
                        state.modal_state = Some(ModalState {
                            action: ModalAction::AssignUser(uid),
                            input_name: String::new(),
                            selected_groups: selected,
                        });
                    }
                }
                if ui.button("+ Add User").clicked() {
                    state.modal_state = Some(ModalState {
                        action: ModalAction::AddUser,
                        input_name: String::new(),
                        selected_groups: std::collections::HashSet::new(),
                    });
                }
            });
        });

        ui.add_space(10.0);
        
        let row_height = 24.0;
        let table = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto()) // index
            .column(Column::initial(200.0).clip(true)) // Email
            .column(Column::initial(300.0).clip(true)) // Groups
            .column(Column::remainder()); // Status

        table.header(20.0, |mut header| {
            header.col(|_| {});
            header.col(|ui| { ui.strong("Email"); });
            header.col(|ui| { ui.strong("Groups"); });
            header.col(|ui| { ui.strong("Status"); });
        })
        .body(|mut body| {
            for (idx, u) in state.users.iter().enumerate() {
                let is_selected = state.selected_user_id == Some(u.id);
                body.row(row_height, |mut row| {
                    row.col(|ui| { 
                        let response = ui.selectable_label(is_selected, format!("{}.", idx + 1)); 
                        if response.clicked() {
                            state.selected_user_id = Some(u.id);
                        }
                    });
                    row.col(|ui| { 
                        let response = ui.selectable_label(is_selected, &u.email);
                        if response.clicked() {
                            state.selected_user_id = Some(u.id);
                        }
                    });
                    row.col(|ui| {
                        let mut group_names = Vec::new();
                        for gid in &u.groups {
                            if let Some(g) = state.groups.iter().find(|x| x.id == Some(*gid)) {
                                group_names.push(g.name.clone());
                            }
                        }
                        if u.is_admin {
                            group_names.push("Admin".to_string());
                        }
                        ui.label(group_names.join(", "));
                    });
                    row.col(|ui| {
                        ui.horizontal(|ui| {
                            if u.is_deleted {
                                ui.label(egui::RichText::new("Disabled").color(egui::Color32::RED));
                            } else {
                                ui.label(egui::RichText::new("Active").color(egui::Color32::DARK_GREEN));
                            }
                            if ui.button(if u.is_deleted { "Enable" } else { "Disable" }).clicked() && state.action_rx.is_none() {
                                let mut req = ehttp::Request::put(format!("{}/admin/users/{}/block", api_url.replace("/documents", ""), u.id), vec![]);
                                if let Some(token) = jwt_token {
                                    req.headers.insert("Authorization", &format!("Bearer {}", token));
                                }
                                let (tx, rx) = std::sync::mpsc::channel();
                                state.action_rx = Some(rx);
                                let ctx_clone = ctx.clone();
                                ehttp::fetch(req, move |_| {
                                    let _ = tx.send(true);
                                    ctx_clone.request_repaint();
                                });
                            }
                        });
                    });
                });
            }
        });
    });

    if let Some(modal) = &mut state.modal_state {
        let mut is_open = true;
        let mut close_requested = false;
        let mut submitted = false;

        let title = match modal.action {
            ModalAction::AddGroup(..) => "New Group Name",
            ModalAction::ConfirmDelete(..) => "Confirm Deletion",
            ModalAction::AssignUser(_) => "Assign to Groups",
            ModalAction::AddUser => "Add New User",
            ModalAction::RenameGroup(_) => "Rename Group",
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
                        ui.label("Are you sure you want to delete this group?");
                        ui.horizontal(|ui| {
                            if ui.button("Yes").clicked() {
                                submitted = true;
                            }
                            if ui.button("No").clicked() {
                                close_requested = true;
                            }
                        });
                    }
                    ModalAction::AddGroup(..) => {
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
                    ModalAction::RenameGroup(..) => {
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
                    ModalAction::AddUser => {
                        ui.label("Adding users manually is not yet implemented.");
                        if ui.button("Close").clicked() {
                            close_requested = true;
                        }
                    }
                    ModalAction::AssignUser(_) => {
                        ui.label("Select groups for user:");
                        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                            for g in &state.groups {
                                if let Some(gid) = g.id {
                                    let mut is_checked = modal.selected_groups.contains(&gid);
                                    if ui.checkbox(&mut is_checked, &g.name).changed() {
                                        if is_checked {
                                            modal.selected_groups.insert(gid);
                                        } else {
                                            modal.selected_groups.remove(&gid);
                                        }
                                    }
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.button("Save").clicked() {
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
                    if state.action_rx.is_none() {
                        let mut req = ehttp::Request::delete(&format!("{}/admin/groups/{}", api_url.replace("/documents", ""), id));
                        if let Some(token) = jwt_token {
                            req.headers.insert("Authorization", &format!("Bearer {}", token));
                        }
                        let (tx, rx) = std::sync::mpsc::channel();
                        state.action_rx = Some(rx);
                        let ctx_clone = ctx.clone();
                        ehttp::fetch(req, move |_| {
                            let _ = tx.send(true);
                            ctx_clone.request_repaint();
                        });
                    }
                    state.modal_state = None;
                }
                ModalAction::AddGroup(parent_id, _position) => {
                    if !modal.input_name.trim().is_empty() && state.action_rx.is_none() {
                        let payload = GroupDto {
                            id: None,
                            name: modal.input_name.trim().to_string(),
                            parent_id,
                        };
                        if let Ok(body) = serde_json::to_vec(&payload) {
                            let mut req = ehttp::Request::post(format!("{}/admin/groups", api_url.replace("/documents", "")), body);
                            if let Some(token) = jwt_token {
                                req.headers.insert("Authorization", &format!("Bearer {}", token));
                            }
                            req.headers.headers.retain(|(k, _)| k.to_lowercase() != "content-type");
                            req.headers.insert("Content-Type", "application/json");
                            let (tx, rx) = std::sync::mpsc::channel();
                            state.action_rx = Some(rx);
                            let ctx_clone = ctx.clone();
                            ehttp::fetch(req, move |_| {
                                let _ = tx.send(true);
                                ctx_clone.request_repaint();
                            });
                        }
                        state.modal_state = None;
                    } else if modal.input_name.trim().is_empty() {
                        submitted = false; // keep open
                    }
                }
                ModalAction::RenameGroup(id) => {
                    if !modal.input_name.trim().is_empty() && state.action_rx.is_none() {
                        if let Some(g) = state.groups.iter().find(|x| x.id == Some(id)) {
                            let payload = GroupDto {
                                id: Some(id),
                                name: modal.input_name.trim().to_string(),
                                parent_id: g.parent_id,
                            };
                            if let Ok(body) = serde_json::to_vec(&payload) {
                                let mut req = ehttp::Request::put(format!("{}/admin/groups/{}", api_url.replace("/documents", ""), id), body);
                                if let Some(token) = jwt_token {
                                    req.headers.insert("Authorization", &format!("Bearer {}", token));
                                }
                                req.headers.headers.retain(|(k, _)| k.to_lowercase() != "content-type");
                                req.headers.insert("Content-Type", "application/json");
                                let (tx, rx) = std::sync::mpsc::channel();
                                state.action_rx = Some(rx);
                                let ctx_clone = ctx.clone();
                                ehttp::fetch(req, move |_| {
                                    let _ = tx.send(true);
                                    ctx_clone.request_repaint();
                                });
                            }
                        }
                        state.modal_state = None;
                    } else if modal.input_name.trim().is_empty() {
                        submitted = false; // keep open
                    }
                }
                ModalAction::AddUser => {
                    state.modal_state = None;
                }
                ModalAction::AssignUser(uid) => {
                    if state.action_rx.is_none() {
                        let payload = SetGroupsDto {
                            group_ids: modal.selected_groups.iter().cloned().collect(),
                        };
                        if let Ok(body) = serde_json::to_vec(&payload) {
                            let mut req = ehttp::Request::post(format!("{}/admin/users/{}/groups", api_url.replace("/documents", ""), uid), body);
                            if let Some(token) = jwt_token {
                                req.headers.insert("Authorization", &format!("Bearer {}", token));
                            }
                            req.headers.headers.retain(|(k, _)| k.to_lowercase() != "content-type");
                            req.headers.insert("Content-Type", "application/json");
                            let (tx, rx) = std::sync::mpsc::channel();
                            state.action_rx = Some(rx);
                            let ctx_clone = ctx.clone();
                            ehttp::fetch(req, move |_| {
                                let _ = tx.send(true);
                                ctx_clone.request_repaint();
                            });
                        }
                    }
                    state.modal_state = None;
                }
            }
        }

        if (!is_open || close_requested) && !submitted {
            state.modal_state = None;
        }
    }
}
