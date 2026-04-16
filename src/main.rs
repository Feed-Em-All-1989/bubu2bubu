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
async fn host_session(state: State<'_, AppState>, port: u16) -> Result<(), String> {
    let mut session = state.session.lock().await;
    session.host(port).await
}

#[tauri::command]
async fn join_session(state: State<'_, AppState>, addr: String) -> Result<(), String> {
    let mut session = state.session.lock().await;
    session.join(&addr).await
}

#[tauri::command]
async fn send_message(state: State<'_, AppState>, text: String) -> Result<(), String> {
    let mut session = state.session.lock().await;
    session.send(&text).await
}

#[tauri::command]
async fn recv_message(
    state: State<'_, AppState>,
) -> Result<bubu2bubu::chat::session::ChatMessage, String> {
    let mut session = state.session.lock().await;
    session.recv().await
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
            host_session,
            join_session,
            send_message,
            recv_message,
            get_history,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run app");
}
