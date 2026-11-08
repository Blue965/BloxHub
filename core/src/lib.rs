//! BloxHub — cœur applicatif (indépendant de Tauri).
//!
//! Ce crate contient toute la logique métier :
//! - `settings`  : réglages de l'app (langue, chat, chemins)
//! - `fflags`    : détection de Roblox, lecture/écriture des FastFlags, catalogue, presets
//! - `launcher`  : lancement de Roblox, détection de processus
//! - `rbxapi`    : client de l'API publique Roblox (profils, avatars, amis)
//! - `chat`      : client WebSocket pour le chat communautaire

pub mod chat;
pub mod fflags;
pub mod launcher;
pub mod rbxapi;
pub mod settings;

pub use settings::AppSettings;
