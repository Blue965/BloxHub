//! Client de l'API publique Roblox (aucune authentification requise).
//!
//! Endpoints utilisés (données publiques) :
//! - `users.roblox.com`      : résolution pseudo → id, détails du compte
//! - `thumbnails.roblox.com` : avatars (headshot + corps complet)
//! - `friends.roblox.com`    : liste / nombre d'amis (peut être privé)

use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

pub struct RbxClient {
    http: Client,
}

fn map_err(e: reqwest::Error) -> String {
    if e.is_timeout() {
        "Délai dépassé — l'API Roblox ne répond pas.".into()
    } else if e.is_connect() {
        "Connexion impossible à l'API Roblox (réseau ?).".into()
    } else {
        e.to_string()
    }
}

fn status_err(status: u16) -> String {
    match status {
        429 => "Trop de requêtes vers l'API Roblox — réessaie dans quelques secondes.".into(),
        404 => "Ressource Roblox introuvable.".into(),
        400 => "Requête refusée par l'API Roblox (pseudo invalide ?).".into(),
        s => format!("API Roblox : code HTTP {s}."),
    }
}

impl Default for RbxClient {
    fn default() -> Self {
        Self::new()
    }
}

impl RbxClient {
    pub fn new() -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(12))
            .user_agent("BloxHub/0.1 (application communautaire non officielle)")
            .build()
            .expect("reqwest client");
        Self { http }
    }

    async fn get(&self, url: &str) -> Result<Value, String> {
        let resp = self.http.get(url).send().await.map_err(map_err)?;
        let status = resp.status().as_u16();
        if !resp.status().is_success() {
            return Err(status_err(status));
        }
        resp.json::<Value>().await.map_err(map_err)
    }

    async fn post(&self, url: &str, body: Value) -> Result<Value, String> {
        let resp = self
            .http
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(map_err)?;
        let status = resp.status().as_u16();
        if !resp.status().is_success() {
            return Err(status_err(status));
        }
        resp.json::<Value>().await.map_err(map_err)
    }

    /// Résout un pseudo → { id, name, displayName } ou erreur "introuvable".
    pub async fn user_by_username(&self, username: &str) -> Result<Value, String> {
        let body = json!({ "usernames": [username], "excludeBannedUsers": false });
        let v = self
            .post("https://users.roblox.com/v1/usernames/users", body)
            .await?;
        let entry = v
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|a| a.first())
            .cloned()
            .ok_or_else(|| "Utilisateur Roblox introuvable.".to_string())?;
        Ok(entry)
    }

    /// Détails d'un compte : description, date de création, ban…
    pub async fn user_details(&self, id: u64) -> Result<Value, String> {
        self.get(&format!("https://users.roblox.com/v1/users/{id}"))
            .await
    }

    /// URL de l'avatar corps complet.
    pub async fn avatar_full(&self, id: u64) -> Result<String, String> {
        self.thumbnail(
            &format!(
                "https://thumbnails.roblox.com/v1/users/avatar?userIds={id}&size=420x420&format=Png&isCircular=false"
            ),
            id,
        )
        .await
    }

    /// URL du headshot (avatar rond).
    pub async fn avatar_headshot(&self, id: u64) -> Result<String, String> {
        self.thumbnail(
            &format!(
                "https://thumbnails.roblox.com/v1/users/avatar-headshot?userIds={id}&size=420x420&format=Png&isCircular=true"
            ),
            id,
        )
        .await
    }

    async fn thumbnail(&self, url: &str, id: u64) -> Result<String, String> {
        let v = self.get(url).await?;
        v.get("data")
            .and_then(|d| d.as_array())
            .and_then(|a| a.first())
            .and_then(|e| e.get("imageUrl"))
            .and_then(|u| u.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| format!("Miniature indisponible pour l'utilisateur {id}."))
    }

    /// Headshots pour plusieurs ids d'un coup (liste d'amis).
    async fn headshots_batch(&self, ids: &[u64]) -> Value {
        let list = ids
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let url = format!(
            "https://thumbnails.roblox.com/v1/users/avatar-headshot?userIds={list}&size=150x150&format=Png&isCircular=true"
        );
        self.get(&url).await.unwrap_or(Value::Null)
    }

    /// Liste des amis (peut être privé → liste vide).
    async fn friends(&self, id: u64) -> Vec<Value> {
        let url = format!("https://friends.roblox.com/v1/users/{id}/friends");
        match self.get(&url).await {
            Ok(v) => v
                .get("data")
                .and_then(|d| d.as_array())
                .cloned()
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    /// Bundle complet pour la vue Profil : user + avatars + amis (12 max).
    pub async fn profile_bundle(&self, username: &str) -> Result<Value, String> {
        let base = self.user_by_username(username).await?;
        let id = base
            .get("id")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| "Réponse API inattendue (id manquant).".to_string())?;

        let details = self.user_details(id).await.unwrap_or(Value::Null);
        let headshot = self.avatar_headshot(id).await.unwrap_or_default();
        let full = self.avatar_full(id).await.unwrap_or_default();

        let mut friends_raw = self.friends(id).await;
        friends_raw.truncate(12);
        let friend_ids: Vec<u64> = friends_raw
            .iter()
            .filter_map(|f| f.get("id").and_then(|v| v.as_u64()))
            .collect();
        let thumbs = self.headshots_batch(&friend_ids).await;
        let thumb_for = |uid: u64| -> String {
            thumbs
                .get("data")
                .and_then(|d| d.as_array())
                .and_then(|arr| {
                    arr.iter()
                        .find(|e| e.get("targetId").and_then(|t| t.as_u64()) == Some(uid))
                })
                .and_then(|e| e.get("imageUrl"))
                .and_then(|u| u.as_str())
                .unwrap_or_default()
                .to_string()
        };
        let friends: Vec<Value> = friends_raw
            .iter()
            .map(|f| {
                json!({
                    "id": f.get("id"),
                    "name": f.get("name"),
                    "displayName": f.get("displayName"),
                    "imageUrl": f.get("id").and_then(|v| v.as_u64()).map(thumb_for).unwrap_or_default(),
                })
            })
            .collect();

        Ok(json!({
            "user": base,
            "details": details,
            "headshotUrl": headshot,
            "avatarUrl": full,
            "friends": friends,
        }))
    }
}
