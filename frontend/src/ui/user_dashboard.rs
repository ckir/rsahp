use crate::ui::document_window::DocumentState;
use eframe::egui;
use serde::Deserialize;
use std::sync::mpsc::{Receiver, channel};

/// Data transfer object representing the file system tree.
#[derive(serde::Deserialize, Clone)]
pub struct TreeDto {
    pub folders: Vec<FolderDto>,
    pub documents: Vec<DocumentDto>,
}

/// Data transfer object representing a folder.
#[derive(serde::Deserialize, Clone, PartialEq)]
pub struct FolderDto {
    pub id: i32,
    pub name: String,
    pub owner_id: i32,
    pub parent_folder_id: Option<i32>,
}

/// Data transfer object representing a document.
#[derive(serde::Deserialize, Clone, PartialEq)]
pub struct DocumentDto {
    pub id: i32,
    pub name: String,
    pub owner_id: i32,
    pub version: i32,
    pub aggregation_method: String,
    pub folder_id: Option<i32>,
}

/// UI state for the user dashboard.
pub struct UserDashboardState {
    pub is_open: bool,
    pub fetched_initial: bool,
    pub tree_rx: Option<Receiver<Result<TreeDto, String>>>,
    pub fetch_in_progress: bool,
    pub error_msg: Option<String>,
    pub documents: Vec<DocumentDto>,
}

impl Default for UserDashboardState {
    fn default() -> Self {
        Self {
            is_open: true,
            fetched_initial: false,
            tree_rx: None,
            fetch_in_progress: false,
            error_msg: None,
            documents: Vec::new(),
        }
    }
}

/// Renders the user dashboard, displaying a folder/document tree.
pub fn render(
    ctx: &egui::Context,
    state: &mut UserDashboardState,
    open_documents: &mut Vec<DocumentState>,
    api_url: &str,
    jwt_token: Option<&str>,
    logged_in_user_id: Option<i32>,
) {
    if !state.is_open {
        return;
    }

    if !state.fetched_initial
        && state.tree_rx.is_none()
        && !state.fetch_in_progress
        && state.error_msg.is_none()
    {
        let (tx, rx) = channel();
        state.tree_rx = Some(rx);
        state.fetch_in_progress = true;
        state.fetched_initial = true;

        let mut request = ehttp::Request::get(format!("{}/tree", api_url));
        if let Some(token) = jwt_token {
            request
                .headers
                .insert("Authorization", &format!("Bearer {}", token));
        }

        let ctx_clone = ctx.clone();
        ehttp::fetch(request, move |result| {
            let res = match result {
                Ok(response) => {
                    if response.status == 200 {
                        if let Some(text) = response.text() {
                            serde_json::from_str::<TreeDto>(text)
                                .map_err(|e| format!("Parse Error: {}", e))
                        } else {
                            Err("Empty response".to_string())
                        }
                    } else {
                        Err(format!("HTTP {}", response.status))
                    }
                }
                Err(e) => Err(e),
            };
            let _ = tx.send(res);
            ctx_clone.request_repaint();
        });
    }

    if let Some(rx) = &state.tree_rx {
        if let Ok(res) = rx.try_recv() {
            state.fetch_in_progress = false;
            state.tree_rx = None;
            match res {
                Ok(tree) => {
                    state.documents = tree.documents;
                    state.error_msg = None;
                }
                Err(e) => {
                    tracing::error!("Failed to fetch documents: {}", e);
                    state.error_msg = Some(e);
                }
            }
        }
    }

    let user_id = logged_in_user_id.unwrap_or(0);
    let mut my_documents: Vec<&DocumentDto> = Vec::new();
    let mut evaluation_tasks: Vec<&DocumentDto> = Vec::new();

    for doc in &state.documents {
        if doc.owner_id == user_id {
            my_documents.push(doc);
        } else {
            evaluation_tasks.push(doc);
        }
    }

    let mut needs_refresh = false;

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.heading("User Dashboard");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄 Refresh Data").clicked() {
                    needs_refresh = true;
                }
                if let Some(err) = &state.error_msg {
                    ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
                }
            });
        });
        ui.separator();

        ui.horizontal(|ui| {
            // Left Panel: Documents to Evaluate
            ui.vertical(|ui| {
                ui.set_width(ui.available_width() / 2.0 - 10.0);
                ui.heading("Documents to Evaluate");
                ui.label(format!(
                    "Pending Evaluation Tasks ({})",
                    evaluation_tasks.len()
                ));
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_source("evaluations_scroll")
                    .show(ui, |ui| {
                        if state.fetch_in_progress {
                            ui.spinner();
                        } else if evaluation_tasks.is_empty() {
                            ui.label("No pending evaluations.");
                        } else {
                            for doc in evaluation_tasks {
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(format!("📋 {}", doc.name));
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui.button("Evaluate").clicked() {
                                                    // Open document
                                                    let mut exists = false;
                                                    for open_doc in open_documents.iter_mut() {
                                                        if open_doc.id == doc.id {
                                                            exists = true;
                                                            break;
                                                        }
                                                    }
                                                    if !exists {
                                                        open_documents.push(DocumentState::new(
                                                            doc.id, &doc.name,
                                                        ));
                                                    }
                                                }
                                            },
                                        );
                                    });
                                });
                            }
                        }
                    });
            });

            ui.separator();

            // Right Panel: My Documents
            ui.vertical(|ui| {
                ui.heading("My Documents");
                ui.label(format!("Projects You Own ({})", my_documents.len()));
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_source("owned_projects_scroll")
                    .show(ui, |ui| {
                        if state.fetch_in_progress {
                            ui.spinner();
                        } else if my_documents.is_empty() {
                            ui.label("You do not own any projects.");
                        } else {
                            for doc in my_documents {
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(format!("📁 {}", doc.name));
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui.button("View / Edit").clicked() {
                                                    // Open document
                                                    let mut exists = false;
                                                    for open_doc in open_documents.iter_mut() {
                                                        if open_doc.id == doc.id {
                                                            exists = true;
                                                            break;
                                                        }
                                                    }
                                                    if !exists {
                                                        open_documents.push(DocumentState::new(
                                                            doc.id, &doc.name,
                                                        ));
                                                    }
                                                }
                                            },
                                        );
                                    });
                                });
                            }
                        }
                    });
            });
        });
    });

    if needs_refresh {
        state.documents.clear();
        state.error_msg = None;
        state.fetch_in_progress = false;
        state.fetched_initial = false;
    }
}
