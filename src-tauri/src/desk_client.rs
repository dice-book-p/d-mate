use once_cell::sync::Lazy;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};

use std::net::UdpSocket;

use crate::database;

static CLIENT: Lazy<Mutex<Option<DeskClient>>> = Lazy::new(|| Mutex::new(None));

#[derive(Clone)]
struct DeskClient {
    client: reqwest::Client,
    server_url: String,
    access_token: String,
    refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenResponse {
    ok: bool,
    access_token: Option<String>,
    refresh_token: Option<String>,
    message: Option<String>,
}

fn get_local_ip() -> String {
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|socket| {
            socket.connect("8.8.8.8:80")?;
            socket.local_addr()
        })
        .map(|addr| addr.ip().to_string())
        .unwrap_or_default()
}

impl DeskClient {
    fn new(server_url: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            server_url: server_url.trim_end_matches('/').to_string(),
            access_token: String::new(),
            refresh_token: String::new(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.server_url, path)
    }

    async fn join(
        &mut self,
        code: &str,
        name: &str,
        device_name: &str,
    ) -> Result<bool, String> {
        let body = serde_json::json!({
            "code": code,
            "name": name,
            "device_name": device_name,
            "ip": get_local_ip(),
            "os": std::env::consts::OS,
            "app_version": env!("CARGO_PKG_VERSION"),
        });

        let resp = self.client
            .post(&self.url("/api/auth/join"))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let data: TokenResponse = resp.json().await.map_err(|e| e.to_string())?;
        if data.ok {
            self.access_token = data.access_token.unwrap_or_default();
            self.refresh_token = data.refresh_token.unwrap_or_default();
            Ok(true)
        } else {
            Err(data.message.unwrap_or("참여 실패".into()))
        }
    }

    async fn refresh_auth(&mut self) -> Result<bool, String> {
        if self.refresh_token.is_empty() {
            return Err("리프레시 토큰 없음".into());
        }
        let body = serde_json::json!({ "refresh_token": &self.refresh_token });
        let resp = self.client
            .post(&self.url("/api/auth/refresh"))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let data: TokenResponse = resp.json().await.map_err(|e| e.to_string())?;
        if data.ok {
            self.access_token = data.access_token.unwrap_or_default();
            self.refresh_token = data.refresh_token.unwrap_or_default();
            Ok(true)
        } else {
            Err("토큰 갱신 실패".into())
        }
    }

    async fn request(
        &mut self,
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let do_request = |client: &reqwest::Client, url: &str, token: &str, method: &str, body: &Option<serde_json::Value>| {
            let mut req = match method {
                "POST" => client.post(url),
                "PUT" => client.put(url),
                "PATCH" => client.patch(url),
                "DELETE" => client.delete(url),
                _ => client.get(url),
            };
            req = req.header("Authorization", format!("Bearer {}", token));
            if let Some(b) = body {
                req = req.json(b);
            }
            req.send()
        };

        let url = self.url(path);
        let resp = do_request(&self.client, &url, &self.access_token, method, &body)
            .await
            .map_err(|e| e.to_string())?;

        if resp.status().as_u16() == 401 {
            self.refresh_auth().await?;
            let resp = do_request(&self.client, &url, &self.access_token, method, &body)
                .await
                .map_err(|e| e.to_string())?;
            resp.json().await.map_err(|e| e.to_string())
        } else {
            resp.json().await.map_err(|e| e.to_string())
        }
    }

    async fn post_error(&self, error_type: &str, message: &str, app_version: &str) -> Result<(), String> {
        let body = serde_json::json!({
            "error_type": error_type,
            "message": message,
            "app_version": app_version,
            "os": std::env::consts::OS,
        });

        // 에러 리포트는 실패해도 무시
        let url = if self.access_token.is_empty() {
            self.url("/api/errors/anonymous")
        } else {
            self.url("/api/errors")
        };

        let mut req = self.client.post(&url).json(&body);
        if !self.access_token.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.access_token));
        }
        req.send().await.ok();
        Ok(())
    }

    async fn submit_feedback(&mut self, category: &str, title: &str, body_text: &str) -> Result<serde_json::Value, String> {
        let body = serde_json::json!({
            "category": category,
            "title": title,
            "body": body_text,
        });
        self.request("POST", "/api/feedback", Some(body)).await
    }

    async fn get_my_feedback(&mut self, page: Option<i32>, per_page: Option<i32>) -> Result<serde_json::Value, String> {
        let p = page.unwrap_or(1);
        let pp = per_page.unwrap_or(10);
        let path = format!("/api/feedback?page={}&per_page={}", p, pp);
        self.request("GET", &path, None).await
    }

    async fn health(&self) -> Result<bool, String> {
        let resp = self.client
            .get(&self.url("/api/health"))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Ok(resp.status().is_success())
    }
}

