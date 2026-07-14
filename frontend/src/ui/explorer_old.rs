// SPDX-License-Identifier: LicenseRef-PolyForm-Noncommercial-1.0.0
//! Module explorer_old.rs
to include a line number before every line, in the format: <line_number>: <original_line>. Please note that any changes targeting the original code should remove the line number, colon, and leading space.
use eframe::egui;
use super::document_window::DocumentState;

#[derive(Default)]
/// Documentation for ExplorerState.
pub struct ExplorerState {
    pub import_status: Option<String>,
}

/// Documentation for render.
pub fn render(ctx: &egui::Context, state: &mut ExplorerState, open_documents: &mut Vec<DocumentState>, api_url: &str) {
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
                        request.headers.headers.retain(|(k, _)| k.to_lowercase() != "content-type");
                        request.headers.headers.retain(|(k, _)| k.to_lowercase() != "content-type");
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
            
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.collapsing("My AHP Documents", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("📄 Q3 Marketing Focus");
                        if ui.button("Open").clicked() {
                            open_documents.push(DocumentState::new(1, "Q3 Marketing Focus"));
                        }
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("📄 Vendor Selection");
                        if ui.button("Open").clicked() {
                            open_documents.push(DocumentState::new(2, "Vendor Selection"));
                        }
                    });
                });
            });
            
            // Dummy right-click context menu via UI hinting
            ui.separator();
            ui.label("Right-click items for: New, Duplicate, Delete");
        });
}