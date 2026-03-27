use serde_json::Value;
use tauri::{AppHandle, Window};

use crate::{alert_hub, crypto, database, desk_client, keyring_store, models::*, mqtt_client, scheduler, swork_client};

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

    let (mut my_overdue_tasks, mut my_deadline_tasks) = (vec![], vec![]);
    let (mut approval_request_tasks, mut overdue_task_tasks) = (vec![], vec![]);

    if !username.is_empty() && keyring_store::has_initial_setup() {
        // worker tasks
        if let Ok(tasks) = swork_client::fetch_worker_tasks().await {
            alert_hub::swork_ok();
            my_overdue_tasks = crate::notification_rules::filter_my_overdue(&tasks, &username);
            my_deadline_tasks = crate::notification_rules::filter_my_deadline(&tasks, &username);
        }

        // manager tasks
        if let Ok(tasks) = swork_client::fetch_manager_tasks().await {
            alert_hub::swork_ok();
            approval_request_tasks = crate::notification_rules::filter_approval_request(&tasks, &username);
            overdue_task_tasks = crate::notification_rules::filter_overdue_task(&tasks, &username);
        }
    }

    Ok(DashboardData {
        my_overdue_tasks,
        my_deadline_tasks,
        approval_request_tasks,
        overdue_task_tasks,
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
    let current = env!("CARGO_PKG_VERSION");

    // 1차: Desk 서버에서 업데이트 체크 (강제 업데이트 포함)
    if let Some(server_url) = desk_client::get_server_url().await {
        let url = format!("{}/api/update/check?version={}", server_url, current);
        let client = reqwest::Client::new();
        if let Ok(resp) = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
        {
            if let Ok(data) = resp.json::<Value>().await {
                if data["ok"].as_bool() == Some(true) {
                    return Ok(data["data"].clone());
                }
            }
        }
    }

    // 2차: 폴백 — Desk 미연결 시 기본값
    Ok(serde_json::json!({
        "available": false,
        "force": false,
        "current": current,
    }))
}

#[tauri::command]
pub async fn report_error(error_message: String, context: String) -> Result<ApiResponse, String> {
    let settings = database::get_settings();
    if settings.error_reporting == 0 {
        return Ok(ApiResponse { ok: false, message: "에러 리포팅 비활성화".into() });
    }

    // Desk 서버로 에러 리포트 전송
    let msg = format!("{} | context: {}", error_message, context);
    desk_client::report_error("app_error", &msg).await;

    Ok(ApiResponse { ok: true, message: "에러 리포트 전송됨".into() })
}

#[tauri::command]
pub async fn get_hostname() -> Result<String, String> {
    Ok(hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string()))
}

// ── Desk ──

#[tauri::command]
pub async fn desk_join(server_url: String, code: String, name: String, device_name: String) -> Result<ApiResponse, String> {
    desk_client::init_client(&server_url).await;
    match desk_client::join(&code, &name, &device_name).await {
        Ok(true) => Ok(ApiResponse { ok: true, message: "Desk 연결 성공".into() }),
        Ok(false) => Ok(ApiResponse { ok: false, message: "연결 실패".into() }),
        Err(e) => Ok(ApiResponse { ok: false, message: e }),
    }
}

#[tauri::command]
pub async fn desk_health() -> Result<Value, String> {
    let connected = desk_client::is_connected().await;
    let reachable = desk_client::health().await.unwrap_or(false);
    let server_url = desk_client::get_server_url().await.unwrap_or_default();
    Ok(serde_json::json!({
        "connected": connected,
        "reachable": reachable,
        "server_url": server_url,
    }))
}

#[tauri::command]
pub async fn desk_disconnect() -> Result<ApiResponse, String> {
    desk_client::disconnect().await;
    Ok(ApiResponse { ok: true, message: "Desk 연결이 해제되었습니다.".into() })
}

#[tauri::command]
pub async fn desk_submit_feedback(category: String, title: String, body: String) -> Result<ApiResponse, String> {
    match desk_client::submit_feedback(&category, &title, &body).await {
        Ok(_) => Ok(ApiResponse { ok: true, message: "피드백이 전송되었습니다.".into() }),
        Err(_) => {
            // 오프라인 큐에 저장
            database::save_feedback_outbox(&category, &title, &body);
            Ok(ApiResponse { ok: true, message: "서버 미연결 — 연결 시 자동 전송됩니다.".into() })
        }
    }
}

