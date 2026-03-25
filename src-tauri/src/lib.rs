pub mod alert_hub;
mod checker;
mod commands;
mod database;
mod keyring_store;
mod mail_checker;
mod models;
mod notification_rules;
mod scheduler;
mod swork_client;
mod telegram;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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

            // 스케줄러 시작
            if keyring_store::has_initial_setup() {
                tauri::async_runtime::spawn(async {
                    scheduler::start().await;
                });
            }

            // 트레이: alert_hub 연동 — 경고 수를 tooltip에 표시
            let alert_count = alert_hub::count();
            let tooltip = if alert_count > 0 {
                format!("D-Mate — ⚠ {}건의 문제", alert_count)
            } else {
                "D-Mate — 업무 도우미".to_string()
            };

            let open_i = MenuItem::with_id(app, "open", "설정 열기", true, None::<&str>)?;
            let check_i = MenuItem::with_id(app, "check", "지금 확인", true, None::<&str>)?;
            let pause_i = MenuItem::with_id(app, "pause", "일시 중지", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "종료", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&open_i, &check_i, &pause_i, &quit_i])?;

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
            commands::hide_window,
            commands::quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running D-Mate");
}
