//! This module renders the user dashboard, displaying a user's projects and pending evaluations.

use crate::ui::document_window::DocumentState;
use eframe::egui;
use serde::Deserialize;
use std::sync::mpsc::{Receiver, channel};

/// Data transfer object representing the file system tree.
#[derive(serde::Deserialize, Clone)]
pub struct TreeDto {
    /// List of folders in the tree.
    pub folders: Vec<FolderDto>,
    /// List of documents in the tree.
    pub documents: Vec<DocumentDto>,
}

/// Data transfer object representing a folder.
#[derive(serde::Deserialize, Clone, PartialEq)]
pub struct FolderDto {
    /// The unique identifier for the folder.
    pub id: i32,
    /// The name of the folder.
    pub name: String,
    /// The ID of the user who owns this folder.
    pub owner_id: i32,
    /// The ID of the parent folder, if any.
    pub parent_folder_id: Option<i32>,
}

/// Data transfer object representing a document.
#[derive(serde::Deserialize, Clone, PartialEq)]
pub struct DocumentDto {
    /// The unique identifier for the document.
    pub id: i32,
    /// The name of the document.
    pub name: String,
    /// The ID of the user who owns this document.
    pub owner_id: i32,
    /// The document version number.
    pub version: i32,
    /// The aggregation method used for the document (e.g., AIJ, AIP).
    pub aggregation_method: String,
    /// The ID of the folder containing this document, if any.
    pub folder_id: Option<i32>,
}

/// UI state for the user dashboard.
pub struct UserDashboardState {
    /// Indicates whether the dashboard panel is open.
    pub is_open: bool,
    /// Indicates if the initial data fetch has been performed.
    pub fetched_initial: bool,
    /// Receiver channel for handling the asynchronous tree data response.
    pub tree_rx: Option<Receiver<Result<TreeDto, String>>>,
    /// Indicates if a fetch request is currently in progress.
    pub fetch_in_progress: bool,
    /// An optional error message to display if fetching fails.
    pub error_msg: Option<String>,
    /// The list of documents retrieved from the server.
    pub documents: Vec<DocumentDto>,
}

/// Default implementation for `UserDashboardState`.
impl Default for UserDashboardState {
    fn default() -> Self {
        Self {
            // Dashboard is open by default.
            is_open: true,
            // Initial fetch has not occurred.
            fetched_initial: false,
            // No receiver channel initially.
            tree_rx: None,
            // No fetch in progress initially.
            fetch_in_progress: false,
            // No error message initially.
            error_msg: None,
            // Document list is empty initially.
            documents: Vec::new(),
        }
    }
}