#[tauri::command]
pub async fn desk_get_feedback(page: Option<i32>, per_page: Option<i32>) -> Result<Value, String> {
    match desk_client::get_my_feedback(page, per_page).await {
        Ok(data) => Ok(data),
        Err(e) => Ok(serde_json::json!({"error": e})),
    }
}

#[tauri::command]
pub async fn desk_request_join(server_url: String, name: String, device_name: String) -> Result<Value, String> {
    let res = desk_client::submit_join_request(&server_url, &name, &device_name).await?;
    // request_id를 desk_config에 저장 (폴링용)
    if let Some(data) = res.get("data") {
        if let Some(id) = data.get("request_id").and_then(|v| v.as_i64()) {
            database::set_desk_config("join_request_id", &id.to_string());
            database::set_desk_config("join_server_url", &server_url);
        }
    }
    Ok(res)
}

#[tauri::command]
pub async fn desk_check_join_status() -> Result<Value, String> {
    let server_url = database::get_desk_config("join_server_url").unwrap_or_default();
    let request_id: i64 = database::get_desk_config("join_request_id")
        .unwrap_or_default()
        .parse()
        .unwrap_or(0);

    if server_url.is_empty() || request_id == 0 {
        return Ok(serde_json::json!({"ok": false, "message": "요청 정보 없음"}));
    }

    let res = desk_client::check_join_status(&server_url, request_id).await?;

    // approved 상태면 자동으로 Desk 연결 (토큰 저장 + MQTT 연결)
    if let Some(data) = res.get("data") {
        if data.get("status").and_then(|v| v.as_str()) == Some("approved") {
            if let (Some(token), Some(refresh)) = (
                data.get("access_token").and_then(|v| v.as_str()),
                data.get("refresh_token").and_then(|v| v.as_str()),
            ) {
                // 연결 정보 저장
                database::set_desk_config("server_url", &server_url);
                database::set_desk_config("access_token", token);
                database::set_desk_config("refresh_token", refresh);
                if let Some(code) = data.get("code").and_then(|v| v.as_str()) {
                    database::set_desk_config("member_code", code);
                }
                // Desk 클라이언트 초기화
                desk_client::init_client(&server_url).await;
                // MQTT 연결
                let host = crate::extract_mqtt_host(&server_url);
                let code = data.get("code").and_then(|v| v.as_str()).unwrap_or("").to_string();
                tokio::spawn(async move {
                    crate::mqtt_client::connect(&host, 1883, &code, &code, &code).await.ok();
                });
                // 요청 정보 정리
                database::delete_desk_config("join_request_id");
                database::delete_desk_config("join_server_url");
            }
        }
    }

    Ok(res)
}

#[tauri::command]
pub async fn desk_cancel_join_request() -> Result<Value, String> {
    database::delete_desk_config("join_request_id");
    database::delete_desk_config("join_server_url");
    Ok(serde_json::json!({"ok": true, "message": "요청이 취소되었습니다."}))
}

// ── Messages / MQTT ──

#[tauri::command]
pub async fn get_messages(conversation_id: Option<String>, limit: Option<i32>, offset: Option<i32>) -> Result<Value, String> {
    let lim = limit.unwrap_or(50);
    let off = offset.unwrap_or(0);
    let messages = match conversation_id {
        Some(cid) if !cid.is_empty() => database::get_local_messages(&cid, lim, off),
        _ => database::get_all_local_messages(lim, off),
    };
    Ok(serde_json::json!(messages))
}

#[tauri::command]
pub async fn mark_message_read(id: String) -> Result<ApiResponse, String> {
    database::mark_message_read(&id);
    Ok(ApiResponse { ok: true, message: "읽음 처리 완료".into() })
}

#[tauri::command]
pub async fn mark_all_read() -> Result<ApiResponse, String> {
    database::mark_all_messages_read();
    Ok(ApiResponse { ok: true, message: "전체 읽음 처리 완료".into() })
}

