use once_cell::sync::Lazy;
use rumqttc::{AsyncClient, EventLoop, MqttOptions, QoS, LastWill};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::{crypto, database, desk_client, native_notify};

static MQTT_CLIENT: Lazy<Mutex<Option<AsyncClient>>> = Lazy::new(|| Mutex::new(None));
static CONNECTED: AtomicBool = AtomicBool::new(false);
static APP_HANDLE: Lazy<Mutex<Option<tauri::AppHandle>>> = Lazy::new(|| Mutex::new(None));
/// Track the eventloop task so we can abort it on disconnect
static EVENTLOOP_HANDLE: Lazy<Mutex<Option<JoinHandle<()>>>> = Lazy::new(|| Mutex::new(None));

/// Store the Tauri AppHandle for emitting frontend events
pub async fn init(handle: tauri::AppHandle) {
    let mut lock = APP_HANDLE.lock().await;
    *lock = Some(handle);
}

/// Connect to the MQTT broker and start the receive loop
pub async fn connect(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    member_code: &str,
) -> Result<(), String> {
    // Disconnect existing connection if any (abort old eventloop task)
    disconnect().await;

    let random_hex: String = {
        use std::fmt::Write;
        let bytes: [u8; 4] = rand_bytes();
        let mut s = String::with_capacity(8);
        for b in bytes {
            write!(s, "{:02x}", b).ok();
        }
        s
    };
    let client_id = format!("d-mate-{}-{}", member_code, random_hex);

    let mut opts = MqttOptions::new(&client_id, host, port);
    opts.set_keep_alive(std::time::Duration::from_secs(30));
    // Use clean_session=true to clear any stale subscriptions on the broker
    opts.set_clean_session(true);
    opts.set_credentials(username, password);

    // Last Will and Testament — offline presence
    let lwt = LastWill::new(
        format!("d-mate/presence/{}", member_code),
        serde_json::to_string(&json!({"status": "offline"})).unwrap(),
        QoS::AtMostOnce,
        false,
    );
    opts.set_last_will(lwt);

    let (client, eventloop) = AsyncClient::new(opts, 64);

    // Subscribe to topics
    let code = member_code.to_string();
    client
        .subscribe(format!("d-mate/notice/all"), QoS::AtLeastOnce)
        .await
        .map_err(|e| format!("subscribe error: {}", e))?;
    client
        .subscribe(format!("d-mate/notice/{}", code), QoS::AtLeastOnce)
        .await
        .map_err(|e| format!("subscribe error: {}", e))?;
    client
        .subscribe(format!("d-mate/dm/{}", code), QoS::ExactlyOnce)
        .await
        .map_err(|e| format!("subscribe error: {}", e))?;

    // Publish online presence
    let presence_payload =
        serde_json::to_string(&json!({"status": "online"})).unwrap();
    client
        .publish(
            format!("d-mate/presence/{}", code),
            QoS::AtMostOnce,
            false,
            presence_payload.as_bytes(),
        )
        .await
        .map_err(|e| format!("publish error: {}", e))?;

    // Store client
    {
        let mut lock = MQTT_CLIENT.lock().await;
        *lock = Some(client);
    }
    CONNECTED.store(true, Ordering::SeqCst);

    // Spawn the event loop in background and store the JoinHandle
    let code_clone = code.clone();
    let handle = tokio::spawn(async move {
        run_eventloop(eventloop, code_clone).await;
    });
    {
        let mut el_lock = EVENTLOOP_HANDLE.lock().await;
        *el_lock = Some(handle);
    }

    log::info!("MQTT connected to {}:{} as {}", host, port, client_id);
    Ok(())
}

/// Internal event loop runner
async fn run_eventloop(mut eventloop: EventLoop, _member_code: String) {
    loop {
        match eventloop.poll().await {
            Ok(event) => {
                if let rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish)) = event {
                    handle_incoming_message(&publish.topic, &publish.payload).await;
                }
            }
            Err(e) => {
                log::warn!("MQTT eventloop error: {}", e);
                CONNECTED.store(false, Ordering::SeqCst);
                // Check if we were intentionally disconnected (client removed)
                let client_exists = {
                    let lock = MQTT_CLIENT.lock().await;
                    lock.is_some()
                };
                if !client_exists {
                    log::info!("MQTT eventloop: client removed, stopping loop");
                    break;
                }
                // Wait before reconnecting (rumqttc handles reconnect internally)
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }
}

