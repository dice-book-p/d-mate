use std::sync::Arc;
use once_cell::sync::Lazy;
use reqwest::cookie::Jar;
use tokio::sync::Mutex;

use crate::models::Task;

static CLIENT: Lazy<Mutex<Option<SworkClient>>> = Lazy::new(|| Mutex::new(None));

struct SworkClient {
    client: reqwest::Client,
    username: String,
    password: String,
    logged_in: bool,
}

impl SworkClient {
    fn new(username: &str, password: &str) -> Self {
        let jar = Arc::new(Jar::default());
        let client = reqwest::Client::builder()
            .cookie_provider(jar)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            username: username.to_string(),
            password: password.to_string(),
            logged_in: false,
        }
    }

    async fn login(&mut self) -> Result<bool, String> {
        log::info!("swork login attempt for: {}", self.username);
        let resp = self
            .client
            .post("https://swork.kr/auth/login")
            .form(&[
                ("username", self.username.as_str()),
                ("password", self.password.as_str()),
            ])
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = resp.status().as_u16();
        self.logged_in = status == 200 || status == 302;
        log::info!("swork login result: status={}, logged_in={}", status, self.logged_in);
        Ok(self.logged_in)
    }

    async fn refresh(&mut self) -> Result<bool, String> {
        let resp = self
            .client
            .post("https://swork.kr/auth/refresh")
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => Ok(true),
            _ => self.login().await,
        }
    }

    async fn fetch_tasks(&mut self, task_type: &str) -> Result<Vec<Task>, String> {
        if !self.logged_in {
            self.login().await?;
        }

        let url = format!("https://swork.kr/my-tasks/api/tasks?type={}", task_type);
        log::info!("Fetching {} tasks...", task_type);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if resp.status().as_u16() == 401 {
            self.refresh().await?;
            let resp = self
                .client
                .get(&url)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            Self::parse_tasks(resp).await
        } else {
            Self::parse_tasks(resp).await
        }
    }

    async fn fetch_manager_tasks(&mut self) -> Result<Vec<Task>, String> {
        self.fetch_tasks("manager").await
    }

    async fn fetch_worker_tasks(&mut self) -> Result<Vec<Task>, String> {
        self.fetch_tasks("worker").await
    }

    async fn parse_tasks(resp: reqwest::Response) -> Result<Vec<Task>, String> {
        let status = resp.status().as_u16();
        let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

        log::info!("swork API response status={}, data_type={}", status,
            if data.is_array() { "array" }
            else if data.is_object() { "object" }
            else { "other" });

        if let Some(tasks) = data.as_array() {
            let result: Vec<Task> = tasks
                .iter()
                .filter_map(|t| serde_json::from_value(t.clone()).ok())
                .collect();
            log::info!("Parsed {} tasks (direct array)", result.len());
            Ok(result)
        } else if let Some(tasks) = data.get("tasks").and_then(|v| v.as_array()) {
            let result: Vec<Task> = tasks
                .iter()
                .filter_map(|t| serde_json::from_value(t.clone()).ok())
                .collect();
            log::info!("Parsed {} tasks (nested)", result.len());
            Ok(result)
        } else {
            log::warn!("No tasks found in response: {}", &data.to_string()[..200.min(data.to_string().len())]);
            Ok(vec![])
        }
    }
}

pub async fn init_client(username: &str, password: &str) {
    log::info!("Initializing swork client for user: {}", username);
    let mut lock = CLIENT.lock().await;
    *lock = Some(SworkClient::new(username, password));
}

pub async fn fetch_manager_tasks() -> Result<Vec<Task>, String> {
    let mut lock = CLIENT.lock().await;
    match lock.as_mut() {
        Some(client) => client.fetch_manager_tasks().await,
        None => Err("swork 클라이언트가 초기화되지 않았습니다.".into()),
    }
}

pub async fn fetch_worker_tasks() -> Result<Vec<Task>, String> {
    let mut lock = CLIENT.lock().await;
    match lock.as_mut() {
        Some(client) => client.fetch_worker_tasks().await,
        None => Err("swork 클라이언트가 초기화되지 않았습니다.".into()),
    }
}

pub async fn verify_login(username: &str, password: &str) -> Result<bool, String> {
    let jar = Arc::new(Jar::default());
    let client = reqwest::Client::builder()
        .cookie_provider(jar)
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post("https://swork.kr/auth/login")
        .form(&[("username", username), ("password", password)])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = resp.status().as_u16();
    Ok(status == 200 || status == 302)
}
