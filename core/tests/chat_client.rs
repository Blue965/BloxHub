//! Tests d'intégration : le client chat du core reçoit le `welcome`,
//! renvoie un message et se déconnecte proprement.
//! Le serveur doit tourner sur ws://127.0.0.1:8799 :
//!
//!   cargo run -p bloxhub-server -- --port 8799
//!   cargo test -p bloxhub-core --test chat_client

use std::time::Duration;

#[tokio::test]
async fn client_receives_welcome() {
    let (handle, mut rx) =
        bloxhub_core::chat::spawn_client("ws://127.0.0.1:8799".into(), "TestRust".into());

    let mut got_welcome = false;
    for _ in 0..20 {
        match tokio::time::timeout(Duration::from_millis(500), rx.recv()).await {
            Ok(Some(ev)) => {
                if ev.get("type").and_then(|t| t.as_str()) == Some("welcome") {
                    got_welcome = true;
                    break;
                }
            }
            _ => break,
        }
    }

    handle.disconnect();
    assert!(got_welcome, "le client n'a pas reçu le welcome du serveur");
}

#[tokio::test]
async fn client_sends_and_receives_message() {
    let (handle, mut rx) =
        bloxhub_core::chat::spawn_client("ws://127.0.0.1:8799".into(), "RustTester".into());

    // attend le welcome
    for _ in 0..20 {
        match tokio::time::timeout(Duration::from_millis(500), rx.recv()).await {
            Ok(Some(ev)) if ev["type"] == "welcome" => break,
            Ok(Some(_)) => continue,
            _ => panic!("pas de welcome"),
        }
    }

    handle.send(bloxhub_core::chat::msg_message("general", "ping depuis le test Rust"));
    let mut echoed = false;
    for _ in 0..20 {
        match tokio::time::timeout(Duration::from_millis(500), rx.recv()).await {
            Ok(Some(ev)) => {
                if ev["type"] == "message" && ev["content"] == "ping depuis le test Rust" {
                    echoed = true;
                    break;
                }
            }
            _ => break,
        }
    }

    handle.disconnect();

    // la tâche doit s'arrêter : événements disconnected puis offline
    let mut offline = false;
    for _ in 0..10 {
        match tokio::time::timeout(Duration::from_millis(500), rx.recv()).await {
            Ok(Some(ev)) if ev["type"] == "status" && ev["state"] == "offline" => {
                offline = true;
                break;
            }
            Ok(Some(_)) => continue,
            _ => break,
        }
    }
    assert!(echoed, "le message n'est pas revenu en broadcast");
    assert!(offline, "la tâche ne s'est pas arrêtée après disconnect()");
}