/// Handle an incoming MQTT message
async fn handle_incoming_message(topic: &str, payload: &[u8]) {
    let text = match std::str::from_utf8(payload) {
        Ok(s) => s,
        Err(_) => return,
    };

    log::info!("MQTT message on {}: {}", topic, text);

    // Parse JSON payload
    let data: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => {
            // If not JSON, wrap it
            json!({
                "title": "메시지",
                "body": text,
            })
        }
    };

    // Determine message type: prefer payload "type" field, fallback to topic
    let msg_type = if topic.starts_with("d-mate/presence/") {
        return; // Don't store presence messages
    } else if let Some(payload_type) = data["type"].as_str() {
        // Use the type from payload if available (e.g. "notice", "notify", "dm")
        payload_type
    } else if topic.starts_with("d-mate/notice/") {
        "notice"
    } else if topic.starts_with("d-mate/dm/") {
        "dm"
    } else {
        "unknown"
    };

    let id = data["id"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("mqtt-{}", chrono::Utc::now().timestamp_millis()));
    let conversation_id = data["conversation_id"]
        .as_str()
        .unwrap_or(topic);
    let sender_code = data["sender_code"].as_str().unwrap_or("");
    let sender_name = data["sender_name"].as_str().unwrap_or("");
    let title = data["title"].as_str().unwrap_or("알림");
    let raw_body = data["body"].as_str().unwrap_or("");
    let ephemeral_key = data["ephemeral_key"].as_str().unwrap_or("");
    let nonce = data["nonce"].as_str().unwrap_or("");

    // DM 복호화 시도: ephemeral_key + nonce가 있으면 E2E 암호화된 메시지
    let body = if msg_type == "dm" && !ephemeral_key.is_empty() && !nonce.is_empty() {
        match database::get_keypair() {
            Some((private_key, _)) => {
                match crypto::decrypt_message(&private_key, ephemeral_key, nonce, raw_body) {
                    Ok(plaintext) => plaintext,
                    Err(e) => {
                        log::warn!("DM 복호화 실패: {}", e);
                        format!("[복호화 실패] {}", raw_body)
                    }
                }
            }
            None => {
                log::warn!("DM 복호화 실패: 암호화 키 없음");
                format!("[복호화 실패] {}", raw_body)
            }
        }
    } else {
        raw_body.to_string()
    };

    let created_at = data["created_at"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string());

    // Save to local DB
    database::save_local_message(
        &id,
        conversation_id,
        msg_type,
        sender_code,
        sender_name,
        title,
        &body,
        &created_at,
    );

    // DM: 전달 확인(delivered) 전송
    if msg_type == "dm" {
        let id_owned = id.clone();
        tokio::spawn(async move {
            desk_client::dm_delivered(&id_owned).await.ok();
        });
    }

    // Send OS notification
    let notify_title = if msg_type == "dm" {
        sender_name
    } else {
        title
    };
    native_notify::send(notify_title, &body);

    // Emit event to frontend
    let event_payload = json!({
        "id": id,
        "conversation_id": conversation_id,
        "type": msg_type,
        "sender_code": sender_code,
        "sender_name": sender_name,
        "title": title,
        "body": body,
        "created_at": created_at,
    });

    let handle_lock = APP_HANDLE.lock().await;
    if let Some(handle) = handle_lock.as_ref() {
        use tauri::Emitter;
        handle.emit("mqtt:message", &event_payload).ok();
    }
}

/// Disconnect from MQTT broker — fully clean up client and eventloop task
pub async fn disconnect() {
    // 1. Take and disconnect the client
    {
        let mut lock = MQTT_CLIENT.lock().await;
        if let Some(client) = lock.take() {
            client.disconnect().await.ok();
        }
    }

    // 2. Abort the eventloop task to prevent lingering background loops
    {
        let mut el_lock = EVENTLOOP_HANDLE.lock().await;
        if let Some(handle) = el_lock.take() {
            handle.abort();
        }
    }

    CONNECTED.store(false, Ordering::SeqCst);
    log::info!("MQTT disconnected (client + eventloop cleaned up)");
}

/// Check if MQTT is currently connected
pub fn is_connected() -> bool {
    CONNECTED.load(Ordering::SeqCst)
}

/// Publish a message to a topic
pub async fn publish(topic: &str, payload: &str, qos: QoS) -> Result<(), String> {
    let lock = MQTT_CLIENT.lock().await;
    match lock.as_ref() {
        Some(client) => {
            client
                .publish(topic, qos, false, payload.as_bytes())
                .await
                .map_err(|e| format!("publish error: {}", e))
        }
        None => Err("MQTT not connected".into()),
    }
}

/// Publish presence status
pub async fn publish_presence(code: &str, status: &str) -> Result<(), String> {
    let payload = serde_json::to_string(&json!({"status": status})).unwrap();
    publish(
        &format!("d-mate/presence/{}", code),
        &payload,
        QoS::AtMostOnce,
    )
    .await
}

/// Generate 4 random bytes using simple method (no extra dependency)
fn rand_bytes() -> [u8; 4] {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let seed = now.as_nanos() as u64;
    [
        (seed & 0xFF) as u8,
        ((seed >> 8) & 0xFF) as u8,
        ((seed >> 16) & 0xFF) as u8,
        ((seed >> 24) & 0xFF) as u8,
    ]
}
