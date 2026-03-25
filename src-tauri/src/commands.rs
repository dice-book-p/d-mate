use serde_json::Value;
use tauri::{AppHandle, Window};

use crate::{alert_hub, database, keyring_store, models::*, scheduler, swork_client};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[tauri::command]
pub async fn get_settings() -> Result<Value, String> {
    let s = database::get_settings();
    let mut resp = serde_json::to_value(&s).map_err(|e| e.to_string())?;
    let obj = resp.as_object_mut().unwrap();

    obj.insert("app_version".into(), APP_VERSION.into());
    obj.insert("swork_username".into(), keyring_store::get_swork_username().into());
    obj.insert("has_swork_password".into(), (!keyring_store::get_swork_password().is_empty()).into());
    obj.insert("swork_tg_token".into(), keyring_store::get_swork_tg_token().into());
    obj.insert("swork_tg_chat_id".into(), keyring_store::get_swork_tg_chat_id().into());
    obj.insert("mail_tg_token".into(), keyring_store::get_mail_tg_token().into());
    obj.insert("mail_tg_chat_id".into(), keyring_store::get_mail_tg_chat_id().into());
    obj.insert("has_mail_password".into(), (!keyring_store::get_mail_password().is_empty()).into());
    obj.insert("mail_account_display".into(), keyring_store::get_swork_username().into());
    obj.insert("is_paused".into(), scheduler::is_paused().into());
    obj.insert("has_initial_setup".into(), keyring_store::has_initial_setup().into());

    // 활성 알림
    let alerts = alert_hub::get_all();
    obj.insert("alerts".into(), serde_json::to_value(&alerts).unwrap_or_default());
    obj.insert("alert_count".into(), alerts.len().into());

    Ok(resp)
}

#[tauri::command]
pub async fn save_settings(data: Value) -> Result<ApiResponse, String> {
    if let Some(obj) = data.as_object() {
        if let (Some(u), Some(p)) = (obj.get("swork_username"), obj.get("swork_password")) {
            let u = u.as_str().unwrap_or("");
            let p = p.as_str().unwrap_or("");
            if !u.is_empty() && !p.is_empty() {
                keyring_store::set_swork_credentials(u, p);
                swork_client::init_client(u, p).await;
            }
        }

        if let Some(t) = obj.get("swork_tg_token") {
            let token = t.as_str().unwrap_or("");
            let chat = obj.get("swork_tg_chat_id").and_then(|v| v.as_str()).unwrap_or("");
            keyring_store::set_swork_tg(token, chat);
        }

        if let Some(t) = obj.get("mail_tg_token") {
            let token = t.as_str().unwrap_or("");
            let chat = obj.get("mail_tg_chat_id").and_then(|v| v.as_str()).unwrap_or("");
            keyring_store::set_mail_tg(token, chat);
        }

        if let Some(mp) = obj.get("mail_password") {
            let pw = mp.as_str().unwrap_or("");
            if !pw.is_empty() {
                keyring_store::set_mail_password(pw);
            }
        }
    }

    database::update_settings(&data);

    if keyring_store::has_initial_setup() && !scheduler::is_running() {
        scheduler::start().await;
    }

    Ok(ApiResponse {
        ok: true,
        message: "설정이 저장되었습니다.".into(),
    })
}

#[tauri::command]
pub async fn verify_swork_login(username: String, password: String) -> Result<ApiResponse, String> {
    match swork_client::verify_login(&username, &password).await {
        Ok(true) => {
            alert_hub::swork_ok();
            Ok(ApiResponse { ok: true, message: "로그인 성공".into() })
        }
        Ok(false) => {
            alert_hub::swork_auth_error("아이디 또는 비밀번호 오류");
            Ok(ApiResponse { ok: false, message: "아이디 또는 비밀번호가 올바르지 않습니다.".into() })
        }
        Err(e) => {
            alert_hub::swork_server_error(&e);
            Ok(ApiResponse { ok: false, message: format!("연결 실패: {}", e) })
        }
    }
}

#[tauri::command]
pub async fn test_telegram(target: String, token: Option<String>, chat_id: Option<String>) -> Result<ApiResponse, String> {
    let (tok, cid) = if let (Some(t), Some(c)) = (token, chat_id) {
        if !t.is_empty() && !c.is_empty() { (t, c) }
        else if target == "mail" {
            (keyring_store::get_mail_tg_token(), keyring_store::get_mail_tg_chat_id())
        } else {
            (keyring_store::get_swork_tg_token(), keyring_store::get_swork_tg_chat_id())
        }
    } else if target == "mail" {
        (keyring_store::get_mail_tg_token(), keyring_store::get_mail_tg_chat_id())
    } else {
        (keyring_store::get_swork_tg_token(), keyring_store::get_swork_tg_chat_id())
    };

    if tok.is_empty() || cid.is_empty() {
        return Ok(ApiResponse {
            ok: false,
            message: "텔레그램 봇 토큰과 채팅 ID를 먼저 설정하세요.".into(),
        });
    }

    let label = if target == "mail" { "메일 알림" } else { "SWORK 알림" };
    let msg = format!("<b>[D-Mate]</b> {} 테스트 메시지입니다. ✅", label);
    let ok = crate::telegram::send_message(&tok, &cid, &msg).await;

    if ok {
        if target == "mail" { alert_hub::mail_tg_ok(); }
        else { alert_hub::swork_tg_ok(); }
        Ok(ApiResponse { ok: true, message: "발송 성공".into() })
    } else {
        if target == "mail" { alert_hub::mail_tg_error("테스트 발송 실패"); }
        else { alert_hub::swork_tg_error("테스트 발송 실패"); }
        Ok(ApiResponse { ok: false, message: "발송 실패. 토큰과 채팅ID를 확인하세요.".into() })
    }
}

