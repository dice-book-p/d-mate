use once_cell::sync::Lazy;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};

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

impl DeskClient {
    fn new(server_url: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap();
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

    async fn get_my_feedback(&mut self) -> Result<serde_json::Value, String> {
        self.request("GET", "/api/feedback", None).await
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

pub async fn join(code: &str, name: &str, device_name: &str) -> Result<bool, String> {
    let mut lock = CLIENT.lock().await;
    match lock.as_mut() {
        Some(client) => client.join(code, name, device_name).await,
        None => Err("Desk 클라이언트 미초기화".into()),
    }
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

pub async fn get_my_feedback() -> Result<serde_json::Value, String> {
    let mut lock = CLIENT.lock().await;
    match lock.as_mut() {
        Some(client) => client.get_my_feedback().await,
        None => Err("Desk 미연결".into()),
    }
}
