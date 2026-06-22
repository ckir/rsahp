use eframe::egui;

mod document_window;
mod explorer;
mod taskbar;

pub struct RsahpApp {
    show_task_list: bool,
    open_documents: Vec<document_window::DocumentState>,
    explorer_state: explorer::ExplorerState,
    config: crate::config::AppConfig,
}

impl RsahpApp {
    pub fn new(config: crate::config::AppConfig) -> Self {
        Self {
            show_task_list: false,
            open_documents: Vec::new(),
            explorer_state: Default::default(),
            config,
        }
    }
}

impl eframe::App for RsahpApp {
    fn ui(&mut self, _ui: &mut egui::Ui, _frame: &mut eframe::Frame) {}

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(scale) = self.config.zoom_scale {
            ctx.set_pixels_per_point(scale);
        }

        let api_url = self.config.api_url.clone().unwrap_or_else(|| "http://127.0.0.1:3002/api/documents".to_string());

        // Render Bottom Taskbar
        taskbar::render(ctx, &mut self.show_task_list, &mut self.config);

        // Render Pinned Explorer
        explorer::render(ctx, &mut self.explorer_state, &mut self.open_documents, &api_url);

        // Render Open Document Windows
        let mut closed_docs = Vec::new();
        for (idx, doc) in self.open_documents.iter_mut().enumerate() {
            let mut is_open = true;
            if !doc.close_requested {
                egui::Window::new(&doc.title)
                    .id(egui::Id::new(doc.id))
                    .open(&mut is_open)
                    .vscroll(true)
                    .default_size(egui::vec2(1000.0, 700.0))
                    .default_pos(ctx.screen_rect().center())
                    .pivot(egui::Align2::CENTER_CENTER)
                    .show(ctx, |ui| {
                        document_window::render(ui, doc, &api_url);
                    });

                if !is_open {
                    if doc.is_modified {
                        doc.close_requested = true;
                    } else {
                        closed_docs.push(idx);
                    }
                }
                
                if let Some(rx) = &doc.duplicated_doc_rx {
                    if let Ok(new_doc) = rx.try_recv() {
                        if let explorer::Node::Directory(dir) = &mut self.explorer_state.tree {
                            dir.children.push(explorer::Node::File(explorer::File {
                                id: self.explorer_state.next_id,
                                name: new_doc.name.clone(),
                                document_id: Some(new_doc.id as usize),
                            }));
                            self.explorer_state.next_id += 1;
                        }
                        doc.id = new_doc.id;
                        doc.title = new_doc.name;
                        doc.version = new_doc.version;
                        doc.save_status = Some(format!("✅ Duplicated! (v{})", doc.version));
                        doc.duplicated_doc_rx = None;
                    }
                }
            } else {
                let mut modal_open = true;
                let mut action = None;
                egui::Window::new("Unsaved Changes")
                    .id(egui::Id::new("close_modal").with(doc.id))
                    .collapsible(false)
                    .resizable(false)
                    .open(&mut modal_open)
                    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                    .show(ctx, |ui| {
                        ui.label(format!("Save changes to '{}' before closing?", doc.title));
                        ui.horizontal(|ui| {
                            if ui.button("Save").clicked() {
                                action = Some("save");
                            }
                            if ui.button("Don't Save").clicked() {
                                action = Some("discard");
                            }
                            if ui.button("Cancel").clicked() {
                                action = Some("cancel");
                            }
                        });
                    });
                
                if !modal_open {
                    doc.close_requested = false;
                }
                match action {
                    Some("save") => {
                        document_window::save_document(doc, &api_url, ctx);
                        closed_docs.push(idx);
                    }
                    Some("discard") => {
                        closed_docs.push(idx);
                    }
                    Some("cancel") => {
                        doc.close_requested = false;
                    }
                    _ => {}
                }
            }
        }

        // Clean up closed windows
        for idx in closed_docs.into_iter().rev() {
            self.open_documents.remove(idx);
        }

        // Render Task List Modal
        let mut show_task_list = self.show_task_list;
        let mut new_doc = None;
        if show_task_list {
            egui::Window::new("Task List")
                .open(&mut show_task_list)
                .show(ctx, |ui| {
                    ui.label("You have 2 pending AHP surveys.");
                    if ui.button("Survey: Vendor Selection (Management Group)").clicked() {
                        new_doc = Some(document_window::DocumentState::new(101, "Vendor Selection Survey"));
                    }
                });
        }
        self.show_task_list = show_task_list;
        if let Some(doc) = new_doc {
            self.open_documents.push(doc);
            self.show_task_list = false;
        }
    }
}
