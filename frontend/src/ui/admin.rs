//! This module renders the administration panel for user and group management.

use eframe::egui;
use egui_extras::{Column, TableBuilder};
use egui_ltreeview::{Action, DirPosition, NodeBuilder, TreeView, TreeViewState};
use serde::{Deserialize, Serialize};

/// Action to take when the admin modal is closed.
#[derive(Clone)]
pub enum ModalAction {
    /// Add a new group with an optional parent ID at a specific position.
    AddGroup(Option<i32>, DirPosition<i32>),
    /// Confirm the deletion of a group by its ID.
    ConfirmDelete(i32),
    /// Assign a user to a selected set of groups.
    AssignUser(i32),
    /// Placeholder for adding a new user.
    AddUser,
    /// Rename an existing group by its ID.
    RenameGroup(i32),
}

/// State for the admin modal dialog.
pub struct ModalState {
    /// The action the modal represents.
    pub action: ModalAction,
    /// The current text input for naming/renaming.
    pub input_name: String,
    /// A set of selected group IDs (used for user assignment).
    pub selected_groups: std::collections::HashSet<i32>,
}

/// UI state for the administration panel.
pub struct AdminState {
    /// Indicates whether the admin panel is currently open.
    pub is_open: bool,
    /// The list of users loaded from the backend.
    pub users: Vec<UserAdminDto>,
    /// The list of groups loaded from the backend.
    pub groups: Vec<GroupDto>,
    /// Receiver channel for handling the initial fetch of users and groups.
    pub fetch_rx:
        Option<std::sync::mpsc::Receiver<Result<(Vec<UserAdminDto>, Vec<GroupDto>), String>>>,
    /// Receiver channel for handling action responses (like edits or deletes).
    pub action_rx: Option<std::sync::mpsc::Receiver<bool>>,
    /// The state of the group tree view.
    pub tree_view_state: TreeViewState<i32>,
    /// The current state of the modal dialog, if open.
    pub modal_state: Option<ModalState>,
    /// The ID of the currently selected user in the table.
    pub selected_user_id: Option<i32>,
}

/// Default implementation for `AdminState`.
impl Default for AdminState {
    fn default() -> Self {
        Self {
            // Admin panel is closed by default.
            is_open: false,
            // Initialize empty users list.
            users: Vec::new(),
            // Initialize empty groups list.
            groups: Vec::new(),
            // No fetch in progress initially.
            fetch_rx: None,
            // No action in progress initially.
            action_rx: None,
            // Default tree view state.
            tree_view_state: TreeViewState::default(),
            // No modal open initially.
            modal_state: None,
            // No user selected initially.
            selected_user_id: None,
        }
    }
}

/// Data transfer object for admin user responses.
#[derive(Serialize, Deserialize, Clone)]
pub struct UserAdminDto {
    /// The user's unique identifier.
    pub id: i32,
    /// The user's email address.
    pub email: String,
    /// Whether the user has admin privileges.
    pub is_admin: bool,
    /// Whether the user's account is disabled/deleted.
    pub is_deleted: bool,
    /// A list of group IDs the user belongs to.
    pub groups: Vec<i32>,
}

/// Data transfer object for group responses.
#[derive(Serialize, Deserialize, Clone)]
pub struct GroupDto {
    /// The group's unique identifier.
    pub id: Option<i32>,
    /// The name of the group.
    pub name: String,
    /// The ID of the parent group, if any.
    pub parent_id: Option<i32>,
}

/// Represents a node in the group tree.
pub struct GroupNode {
    /// The unique identifier of the group.
    pub id: i32,
    /// The name of the group.
    pub name: String,
    /// The children group nodes.
    pub children: Vec<GroupNode>,
}

