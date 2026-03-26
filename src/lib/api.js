import { invoke } from "@tauri-apps/api/core";

export async function getSettings() {
  return invoke("get_settings");
}

export async function saveSettings(data) {
  return invoke("save_settings", { data });
}

export async function verifySworkLogin(username, password) {
  return invoke("verify_swork_login", { username, password });
}

export async function testTelegram(target, token = null, chatId = null) {
  return invoke("test_telegram", { target, token, chatId });
}

export async function lookupTelegramChats(botToken) {
  return invoke("lookup_telegram_chats", { botToken });
}

export async function getAlerts() {
  return invoke("get_alerts");
}

export async function getDashboardData() {
  return invoke("get_dashboard_data");
}

export async function triggerCheckNow() {
  return invoke("trigger_check_now");
}

export async function togglePause() {
  return invoke("toggle_pause");
}

export async function verifyMailLogin(server, port, useSsl, account, password) {
  return invoke("verify_mail_login", { server, port, useSsl, account, password });
}

export async function disconnectService(target) {
  return invoke("disconnect_service", { target });
}

export async function setAutostart(enabled) {
  return invoke("set_autostart", { enabled });
}

export async function checkUpdate() {
  return invoke("check_update");
}

export async function reportError(errorMessage, context = "") {
  return invoke("report_error", { errorMessage, context });
}

export async function resetAllData() {
  return invoke("reset_all_data");
}

// ── Desk ──

export async function deskJoin(serverUrl, code, name, deviceName) {
  return invoke("desk_join", { serverUrl, code, name, deviceName });
}

export async function deskHealth() {
  return invoke("desk_health");
}

export async function deskSubmitFeedback(category, title, body) {
  return invoke("desk_submit_feedback", { category, title, body });
}

export async function deskGetFeedback() {
  return invoke("desk_get_feedback");
}

export async function hideWindow() {
  return invoke("hide_window");
}

export async function quitApp() {
  return invoke("quit_app");
}