#[tauri::command]
pub async fn lookup_telegram_chats(bot_token: String) -> Result<Value, String> {
    if bot_token.is_empty() {
        return Ok(serde_json::json!({"error": "봇 토큰을 입력하세요."}));
    }

    let url = format!("https://api.telegram.org/bot{}/getUpdates", bot_token);
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|_| "Telegram API 연결 실패".to_string())?;

    let data: Value = resp.json().await.map_err(|e| e.to_string())?;
    if !data["ok"].as_bool().unwrap_or(false) {
        return Ok(serde_json::json!({"error": "유효하지 않은 봇 토큰입니다."}));
    }

    let mut chats = std::collections::HashMap::<String, Value>::new();
    if let Some(results) = data["result"].as_array() {
        for update in results {
            let msg = update.get("message").or_else(|| update.get("channel_post"));
            if let Some(msg) = msg {
                if let Some(chat) = msg.get("chat") {
                    let chat_id = chat["id"].to_string();
                    let first = chat["first_name"].as_str().unwrap_or("");
                    let last = chat["last_name"].as_str().unwrap_or("");
                    let name = format!("{} {}", first, last).trim().to_string();
                    let title = chat["title"]
                        .as_str()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| if name.is_empty() { "-".into() } else { name });
                    let chat_type = chat["type"].as_str().unwrap_or("");

                    chats.insert(
                        chat_id.clone(),
                        serde_json::json!({
                            "chat_id": chat_id,
                            "type": chat_type,
                            "title": title,
                        }),
                    );
                }
            }
        }
    }

    if chats.is_empty() {
        return Ok(serde_json::json!({"error": "조회된 채팅이 없습니다. 봇에게 먼저 메시지를 보내주세요."}));
    }

    let chat_list: Vec<Value> = chats.into_values().collect();
    Ok(serde_json::json!({"chats": chat_list}))
}

#[tauri::command]
pub async fn get_dashboard_data() -> Result<DashboardData, String> {
    let settings = database::get_settings();
    let recent_logs = database::get_recent_notifications(50);
    let username = keyring_store::get_swork_username();

    let (rule1_tasks, rule2_tasks) = if !username.is_empty() && keyring_store::has_initial_setup() {
        match swork_client::fetch_manager_tasks().await {
            Ok(tasks) => {
                alert_hub::swork_ok();
                let r1 = crate::notification_rules::filter_rule1(&tasks, &username);
                let r2 = crate::notification_rules::filter_rule2(&tasks, &username);
                (r1, r2)
            }
            Err(_) => (vec![], vec![]),
        }
    } else {
        (vec![], vec![])
    };

    Ok(DashboardData {
        rule1_tasks,
        rule2_tasks,
        recent_logs,
        error: String::new(),
        settings,
        is_paused: scheduler::is_paused(),
    })
}

#[tauri::command]
pub async fn get_alerts() -> Result<Value, String> {
    let alerts = alert_hub::get_all();
    Ok(serde_json::json!({
        "alerts": alerts,
        "count": alerts.len(),
        "has_errors": alert_hub::has_errors(),
    }))
}

#[tauri::command]
pub async fn trigger_check_now() -> Result<ApiResponse, String> {
    scheduler::trigger_now().await;
    Ok(ApiResponse { ok: true, message: "확인을 시작했습니다.".into() })
}

#[tauri::command]
pub async fn toggle_pause() -> Result<Value, String> {
    let paused = scheduler::toggle_pause();
    Ok(serde_json::json!({"paused": paused}))
}

#[tauri::command]
pub async fn reset_all_data() -> Result<ApiResponse, String> {
    scheduler::stop().await;
    database::clear_all_data();
    keyring_store::clear_all();
    alert_hub::clear_all();
    Ok(ApiResponse { ok: true, message: "모든 데이터가 초기화되었습니다.".into() })
}

