//! BloxHub Chat — serveur WebSocket autonome.
//!
//! À lancer sur ton PC ou un VPS :
//!   cargo run -p bloxhub-server -- --port 8787 --data ./bloxhub-chat-data
//!
//! Protocole (JSON, une valeur par frame WebSocket) :
//!   client → serveur : hello, join, leave, message, dm, typing
//!   serveur → client : welcome, presence, message, dm, typing, room_history, error
//!
//! Salons par défaut : general, fastflags, jeux, aide, off-topic.
//! Historique : 200 derniers messages/salon, persistés dans `history.json`.

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

const DEFAULT_ROOMS: [&str; 5] = ["general", "fastflags", "jeux", "aide", "off-topic"];
const HISTORY_LIMIT: usize = 200;
const HISTORY_SNAPSHOT: usize = 50;
const MAX_CONTENT: usize = 2000;
const FLOOD_LIMIT: usize = 8; // messages par fenêtre de 10 s
const PALETTE: [&str; 10] = [
    "#ff4d45", "#f5cd30", "#0da84c", "#00a2ff", "#9b59f7", "#ff8c1a", "#1fb2cd", "#ff7bac",
    "#b8e23a", "#c98fff",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Msg {
    id: u64,
    from: String,
    from_id: u64,
    color: String,
    content: String,
    ts: u64,
}

struct User {
    id: u64,
    username: String,
    color: String,
    rooms: HashSet<String>,
    out: mpsc::UnboundedSender<Message>,
}

struct Hub {
    users: Mutex<HashMap<u64, User>>,
    history: Mutex<HashMap<String, Vec<Msg>>>,
    next_id: AtomicU64,
    rooms: Vec<String>,
    data_dir: PathBuf,
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn color_for(name: &str) -> String {
    PALETTE[name.bytes().map(|b| b as usize).sum::<usize>() % PALETTE.len()].to_string()
}

fn sanitize_username(raw: &str) -> Option<String> {
    let cleaned: String = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let cleaned: String = cleaned.chars().take(24).collect();
    if cleaned.chars().count() < 2 {
        None
    } else {
        Some(cleaned)
    }
}

/// Envoie un texte à tous les membres d'un salon (sauf éventuellement un id).
fn broadcast_room(hub: &Hub, room: &str, text: &str, except: Option<u64>) {
    let users = hub.users.lock().unwrap();
    for u in users.values() {
        if Some(u.id) == except || !u.rooms.contains(room) {
            continue;
        }
        let _ = u.out.send(Message::text(text));
    }
}

/// Diffuse la liste complète des utilisateurs connectés.
fn broadcast_presence(hub: &Hub) {
    let users = hub.users.lock().unwrap();
    let list: Vec<Value> = users
        .values()
        .map(|u| {
            json!({
                "id": u.id,
                "username": u.username,
                "color": u.color,
                "rooms": u.rooms.iter().cloned().collect::<Vec<_>>(),
            })
        })
        .collect();
    let text = json!({ "type": "presence", "users": list }).to_string();
    for u in users.values() {
        let _ = u.out.send(Message::text(&text));
    }
}

/// Sauvegarde l'historique (tâche détachée, snapshot cloné sous lock).
fn persist_history(hub: &Arc<Hub>) {
    let snapshot: HashMap<String, Vec<Msg>> = hub.history.lock().unwrap().clone();
    let path = hub.data_dir.join("history.json");
    tokio::spawn(async move {
        if let Ok(data) = serde_json::to_string(&snapshot) {
            let _ = tokio::fs::write(&path, data).await;
        }
    });
}

fn last_messages(hub: &Hub, room: &str) -> Vec<Msg> {
    hub.history
        .lock()
        .unwrap()
        .get(room)
        .map(|v| v.iter().rev().take(HISTORY_SNAPSHOT).rev().cloned().collect())
        .unwrap_or_default()
}

async fn handle_conn(hub: Arc<Hub>, stream: TcpStream, _addr: SocketAddr) {
    let Ok(ws) = tokio_tungstenite::accept_async(stream).await else {
        return;
    };
    let (mut sink, mut stream) = ws.split();

    // --- Handshake : premier message = hello ---
    let username = match tokio::time::timeout(Duration::from_secs(10), stream.next()).await {
        Ok(Some(Ok(Message::Text(txt)))) => match serde_json::from_str::<Value>(&txt) {
            Ok(v) if v.get("type").and_then(|t| t.as_str()) == Some("hello") => v
                .get("username")
                .and_then(|u| u.as_str())
                .and_then(sanitize_username),
            _ => None,
        },
        _ => None,
    };
    let Some(username) = username else {
        let _ = sink
            .send(Message::text(
                json!({"type":"error","code":"bad_hello","detail":"Premier message attendu : {\"type\":\"hello\",\"username\":\"…\"}"}).to_string(),
            ))
            .await;
        return;
    };

    // --- Enregistrement (pseudo unique) — lock libéré avant tout `.await` ---
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Message>();
    let id = hub.next_id.fetch_add(1, Ordering::Relaxed);
    let color = color_for(&username);

    let mut registered = false;
    {
        let mut users = hub.users.lock().unwrap();
        let taken = users
            .values()
            .any(|u| u.username.eq_ignore_ascii_case(&username));
        if !taken {
            let mut rooms = HashSet::new();
            rooms.insert("general".to_string()); // salon d'entrée automatique
            users.insert(
                id,
                User {
                    id,
                    username: username.clone(),
                    color: color.clone(),
                    rooms,
                    out: out_tx.clone(),
                },
            );
            registered = true;
        }
    }
    if !registered {
        let _ = sink
            .send(Message::text(
                json!({"type":"error","code":"username_taken","detail":username}).to_string(),
            ))
            .await;
        return;
    }

    // --- Écrivain dédié (une seule tâche écrit dans le socket) ---
    let writer = {
        let mut sink = sink;
        tokio::spawn(async move {
            while let Some(m) = out_rx.recv().await {
                if sink.send(m).await.is_err() {
                    break;
                }
            }
            let _ = sink.send(Message::Close(None)).await;
        })
    };

    // Bienvenue + historique du salon général
    let joined_rooms: Vec<String> = vec!["general".to_string()];
    let mut history_obj = serde_json::Map::new();
    for room in &joined_rooms {
        history_obj.insert(room.clone(), json!(last_messages(&hub, room)));
    }
    let _ = out_tx.send(Message::text(
        json!({
            "type": "welcome",
            "user_id": id,
            "color": color,
            "rooms": hub.rooms,
            "joined": joined_rooms,
            "history": Value::Object(history_obj),
        })
        .to_string(),
    ));
    broadcast_presence(&hub);
    broadcast_room(
        &hub,
        "general",
        &json!({"type":"system","room":"general","event":"join","username":username}).to_string(),
        Some(id),
    );

    // --- Boucle de lecture ---
    let mut flood: VecDeque<u64> = VecDeque::new();
    while let Some(Ok(msg)) = stream.next().await {
        let Message::Text(txt) = msg else {
            continue;
        };
        let Ok(v) = serde_json::from_str::<Value>(&txt) else {
            continue;
        };
        let mtype = v.get("type").and_then(|t| t.as_str()).unwrap_or("");

        // Anti-flood sur les messages de salon.
        if mtype == "message" {
            let now = now_secs();
            while flood.front().map(|t| now - *t > 10).unwrap_or(false) {
                flood.pop_front();
            }
            if flood.len() >= FLOOD_LIMIT {
                let _ = out_tx.send(Message::text(
                    json!({"type":"error","code":"rate_limited","detail":"Trop de messages, ralentis un peu !"}).to_string(),
                ));
                continue;
            }
            flood.push_back(now);
        }

        match mtype {
            "message" => {
                let room = v.get("room").and_then(|r| r.as_str()).unwrap_or("general");
                if !hub.rooms.iter().any(|r| r == room) {
                    let _ = out_tx.send(Message::text(
                        json!({"type":"error","code":"unknown_room","detail":room}).to_string(),
                    ));
                    continue;
                }
                let content = v
                    .get("content")
                    .and_then(|c| c.as_str())
                    .map(str::trim)
                    .unwrap_or("");
                if content.is_empty() {
                    continue;
                }
                let content: String = content.chars().take(MAX_CONTENT).collect();

                let msg = Msg {
                    id: hub.next_id.fetch_add(1, Ordering::Relaxed),
                    from: username.clone(),
                    from_id: id,
                    color: color.clone(),
                    content,
                    ts: now_secs(),
                };
                {
                    let mut hist = hub.history.lock().unwrap();
                    let entry = hist.entry(room.to_string()).or_default();
                    entry.push(msg.clone());
                    let len = entry.len();
                    if len > HISTORY_LIMIT {
                        let overflow = len - HISTORY_LIMIT;
                        let drained: Vec<Msg> = entry.drain(..overflow).collect();
                        drop(hist);
                        drop(drained);
                    }
                }
                persist_history(&hub);
                broadcast_room(
                    &hub,
                    room,
                    &json!({
                        "type": "message", "room": room, "id": msg.id,
                        "from": msg.from, "from_id": msg.from_id, "color": msg.color,
                        "content": msg.content, "ts": msg.ts
                    })
                    .to_string(),
                    None,
                );
            }
            "dm" => {
                let to = v.get("to").and_then(|t| t.as_str()).unwrap_or("");
                let content = v
                    .get("content")
                    .and_then(|c| c.as_str())
                    .map(str::trim)
                    .unwrap_or("");
                if content.is_empty() {
                    continue;
                }
                let content: String = content.chars().take(MAX_CONTENT).collect();
                let target = {
                    let users = hub.users.lock().unwrap();
                    users
                        .values()
                        .find(|u| u.username.eq_ignore_ascii_case(to))
                        .map(|u| (u.id, u.out.clone(), u.username.clone()))
                };
                match target {
                    Some((_tid, tout, tname)) => {
                        let payload = json!({
                            "type": "dm", "id": hub.next_id.fetch_add(1, Ordering::Relaxed),
                            "from": username, "from_id": id, "color": color,
                            "content": content, "ts": now_secs(), "from_self": false
                        })
                        .to_string();
                        let echo = json!({
                            "type": "dm", "id": hub.next_id.fetch_add(1, Ordering::Relaxed),
                            "from": username, "from_id": id, "color": color,
                            "content": content, "ts": now_secs(), "from_self": true,
                            "to": tname
                        })
                        .to_string();
                        let _ = tout.send(Message::text(payload));
                        let _ = out_tx.send(Message::text(echo));
                    }
                    None => {
                        let _ = out_tx.send(Message::text(
                            json!({"type":"error","code":"unknown_user","detail":to}).to_string(),
                        ));
                    }
                }
            }
            "join" => {
                let room = v.get("room").and_then(|r| r.as_str()).unwrap_or("");
                if hub.rooms.iter().any(|r| r == room) {
                    {
                        let mut users = hub.users.lock().unwrap();
                        if let Some(u) = users.get_mut(&id) {
                            u.rooms.insert(room.to_string());
                        }
                    }
                    let _ = out_tx.send(Message::text(
                        json!({"type":"room_history","room":room,"messages":last_messages(&hub, room)}).to_string(),
                    ));
                    broadcast_presence(&hub);
                } else {
                    let _ = out_tx.send(Message::text(
                        json!({"type":"error","code":"unknown_room","detail":room}).to_string(),
                    ));
                }
            }
            "leave" => {
                let room = v.get("room").and_then(|r| r.as_str()).unwrap_or("");
                {
                    let mut users = hub.users.lock().unwrap();
                    if let Some(u) = users.get_mut(&id) {
                        u.rooms.remove(room);
                    }
                }
                broadcast_presence(&hub);
            }
            "typing" => {
                let room = v.get("room").and_then(|r| r.as_str()).unwrap_or("general");
                broadcast_room(
                    &hub,
                    room,
                    &json!({"type":"typing","room":room,"from":username}).to_string(),
                    Some(id),
                );
            }
            _ => {}
        }
    }

    // --- Déconnexion ---
    {
        let mut users = hub.users.lock().unwrap();
        users.remove(&id);
    }
    drop(out_tx); // ferme le canal → l'écrivain envoie Close et termine
    let _ = writer.await;
    broadcast_presence(&hub);
    broadcast_room(
        &hub,
        "general",
        &json!({"type":"system","room":"general","event":"leave","username":username}).to_string(),
        None,
    );
}

#[tokio::main]
async fn main() {
    let mut port: u16 = 8787;
    let mut data = PathBuf::from("bloxhub-chat-data");

    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" if i + 1 < args.len() => {
                port = args[i + 1].parse().unwrap_or(8787);
                i += 1;
            }
            "--data" if i + 1 < args.len() => {
                data = PathBuf::from(&args[i + 1]);
                i += 1;
            }
            "--help" | "-h" => {
                println!("BloxHub Chat — serveur WebSocket communautaire");
                println!("  --port <n>    port d'écoute (défaut : 8787)");
                println!("  --data <dir>  dossier de l'historique (défaut : ./bloxhub-chat-data)");
                return;
            }
            _ => {}
        }
        i += 1;
    }

    let _ = std::fs::create_dir_all(&data);
    let history: HashMap<String, Vec<Msg>> = tokio::fs::read(data.join("history.json"))
        .await
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    let saved_msgs: usize = history.values().map(|v| v.len()).sum();

    let hub = Arc::new(Hub {
        users: Mutex::new(HashMap::new()),
        history: Mutex::new(history),
        next_id: AtomicU64::new(1),
        rooms: DEFAULT_ROOMS.iter().map(|s| s.to_string()).collect(),
        data_dir: data.clone(),
    });

    let listener = TcpListener::bind(("0.0.0.0", port))
        .await
        .unwrap_or_else(|e| panic!("Impossible d'écouter sur le port {port} : {e}"));

    println!("┌──────────────────────────────────────────────────┐");
    println!("│           BloxHub Chat — serveur prêt            │");
    println!("├──────────────────────────────────────────────────┤");
    println!("│ Adresse   : ws://<ton-ip>:{port}                   │");
    println!("│ Salons    : {}                         │", DEFAULT_ROOMS.join(", "));
    println!("│ Historique: {saved_msgs} messages chargés                      │");
    println!("│ Données   : {}", data.display());
    println!("└──────────────────────────────────────────────────┘");

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                let hub = hub.clone();
                tokio::spawn(handle_conn(hub, stream, addr));
            }
            Err(e) => eprintln!("accept() : {e}"),
        }
    }
}
