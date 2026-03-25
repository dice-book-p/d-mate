use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use once_cell::sync::Lazy;

const SERVICE: &str = "d-mate";
const ACCOUNT: &str = "credentials";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Credentials {
    swork_username: String,
    swork_password: String,
    swork_tg_token: String,
    swork_tg_chat_id: String,
    mail_tg_token: String,
    mail_tg_chat_id: String,
    mail_password: String,
}

static CACHE: Lazy<Mutex<Credentials>> = Lazy::new(|| {
    Mutex::new(load_from_keyring())
});

fn load_from_keyring() -> Credentials {
    match Entry::new(SERVICE, ACCOUNT) {
        Ok(entry) => match entry.get_password() {
            Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
            Err(keyring::Error::NoEntry) => Credentials::default(),
            Err(e) => {
                log::warn!("Keyring load failed: {:?}", e);
                Credentials::default()
            }
        },
        Err(e) => {
            log::error!("Keyring entry creation failed: {:?}", e);
            Credentials::default()
        }
    }
}

fn save_to_keyring(creds: &Credentials) {
    match Entry::new(SERVICE, ACCOUNT) {
        Ok(entry) => {
            let json = serde_json::to_string(creds).unwrap_or_default();
            if let Err(e) = entry.set_password(&json) {
                log::error!("Keyring save failed: {:?}", e);
            }
        }
        Err(e) => {
            log::error!("Keyring entry creation failed: {:?}", e);
        }
    }
}

fn with_creds<F, R>(f: F) -> R
where
    F: FnOnce(&Credentials) -> R,
{
    let cache = CACHE.lock().unwrap();
    f(&cache)
}

fn update_creds<F>(f: F)
where
    F: FnOnce(&mut Credentials),
{
    let mut cache = CACHE.lock().unwrap();
    f(&mut cache);
    save_to_keyring(&cache);
}

// ── swork 계정 ───────────────────────────────────
pub fn get_swork_username() -> String {
    with_creds(|c| c.swork_username.clone())
}
pub fn get_swork_password() -> String {
    with_creds(|c| c.swork_password.clone())
}
pub fn set_swork_credentials(username: &str, password: &str) {
    update_creds(|c| {
        c.swork_username = username.to_string();
        c.swork_password = password.to_string();
    });
}

// ── swork 텔레그램 봇 ───────────────────────────
pub fn get_swork_tg_token() -> String {
    with_creds(|c| c.swork_tg_token.clone())
}
pub fn get_swork_tg_chat_id() -> String {
    with_creds(|c| c.swork_tg_chat_id.clone())
}
pub fn set_swork_tg(token: &str, chat_id: &str) {
    update_creds(|c| {
        c.swork_tg_token = token.to_string();
        c.swork_tg_chat_id = chat_id.to_string();
    });
}

// ── 메일 텔레그램 봇 ────────────────────────────
pub fn get_mail_tg_token() -> String {
    with_creds(|c| c.mail_tg_token.clone())
}
pub fn get_mail_tg_chat_id() -> String {
    with_creds(|c| c.mail_tg_chat_id.clone())
}
pub fn set_mail_tg(token: &str, chat_id: &str) {
    update_creds(|c| {
        c.mail_tg_token = token.to_string();
        c.mail_tg_chat_id = chat_id.to_string();
    });
}

// ── 메일 계정 ────────────────────────────────────
pub fn get_mail_password() -> String {
    with_creds(|c| c.mail_password.clone())
}
pub fn set_mail_password(password: &str) {
    update_creds(|c| {
        c.mail_password = password.to_string();
    });
}

// ── 초기 설정 확인 ──────────────────────────────
pub fn has_initial_setup() -> bool {
    with_creds(|c| {
        let swork_ok = !c.swork_tg_token.is_empty() && !c.swork_tg_chat_id.is_empty();
        let mail_ok = !c.mail_tg_token.is_empty() && !c.mail_tg_chat_id.is_empty();
        swork_ok || mail_ok
    })
}

// ── 개별 삭제 ────────────────────────────────────
pub fn clear_swork() {
    update_creds(|c| {
        c.swork_username.clear();
        c.swork_password.clear();
    });
}

pub fn clear_swork_tg() {
    update_creds(|c| {
        c.swork_tg_token.clear();
        c.swork_tg_chat_id.clear();
    });
}

pub fn clear_mail() {
    update_creds(|c| {
        c.mail_password.clear();
    });
}

pub fn clear_mail_tg() {
    update_creds(|c| {
        c.mail_tg_token.clear();
        c.mail_tg_chat_id.clear();
    });
}

// ── 전체 삭제 ────────────────────────────────────
pub fn clear_all() {
    {
        let mut cache = CACHE.lock().unwrap();
        *cache = Credentials::default();
    }
    if let Ok(entry) = Entry::new(SERVICE, ACCOUNT) {
        let _ = entry.delete_credential();
    }
}