/// Renders the user dashboard, displaying a folder/document tree.
///
/// This function handles data fetching, categorizing documents into "owned" and "evaluations",
/// and rendering the dashboard UI panels for the current user.
pub fn render(
    ctx: &egui::Context,
    state: &mut UserDashboardState,
    open_documents: &mut Vec<DocumentState>,
    api_url: &str,
    jwt_token: Option<&str>,
    logged_in_user_id: Option<i32>,
) {
    // If the dashboard is not open, skip rendering.
    if !state.is_open {
        return;
    }

    // Check if we need to initiate an initial data fetch.
    if !state.fetched_initial
        && state.tree_rx.is_none()
        && !state.fetch_in_progress
        && state.error_msg.is_none()
    {
        // Create a channel for receiving the fetch result.
        let (tx, rx) = channel();

        // Update state to indicate fetching has started.
        state.tree_rx = Some(rx);
        state.fetch_in_progress = true;
        state.fetched_initial = true;

        // Construct the GET request for the document tree.
        let mut request = ehttp::Request::get(format!("{}/tree", api_url));

        // Add the Authorization header if a token is provided.
        if let Some(token) = jwt_token {
            request
                .headers
                .insert("Authorization", &format!("Bearer {}", token));
        }

        // Clone the egui context to request a repaint later.
        let ctx_clone = ctx.clone();

        // Execute the background fetch request.
        ehttp::fetch(request, move |result| {
            let res = match result {
                Ok(response) => {
                    // Check for HTTP 200 OK status.
                    if response.status == 200 {
                        if let Some(text) = response.text() {
                            // Attempt to parse the JSON response.
                            serde_json::from_str::<TreeDto>(text)
                                .map_err(|e| format!("Parse Error: {}", e))
                        } else {
                            Err("Empty response".to_string())
                        }
                    } else {
                        // Handle non-200 responses.
                        Err(format!("HTTP {}", response.status))
                    }
                }
                // Handle network errors.
                Err(e) => Err(e),
            };

            // Send the result back to the main thread.
            let _ = tx.send(res);

            // Request a UI repaint to process the result.
            ctx_clone.request_repaint();
        });
    }

    // Process any incoming fetch results from the background thread.
    if let Some(rx) = &state.tree_rx {
        if let Ok(res) = rx.try_recv() {
            // Update state to indicate the fetch is complete.
            state.fetch_in_progress = false;
            state.tree_rx = None;
            match res {
                Ok(tree) => {
                    // On success, store the documents and clear any errors.
                    state.documents = tree.documents;
                    state.error_msg = None;
                }
                Err(e) => {
                    // On failure, log the error and update the error message.
                    tracing::error!("Failed to fetch documents: {}", e);
                    state.error_msg = Some(e);
                }
            }
        }
    }

    // Fallback user ID to 0 if none is provided.
    let user_id = logged_in_user_id.unwrap_or(0);

    // Separate documents into "owned" and "evaluations" lists.
    let mut my_documents: Vec<&DocumentDto> = Vec::new();
    let mut evaluation_tasks: Vec<&DocumentDto> = Vec::new();

    for doc in &state.documents {
        // Check if the document belongs to the logged-in user.
        if doc.owner_id == user_id {
            my_documents.push(doc);
        } else {
            // Otherwise, it is an evaluation task.
            evaluation_tasks.push(doc);
        }
    }

    // Flag indicating whether a manual refresh was triggered.
    let mut needs_refresh = false;

    // Render the main central panel for the dashboard.
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.add_space(10.0);

        // Render the top header area.
        ui.horizontal(|ui| {
            ui.heading("User Dashboard");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Render the refresh button.
                if ui.button("🔄 Refresh Data").clicked() {
                    needs_refresh = true;
                }

                // Display the error message if present.
                if let Some(err) = &state.error_msg {
                    ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
                }
            });
        });
        ui.separator();

        // Render the split layout for evaluation tasks and owned documents.
        ui.horizontal(|ui| {
            // Left Panel: Documents to Evaluate
            ui.vertical(|ui| {
                // Set the width to half the available space.
                ui.set_width(ui.available_width() / 2.0 - 10.0);
                ui.heading("Documents to Evaluate");

                // Display the count of pending tasks.
                ui.label(format!(
                    "Pending Evaluation Tasks ({})",
                    evaluation_tasks.len()
                ));
                ui.separator();

                // Scrollable area for evaluation tasks.
                egui::ScrollArea::vertical()
                    .id_source("evaluations_scroll")
                    .show(ui, |ui| {
                        if state.fetch_in_progress {
                            // Display spinner while loading.
                            ui.spinner();
                        } else if evaluation_tasks.is_empty() {
                            // Display empty state message.
                            ui.label("No pending evaluations.");
                        } else {
                            // Render a group for each task.
                            for doc in evaluation_tasks {
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(format!("📋 {}", doc.name));
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                // Handle clicking the evaluate button.
                                                if ui.button("Evaluate").clicked() {
                                                    // Check if the document is already open.
                                                    let mut exists = false;
                                                    for open_doc in open_documents.iter_mut() {
                                                        if open_doc.id == doc.id {
                                                            exists = true;
                                                            break;
                                                        }
                                                    }
                                                    // If not, open it.
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

                // Display the count of owned projects.
                ui.label(format!("Projects You Own ({})", my_documents.len()));
                ui.separator();

                // Scrollable area for owned projects.
                egui::ScrollArea::vertical()
                    .id_source("owned_projects_scroll")
                    .show(ui, |ui| {
                        if state.fetch_in_progress {
                            // Display spinner while loading.
                            ui.spinner();
                        } else if my_documents.is_empty() {
                            // Display empty state message.
                            ui.label("You do not own any projects.");
                        } else {
                            // Render a group for each owned document.
                            for doc in my_documents {
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(format!("📁 {}", doc.name));
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                // Handle clicking the view/edit button.
                                                if ui.button("View / Edit").clicked() {
                                                    // Check if the document is already open.
                                                    let mut exists = false;
                                                    for open_doc in open_documents.iter_mut() {
                                                        if open_doc.id == doc.id {
                                                            exists = true;
                                                            break;
                                                        }
                                                    }
                                                    // If not, open it.
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

    // Reset state if a refresh was requested.
    if needs_refresh {
        state.documents.clear();
        state.error_msg = None;
        state.fetch_in_progress = false;
        state.fetched_initial = false;
    }
}
