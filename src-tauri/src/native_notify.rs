use once_cell::sync::OnceCell;
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use crate::database;

static APP_HANDLE: OnceCell<AppHandle> = OnceCell::new();

pub fn init(handle: AppHandle) {
    APP_HANDLE.set(handle).ok();
}

/// OS 네이티브 알림 발송 (설정에서 on/off 가능)
pub fn send(title: &str, body: &str) {
    let settings = database::get_settings();
    if settings.os_notification_enabled == 0 {
        return;
    }

    let Some(app) = APP_HANDLE.get() else {
        return;
    };

    if let Err(e) = app
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show()
    {
        log::warn!("OS notification failed: {}", e);
    }
}
