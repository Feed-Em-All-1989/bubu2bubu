#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bubu2bubu::chat::session::ChatSession;
use bubu2bubu::chat::profile::PeerProfile;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{State, Manager};

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
async fn connect_to_server(
    state: State<'_, AppState>,
    addr: String,
) -> Result<String, String> {
    let mut session = state.session.lock().await;
    session.connect(&addr).await
}

#[tauri::command]
async fn reconnect(state: State<'_, AppState>) -> Result<String, String> {
    let mut session = state.session.lock().await;
    session.reconnect().await
}

#[tauri::command]
async fn set_encryption_key(
    state: State<'_, AppState>,
    key: String,
) -> Result<(), String> {
    let mut session = state.session.lock().await;
    session.set_encryption_key(key);
    Ok(())
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

#[tauri::command]
async fn get_online_users(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let session = state.session.lock().await;
    Ok(session.online_users().to_vec())
}

#[tauri::command]
async fn get_key_tag(state: State<'_, AppState>) -> Result<String, String> {
    let session = state.session.lock().await;
    Ok(session.key_tag().to_string())
}

#[tauri::command]
async fn get_saved_username(state: State<'_, AppState>) -> Result<String, String> {
    let session = state.session.lock().await;
    Ok(session.username().to_string())
}

#[tauri::command]
async fn get_avatar(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let session = state.session.lock().await;
    Ok(session.avatar().clone())
}

#[tauri::command]
async fn set_avatar(state: State<'_, AppState>, data: String) -> Result<String, String> {
    let sanitized = tokio::task::spawn_blocking(move || {
        bubu2bubu::chat::profile::validate_avatar(&data)
    }).await.map_err(|e| e.to_string())??;
    let mut session = state.session.lock().await;
    session.set_avatar_validated(sanitized.clone()).await?;
    Ok(sanitized)
}

#[tauri::command]
async fn update_username(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let mut session = state.session.lock().await;
    session.update_username(name).await
}

#[tauri::command]
async fn get_peer_profiles(state: State<'_, AppState>) -> Result<Vec<PeerProfile>, String> {
    let session = state.session.lock().await;
    Ok(session.peer_profiles())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir().ok();
            let session = ChatSession::new(data_dir);
            app.manage(AppState {
                session: Arc::new(Mutex::new(session)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_public_key,
            set_username,
            connect_to_server,
            reconnect,
            set_encryption_key,
            send_message,
            recv_message,
            get_history,
            get_online_users,
            get_key_tag,
            get_saved_username,
            get_avatar,
            set_avatar,
            get_peer_profiles,
            update_username,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run app");
}
