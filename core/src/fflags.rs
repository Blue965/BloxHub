//! Gestion des FastFlags Roblox.
//!
//! Les FastFlags sont stockés dans `ClientSettings/ClientAppSettings.json`,
//! à côté de `RobloxPlayerBeta.exe`. BloxHub écrit ce fichier (avec sauvegarde)
//! et fournit un catalogue de flags connus de la communauté ainsi que des presets.
//!
//! ⚠️ Les FastFlags ne sont pas officiellement supportés par Roblox :
//! un flag peut être ignoré, renommé ou retiré à tout moment.

use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Map triée flag → valeur (les clés JSON seront écrites dans l'ordre alphabétique).
pub type FlagMap = BTreeMap<String, Value>;

/// Informations sur l'installation Roblox détectée.
#[derive(Debug, Clone, Serialize)]
pub struct RobloxInstall {
    /// "windows" | "macos" | "linux"
    pub platform: String,
    pub installed: bool,
    /// Chemin de `RobloxPlayerBeta.exe` (ou binaire équivalent).
    pub player_path: Option<String>,
    /// Chemin du fichier `ClientAppSettings.json` géré par BloxHub.
    pub settings_file: Option<String>,
    /// Nom du dossier de version (`version-xxxxxxxx`).
    pub version_dir: Option<String>,
}

/// Entrée du catalogue de flags connus.
#[derive(Debug, Clone, Serialize)]
pub struct CatalogEntry {
    pub key: String,
    pub value: Value,
    /// Type de valeur attendu : "bool" | "int" | "string".
    pub vtype: String,
    /// Catégorie : performances | graphismes | moteur | reseau | telemetrie | divers.
    pub category: String,
    pub desc_fr: String,
    pub desc_en: String,
}

/// Preset = paquet de flags prêts à appliquer.
#[derive(Debug, Clone, Serialize)]
pub struct Preset {
    pub id: String,
    pub name_fr: String,
    pub name_en: String,
    pub desc_fr: String,
    pub desc_en: String,
    pub flags: FlagMap,
}

// ---------------------------------------------------------------------------
// Détection de Roblox
// ---------------------------------------------------------------------------

fn platform_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

/// Construit le résultat pour une installation trouvée.
fn found(player: PathBuf) -> RobloxInstall {
    let version_dir = player
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string());
    let settings_file = player
        .parent()
        .map(|p| p.join("ClientSettings").join("ClientAppSettings.json"));
    RobloxInstall {
        platform: platform_name().to_string(),
        installed: true,
        player_path: Some(player.to_string_lossy().to_string()),
        settings_file: settings_file.map(|p| p.to_string_lossy().to_string()),
        version_dir,
    }
}

fn not_found() -> RobloxInstall {
    RobloxInstall {
        platform: platform_name().to_string(),
        installed: false,
        player_path: None,
        settings_file: None,
        version_dir: None,
    }
}

/// Vérifie qu'un dossier de version contient bien le player.
fn version_dir_has_player(dir: &Path) -> bool {
    dir.join("RobloxPlayerBeta.exe").is_file()
}

/// Détecte l'installation Roblox. `custom_path` (réglages utilisateur) est
/// consulté en priorité : il peut pointer vers l'exe du player ou son dossier.
pub fn detect_roblox(custom_path: Option<&str>) -> RobloxInstall {
    // 1. Chemin personnalisé (utile sous Linux/Wine ou installation non standard).
    if let Some(custom) = custom_path {
        let p = PathBuf::from(custom);
        if p.is_file() {
            return found(p);
        }
        if p.is_dir() {
            if let Ok(exe) = find_player_in_dir(&p) {
                return found(exe);
            }
        }
    }

    // 2. Détection par OS.
    if cfg!(target_os = "windows") {
        if let Some(local) = dirs::data_local_dir() {
            let versions = local.join("Roblox").join("Versions");
            if let Ok(Some(exe)) = newest_player(&versions) {
                return found(exe);
            }
        }
        return not_found();
    }

    if cfg!(target_os = "macos") {
        let exe = PathBuf::from("/Applications/Roblox.app/Contents/MacOS/RobloxPlayerBeta");
        if exe.is_file() {
            return found(exe);
        }
        return not_found();
    }

    // Linux : installation par défaut inexistante, uniquement le chemin custom.
    not_found()
}

