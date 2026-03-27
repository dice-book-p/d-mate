pub mod alert_hub;
mod checker;
mod commands;
mod crypto;
mod database;
mod desk_client;
mod keyring_store;
mod mail_checker;
mod models;
pub mod mqtt_client;
pub mod native_notify;
mod notification_rules;
mod scheduler;
mod swork_client;
mod telegram;

use serde_json::Value;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};

/// Extract hostname from a URL (e.g., "http://192.168.1.1:3000" -> "192.168.1.1")
fn extract_mqtt_host(url: &str) -> String {
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // 이미 실행 중이면 기존 창을 포커스
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            database::init_db();
            native_notify::init(app.handle().clone());
            log::info!("D-Mate database initialized");

            // swork 클라이언트 초기화
            let username = keyring_store::get_swork_username();
            let password = keyring_store::get_swork_password();
            if !username.is_empty() && !password.is_empty() {
                let u = username.clone();
                let p = password.clone();
                tauri::async_runtime::spawn(async move {
                    swork_client::init_client(&u, &p).await;
                });
            }

            // MQTT 핸들 초기화
            {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    mqtt_client::init(handle).await;
                });
            }

            // Desk 클라이언트 자동 복원 + MQTT 연결 + E2E 키 초기화
            tauri::async_runtime::spawn(async {
                let restored = desk_client::restore_connection().await;
                if restored {
                    // Desk 복원 성공 시 MQTT도 연결 시도
                    if let Some(server_url) = desk_client::get_server_url().await {
                        if let Some(code) = database::get_desk_config("member_code") {
                            let host = extract_mqtt_host(&server_url);
                            mqtt_client::connect(&host, 1883, &code, &code, &code).await.ok();
                        }
                    }
                    // E2E 암호화 키 초기화 + 공개키 업로드
                    if !database::has_keypair() {
                        let (private_key, public_key) = crypto::generate_keypair();
                        database::save_keypair(&private_key, &public_key);
                    }
                    if let Some((_, pub_key)) = database::get_keypair() {
                        desk_client::upload_public_key(&pub_key).await.ok();
                    }
                }
            });

            // Desk 복원 후 업데이트 체크 (5초 대기 후)
            {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

                    if let Some(server_url) = desk_client::get_server_url().await {
                        let current = env!("CARGO_PKG_VERSION");
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
                                    let update_data = data["data"].clone();
                                    if update_data["force"].as_bool() == Some(true) {
                                        handle.emit("update:force", &update_data).ok();
                                    } else if update_data["available"].as_bool() == Some(true) {
                                        handle.emit("update:available", &update_data).ok();
                                    }
                                }
                            }
                        }
                    }
                });
            }

            // 스케줄러 시작
            if keyring_store::has_initial_setup() {
                tauri::async_runtime::spawn(async {
                    scheduler::start().await;
                });
            }

            // 트레이: alert_hub + 미읽 메시지 수를 tooltip에 표시
            let alert_count = alert_hub::count();
            let unread_msg_count = database::get_unread_count();
            let tooltip = if unread_msg_count > 0 && alert_count > 0 {
                format!("D-Mate — 📬 {}건의 새 메시지, ⚠ {}건의 문제", unread_msg_count, alert_count)
            } else if unread_msg_count > 0 {
                format!("D-Mate — 📬 {}건의 새 메시지", unread_msg_count)
            } else if alert_count > 0 {
                format!("D-Mate — ⚠ {}건의 문제", alert_count)
            } else {
                "D-Mate — 업무 도우미".to_string()
            };

            let unread_label = if unread_msg_count > 0 {
                format!("새 메시지: {}건", unread_msg_count)
            } else {
                "새 메시지 없음".to_string()
            };

            let open_i = MenuItem::with_id(app, "open", "설정 열기", true, None::<&str>)?;
            let msg_i = MenuItem::with_id(app, "msg_info", &unread_label, false, None::<&str>)?;
            let check_i = MenuItem::with_id(app, "check", "지금 확인", true, None::<&str>)?;
            let pause_i = MenuItem::with_id(app, "pause", "일시 중지", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "종료", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&open_i, &msg_i, &check_i, &pause_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip(&tooltip)
                .menu(&menu)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "open" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "check" => {
                        tauri::async_runtime::spawn(async {
                            scheduler::trigger_now().await;
                        });
                    }
                    "pause" => {
                        let paused = scheduler::toggle_pause();
                        log::info!("Scheduler paused: {}", paused);
                    }
                    "quit" => {
                        tauri::async_runtime::spawn(async {
                            scheduler::stop().await;
                        });
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                })
                .build(app)?;

            log::info!("D-Mate system tray created");
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::verify_swork_login,
            commands::get_dashboard_data,
            commands::get_alerts,
            commands::test_telegram,
            commands::lookup_telegram_chats,
            commands::trigger_check_now,
            commands::toggle_pause,
            commands::verify_mail_login,
            commands::disconnect_service,
            commands::set_autostart,
            commands::check_update,
            commands::report_error,
            commands::reset_all_data,
            commands::desk_join,
            commands::desk_request_join,
            commands::desk_check_join_status,
            commands::desk_cancel_join_request,
            commands::desk_health,
            commands::desk_disconnect,
            commands::desk_submit_feedback,
            commands::desk_get_feedback,
            commands::get_messages,
            commands::mark_message_read,
            commands::mark_all_read,
            commands::get_unread_count,
            commands::get_contacts,
            commands::send_dm,
            commands::get_conversations,
            commands::get_conversation_messages,
            commands::dm_delivered,
            commands::dm_read,
            commands::init_encryption,
            commands::mqtt_status,
            commands::mqtt_reconnect,
            commands::get_hostname,
            commands::hide_window,
            commands::quit_app,
        ])
        .build(tauri::generate_context!())
        .expect("error while building D-Mate")
        .run(|_app, _event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = _event {
                if let Some(win) = _app.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.unminimize();
                    let _ = win.set_focus();
                }
            }
        });
}
