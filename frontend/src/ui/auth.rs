use eframe::egui;
use serde::{Deserialize, Serialize};

/// UI state for the authentication modal.
pub struct AuthState {
    pub email_input: String,
    pub password_input: String,
    pub is_registering: bool,
    pub error_msg: Option<String>,
    pub jwt_token: Option<String>,
    pub logged_in_email: Option<String>,
    pub logged_in_user_id: Option<i32>,
    pub is_admin: bool,
    // communication channels
    pub auth_request_in_progress: bool,
    pub rx: Option<std::sync::mpsc::Receiver<Result<AuthResponse, String>>>,
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            email_input: String::new(),
            password_input: String::new(),
            is_registering: false,
            error_msg: None,
            jwt_token: None,
            logged_in_email: None,
            logged_in_user_id: None,
            is_admin: false,
            auth_request_in_progress: false,
            rx: None,
        }
    }
}

/// Data transfer object for login/register payloads.
#[derive(Serialize)]
pub struct AuthPayload {
    pub email: String,
    pub password: String,
}

/// Data transfer object for authentication responses.
#[derive(Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserDto,
}

/// Data transfer object for basic user info.
#[derive(Deserialize)]
pub struct UserDto {
    pub id: i32,
    pub email: String,
    pub is_admin: bool,
}

/// Renders the authentication modal (Login/Register).
pub fn render_login_modal(ctx: &egui::Context, state: &mut AuthState, api_base_url: &str) {
    if state.jwt_token.is_some() {
        return; // Already logged in
    }

    // Check rx for results
    if let Some(rx) = &state.rx {
        if let Ok(res) = rx.try_recv() {
            state.auth_request_in_progress = false;
            match res {
                Ok(auth_res) => {
                    state.jwt_token = Some(auth_res.token);
                    state.logged_in_email = Some(auth_res.user.email);
                    state.logged_in_user_id = Some(auth_res.user.id);
                    state.is_admin = auth_res.user.is_admin;
                    state.error_msg = None;
                }
                Err(e) => {
                    state.error_msg = Some(e);
                }
            }
        }
    }

    let mut is_open = true;
    egui::Window::new(if state.is_registering {
        "Create Account"
    } else {
        "Login"
    })
    .id(egui::Id::new("auth_modal"))
    .collapsible(false)
    .resizable(false)
    .open(&mut is_open)
    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
    .show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label("Email:");
            ui.text_edit_singleline(&mut state.email_input);
        });
        ui.horizontal(|ui| {
            ui.label("Password:");
            ui.add(egui::TextEdit::singleline(&mut state.password_input).password(true));
        });

        if let Some(err) = &state.error_msg {
            ui.colored_label(egui::Color32::RED, err);
        }

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            if state.auth_request_in_progress {
                ui.spinner();
                ui.label("Please wait...");
            } else {
                let btn_text = if state.is_registering {
                    "Register"
                } else {
                    "Login"
                };
                if ui.button(btn_text).clicked() {
                    let endpoint = if state.is_registering {
                        "/register"
                    } else {
                        "/login"
                    };
                    let url = format!(
                        "{}/auth{}",
                        api_base_url.replace("/documents", ""),
                        endpoint
                    ); // Very hacky url handling for now

                    let payload = AuthPayload {
                        email: state.email_input.clone(),
                        // We aren't actually hashing on client side in this simple example.
                        // Wait, the backend uses `bcrypt::verify(password, hash)` so backend expects plain text!
                        password: state.password_input.clone(),
                    };

                    if let Ok(body) = serde_json::to_vec(&payload) {
                        let mut request = ehttp::Request::post(url, body);
                        request
                            .headers
                            .headers
                            .retain(|(k, _)| k.to_lowercase() != "content-type");
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
                        state.rx = Some(rx);
                        state.auth_request_in_progress = true;

                        ehttp::fetch(request, move |result| {
                            let res = match result {
                                Ok(response) => {
                                    if response.status == 200 {
                                        if let Some(text) = response.text() {
                                            serde_json::from_str::<AuthResponse>(text)
                                                .map_err(|e| format!("Parse error: {}", e))
                                        } else {
                                            Err("No response body".to_string())
                                        }
                                    } else if response.status == 401 {
                                        Err("Invalid username or password".to_string())
                                    } else {
                                        Err(format!(
                                            "Error {}: {:?}",
                                            response.status,
                                            response.text()
                                        ))
                                    }
                                }
                                Err(e) => Err(e),
                            };
                            let _ = tx.send(res);
                        });
                    }
                }

                let toggle_text = if state.is_registering {
                    "Switch to Login"
                } else {
                    "Create Account"
                };
                if ui.button(toggle_text).clicked() {
                    state.is_registering = !state.is_registering;
                    state.error_msg = None;
                }
            }
        });
    });
}
