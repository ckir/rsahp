//! This module handles the rendering of the application's top navigation taskbar.

use crate::ui::admin::AdminState;
use crate::ui::auth::AuthState;
use crate::ui::explorer::ExplorerState;
use eframe::egui;

/// Renders the top taskbar for the application.
/// 
/// This function draws the top navigation bar containing the logo, navigation links,
/// task list toggle, zoom controls, and logout functionality.
pub fn render(
    ctx: &egui::Context,
    show_task_list: &mut bool,
    explorer_state: &mut ExplorerState,
    auth_state: &mut AuthState,
    admin_state: &mut AdminState,
    config: &mut crate::config::AppConfig,
) {
    // We allow the deprecated TopBottomPanel usage as it's part of the existing egui code.
    #[allow(deprecated)]
    egui::TopBottomPanel::top("top_navbar")
        .exact_height(40.0)
        .show(ctx, |ui| {
            // Horizontally center the taskbar elements.
            ui.horizontal_centered(|ui| {
                // Render the system logo text.
                ui.heading("AHP System Logo");
                
                // Add a vertical separator.
                ui.separator();
                
                // Render navigation links (dummy state for now).
                ui.selectable_label(true, "🏠 Home");
                ui.selectable_label(false, "📋 Tasks");
                ui.selectable_label(false, "👤 Profile");
                ui.selectable_label(false, "⚙ Settings");
                
                // Add another separator.
                ui.separator();

                // If the user is not an admin, show the Task List button.
                if !auth_state.is_admin {
                    // Toggle the visibility of the task list when the button is clicked.
                    if ui.button("📝 Task List (2)").clicked() {
                        *show_task_list = !*show_task_list;
                    }
                }

                // Align the remaining elements to the right side of the taskbar.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Render the logout button displaying the logged-in user's email.
                    if ui
                        .button(format!(
                            "🚪 Logout ({})",
                            auth_state.logged_in_email.as_deref().unwrap_or("user")
                        ))
                        .clicked()
                    {
                        // Clear the authentication state on logout.
                        auth_state.jwt_token = None;
                        auth_state.logged_in_email = None;
                        auth_state.is_admin = false;
                        // Close the admin panel.
                        admin_state.is_open = false;
                    }
                    
                    // Add a separator before the logout button (renders right-to-left).
                    ui.separator();

                    // Render a placeholder system time label.
                    ui.label("System Time: 12:00 PM");

                    // Add another separator.
                    ui.separator();

                    // Handle the Zoom Feature.
                    // Retrieve the current zoom scale or default to 1.25.
                    let mut current_zoom = config.zoom_scale.unwrap_or(1.25);
                    let mut zoom_changed = false;

                    // Display a menu button showing the current zoom percentage.
                    ui.menu_button(
                        format!("🔍 {}%", (current_zoom * 100.0_f32).round()),
                        |ui| {
                            // Define available zoom levels.
                            let zoom_levels = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0];
                            
                            // Render selectable options for each zoom level.
                            for &level in zoom_levels.iter() {
                                if ui
                                    .selectable_value(
                                        &mut current_zoom,
                                        level,
                                        format!("{}%", (level * 100.0_f32).round()),
                                    )
                                    .changed()
                                {
                                    // Mark that the zoom level was changed and close the menu.
                                    zoom_changed = true;
                                    ui.close_menu();
                                }
                            }
                            
                            // Add a separator before the reset option.
                            ui.separator();
                            
                            // Render a button to reset the zoom to the default 125%.
                            if ui.button("Reset (125%)").clicked() {
                                current_zoom = 1.25;
                                zoom_changed = true;
                                ui.close_menu();
                            }
                        },
                    );

                    // If the zoom level was changed, update the config and save it.
                    if zoom_changed {
                        config.zoom_scale = Some(current_zoom);
                        config.save();
                    }
                });
            });
        });
}
