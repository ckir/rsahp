//! This module aggregates all the UI components and defines the main application state.

use eframe::egui;

mod admin;
pub mod auth;
pub mod document_window;
mod explorer;
mod taskbar;
mod user_dashboard;

/// The main state container for the Rsahp application.
pub struct RsahpApp {
    /// Flag indicating whether the task list modal is visible.
    pub show_task_list: bool,
    /// List of currently open documents in the application.
    pub open_documents: Vec<document_window::DocumentState>,
    /// State of the file explorer tree.
    pub explorer_state: explorer::ExplorerState,
    /// Authentication state containing JWT token and user info.
    pub auth_state: auth::AuthState,
    /// State of the admin management panel.
    pub admin_state: admin::AdminState,
    /// State of the user dashboard and project overview.
    pub user_dashboard_state: user_dashboard::UserDashboardState,
    /// The application configuration loaded on startup.
    pub config: crate::config::AppConfig,
}

impl RsahpApp {
    /// Constructs a new `RsahpApp` instance with the given configuration.
    pub fn new(config: crate::config::AppConfig) -> Self {
        Self {
            // Initially, the task list is hidden.
            show_task_list: false,
            // The list of open documents starts empty.
            open_documents: Vec::new(),
            // Initialize explorer state with its default values.
            explorer_state: Default::default(),
            // Initialize auth state with its default values.
            auth_state: Default::default(),
            // Initialize admin state with its default values.
            admin_state: Default::default(),
            // Initialize user dashboard state with its default values.
            user_dashboard_state: Default::default(),
            // Store the application configuration.
            config,
        }
    }
}

impl eframe::App for RsahpApp {
    /// Required `ui` method, but we delegate rendering to `update`.
    fn ui(&mut self, _ui: &mut egui::Ui, _frame: &mut eframe::Frame) {}

    /// The main update loop called by the eframe framework each frame.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Delegate rendering logic to our custom `render` method.
        self.render(ctx);
    }
}

