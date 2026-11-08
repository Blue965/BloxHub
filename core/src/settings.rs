//! Réglages de l'application, persistés dans le dossier de config utilisateur.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_CHAT_URL: &str = "ws://127.0.0.1:8787";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppSettings {
    /// Langue de l'interface : "fr" ou "en".
    pub language: String,
    /// URL du serveur de chat (WebSocket).
    pub chat_server_url: String,
    /// Pseudo utilisé dans le chat.
    pub chat_username: String,
    /// Se connecter automatiquement au chat au démarrage.
    pub auto_connect_chat: bool,
    /// Chemin personnalisé vers Roblox (Linux/Wine, installations non standard).
    pub roblox_custom_path: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: "fr".to_string(),
            chat_server_url: DEFAULT_CHAT_URL.to_string(),
            chat_username: "Joueur".to_string(),
            auto_connect_chat: true,
            roblox_custom_path: None,
        }
    }
}

/// Chemin du fichier de réglages : `<config>/bloxhub/settings.json`.
pub fn settings_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("bloxhub").join("settings.json"))
}

/// Charge les réglages ; retourne les défauts si le fichier est absent/corrompu.
pub fn load() -> AppSettings {
    let Some(path) = settings_path() else {
        return AppSettings::default();
    };
    match std::fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str::<AppSettings>(&raw).unwrap_or_default(),
        Err(_) => AppSettings::default(),
    }
}

/// Sauvegarde les réglages sur disque.
pub fn save(settings: &AppSettings) -> Result<(), String> {
    let path = settings_path().ok_or_else(|| "Dossier de configuration introuvable".to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let raw = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, raw).map_err(|e| e.to_string())
}