/// Cherche récursivement (profondeur 3) `RobloxPlayerBeta(.exe)` dans un dossier.
fn find_player_in_dir(dir: &Path) -> Result<PathBuf, ()> {
    const NAMES: [&str; 2] = ["RobloxPlayerBeta.exe", "RobloxPlayerBeta"];
    for name in NAMES {
        let cand = dir.join(name);
        if cand.is_file() {
            return Ok(cand);
        }
    }
    let depth = 3;
    let mut stack = vec![(dir.to_path_buf(), 0usize)];
    while let Some((d, lvl)) = stack.pop() {
        if lvl >= depth {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push((p, lvl + 1));
            } else if let Some(fname) = p.file_name().and_then(|n| n.to_str()) {
                if NAMES.contains(&fname) {
                    return Ok(p);
                }
            }
        }
    }
    Err(())
}

/// Trouve le player le plus récent dans `<roblox>/Versions/version-*`.
fn newest_player(versions_dir: &Path) -> Result<Option<PathBuf>, ()> {
    let Ok(entries) = std::fs::read_dir(versions_dir) else {
        return Err(());
    };
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in entries.flatten() {
        let dir = entry.path();
        if dir.is_dir()
            && dir
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("version-"))
                .unwrap_or(false)
            && version_dir_has_player(&dir)
        {
            let mtime = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            if best.as_ref().map(|(t, _)| mtime > *t).unwrap_or(true) {
                best = Some((mtime, dir.join("RobloxPlayerBeta.exe")));
            }
        }
    }
    Ok(best.map(|(_, p)| p))
}

// ---------------------------------------------------------------------------
// Lecture / écriture des flags
// ---------------------------------------------------------------------------

/// Résout le fichier ClientAppSettings.json effectif, ou renvoie une erreur claire.
pub fn resolve_settings_file(custom_path: Option<&str>) -> Result<String, String> {
    let install = detect_roblox(custom_path);
    install
        .settings_file
        .ok_or_else(|| "Roblox introuvable — installe Roblox ou définis un chemin personnalisé dans les paramètres.".to_string())
}

/// Lit les flags actuels. Fichier absent → map vide (Roblox n'a encore aucun flag).
pub fn read_flags(settings_file: &str) -> Result<FlagMap, String> {
    let path = Path::new(settings_file);
    if !path.exists() {
        return Ok(FlagMap::new());
    }
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let parsed: Value = serde_json::from_str(&raw).map_err(|e| format!("JSON invalide : {e}"))?;
    match parsed {
        Value::Object(map) => Ok(map.into_iter().collect()),
        Value::Null => Ok(FlagMap::new()),
        other => Err(format!("Contenu inattendu dans ClientAppSettings.json : {other}")),
    }
}

