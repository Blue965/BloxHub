//! BloxHub — coquille Tauri 2.
//!
//! Toute la logique vit dans `bloxhub-core` ; ce crate expose les commandes
//! `invoke()` à l'interface web et relaie les événements du chat.

use bloxhub_core::{chat, fflags, launcher, rbxapi};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

/// État partagé de l'app.
pub struct AppState {
    chat: Mutex<Option<chat::ChatHandle>>,
    rbx: rbxapi::RbxClient,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            chat: Mutex::new(None),
            rbx: rbxapi::RbxClient::new(),
        }
    }
}

fn custom_path(s: &bloxhub_core::AppSettings) -> Option<&str> {
    s.roblox_custom_path
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
}

fn load_settings() -> bloxhub_core::AppSettings {
    bloxhub_core::settings::load()
}

// ---------------------------------------------------------------------------
// Réglages
// ---------------------------------------------------------------------------

#[tauri::command]
async fn get_settings() -> bloxhub_core::AppSettings {
    load_settings()
}

#[tauri::command]
async fn save_settings(settings: bloxhub_core::AppSettings) -> Result<(), String> {
    bloxhub_core::settings::save(&settings)
}

// ---------------------------------------------------------------------------
// Roblox : détection, lanceur
// ---------------------------------------------------------------------------

#[tauri::command]
async fn detect_roblox() -> fflags::RobloxInstall {
    let s = load_settings();
    fflags::detect_roblox(custom_path(&s))
}

#[tauri::command]
async fn roblox_status() -> launcher::LaunchInfo {
    let s = load_settings();
    launcher::status(custom_path(&s))
}

#[tauri::command]
async fn launch_roblox(place_id: Option<u64>) -> Result<String, String> {
    let s = load_settings();
    launcher::launch(place_id, custom_path(&s))
}

// ---------------------------------------------------------------------------
// FastFlags
// ---------------------------------------------------------------------------

#[tauri::command]
async fn get_flags() -> Result<Value, String> {
    let s = load_settings();
    let file = fflags::resolve_settings_file(custom_path(&s))?;
    let flags = fflags::read_flags(&file)?;
    Ok(json!({ "file": file, "flags": flags }))
}

#[tauri::command]
async fn apply_flags(flags: BTreeMap<String, Value>) -> Result<Value, String> {
    let s = load_settings();
    let file = fflags::resolve_settings_file(custom_path(&s))?;
    let previous_count = fflags::read_flags(&file)?.len();
    fflags::write_flags(&file, &flags)?;
    Ok(json!({
        "applied": flags.len(),
        "previous": previous_count,
        "file": file,
        "backup": true
    }))
}

#[tauri::command]
async fn clear_flags() -> Result<Value, String> {
    let s = load_settings();
    let file = fflags::resolve_settings_file(custom_path(&s))?;
    let previous_count = fflags::read_flags(&file)?.len();
    fflags::write_flags(&file, &BTreeMap::new())?;
    Ok(json!({ "applied": 0, "previous": previous_count, "file": file }))
}

/// Exporte les flags actuels vers `<config>/bloxhub/exports/flags-<ts>.json`.
#[tauri::command]
async fn export_flags() -> Result<Value, String> {
    let s = load_settings();
    let file = fflags::resolve_settings_file(custom_path(&s))?;
    let flags = fflags::read_flags(&file)?;
    let base = dirs::config_dir()
        .ok_or_else(|| "Dossier de configuration introuvable".to_string())?
        .join("bloxhub")
        .join("exports");
    std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let dest = base.join(format!("flags-{ts}.json"));
    let pretty =
        serde_json::to_string_pretty(&flags).map_err(|e| e.to_string())?;
    std::fs::write(&dest, pretty).map_err(|e| e.to_string())?;
    Ok(json!({ "path": dest.to_string_lossy(), "count": flags.len() }))
}

/// Importe un JSON de flags : `{"FFlag…": value}`.
/// `mode` = "merge" (fusionne avec l'existant) ou "replace".
#[tauri::command]
async fn import_flags(payload: String, mode: String) -> Result<Value, String> {
    let s = load_settings();
    let file = fflags::resolve_settings_file(custom_path(&s))?;
    let incoming: BTreeMap<String, Value> = serde_json::from_str(&payload)
        .map_err(|e| format!("JSON invalide : {e}"))?;
    let mut merged = match mode.as_str() {
        "replace" => BTreeMap::new(),
        _ => fflags::read_flags(&file)?,
    };
    let imported = incoming.len();
    for (k, v) in incoming {
        merged.insert(k, v);
    }
    fflags::write_flags(&file, &merged)?;
    Ok(json!({ "imported": imported, "total": merged.len() }))
}

