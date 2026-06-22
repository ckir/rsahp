//! This module provides the authentication UI and logic, including login and registration.

use eframe::egui;
use serde::{Deserialize, Serialize};

/// UI state for the authentication modal.
pub struct AuthState {
    /// The current text in the email input field.
    pub email_input: String,
    /// The current text in the password input field.
    pub password_input: String,
    /// Indicates whether the modal is in registration mode.
    pub is_registering: bool,
    /// An optional error message to display to the user.
    pub error_msg: Option<String>,
    /// The JSON Web Token obtained after a successful login.
    pub jwt_token: Option<String>,
    /// The email address of the currently logged-in user.
    pub logged_in_email: Option<String>,
    /// The database ID of the currently logged-in user.
    pub logged_in_user_id: Option<i32>,
    /// Indicates whether the logged-in user has admin privileges.
    pub is_admin: bool,
    /// Indicates if an authentication network request is currently in progress.
    pub auth_request_in_progress: bool,
    /// Receiver channel for handling the asynchronous authentication response.
    pub rx: Option<std::sync::mpsc::Receiver<Result<AuthResponse, String>>>,
}

/// Default implementation for `AuthState`.
impl Default for AuthState {
    fn default() -> Self {
        Self {
            // Initialize email input as empty.
            email_input: String::new(),
            // Initialize password input as empty.
            password_input: String::new(),
            // Start in login mode, not registration mode.
            is_registering: false,
            // No error message initially.
            error_msg: None,
            // No JWT token initially.
            jwt_token: None,
            // No user logged in initially.
            logged_in_email: None,
            // No user ID initially.
            logged_in_user_id: None,
            // Not an admin initially.
            is_admin: false,
            // No authentication request in progress initially.
            auth_request_in_progress: false,
            // No receiver channel initially.
            rx: None,
        }
    }
}

/// Data transfer object for login/register payloads.
#[derive(Serialize)]
pub struct AuthPayload {
    /// The user's email address.
    pub email: String,
    /// The user's password.
    pub password: String,
}

/// Data transfer object for authentication responses.
#[derive(Deserialize)]
pub struct AuthResponse {
    /// The JSON Web Token provided by the server.
    pub token: String,
    /// Information about the authenticated user.
    pub user: UserDto,
}

/// Data transfer object for basic user info.
#[derive(Deserialize)]
pub struct UserDto {
    /// The user's unique identifier.
    pub id: i32,
    /// The user's email address.
    pub email: String,
    /// Whether the user has admin privileges.
    pub is_admin: bool,
}

/// Renders the authentication modal (Login/Register).
/// 
/// This function handles the display and logic for user authentication,
/// including switching between login and registration, input fields,
/// submit buttons, error display, and managing the background network request.
pub fn render_login_modal(ctx: &egui::Context, state: &mut AuthState, api_base_url: &str) {
    // If a JWT token is already present, the user is logged in, so we do not render the modal.
    if state.jwt_token.is_some() {
        return; 
    }

    // Check the receiver channel for any results from a background authentication request.
    if let Some(rx) = &state.rx {
        // Attempt to receive a message without blocking.
        if let Ok(res) = rx.try_recv() {
            // Request finished, so clear the in-progress flag.
            state.auth_request_in_progress = false;
            match res {
                // If authentication was successful, update the state with the user's data.
                Ok(auth_res) => {
                    state.jwt_token = Some(auth_res.token);
                    state.logged_in_email = Some(auth_res.user.email);
                    state.logged_in_user_id = Some(auth_res.user.id);
                    state.is_admin = auth_res.user.is_admin;
                    state.error_msg = None;
                }
                // If authentication failed, update the state with the error message.
                Err(e) => {
                    state.error_msg = Some(e);
                }
            }
        }
    }

    // Define the window to be open.
    let mut is_open = true;
    
    // Create a new modal window for authentication.
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
        // Render the email input field.
        ui.horizontal(|ui| {
            ui.label("Email:");
            ui.text_edit_singleline(&mut state.email_input);
        });
        
        // Render the password input field, masking the input.
        ui.horizontal(|ui| {
            ui.label("Password:");
            ui.add(egui::TextEdit::singleline(&mut state.password_input).password(true));
        });

        // Display any existing error message in red.
        if let Some(err) = &state.error_msg {
            ui.colored_label(egui::Color32::RED, err);
        }

        // Add some spacing before the buttons.
        ui.add_space(10.0);

        // Render the action buttons and spinner.
        ui.horizontal(|ui| {
            // If a request is in progress, show a spinner instead of the submit button.
            if state.auth_request_in_progress {
                ui.spinner();
                ui.label("Please wait...");
            } else {
                // Determine the text for the submit button based on the mode.
                let btn_text = if state.is_registering {
                    "Register"
                } else {
                    "Login"
                };
                
                // Render the submit button and handle clicks.
                if ui.button(btn_text).clicked() {
                    // Determine the API endpoint based on the mode.
                    let endpoint = if state.is_registering {
                        "/register"
                    } else {
                        "/login"
                    };
                    
                    // Construct the full URL, removing "/documents" if it was present in the base URL.
                    let url = format!(
                        "{}/auth{}",
                        api_base_url.replace("/documents", ""),
                        endpoint
                    ); // Very hacky url handling for now

                    // Create the authentication payload from the user inputs.
                    let payload = AuthPayload {
                        email: state.email_input.clone(),
                        // We aren't actually hashing on client side in this simple example.
                        // Wait, the backend uses `bcrypt::verify(password, hash)` so backend expects plain text!
                        password: state.password_input.clone(),
                    };

                    // Attempt to serialize the payload to JSON.
                    if let Ok(body) = serde_json::to_vec(&payload) {
                        // Create a POST request with the JSON body.
                        let mut request = ehttp::Request::post(url, body);
                        
                        // Remove any existing Content-Type headers.
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
                            
                        // Insert the correct Content-Type header.
                        request.headers.insert("Content-Type", "application/json");
                        
                        // Create a channel for receiving the background request result.
                        let (tx, rx) = std::sync::mpsc::channel();
                        
                        // Store the receiver in the state.
                        state.rx = Some(rx);
                        // Mark the request as in progress.
                        state.auth_request_in_progress = true;

                        // Start the background network request.
                        ehttp::fetch(request, move |result| {
                            let res = match result {
                                Ok(response) => {
                                    // Handle a successful HTTP 200 response.
                                    if response.status == 200 {
                                        if let Some(text) = response.text() {
                                            serde_json::from_str::<AuthResponse>(text)
                                                .map_err(|e| format!("Parse error: {}", e))
                                        } else {
                                            Err("No response body".to_string())
                                        }
                                    // Handle HTTP 401 Unauthorized errors specifically.
                                    } else if response.status == 401 {
                                        Err("Invalid username or password".to_string())
                                    // Handle other HTTP errors generically.
                                    } else {
                                        Err(format!(
                                            "Error {}: {:?}",
                                            response.status,
                                            response.text()
                                        ))
                                    }
                                }
                                // Handle lower-level network errors.
                                Err(e) => Err(e),
                            };
                            // Send the parsed result back through the channel.
                            let _ = tx.send(res);
                        });
                    }
                }

                // Render the toggle button to switch between login and registration.
                let toggle_text = if state.is_registering {
                    "Switch to Login"
                } else {
                    "Create Account"
                };
                if ui.button(toggle_text).clicked() {
                    // Toggle the registration flag.
                    state.is_registering = !state.is_registering;
                    // Clear any existing error messages.
                    state.error_msg = None;
                }
            }
        });
    });
}