// ── 공개 API ──

pub async fn init_client(server_url: &str) {
    let mut lock = CLIENT.lock().await;
    *lock = Some(DeskClient::new(server_url));
}

/// 저장된 연결 정보로 자동 복원 (앱 시작 시 호출)
pub async fn restore_connection() -> bool {
    let server_url = match database::get_desk_config("server_url") {
        Some(u) if !u.is_empty() => u,
        _ => return false,
    };
    let refresh_token = match database::get_desk_config("refresh_token") {
        Some(t) if !t.is_empty() => t,
        _ => return false,
    };

    let mut client = DeskClient::new(&server_url);
    client.refresh_token = refresh_token;

    match client.refresh_auth().await {
        Ok(true) => {
            // 갱신된 토큰 다시 저장
            database::set_desk_config("access_token", &client.access_token);
            database::set_desk_config("refresh_token", &client.refresh_token);
            let mut lock = CLIENT.lock().await;
            *lock = Some(client);
            log::info!("Desk 연결 자동 복원 완료");

            // 복원 성공 후 오프라인 큐 flush
            drop(lock);
            flush_feedback_outbox().await;

            true
        }
        _ => {
            log::warn!("Desk 연결 자동 복원 실패");
            false
        }
    }
}

/// 연결 정보를 로컬 DB에 저장
fn persist_connection(server_url: &str, access_token: &str, refresh_token: &str) {
    database::set_desk_config("server_url", server_url);
    database::set_desk_config("access_token", access_token);
    database::set_desk_config("refresh_token", refresh_token);
}

pub async fn join(code: &str, name: &str, device_name: &str) -> Result<bool, String> {
    let mut lock = CLIENT.lock().await;
    match lock.as_mut() {
        Some(client) => {
            let result = client.join(code, name, device_name).await?;
            persist_connection(&client.server_url, &client.access_token, &client.refresh_token);
            // member_code 저장 (MQTT 연결 시 사용)
            database::set_desk_config("member_code", code);

            // MQTT 연결 시도
            let host = extract_host(&client.server_url);
            let code_owned = code.to_string();
            tokio::spawn(async move {
                crate::mqtt_client::connect(&host, 1883, &code_owned, &code_owned, &code_owned)
                    .await
                    .ok();
            });

            Ok(result)
        }
        None => Err("Desk 클라이언트 미초기화".into()),
    }
}

/// URL에서 host 부분만 추출
fn extract_host(url: &str) -> String {
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    without_scheme
        .split(':')
        .next()
        .unwrap_or(without_scheme)
        .split('/')
        .next()
        .unwrap_or(without_scheme)
        .to_string()
}

pub async fn health() -> Result<bool, String> {
    let lock = CLIENT.lock().await;
    match lock.as_ref() {
        Some(client) => client.health().await,
        None => Ok(false),
    }
}

pub async fn is_connected() -> bool {
    let lock = CLIENT.lock().await;
    match lock.as_ref() {
        Some(client) => !client.access_token.is_empty(),
        None => false,
    }
}

pub async fn report_error(error_type: &str, message: &str) {
    let lock = CLIENT.lock().await;
    if let Some(client) = lock.as_ref() {
        client.post_error(error_type, message, env!("CARGO_PKG_VERSION")).await.ok();
    }
}

pub async fn submit_feedback(category: &str, title: &str, body: &str) -> Result<serde_json::Value, String> {
    let mut lock = CLIENT.lock().await;
    match lock.as_mut() {
        Some(client) => client.submit_feedback(category, title, body).await,
        None => Err("Desk 미연결".into()),
    }
}

pub async fn get_my_feedback(page: Option<i32>, per_page: Option<i32>) -> Result<serde_json::Value, String> {
    let mut lock = CLIENT.lock().await;
    match lock.as_mut() {
        Some(client) => client.get_my_feedback(page, per_page).await,
        None => Err("Desk 미연결".into()),
    }
}

pub async fn disconnect() {
    // MQTT 먼저 끊기
    crate::mqtt_client::disconnect().await;

    let mut lock = CLIENT.lock().await;
    *lock = None;
    database::clear_desk_config();
    // 연결 해제 시 해당 서버의 로컬 메시지/아웃박스도 정리
    database::clear_local_messages();
}