/// Sauvegarde horodatée : `ClientAppSettings.json.bak-<unix>`.
fn backup_existing(path: &Path) -> Result<(), String> {
    if path.exists() {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let bak = path.with_file_name(format!(
            "ClientAppSettings.json.bak-{ts}"
        ));
        std::fs::copy(path, bak).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Écrit les flags (crée le dossier `ClientSettings`, sauvegarde l'ancien fichier).
pub fn write_flags(settings_file: &str, flags: &FlagMap) -> Result<(), String> {
    let path = Path::new(settings_file);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    backup_existing(path)?;
    // BTreeMap sérialise déjà avec les clés triées → JSON stable et lisible.
    let map: serde_json::Map<String, Value> = flags
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let pretty = serde_json::to_string_pretty(&Value::Object(map)).map_err(|e| e.to_string())?;
    std::fs::write(path, pretty).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Catalogue de flags connus (source : communauté FastFlags / Bloxstrap)
// ---------------------------------------------------------------------------

fn bool_v() -> Value {
    Value::Bool(true)
}
fn int_v(n: i64) -> Value {
    Value::from(n)
}

const CAT_PERF: &str = "performances";
const CAT_GFX: &str = "graphismes";
const CAT_ENGINE: &str = "moteur";
const CAT_NET: &str = "reseau";
const CAT_TELEM: &str = "telemetrie";
const CAT_MISC: &str = "divers";

/// Catalogue des flags populaires, avec descriptions FR/EN.
///
/// Ces usages sont documentés par la communauté (FastFlags community, Bloxstrap…).
/// Roblox peut ignorer ou retirer n'importe quel flag sans préavis.
pub fn catalog() -> Vec<CatalogEntry> {
    let entries: Vec<(&str, Value, &str, &str, &str, &str)> = vec![
        (
            "DFIntTaskSchedulerTargetFps",
            int_v(9999),
            "int",
            CAT_PERF,
            "FPS cible du moteur : 60 (défaut), 144, 240… 9999 débloque le framerate sur écrans hautes fréquences.",
            "Engine target FPS: 60 (default), 144, 240… 9999 unlocks the framerate on high-refresh displays.",
        ),
        (
            "DFIntDebugFRMQualityLevelOverride",
            int_v(1),
            "int",
            CAT_GFX,
            "Force le niveau de qualité graphique de 1 (minimal) à 21 (maximal), indépendamment des réglages.",
            "Forces the graphics quality level from 1 (low) to 21 (max), regardless of in-game settings.",
        ),
        (
            "DFFlagDebugRenderForceTechnologyVoxel",
            bool_v(),
            "bool",
            CAT_GFX,
            "Force l'éclairage Voxel (plus léger que ShadowMap/Future) : gros gain de FPS sur les gros jeux.",
            "Forces Voxel lighting (lighter than ShadowMap/Future): big FPS gain on large games.",
        ),
        (
            "FFlagDisablePostFx",
            bool_v(),
            "bool",
            CAT_GFX,
            "Désactive les effets post-traitement (bloom, flou, profondeur de champ…).",
            "Disables post-processing effects (bloom, blur, depth of field…).",
        ),
        (
            "FIntRenderShadowIntensity",
            int_v(0),
            "int",
            CAT_GFX,
            "Intensité des ombres : 0 les désactive complètement (gain de perf).",
            "Shadow intensity: 0 disables shadows entirely (performance gain).",
        ),
        (
            "FIntDebugForceMSAASamples",
            int_v(0),
            "int",
            CAT_GFX,
            "Échantillons d'anti-aliasing MSAA : 0 (aucun, plus rapide), 1, 2, 4 ou 8.",
            "MSAA anti-aliasing samples: 0 (none, fastest), 1, 2, 4 or 8.",
        ),
        (
            "DFFlagTextureQualityOverrideEnabled",
            bool_v(),
            "bool",
            CAT_GFX,
            "Active l'override de qualité des textures (à utiliser avec DFIntTextureQualityOverride).",
            "Enables the texture quality override (use with DFIntTextureQualityOverride).",
        ),
        (
            "DFIntTextureQualityOverride",
            int_v(0),
            "int",
            CAT_GFX,
            "Qualité des textures : 0 (basse, rapide) à 3 (haute).",
            "Texture quality: 0 (low, fast) to 3 (high).",
        ),
        (
            "FFlagDebugGraphicsPreferVulkan",
            bool_v(),
            "bool",
            CAT_ENGINE,
            "Prefère le rendu Vulkan. Un seul sélecteur de moteur à la fois !",
            "Prefers the Vulkan renderer. Use only one renderer selector at a time!",
        ),
        (
            "FFlagDebugGraphicsPreferD3D11",
            bool_v(),
            "bool",
            CAT_ENGINE,
            "Prefère le rendu Direct3D 11 (Windows). Un seul sélecteur de moteur à la fois !",
            "Prefers the Direct3D 11 renderer (Windows). Use only one renderer selector at a time!",
        ),
        (
            "FFlagDebugGraphicsPreferOpenGL",
            bool_v(),
            "bool",
            CAT_ENGINE,
            "Prefère le rendu OpenGL (peut aider sur vieux GPU). Un seul sélecteur à la fois !",
            "Prefers the OpenGL renderer (may help on older GPUs). Use only one selector at a time!",
        ),
        (
            "DFIntConnectionMTUSize",
            int_v(1400),
            "int",
            CAT_NET,
            "Taille MTU des paquets réseau (valeurs usuelles 900–1472). Peut réduire la latence sur certains réseaux.",
            "Network packet MTU size (usual values 900–1472). May reduce latency on some networks.",
        ),
        (
            "DFIntS2PhysicsSenderRate",
            int_v(30),
            "int",
            CAT_NET,
            "Fréquence d'envoi de la physique côté serveur (défaut ~60). Des valeurs plus basses réduisent la bande passante.",
            "Server physics send rate (default ~60). Lower values reduce bandwidth usage.",
        ),
        (
            "FFlagDebugDisableTelemetryEphemeralCounter",
            bool_v(),
            "bool",
            CAT_TELEM,
            "Désactive l'envoi des compteurs éphémères de télémétrie.",
            "Disables ephemeral telemetry counters.",
        ),
        (
            "FFlagDebugDisableTelemetryEphemeralStat",
            bool_v(),
            "bool",
            CAT_TELEM,
            "Désactive l'envoi des stats éphémères de télémétrie.",
            "Disables ephemeral telemetry stats.",
        ),
        (
            "FFlagDebugDisableTelemetryEventIngest",
            bool_v(),
            "bool",
            CAT_TELEM,
            "Désactive l'ingestion des événements de télémétrie.",
            "Disables telemetry event ingestion.",
        ),
        (
            "FFlagDebugDisableTelemetryPoint",
            bool_v(),
            "bool",
            CAT_TELEM,
            "Désactive l'envoi des points de télémétrie.",
            "Disables telemetry points.",
        ),
        (
            "FFlagDebugDisableTelemetryV2Counter",
            bool_v(),
            "bool",
            CAT_TELEM,
            "Désactive les compteurs de télémétrie v2.",
            "Disables v2 telemetry counters.",
        ),
        (
            "FFlagDebugDisableTelemetryV2Event",
            bool_v(),
            "bool",
            CAT_TELEM,
            "Désactive les événements de télémétrie v2.",
            "Disables v2 telemetry events.",
        ),
        (
            "FFlagDebugDisableTelemetryV2Stat",
            bool_v(),
            "bool",
            CAT_TELEM,
            "Désactive les stats de télémétrie v2.",
            "Disables v2 telemetry stats.",
        ),
        (
            "FFlagDebugDisableTimeoutDisconnect",
            bool_v(),
            "bool",
            CAT_MISC,
            "Évite d'être déconnecté lors d'un timeout réseau bref (utile sur Wi-Fi instable).",
            "Prevents disconnects on brief network timeouts (useful on unstable Wi-Fi).",
        ),
        (
            "FIntFullscreenTitleBarTriggerDelayMillis",
            int_v(3600000),
            "int",
            CAT_MISC,
            "Retarde l'apparition de la barre de titre en plein écran (1h ici, donc invisible).",
            "Delays the fullscreen title bar trigger (1h here, so effectively hidden).",
        ),
        (
            "FFlagPreloadAllFonts",
            bool_v(),
            "bool",
            CAT_MISC,
            "Précharge toutes les polices au démarrage : évite les micro-freezes quand du texte apparaît.",
            "Preloads all fonts at startup: avoids micro-freezes when text appears.",
        ),
    ];

    entries
        .into_iter()
        .map(|(key, value, vtype, category, desc_fr, desc_en)| CatalogEntry {
            key: key.to_string(),
            value,
            vtype: vtype.to_string(),
            category: category.to_string(),
            desc_fr: desc_fr.to_string(),
            desc_en: desc_en.to_string(),
        })
        .collect()
}

/// Presets prêts à l'emploi.
pub fn presets() -> Vec<Preset> {
    macro_rules! map {
        ($($k:literal => $v:expr),* $(,)?) => {{
            let mut m = FlagMap::new();
            $( m.insert($k.to_string(), $v); )*
            m
        }};
    }

    vec![
        Preset {
            id: "fps_max".into(),
            name_fr: "FPS Maximum".into(),
            name_en: "Max FPS".into(),
            desc_fr: "Priorité absolue aux performances : graphismes min, pas d'ombres, pas d'effets.".into(),
            desc_en: "Absolute performance priority: min graphics, no shadows, no effects.".into(),
            flags: map! {
                "DFIntTaskSchedulerTargetFps" => int_v(9999),
                "DFIntDebugFRMQualityLevelOverride" => int_v(1),
                "DFFlagDebugRenderForceTechnologyVoxel" => bool_v(),
                "FFlagDisablePostFx" => bool_v(),
                "FIntRenderShadowIntensity" => int_v(0),
                "FIntDebugForceMSAASamples" => int_v(0),
                "DFFlagTextureQualityOverrideEnabled" => bool_v(),
                "DFIntTextureQualityOverride" => int_v(0),
                "FFlagPreloadAllFonts" => bool_v(),
            },
        },
        Preset {
            id: "equilibre".into(),
            name_fr: "Équilibré".into(),
            name_en: "Balanced".into(),
            desc_fr: "Bon compromis : FPS élevé, textures moyennes, éclairage Voxel.".into(),
            desc_en: "Good trade-off: high FPS, medium textures, Voxel lighting.".into(),
            flags: map! {
                "DFIntTaskSchedulerTargetFps" => int_v(240),
                "DFFlagDebugRenderForceTechnologyVoxel" => bool_v(),
                "FIntDebugForceMSAASamples" => int_v(2),
                "DFFlagTextureQualityOverrideEnabled" => bool_v(),
                "DFIntTextureQualityOverride" => int_v(2),
            },
        },
        Preset {
            id: "qualite_max".into(),
            name_fr: "Qualité Maximum".into(),
            name_en: "Max Quality".into(),
            desc_fr: "Pour les grosses configs : qualité 21, MSAA 4, textures hautes.".into(),
            desc_en: "For beefy rigs: quality 21, MSAA 4, high textures.".into(),
            flags: map! {
                "DFIntTaskSchedulerTargetFps" => int_v(240),
                "DFIntDebugFRMQualityLevelOverride" => int_v(21),
                "FIntDebugForceMSAASamples" => int_v(4),
                "DFFlagTextureQualityOverrideEnabled" => bool_v(),
                "DFIntTextureQualityOverride" => int_v(3),
            },
        },
        Preset {
            id: "latence".into(),
            name_fr: "Latence réduite".into(),
            name_en: "Low Latency".into(),
            desc_fr: "Réglages réseau orientés ping et stabilité de connexion.".into(),
            desc_en: "Network settings focused on ping and connection stability.".into(),
            flags: map! {
                "DFIntConnectionMTUSize" => int_v(1400),
                "DFIntS2PhysicsSenderRate" => int_v(30),
                "FFlagDebugDisableTimeoutDisconnect" => bool_v(),
            },
        },
        Preset {
            id: "telemetrie_off".into(),
            name_fr: "Télémétrie OFF".into(),
            name_en: "Telemetry OFF".into(),
            desc_fr: "Désactive les 7 canaux de télémétrie du client Roblox.".into(),
            desc_en: "Disables the 7 telemetry channels of the Roblox client.".into(),
            flags: map! {
                "FFlagDebugDisableTelemetryEphemeralCounter" => bool_v(),
                "FFlagDebugDisableTelemetryEphemeralStat" => bool_v(),
                "FFlagDebugDisableTelemetryEventIngest" => bool_v(),
                "FFlagDebugDisableTelemetryPoint" => bool_v(),
                "FFlagDebugDisableTelemetryV2Counter" => bool_v(),
                "FFlagDebugDisableTelemetryV2Event" => bool_v(),
                "FFlagDebugDisableTelemetryV2Stat" => bool_v(),
            },
        },
    ]
}
