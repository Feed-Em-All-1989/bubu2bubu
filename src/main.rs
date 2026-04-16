#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bubu2bubu::chat::session::ChatSession;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::State;

struct AppState {
    session: Arc<Mutex<ChatSession>>,
}

#[tauri::command]
async fn get_public_key(state: State<'_, AppState>) -> Result<String, String> {
    let session = state.session.lock().await;
    Ok(session.public_key_hex())
}

#[tauri::command]
async fn set_username(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let mut session = state.session.lock().await;
    session.set_username(name);
    Ok(())
}

#[tauri::command]
async fn connect_to_server(state: State<'_, AppState>, addr: String) -> Result<(), String> {
    let mut session = state.session.lock().await;
    session.connect(&addr).await
}

#[tauri::command]
async fn send_message(
    state: State<'_, AppState>,
    text: String,
    reply_to: Option<String>,
) -> Result<bubu2bubu::chat::session::ChatMessage, String> {
    let mut session = state.session.lock().await;
    session.send(&text, reply_to).await
}

#[tauri::command]
async fn recv_message(
    state: State<'_, AppState>,
) -> Result<bubu2bubu::chat::session::ChatMessage, String> {
    let mut session = state.session.lock().await;
    session.recv()
}

#[tauri::command]
async fn get_history(
    state: State<'_, AppState>,
) -> Result<Vec<bubu2bubu::chat::session::ChatMessage>, String> {
    let session = state.session.lock().await;
    Ok(session.history().to_vec())
}

fn main() {
    let state = AppState {
        session: Arc::new(Mutex::new(ChatSession::new())),
    };

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            get_public_key,
            set_username,
            connect_to_server,
            send_message,
            recv_message,
            get_history,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run app");
}
