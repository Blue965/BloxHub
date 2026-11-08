//! Client WebSocket du chat communautaire BloxHub.
//!
//! Indépendant de Tauri : la coquille `src-tauri` transmet les événements reçus
//! vers l'UI via `app.emit("chat-event", …)`. Reconnexion automatique avec
//! backoff exponentiel (1 → 15 s), arrêt immédiat à la demande.

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::connect_async;

/// Poignée pour piloter le client chat depuis l'app.
#[derive(Clone)]
pub struct ChatHandle {
    out_tx: mpsc::UnboundedSender<Value>,
    gen_tx: watch::Sender<u64>,
    connected: Arc<AtomicBool>,
}

impl ChatHandle {
    /// Enfile un message de protocole vers le serveur (false = client arrêté).
    pub fn send(&self, msg: Value) -> bool {
        self.out_tx.send(msg).is_ok()
    }

    /// Demande la déconnexion et l'arrêt du client.
    pub fn disconnect(&self) {
        let _ = self.out_tx.send(json!({ "type": "__close" }));
        // send_modify : pas de garde borrow() vivant pendant l'écriture
        // (send(borrow()+1) créerait un deadlock read/write sur le même thread).
        self.gen_tx.send_modify(|gen| *gen = gen.wrapping_add(1));
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }
}

/// Lance le client dans une tâche tokio.
/// Retourne (poignée, flux des événements reçus du serveur).
pub fn spawn_client(
    url: String,
    username: String,
) -> (ChatHandle, mpsc::UnboundedReceiver<Value>) {
    let (out_tx, out_rx) = mpsc::unbounded_channel();
    let (ev_tx, ev_rx) = mpsc::unbounded_channel();
    let (gen_tx, gen_rx) = watch::channel(0u64);
    let connected = Arc::new(AtomicBool::new(false));
    let handle = ChatHandle {
        out_tx,
        gen_tx,
        connected: connected.clone(),
    };
    tokio::spawn(run_client(url, username, ev_tx, out_rx, gen_rx, connected));
    (handle, ev_rx)
}

// --- Constructeurs de messages de protocole --------------------------------

pub fn msg_join(room: &str) -> Value {
    json!({ "type": "join", "room": room })
}
pub fn msg_leave(room: &str) -> Value {
    json!({ "type": "leave", "room": room })
}
pub fn msg_message(room: &str, content: &str) -> Value {
    json!({ "type": "message", "room": room, "content": content })
}
pub fn msg_dm(to: &str, content: &str) -> Value {
    json!({ "type": "dm", "to": to, "content": content })
}
pub fn msg_typing(room: &str) -> Value {
    json!({ "type": "typing", "room": room })
}

// --- Boucle principale -------------------------------------------------------

async fn run_client(
    url: String,
    username: String,
    events: mpsc::UnboundedSender<Value>,
    mut out_rx: mpsc::UnboundedReceiver<Value>,
    mut gen_rx: watch::Receiver<u64>,
    connected: Arc<AtomicBool>,
) {
    let my_gen = *gen_rx.borrow();
    let mut backoff: u64 = 1;

    loop {
        if *gen_rx.borrow() != my_gen {
            break;
        }
        let _ = events.send(json!({ "type": "status", "state": "connecting", "url": url }));

        match connect_async(url.as_str()).await {
            Ok((ws, _resp)) => {
                backoff = 1;
                connected.store(true, Ordering::Relaxed);
                let _ = events.send(json!({ "type": "status", "state": "connected", "url": url }));
                let (mut sink, mut stream) = ws.split();

                // Le serveur attend un `hello` en premier message.
                let hello = json!({ "type": "hello", "username": username });
                if sink.send(Message::Text(hello.to_string())).await.is_err() {
                    connected.store(false, Ordering::Relaxed);
                    continue;
                }

                loop {
                    tokio::select! {
                        out = out_rx.recv() => {
                            match out {
                                Some(v) => {
                                    if v.get("type").and_then(|t| t.as_str()) == Some("__close") {
                                        let _ = sink.send(Message::Close(None)).await;
                                        break;
                                    }
                                    if sink.send(Message::Text(v.to_string())).await.is_err() {
                                        break;
                                    }
                                }
                                None => break,
                            }
                        }
                        incoming = stream.next() => {
                            match incoming {
                                Some(Ok(Message::Text(txt))) => {
                                    if let Ok(v) = serde_json::from_str::<Value>(&txt) {
                                        let _ = events.send(v);
                                    }
                                }
                                Some(Ok(Message::Ping(payload))) => {
                                    let _ = sink.send(Message::Pong(payload)).await;
                                }
                                Some(Ok(Message::Close(_))) => break,
                                Some(Ok(_)) => {}
                                Some(Err(_)) | None => break,
                            }
                        }
                        _ = gen_rx.changed() => {
                            if *gen_rx.borrow() != my_gen {
                                let _ = sink.send(Message::Close(None)).await;
                                break;
                            }
                        }
                        _ = tokio::time::sleep(Duration::from_secs(15)) => {
                            // Entretien : vide les Pong en attente vers le serveur.
                            let _ = sink.flush().await;
                        }
                    }
                }
                connected.store(false, Ordering::Relaxed);
            }
            Err(e) => {
                let _ = events.send(json!({
                    "type": "status", "state": "error",
                    "detail": e.to_string(), "url": url
                }));
            }
        }

        let _ = events.send(json!({ "type": "status", "state": "disconnected", "url": url }));
        if *gen_rx.borrow() != my_gen {
            break;
        }

        // Pause de reconnexion : interrompue par un `__close` ou une demande d'arrêt.
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(backoff)) => {}
            out = out_rx.recv() => {
                match out {
                    Some(v) => {
                        if v.get("type").and_then(|t| t.as_str()) == Some("__close") {
                            break;
                        }
                        // Les autres messages restent en file d'attente pour la reconnexion.
                    }
                    None => break,
                }
            }
            _ = gen_rx.changed() => {
                if *gen_rx.borrow() != my_gen {
                    break;
                }
            }
        }
        backoff = (backoff * 2).min(15);
    }

    connected.store(false, Ordering::Relaxed);
    let _ = events.send(json!({ "type": "status", "state": "offline" }));
}
