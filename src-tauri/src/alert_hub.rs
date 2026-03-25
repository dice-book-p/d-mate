use serde::Serialize;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use chrono::Local;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum AlertLevel {
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct Alert {
    pub id: String,
    pub level: AlertLevel,
    pub source: String,
    pub title: String,
    pub message: String,
    pub action: String,      // "navigate:swork", "navigate:mail", "navigate:system"
    pub timestamp: String,
}

static ALERTS: Lazy<Mutex<Vec<Alert>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// 알림 등록 (동일 id면 갱신)
pub fn push(id: &str, level: AlertLevel, source: &str, title: &str, message: &str, action: &str) {
    let mut alerts = ALERTS.lock().unwrap();
    let now = Local::now().format("%H:%M:%S").to_string();

    let alert = Alert {
        id: id.to_string(),
        level,
        source: source.to_string(),
        title: title.to_string(),
        message: message.to_string(),
        action: action.to_string(),
        timestamp: now,
    };

    if let Some(existing) = alerts.iter_mut().find(|a| a.id == id) {
        *existing = alert;
    } else {
        alerts.push(alert);
    }
}

/// 알림 해소
pub fn resolve(id: &str) {
    let mut alerts = ALERTS.lock().unwrap();
    alerts.retain(|a| a.id != id);
}

/// 특정 source의 모든 알림 해소
pub fn resolve_source(source: &str) {
    let mut alerts = ALERTS.lock().unwrap();
    alerts.retain(|a| a.source != source);
}

/// 현재 활성 알림 목록
pub fn get_all() -> Vec<Alert> {
    ALERTS.lock().unwrap().clone()
}

/// 활성 알림 수
pub fn count() -> usize {
    ALERTS.lock().unwrap().len()
}

/// error 레벨 알림 존재 여부
pub fn has_errors() -> bool {
    ALERTS.lock().unwrap().iter().any(|a| a.level == AlertLevel::Error)
}

/// 전체 초기화
pub fn clear_all() {
    ALERTS.lock().unwrap().clear();
}

// ── 편의 함수: checker에서 호출 ──────────────────

pub fn swork_ok() {
    resolve("swork_auth");
    resolve("swork_server");
}

pub fn swork_auth_error(msg: &str) {
    push("swork_auth", AlertLevel::Error, "swork",
        "swork 로그인 실패",
        &format!("아이디/비밀번호를 확인하세요. ({})", msg),
        "navigate:swork");
}

pub fn swork_server_error(msg: &str) {
    push("swork_server", AlertLevel::Warning, "swork",
        "swork 서버 연결 실패",
        &format!("일시적 오류일 수 있습니다. ({})", msg),
        "navigate:swork");
}

pub fn swork_tg_ok() {
    resolve("swork_tg_auth");
    resolve("swork_tg_send");
}

pub fn swork_tg_error(msg: &str) {
    push("swork_tg_send", AlertLevel::Error, "swork_tg",
        "SWORK 텔레그램 발송 실패",
        &format!("봇 토큰/채팅ID를 확인하세요. ({})", msg),
        "navigate:swork");
}

pub fn mail_ok() {
    resolve("mail_auth");
    resolve("mail_server");
}

pub fn mail_auth_error(msg: &str) {
    push("mail_auth", AlertLevel::Error, "mail",
        "메일 로그인 실패",
        &format!("이메일 계정/비밀번호를 확인하세요. ({})", msg),
        "navigate:mail");
}

pub fn mail_server_error(msg: &str) {
    push("mail_server", AlertLevel::Warning, "mail",
        "메일 서버 연결 실패",
        &format!("일시적 오류일 수 있습니다. ({})", msg),
        "navigate:mail");
}

pub fn mail_tg_ok() {
    resolve("mail_tg_auth");
    resolve("mail_tg_send");
}

pub fn mail_tg_error(msg: &str) {
    push("mail_tg_send", AlertLevel::Error, "mail_tg",
        "메일 텔레그램 발송 실패",
        &format!("봇 토큰/채팅ID를 확인하세요. ({})", msg),
        "navigate:mail");
}