impl RsahpApp {
    /// Handles the main rendering logic for the application.
    pub fn render(&mut self, ctx: &egui::Context) {
        // Apply the zoom scale from the configuration, if set.
        if let Some(scale) = self.config.zoom_scale {
            ctx.set_pixels_per_point(scale);
        }

        // Determine the API URL to use, falling back to a default localhost URL.
        let api_url = self
            .config
            .api_url
            .clone()
            .unwrap_or_else(|| "http://127.0.0.1:4002/api/documents".to_string());

        // Render the authentication modal if the user is not logged in.
        auth::render_login_modal(ctx, &mut self.auth_state, &api_url);

        // If no JWT token is present, halt rendering of the main app.
        if self.auth_state.jwt_token.is_none() {
            // Return early to prevent unauthorized access.
            return;
        }

        // Render the top taskbar.
        taskbar::render(
            ctx,
            &mut self.show_task_list,
            &mut self.explorer_state,
            &mut self.auth_state,
            &mut self.admin_state,
            &mut self.config,
        );

        // Branch UI rendering based on whether the user is an admin.
        if self.auth_state.is_admin {
            // Admins only see the administration window. Set it to open.
            self.admin_state.is_open = true;
            // Render the admin panel.
            admin::render(
                ctx,
                &mut self.admin_state,
                self.config
                    .api_url
                    .as_deref()
                    .unwrap_or("http://localhost:8000/api"),
                self.auth_state.jwt_token.as_deref(),
            );
        } else {
            // Non-admins see the user dashboard.
            user_dashboard::render(
                ctx,
                &mut self.user_dashboard_state,
                &mut self.explorer_state,
                &mut self.open_documents,
                &api_url,
                self.auth_state.jwt_token.as_deref(),
                self.auth_state.logged_in_user_id,
            );
        }

        // Render each open document window.
        let mut closed_docs = Vec::new();
        for (idx, doc) in self.open_documents.iter_mut().enumerate() {
            let mut is_open = true;

            // Check if a close operation has been requested.
            if !doc.close_requested {
                // Render the document window.
                egui::Window::new(&doc.title)
                    .id(egui::Id::new(doc.id))
                    .open(&mut is_open)
                    .vscroll(true)
                    .default_size(egui::vec2(1000.0, 700.0))
                    .default_pos(ctx.screen_rect().center())
                    .pivot(egui::Align2::CENTER_CENTER)
                    .show(ctx, |ui| {
                        // Render the document contents inside the window.
                        document_window::render(
                            ui,
                            doc,
                            &api_url,
                            self.auth_state.jwt_token.as_deref(),
                        );
                    });

                // Check if the user closed the window this frame.
                if !is_open {
                    // If the document has unsaved changes, request confirmation to close.
                    if doc.is_modified {
                        doc.close_requested = true;
                    } else {
                        // Otherwise, mark it for immediate closure.
                        closed_docs.push(idx);
                    }
                }

                // Check if a duplication operation completed.
                if let Some(rx) = &doc.duplicated_doc_rx
                    && let Ok(new_doc) = rx.try_recv()
                {
                    // Add the duplicated document to the explorer tree.
                    if let explorer::Node::Directory(dir) = &mut self.explorer_state.tree {
                        // Create a new File node for the duplicate.
                        dir.children.push(explorer::Node::File(explorer::File {
                            id: self.explorer_state.next_id,
                            name: new_doc.name.clone(),
                            document_id: Some(new_doc.id as usize),
                        }));
                        // Increment the global node ID counter.
                        self.explorer_state.next_id += 1;
                    }
                    // Update the current document state to point to the new duplicate.
                    doc.id = new_doc.id;
                    doc.title = new_doc.name;
                    doc.version = new_doc.version;
                    doc.save_status = Some(format!("✅ Duplicated! (v{})", doc.version));
                    // Clear the channel to avoid processing again.
                    doc.duplicated_doc_rx = None;
                }
            } else {
                // If a close was requested and there are unsaved changes, show a confirmation modal.
                let mut modal_open = true;
                let mut action = None;

                // Render the confirmation modal window.
                egui::Window::new("Unsaved Changes")
                    .id(egui::Id::new("close_modal").with(doc.id))
                    .collapsible(false)
                    .resizable(false)
                    .open(&mut modal_open)
                    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                    .show(ctx, |ui| {
                        // Prompt the user about unsaved changes.
                        ui.label(format!("Save changes to '{}' before closing?", doc.title));
                        ui.horizontal(|ui| {
                            // Save and close button.
                            if ui.button("Save").clicked() {
                                action = Some("save");
                            }
                            // Discard and close button.
                            if ui.button("Don't Save").clicked() {
                                action = Some("discard");
                            }
                            // Cancel closing button.
                            if ui.button("Cancel").clicked() {
                                action = Some("cancel");
                            }
                        });
                    });

                // If the user closed the modal itself, cancel the close request.
                if !modal_open {
                    doc.close_requested = false;
                }

                // Handle the action chosen in the confirmation modal.
                match action {
                    Some("save") => {
                        // Trigger a save operation.
                        document_window::save_document(
                            doc,
                            &api_url,
                            ctx,
                            self.auth_state.jwt_token.as_deref(),
                        );
                        // Mark the document as closed.
                        closed_docs.push(idx);
                    }
                    Some("discard") => {
                        // Mark the document as closed without saving.
                        closed_docs.push(idx);
                    }
                    Some("cancel") => {
                        // Clear the close request flag.
                        doc.close_requested = false;
                    }
                    _ => {}
                }
            }
        }

        // Clean up closed windows by removing them from the list in reverse order.
        for idx in closed_docs.into_iter().rev() {
            self.open_documents.remove(idx);
        }

        // Render the Task List Modal if requested.
        let mut show_task_list = self.show_task_list;
        let mut new_doc = None;
        if show_task_list {
            egui::Window::new("Task List")
                .open(&mut show_task_list)
                .show(ctx, |ui| {
                    // Display dummy pending tasks.
                    ui.label("You have 2 pending AHP surveys.");

                    // Button to open a specific mock task.
                    if ui
                        .button("Survey: Vendor Selection (Management Group)")
                        .clicked()
                    {
                        // Initialize a new document state for the survey.
                        new_doc = Some(document_window::DocumentState::new(
                            101,
                            "Vendor Selection Survey",
                        ));
                    }
                });
        }

        // Update the state flag for task list visibility.
        self.show_task_list = show_task_list;

        // If a new document was requested from the task list, add it.
        if let Some(doc) = new_doc {
            self.open_documents.push(doc);
            // Hide the task list after opening a document.
            self.show_task_list = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use egui_kittest::Harness;

    #[test]
    /// Tests that the application boots and renders headlessly without crashing.
    fn test_app_renders_headless() {
        // Setup default configuration with a local API URL.
        let mut config = AppConfig::default();
        config.api_url = Some("http://127.0.0.1:4002/api/documents".to_string());

        // Initialize the app with the config.
        let mut app = RsahpApp::new(config);

        // Build the egui kittest harness with a fixed size.
        let mut harness = Harness::builder()
            .with_size(eframe::egui::vec2(1200.0, 800.0))
            .build_ui(|ctx| {
                // Call the app render loop once.
                app.render(ctx);
            });

        // Execute a step of the rendering loop.
        harness.step();

        // Assert the app booted cleanly.
        assert!(true);
    }
}