#[tauri::command]
pub async fn verify_mail_login(server: String, port: i32, use_ssl: bool, account: String, password: String) -> Result<ApiResponse, String> {
    let pw = if password.is_empty() { keyring_store::get_mail_password() } else { password };
    if pw.is_empty() {
        return Ok(ApiResponse { ok: false, message: "비밀번호를 입력하세요.".into() });
    }
    match crate::mail_checker::fetch_new_mails(&server, port as u16, use_ssl, &account, &pw).await {
        Ok(_) => {
            alert_hub::mail_ok();
            Ok(ApiResponse { ok: true, message: "메일 서버 연결 성공".into() })
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.to_lowercase().contains("login") || msg.to_lowercase().contains("auth") || msg.to_lowercase().contains("pass") {
                alert_hub::mail_auth_error(&msg);
                Ok(ApiResponse { ok: false, message: format!("계정/비밀번호 오류: {}", msg) })
            } else {
                alert_hub::mail_server_error(&msg);
                Ok(ApiResponse { ok: false, message: format!("서버 연결 실패: {}", msg) })
            }
        }
    }
}

#[tauri::command]
pub async fn disconnect_service(target: String) -> Result<ApiResponse, String> {
    match target.as_str() {
        "swork" => {
            keyring_store::clear_swork();
            alert_hub::resolve_source("swork");
            // swork 클라이언트 초기화 해제는 자연스럽게 빈 credential로 처리
            log::info!("Disconnected: swork account");
        }
        "swork_tg" => {
            keyring_store::clear_swork_tg();
            alert_hub::resolve("swork_tg_send");
            alert_hub::resolve("swork_tg_auth");
            log::info!("Disconnected: swork telegram");
        }
        "mail" => {
            keyring_store::clear_mail();
            database::update_settings(&serde_json::json!({
                "mail_server": "",
                "mail_account": "",
                "mail_port": 110,
                "mail_notify_enabled": 0
            }));
            alert_hub::resolve_source("mail");
            log::info!("Disconnected: mail server");
        }
        "mail_tg" => {
            keyring_store::clear_mail_tg();
            alert_hub::resolve("mail_tg_send");
            alert_hub::resolve("mail_tg_auth");
            log::info!("Disconnected: mail telegram");
        }
        _ => {
            return Ok(ApiResponse { ok: false, message: "알 수 없는 서비스".into() });
        }
    }

    // 초기 설정이 모두 해제되면 스케줄러 중지
    if !keyring_store::has_initial_setup() && scheduler::is_running() {
        scheduler::stop().await;
        log::info!("Scheduler stopped: no initial setup");
    }

    Ok(ApiResponse { ok: true, message: "연결이 해제되었습니다.".into() })
}

#[tauri::command]
pub async fn set_autostart(app: AppHandle, enabled: bool) -> Result<ApiResponse, String> {
    use tauri_plugin_autostart::ManagerExt;
    let autostart = app.autolaunch();
    let result = if enabled { autostart.enable() } else { autostart.disable() };
    match result {
        Ok(_) => {
            database::update_settings(&serde_json::json!({"autostart": if enabled { 1 } else { 0 }}));
            Ok(ApiResponse { ok: true, message: if enabled { "자동 시작 활성화".into() } else { "자동 시작 비활성화".into() } })
        }
        Err(e) => Ok(ApiResponse { ok: false, message: format!("설정 실패: {}", e) }),
    }
}

#[tauri::command]
pub async fn check_update() -> Result<Value, String> {
    let settings = database::get_settings();
    let url = settings.update_server_url;
    if url.is_empty() {
        return Ok(serde_json::json!({"available": false, "message": "업데이트 서버가 설정되지 않았습니다."}));
    }

    let check_url = format!("{}/update/version.json", url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    match client.get(&check_url).timeout(std::time::Duration::from_secs(10)).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let data: Value = resp.json().await.unwrap_or_default();
                let latest = data["version"].as_str().unwrap_or("0.0.0");
                let current = env!("CARGO_PKG_VERSION");
                let available = latest > current;
                let os = if cfg!(target_os = "macos") { "mac" } else { "windows" };
                let download_url = data["download"][os].as_str().unwrap_or("");
                Ok(serde_json::json!({
                    "available": available,
                    "current": current,
                    "latest": latest,
                    "notes": data["notes"].as_str().unwrap_or(""),
                    "download_url": download_url,
                }))
            } else {
                Ok(serde_json::json!({"available": false, "message": "서버 응답 오류"}))
            }
        }
        Err(e) => Ok(serde_json::json!({"available": false, "message": format!("연결 실패: {}", e)})),
    }
}

#[tauri::command]
pub async fn report_error(error_message: String, context: String) -> Result<ApiResponse, String> {
    let settings = database::get_settings();
    if settings.error_reporting == 0 || settings.update_server_url.is_empty() {
        return Ok(ApiResponse { ok: false, message: "에러 리포팅 비활성화".into() });
    }

    let url = format!("{}/report/error", settings.update_server_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "app": "d-mate",
        "version": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "error": error_message,
        "context": context,
    });

    match client.post(&url).json(&body).timeout(std::time::Duration::from_secs(10)).send().await {
        Ok(resp) if resp.status().is_success() => {
            Ok(ApiResponse { ok: true, message: "에러 리포트 전송됨".into() })
        }
        _ => Ok(ApiResponse { ok: false, message: "전송 실패".into() }),
    }
}

#[tauri::command]
pub async fn hide_window(window: Window) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn quit_app(app: AppHandle) -> Result<(), String> {
    scheduler::stop().await;
    app.exit(0);
    Ok(())
}