/// Builds a hierarchical tree of groups from a flat list.
fn build_group_tree(groups: &[GroupDto], parent_id: Option<i32>) -> Vec<GroupNode> {
    let mut children = Vec::new();
    // Iterate over all groups to find children of the given parent_id.
    for g in groups {
        if g.parent_id == parent_id {
            if let Some(id) = g.id {
                // Recursively build the tree for each child.
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

/// Actions available from the context menu in the group tree.
enum ContextMenuActions {
    /// Delete the specified group.
    Delete(i32),
    /// Add a sub-group under the specified group.
    AddSubGroup(i32, DirPosition<i32>),
    /// Rename the specified group.
    Rename(i32),
}

/// Recursively renders a group node in the tree view.
fn show_group_node(
    builder: &mut egui_ltreeview::TreeViewBuilder<i32>,
    node: &GroupNode,
    actions: &mut Vec<ContextMenuActions>,
) {
    // Render the current directory node.
    builder.node(
        NodeBuilder::dir(node.id)
            .label(&node.name)
            .default_open(true)
            .context_menu(|ui| {
                ui.set_width(100.0);
                ui.label("group:");
                ui.label(&node.name);
                ui.separator();
                // Rename action.
                if ui.button("rename").clicked() {
                    actions.push(ContextMenuActions::Rename(node.id));
                    ui.close();
                }
                // Delete action.
                if ui.button("delete").clicked() {
                    actions.push(ContextMenuActions::Delete(node.id));
                    ui.close();
                }
                ui.separator();
                // New sub-group action.
                if ui.button("new sub-group").clicked() {
                    actions.push(ContextMenuActions::AddSubGroup(node.id, DirPosition::Last));
                    ui.close();
                }
            }),
    );
    // Recursively render children nodes.
    for child in &node.children {
        show_group_node(builder, child, actions);
    }
    // Close the current directory node builder.
    builder.close_dir();
}

/// Payload for updating a user's assigned groups.
#[derive(Serialize)]
struct SetGroupsDto {
    /// The new list of group IDs for the user.
    group_ids: Vec<i32>,
}

/// Renders the admin panel UI.
pub fn render(ctx: &egui::Context, state: &mut AdminState, api_url: &str, jwt_token: Option<&str>) {
    // Skip rendering if the admin panel is not open.
    if !state.is_open {
        return;
    }

    // Refresh data if lists are empty and no fetch is in progress.
    if state.users.is_empty() && state.groups.is_empty() && state.fetch_rx.is_none() {
        let (tx, rx) = std::sync::mpsc::channel();
        state.fetch_rx = Some(rx);

        // Prepare request to fetch users.
        let mut req1 =
            ehttp::Request::get(format!("{}/admin/users", api_url.replace("/documents", "")));
        // Prepare request to fetch groups.
        let mut req2 = ehttp::Request::get(format!(
            "{}/admin/groups",
            api_url.replace("/documents", "")
        ));
        
        // Add authorization headers if token is present.
        if let Some(token) = jwt_token {
            req1.headers
                .insert("Authorization", &format!("Bearer {}", token));
            req2.headers
                .insert("Authorization", &format!("Bearer {}", token));
        }

        let ctx_clone = ctx.clone();
        
        // Execute the first fetch request (users).
        ehttp::fetch(req1, move |res1| {
            // Process the response for users.
            let u = res1.and_then(|r| {
                if let Some(txt) = r.text() {
                    serde_json::from_str::<Vec<UserAdminDto>>(txt).map_err(|e| e.to_string())
                } else {
                    Err("No body".to_string())
                }
            });
            // Execute the second fetch request (groups).
            ehttp::fetch(req2, move |res2| {
                // Process the response for groups.
                let g = res2.and_then(|r| {
                    if let Some(txt) = r.text() {
                        serde_json::from_str::<Vec<GroupDto>>(txt).map_err(|e| e.to_string())
                    } else {
                        Err("No body".to_string())
                    }
                });

                // Combine the results of both fetches.
                let combined = match (u, g) {
                    (Ok(users), Ok(groups)) => Ok((users, groups)),
                    (Err(e), _) | (_, Err(e)) => Err(e),
                };
                
                // Send the combined result back to the main thread.
                let _ = tx.send(combined);
                // Request a UI repaint to process the result.
                ctx_clone.request_repaint();
            });
        });
    }

    // Check for fetch results.
    if let Some(rx) = &state.fetch_rx {
        if let Ok(res) = rx.try_recv() {
            // Fetch complete, clear the receiver.
            state.fetch_rx = None;
            // Update state with fetched data if successful.
            if let Ok((users, groups)) = res {
                state.users = users;
                state.groups = groups;
            }
        }
    }

    // Check for action completion (e.g., update, delete).
    if let Some(rx) = &state.action_rx {
        if let Ok(_) = rx.try_recv() {
            // Action complete, clear the receiver.
            state.action_rx = None;
            // Clear current data to trigger a refresh on the next frame.
            state.users.clear();
            state.groups.clear();
        }
    }

    // Render the left side panel for the group tree.
    egui::SidePanel::left("admin_group_tree_panel")
        .resizable(true)
        .default_width(250.0)
        .show(ctx, |ui| {
            ui.heading("User Groups");
            ui.separator();

            // Scrollable area for the group tree.
            egui::ScrollArea::both().id_source("groups").show(ui, |ui| {
                // Build the group tree from the flat list.
                let root_children = build_group_tree(&state.groups, None);
                let root = GroupNode {
                    id: -1,
                    name: "All Groups (Root)".to_string(),
                    children: root_children,
                };

                let mut context_menu_actions = Vec::<ContextMenuActions>::new();

                /// Helper function to show a custom group node with context menu.
                fn show_custom_group_node(
                    node: &GroupNode,
                    actions: &mut Vec<ContextMenuActions>,
                    ui: &mut egui::Ui,
                ) {
                    let id = ui.make_persistent_id(format!("group_node_{}", node.id));

                    // Create a collapsing header for the group.
                    let mut header = egui::CollapsingHeader::new(&node.name)
                        .id_salt(id)
                        .default_open(true);

                    // Remove the icon if the group has no children.
                    if node.children.is_empty() {
                        header = header.icon(|_ui, _open, _rect| {});
                    }

                    // Render the header and its children.
                    let response = header.show(ui, |ui| {
                        for child in &node.children {
                            show_custom_group_node(child, actions, ui);
                        }
                    });

                    // Handle double-click to rename (if not the root node).
                    if response.header_response.double_clicked() {
                        if node.id != -1 {
                            actions.push(ContextMenuActions::Rename(node.id));
                        }
                    }

                    // Render the context menu.
                    response.header_response.context_menu(|ui| {
                        ui.set_width(100.0);
                        if node.id == -1 {
                            // Root node context menu.
                            ui.label("Root:");
                            ui.separator();
                            if ui.button("new sub-group").clicked() {
                                actions.push(ContextMenuActions::AddSubGroup(
                                    node.id,
                                    DirPosition::Last,
                                ));
                                ui.close();
                            }
                        } else {
                            // Regular group context menu.
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
                                actions.push(ContextMenuActions::AddSubGroup(
                                    node.id,
                                    DirPosition::Last,
                                ));
                                ui.close();
                            }
                        }
                    });
                }

                // Render the root node.
                show_custom_group_node(&root, &mut context_menu_actions, ui);

                // Process queued context menu actions.
                for action in context_menu_actions {
                    match action {
                        ContextMenuActions::Delete(id) => {
                            // Prompt for delete confirmation.
                            state.modal_state = Some(ModalState {
                                action: ModalAction::ConfirmDelete(id),
                                input_name: String::new(),
                                selected_groups: std::collections::HashSet::new(),
                            });
                        }
                        ContextMenuActions::AddSubGroup(parent_id, position) => {
                            // Prompt for new sub-group name.
                            state.modal_state = Some(ModalState {
                                action: ModalAction::AddGroup(
                                    if parent_id == -1 {
                                        None
                                    } else {
                                        Some(parent_id)
                                    },
                                    position,
                                ),
                                input_name: String::new(),
                                selected_groups: std::collections::HashSet::new(),
                            });
                        }
                        ContextMenuActions::Rename(id) => {
                            // Pre-fill the input name with the current group name.
                            let current_name = state
                                .groups
                                .iter()
                                .find(|g| g.id == Some(id))
                                .map(|g| g.name.clone())
                                .unwrap_or_default();
                            
                            // Prompt for renaming the group.
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

    // Render the main central panel for user management.
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading("Admin User Management");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Assign user to group button.
                if ui.button("Assign to Group").clicked() {
                    if let Some(uid) = state.selected_user_id {
                        let mut selected = std::collections::HashSet::new();
                        // Pre-select the groups the user already belongs to.
                        if let Some(user) = state.users.iter().find(|u| u.id == uid) {
                            for g in &user.groups {
                                selected.insert(*g);
                            }
                        }
                        // Open the group assignment modal.
                        state.modal_state = Some(ModalState {
                            action: ModalAction::AssignUser(uid),
                            input_name: String::new(),
                            selected_groups: selected,
                        });
                    }
                }
                // Add user button (placeholder).
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
        
        // Define the table structure for user data.
        let table = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto()) // index column
            .column(Column::initial(200.0).clip(true)) // Email column
            .column(Column::initial(300.0).clip(true)) // Groups column
            .column(Column::remainder()); // Status column

        // Render the table header.
        table
            .header(20.0, |mut header| {
                header.col(|_| {});
                header.col(|ui| {
                    ui.strong("Email");
                });
                header.col(|ui| {
                    ui.strong("Groups");
                });
                header.col(|ui| {
                    ui.strong("Status");
                });
            })
            // Render the table body.
            .body(|mut body| {
                for (idx, u) in state.users.iter().enumerate() {
                    let is_selected = state.selected_user_id == Some(u.id);
                    body.row(row_height, |mut row| {
                        // Render index column.
                        row.col(|ui| {
                            let response =
                                ui.selectable_label(is_selected, format!("{}.", idx + 1));
                            if response.clicked() {
                                state.selected_user_id = Some(u.id);
                            }
                        });
                        // Render email column.
                        row.col(|ui| {
                            let response = ui.selectable_label(is_selected, &u.email);
                            if response.clicked() {
                                state.selected_user_id = Some(u.id);
                            }
                        });
                        // Render groups column.
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
                        // Render status column.
                        row.col(|ui| {
                            ui.horizontal(|ui| {
                                if u.is_deleted {
                                    ui.label(
                                        egui::RichText::new("Disabled").color(egui::Color32::RED),
                                    );
                                } else {
                                    ui.label(
                                        egui::RichText::new("Active")
                                            .color(egui::Color32::DARK_GREEN),
                                    );
                                }
                                // Toggle enable/disable status.
                                if ui
                                    .button(if u.is_deleted { "Enable" } else { "Disable" })
                                    .clicked()
                                    && state.action_rx.is_none()
                                {
                                    // Construct API request to block/unblock the user.
                                    let mut req = ehttp::Request::put(
                                        format!(
                                            "{}/admin/users/{}/block",
                                            api_url.replace("/documents", ""),
                                            u.id
                                        ),
                                        vec![],
                                    );
                                    if let Some(token) = jwt_token {
                                        req.headers
                                            .insert("Authorization", &format!("Bearer {}", token));
                                    }
                                    let (tx, rx) = std::sync::mpsc::channel();
                                    state.action_rx = Some(rx);
                                    let ctx_clone = ctx.clone();
                                    
                                    // Execute the request.
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

    // Render the modal dialog if needed.
    if let Some(modal) = &mut state.modal_state {
        let mut is_open = true;
        let mut close_requested = false;
        let mut submitted = false;

        // Determine the modal title based on the action.
        let title = match modal.action {
            ModalAction::AddGroup(..) => "New Group Name",
            ModalAction::ConfirmDelete(..) => "Confirm Deletion",
            ModalAction::AssignUser(_) => "Assign to Groups",
            ModalAction::AddUser => "Add New User",
            ModalAction::RenameGroup(_) => "Rename Group",
        };

        // Render the modal window.
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

                // Render content based on the modal action.
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
                        egui::ScrollArea::vertical()
                            .max_height(200.0)
                            .show(ui, |ui| {
                                // List all groups with checkboxes.
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

        // Handle modal submission.
        if submitted {
            match modal.action.clone() {
                ModalAction::ConfirmDelete(id) => {
                    if state.action_rx.is_none() {
                        // Construct the DELETE request for the group.
                        let mut req = ehttp::Request::delete(&format!(
                            "{}/admin/groups/{}",
                            api_url.replace("/documents", ""),
                            id
                        ));
                        if let Some(token) = jwt_token {
                            req.headers
                                .insert("Authorization", &format!("Bearer {}", token));
                        }
                        let (tx, rx) = std::sync::mpsc::channel();
                        state.action_rx = Some(rx);
                        let ctx_clone = ctx.clone();
                        
                        // Execute the request.
                        ehttp::fetch(req, move |_| {
                            let _ = tx.send(true);
                            ctx_clone.request_repaint();
                        });
                    }
                    state.modal_state = None;
                }
                ModalAction::AddGroup(parent_id, _position) => {
                    // Ensure name is not empty.
                    if !modal.input_name.trim().is_empty() && state.action_rx.is_none() {
                        let payload = GroupDto {
                            id: None,
                            name: modal.input_name.trim().to_string(),
                            parent_id,
                        };
                        if let Ok(body) = serde_json::to_vec(&payload) {
                            // Construct the POST request for creating a group.
                            let mut req = ehttp::Request::post(
                                format!("{}/admin/groups", api_url.replace("/documents", "")),
                                body,
                            );
                            if let Some(token) = jwt_token {
                                req.headers
                                    .insert("Authorization", &format!("Bearer {}", token));
                            }
                            
                            // Adjust headers.
                            req.headers
                                .headers
                                .retain(|(k, _)| k.to_lowercase() != "content-type");
                            req.headers
                                .headers
                                .retain(|(k, _)| k.to_lowercase() != "content-type");
                            req.headers.insert("Content-Type", "application/json");
                            
                            let (tx, rx) = std::sync::mpsc::channel();
                            state.action_rx = Some(rx);
                            let ctx_clone = ctx.clone();
                            
                            // Execute the request.
                            ehttp::fetch(req, move |_| {
                                let _ = tx.send(true);
                                ctx_clone.request_repaint();
                            });
                        }
                        state.modal_state = None;
                    } else if modal.input_name.trim().is_empty() {
                        // Reject submission if name is empty.
                        submitted = false; 
                    }
                }
                ModalAction::RenameGroup(id) => {
                    // Ensure name is not empty.
                    if !modal.input_name.trim().is_empty() && state.action_rx.is_none() {
                        if let Some(g) = state.groups.iter().find(|x| x.id == Some(id)) {
                            let payload = GroupDto {
                                id: Some(id),
                                name: modal.input_name.trim().to_string(),
                                parent_id: g.parent_id,
                            };
                            if let Ok(body) = serde_json::to_vec(&payload) {
                                // Construct the PUT request for renaming a group.
                                let mut req = ehttp::Request::put(
                                    format!(
                                        "{}/admin/groups/{}",
                                        api_url.replace("/documents", ""),
                                        id
                                    ),
                                    body,
                                );
                                if let Some(token) = jwt_token {
                                    req.headers
                                        .insert("Authorization", &format!("Bearer {}", token));
                                }
                                
                                // Adjust headers.
                                req.headers
                                    .headers
                                    .retain(|(k, _)| k.to_lowercase() != "content-type");
                                req.headers
                                    .headers
                                    .retain(|(k, _)| k.to_lowercase() != "content-type");
                                req.headers.insert("Content-Type", "application/json");
                                
                                let (tx, rx) = std::sync::mpsc::channel();
                                state.action_rx = Some(rx);
                                let ctx_clone = ctx.clone();
                                
                                // Execute the request.
                                ehttp::fetch(req, move |_| {
                                    let _ = tx.send(true);
                                    ctx_clone.request_repaint();
                                });
                            }
                        }
                        state.modal_state = None;
                    } else if modal.input_name.trim().is_empty() {
                        // Reject submission if name is empty.
                        submitted = false; 
                    }
                }
                ModalAction::AddUser => {
                    // Just close for now.
                    state.modal_state = None;
                }
                ModalAction::AssignUser(uid) => {
                    if state.action_rx.is_none() {
                        let payload = SetGroupsDto {
                            group_ids: modal.selected_groups.iter().cloned().collect(),
                        };
                        if let Ok(body) = serde_json::to_vec(&payload) {
                            // Construct the POST request for updating user groups.
                            let mut req = ehttp::Request::post(
                                format!(
                                    "{}/admin/users/{}/groups",
                                    api_url.replace("/documents", ""),
                                    uid
                                ),
                                body,
                            );
                            if let Some(token) = jwt_token {
                                req.headers
                                    .insert("Authorization", &format!("Bearer {}", token));
                            }
                            
                            // Adjust headers.
                            req.headers
                                .headers
                                .retain(|(k, _)| k.to_lowercase() != "content-type");
                            req.headers
                                .headers
                                .retain(|(k, _)| k.to_lowercase() != "content-type");
                            req.headers.insert("Content-Type", "application/json");
                            
                            let (tx, rx) = std::sync::mpsc::channel();
                            state.action_rx = Some(rx);
                            let ctx_clone = ctx.clone();
                            
                            // Execute the request.
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

        // Close the modal if requested or clicked outside.
        if (!is_open || close_requested) && !submitted {
            state.modal_state = None;
        }
    }
}