pub async fn get_server_url() -> Option<String> {
    let lock = CLIENT.lock().await;
    lock.as_ref().map(|c| c.server_url.clone())
}

// ── DM API ──

pub async fn send_dm(target_code: &str, body: &str) -> Result<serde_json::Value, String> {
    let mut lock = CLIENT.lock().await;
    match lock.as_mut() {
        Some(client) => {
            let payload = serde_json::json!({
                "target_code": target_code,
                "body": body,
            });
            client.request("POST", "/api/dm", Some(payload)).await
        }
        None => Err("Desk 미연결".into()),
    }
}

pub async fn dm_delivered(msg_id: &str) -> Result<(), String> {
    let mut lock = CLIENT.lock().await;
    match lock.as_mut() {
        Some(client) => {
            let path = format!("/api/dm/{}/delivered", msg_id);
            client.request("POST", &path, None).await?;
            Ok(())
        }
        None => Err("Desk 미연결".into()),
    }
}

pub async fn dm_read(msg_id: &str) -> Result<(), String> {
    let mut lock = CLIENT.lock().await;
    match lock.as_mut() {
        Some(client) => {
            let path = format!("/api/dm/{}/read", msg_id);
            client.request("POST", &path, None).await?;
            Ok(())
        }
        None => Err("Desk 미연결".into()),
    }
}

pub async fn get_conversations() -> Result<serde_json::Value, String> {
    let mut lock = CLIENT.lock().await;
    match lock.as_mut() {
        Some(client) => client.request("GET", "/api/dm/conversations", None).await,
        None => Err("Desk 미연결".into()),
    }
}

pub async fn get_conversation_messages(conv_id: &str, limit: Option<i32>, offset: Option<i32>) -> Result<serde_json::Value, String> {
    let mut lock = CLIENT.lock().await;
    match lock.as_mut() {
        Some(client) => {
            let lim = limit.unwrap_or(50);
            let off = offset.unwrap_or(0);
            let path = format!("/api/dm/conversations/{}/messages?limit={}&offset={}", conv_id, lim, off);
            client.request("GET", &path, None).await
        }
        None => Err("Desk 미연결".into()),
    }
}

pub async fn get_contacts_list() -> Result<serde_json::Value, String> {
    let mut lock = CLIENT.lock().await;
    match lock.as_mut() {
        Some(client) => client.request("GET", "/api/contacts", None).await,
        None => Err("Desk 미연결".into()),
    }
}

// ── E2E 암호화 공개키 API ──

pub async fn upload_public_key(public_key: &str) -> Result<(), String> {
    let mut lock = CLIENT.lock().await;
    match lock.as_mut() {
        Some(client) => {
            let body = serde_json::json!({ "public_key": public_key });
            client.request("PUT", "/api/contacts/public-key", Some(body)).await?;
            Ok(())
        }
        None => Err("Desk 미연결".into()),
    }
}

pub async fn get_public_key(code: &str) -> Result<String, String> {
    let mut lock = CLIENT.lock().await;
    match lock.as_mut() {
        Some(client) => {
            let path = format!("/api/contacts/{}/public-key", code);
            let resp = client.request("GET", &path, None).await?;
            resp["public_key"].as_str().map(|s| s.to_string())
                .ok_or("공개키 없음".into())
        }
        None => Err("Desk 미연결".into()),
    }
}

pub async fn send_dm_encrypted(
    target_code: &str,
    body: &str,
    ephemeral_key: &str,
    nonce: &str,
) -> Result<serde_json::Value, String> {
    let mut lock = CLIENT.lock().await;
    match lock.as_mut() {
        Some(client) => {
            let payload = serde_json::json!({
                "target_code": target_code,
                "body": body,
                "ephemeral_key": ephemeral_key,
                "nonce": nonce,
            });
            client.request("POST", "/api/dm", Some(payload)).await
        }
        None => Err("Desk 미연결".into()),
    }
}

/// 오프라인 피드백 큐를 서버로 전송
pub async fn flush_feedback_outbox() {
    let items = database::get_feedback_outbox();
    if items.is_empty() {
        return;
    }
    log::info!("피드백 outbox flush: {}건", items.len());
    for (id, cat, title, body) in items {
        match submit_feedback(&cat, &title, &body).await {
            Ok(_) => {
                database::delete_feedback_outbox(id);
                log::info!("피드백 outbox #{} 전송 성공", id);
            }
            Err(e) => {
                log::warn!("피드백 outbox #{} 전송 실패: {}", id, e);
                break; // 하나 실패하면 나머지는 다음 기회에
            }
        }
    }
}
