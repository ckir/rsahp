use crate::config::AppConfig;

pub fn render(ctx: &egui::Context, show_task_list: &mut bool, config: &mut AppConfig) {
    #[allow(deprecated)]
    egui::TopBottomPanel::bottom("taskbar")
        .exact_height(40.0)
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.heading("AHP System");
                ui.separator();
                
                if ui.button("📝 Task List (2)").clicked() {
                    *show_task_list = !*show_task_list;
                }
                
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("System Time: 12:00 PM"); // Placeholder

                    ui.separator();

                    // Zoom Feature
                    let mut current_zoom = config.zoom_scale.unwrap_or(1.25);
                    let mut zoom_changed = false;
                    
                    ui.menu_button(format!("🔍 {}%", (current_zoom * 100.0).round()), |ui| {
                        let zoom_levels = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0];
                        for &level in zoom_levels.iter() {
                            if ui.selectable_value(&mut current_zoom, level, format!("{}%", (level * 100.0).round())).changed() {
                                zoom_changed = true;
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        if ui.button("Reset (125%)").clicked() {
                            current_zoom = 1.25;
                            zoom_changed = true;
                            ui.close_menu();
                        }
                    });

                    if zoom_changed {
                        config.zoom_scale = Some(current_zoom);
                        config.save();
                    }
                });
            });
        });
}
