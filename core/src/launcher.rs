//! Lanceur Roblox : ouverture du jeu via le protocole `roblox://` ou l'exécutable,
//! et détection d'un process Roblox en cours.

use crate::fflags::{detect_roblox, RobloxInstall};
use serde::Serialize;
use std::process::Command;
#[cfg(all(unix, not(target_os = "macos")))]
use std::process::Stdio;

#[derive(Debug, Clone, Serialize)]
pub struct LaunchInfo {
    pub installed: bool,
    pub player_path: Option<String>,
    pub version_dir: Option<String>,
    pub running: bool,
}

/// Ouvre une URL avec le gestionnaire par défaut (protocole `roblox://`).
fn open_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", url])
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
            .output()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(url)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// True si un process Roblox tourne actuellement.
pub fn is_running() -> bool {
    #[cfg(target_os = "windows")]
    {
        Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq RobloxPlayerBeta.exe", "/NH"])
            .creation_flags(0x0800_0000)
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("RobloxPlayerBeta"))
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("pgrep")
            .args(["-f", "RobloxPlayerBeta"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

/// Statut global : install + process.
pub fn status(custom_path: Option<&str>) -> LaunchInfo {
    let install: RobloxInstall = detect_roblox(custom_path);
    LaunchInfo {
        installed: install.installed,
        player_path: install.player_path.clone(),
        version_dir: install.version_dir.clone(),
        running: is_running(),
    }
}

/// Lance Roblox.
/// - avec `place_id` : protocole `roblox://experiences/start?placeId=…`
/// - sans : exécution directe du player (page d'accueil) si trouvé.
pub fn launch(place_id: Option<u64>, custom_path: Option<&str>) -> Result<String, String> {
    if let Some(pid) = place_id {
        let url = format!("roblox://experiences/start?placeId={pid}");
        open_url(&url)?;
        return Ok(format!("roblox://experiences/start?placeId={pid}"));
    }

    let install = detect_roblox(custom_path);
    if let Some(exe) = install.player_path {
        #[cfg(target_os = "windows")]
        {
            Command::new(&exe)
                .creation_flags(0x0800_0000)
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        #[cfg(target_os = "macos")]
        {
            Command::new("open").arg(&exe).spawn().map_err(|e| e.to_string())?;
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            Command::new(&exe)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| format!("Impossible de lancer {exe} (sous Linux utilise le protocole roblox:// ou Wine) : {e}"))?;
        }
        Ok(exe)
    } else {
        Err("Roblox introuvable — installe Roblox ou définis un chemin personnalisé dans les paramètres.".to_string())
    }
}

#[cfg(target_os = "windows")]
trait WinCreationFlags {
    fn creation_flags(&mut self, flags: u32) -> &mut Command;
}
#[cfg(target_os = "windows")]
impl WinCreationFlags for Command {
    fn creation_flags(&mut self, flags: u32) -> &mut Command {
        std::os::windows::process::CommandExt::creation_flags(self, flags)
    }
}