#[tauri::command]
pub async fn get_unread_count() -> Result<Value, String> {
    let count = database::get_unread_count();
    Ok(serde_json::json!({"count": count}))
}

#[tauri::command]
pub async fn get_contacts() -> Result<Value, String> {
    if !desk_client::is_connected().await {
        return Ok(serde_json::json!({"error": "Desk 미연결"}));
    }
    match desk_client::get_contacts_list().await {
        Ok(data) => Ok(data),
        Err(e) => Ok(serde_json::json!({"error": e})),
    }
}

#[tauri::command]
pub async fn send_dm(target_code: String, body: String) -> Result<ApiResponse, String> {
    // 1. 상대방 공개키 조회 시도
    match desk_client::get_public_key(&target_code).await {
        Ok(target_pubkey) => {
            // 2. 메시지 암호화
            let (ciphertext, ephemeral_key, nonce) = crypto::encrypt_message(&target_pubkey, &body)
                .map_err(|e| format!("암호화 실패: {}", e))?;
            // 3. 암호문 전송
            match desk_client::send_dm_encrypted(&target_code, &ciphertext, &ephemeral_key, &nonce).await {
                Ok(_) => Ok(ApiResponse { ok: true, message: "전송 완료".into() }),
                Err(e) => Ok(ApiResponse { ok: false, message: e }),
            }
        }
        Err(_) => {
            // 공개키 없으면 평문 전송 (하위 호환)
            match desk_client::send_dm(&target_code, &body).await {
                Ok(_) => Ok(ApiResponse { ok: true, message: "전송 완료".into() }),
                Err(e) => Ok(ApiResponse { ok: false, message: e }),
            }
        }
    }
}

#[tauri::command]
pub async fn init_encryption() -> Result<ApiResponse, String> {
    // 키 쌍이 없으면 생성
    if !database::has_keypair() {
        let (private_key, public_key) = crypto::generate_keypair();
        database::save_keypair(&private_key, &public_key);
    }
    // Desk 연결 중이면 공개키 업로드
    let (_, pub_key) = database::get_keypair().ok_or("키 없음")?;
    if desk_client::is_connected().await {
        desk_client::upload_public_key(&pub_key).await?;
    }
    Ok(ApiResponse { ok: true, message: "암호화 키 초기화 완료".into() })
}

#[tauri::command]
pub async fn get_conversations() -> Result<Value, String> {
    match desk_client::get_conversations().await {
        Ok(data) => Ok(data),
        Err(e) => Ok(serde_json::json!({"error": e})),
    }
}

#[tauri::command]
pub async fn get_conversation_messages(conv_id: String, limit: Option<i32>, offset: Option<i32>) -> Result<Value, String> {
    match desk_client::get_conversation_messages(&conv_id, limit, offset).await {
        Ok(data) => Ok(data),
        Err(e) => Ok(serde_json::json!({"error": e})),
    }
}

#[tauri::command]
pub async fn dm_delivered(msg_id: String) -> Result<ApiResponse, String> {
    match desk_client::dm_delivered(&msg_id).await {
        Ok(_) => Ok(ApiResponse { ok: true, message: "전달 확인".into() }),
        Err(e) => Ok(ApiResponse { ok: false, message: e }),
    }
}

#[tauri::command]
pub async fn dm_read(msg_id: String) -> Result<ApiResponse, String> {
    match desk_client::dm_read(&msg_id).await {
        Ok(_) => Ok(ApiResponse { ok: true, message: "읽음 확인".into() }),
        Err(e) => Ok(ApiResponse { ok: false, message: e }),
    }
}

#[tauri::command]
pub async fn mqtt_status() -> Result<Value, String> {
    Ok(serde_json::json!({
        "connected": mqtt_client::is_connected(),
    }))
}

#[tauri::command]
pub async fn mqtt_reconnect() -> Result<ApiResponse, String> {
    match mqtt_client::reconnect().await {
        Ok(_) => Ok(ApiResponse { ok: true, message: "MQTT 재연결 성공".into() }),
        Err(e) => Ok(ApiResponse { ok: false, message: format!("MQTT 재연결 실패: {}", e) }),
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