#[tauri::command]
async fn flag_catalog() -> Value {
    json!(fflags::catalog())
}

#[tauri::command]
async fn flag_presets() -> Value {
    json!(fflags::presets())
}

// ---------------------------------------------------------------------------
// Profil Roblox (API publique)
// ---------------------------------------------------------------------------

#[tauri::command]
async fn roblox_profile(
    state: State<'_, AppState>,
    username: String,
) -> Result<Value, String> {
    let name = username.trim();
    if name.is_empty() {
        return Err("Entre un nom d'utilisateur Roblox.".into());
    }
    state.rbx.profile_bundle(name).await
}

// ---------------------------------------------------------------------------
// Chat
// ---------------------------------------------------------------------------

#[tauri::command]
async fn chat_state(state: State<'_, AppState>) -> Result<Value, String> {
    let guard = state.chat.lock().await;
    Ok(json!({ "connected": guard.as_ref().map(|h| h.is_connected()).unwrap_or(false) }))
}

#[tauri::command]
async fn chat_connect(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.chat.lock().await;
    // Coupe proprement tout ancien client (évite deux tâches de reconnexion en parallèle).
    if let Some(old) = guard.take() {
        old.disconnect();
    }
    let s = load_settings();
    let (handle, mut ev_rx) =
        chat::spawn_client(s.chat_server_url.clone(), s.chat_username.clone());

    // Relais des événements serveur → UI.
    let relay_app = app.clone();
    tokio::spawn(async move {
        while let Some(ev) = ev_rx.recv().await {
            let _ = relay_app.emit("chat-event", ev);
        }
    });

    *guard = Some(handle);
    Ok(())
}

#[tauri::command]
async fn chat_disconnect(state: State<'_, AppState>) -> Result<(), String> {
    if let Some(h) = state.chat.lock().await.take() {
        h.disconnect();
    }
    Ok(())
}

async fn with_chat<T>(
    state: &State<'_, AppState>,
    f: impl FnOnce(&chat::ChatHandle) -> Option<T>,
) -> Result<T, String> {
    let guard = state.chat.lock().await;
    let h = guard.as_ref().ok_or("Chat non connecté.")?;
    f(h).ok_or_else(|| "Chat déconnecté.".to_string())
}

#[tauri::command]
async fn chat_send(room: String, content: String, state: State<'_, AppState>) -> Result<(), String> {
    with_chat(&state, |h| h.send(chat::msg_message(&room, &content)).then_some(())).await
}

#[tauri::command]
async fn chat_dm(to: String, content: String, state: State<'_, AppState>) -> Result<(), String> {
    with_chat(&state, |h| h.send(chat::msg_dm(&to, &content)).then_some(())).await
}

#[tauri::command]
async fn chat_join(room: String, state: State<'_, AppState>) -> Result<(), String> {
    with_chat(&state, |h| h.send(chat::msg_join(&room)).then_some(())).await
}

#[tauri::command]
async fn chat_leave(room: String, state: State<'_, AppState>) -> Result<(), String> {
    with_chat(&state, |h| h.send(chat::msg_leave(&room)).then_some(())).await
}

#[tauri::command]
async fn chat_typing(room: String, state: State<'_, AppState>) -> Result<(), String> {
    with_chat(&state, |h| h.send(chat::msg_typing(&room)).then_some(())).await
}

// ---------------------------------------------------------------------------
// Démarrage
// ---------------------------------------------------------------------------

pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            detect_roblox,
            roblox_status,
            launch_roblox,
            get_flags,
            apply_flags,
            clear_flags,
            export_flags,
            import_flags,
            flag_catalog,
            flag_presets,
            roblox_profile,
            chat_state,
            chat_connect,
            chat_disconnect,
            chat_send,
            chat_dm,
            chat_join,
            chat_leave,
            chat_typing
        ])
        .run(tauri::generate_context!())
        .expect("erreur pendant l'exécution de BloxHub");
}
