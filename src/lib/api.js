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

export async function deskRequestJoin(serverUrl, name, deviceName) {
  return invoke("desk_request_join", { serverUrl, name, deviceName });
}

export async function deskCheckJoinStatus() {
  return invoke("desk_check_join_status");
}

export async function deskCancelJoinRequest() {
  return invoke("desk_cancel_join_request");
}

export async function deskHealth() {
  return invoke("desk_health");
}

export async function deskDisconnect() {
  return invoke("desk_disconnect");
}

export async function deskSubmitFeedback(category, title, body) {
  return invoke("desk_submit_feedback", { category, title, body });
}

export async function deskGetFeedback(page = 1, perPage = 10) {
  return invoke("desk_get_feedback", { page, perPage });
}

// ── Messages / MQTT ──

export async function getMessages(conversationId = null, limit = 30, offset = 0) {
  return invoke("get_messages", { conversationId, limit, offset });
}

export async function markMessageRead(id) {
  return invoke("mark_message_read", { id });
}

export async function markAllRead() {
  return invoke("mark_all_read");
}

export async function getUnreadCount() {
  return invoke("get_unread_count");
}

export async function getContacts() {
  return invoke("get_contacts");
}

// ── E2E 암호화 ──

export async function initEncryption() {
  return invoke("init_encryption");
}

// ── DM ──

export async function sendDm(targetCode, body) {
  return invoke("send_dm", { targetCode, body });
}

export async function getConversations() {
  return invoke("get_conversations");
}

export async function getConversationMessages(convId, limit = 50, offset = 0) {
  return invoke("get_conversation_messages", { convId, limit, offset });
}

export async function dmDelivered(msgId) {
  return invoke("dm_delivered", { msgId });
}

export async function dmRead(msgId) {
  return invoke("dm_read", { msgId });
}

export async function getMqttStatus() {
  return invoke("mqtt_status");
}

export async function mqttReconnect() {
  return invoke("mqtt_reconnect");
}

export async function getHostname() {
  return invoke("get_hostname");
}

export async function hideWindow() {
  return invoke("hide_window");
}

export async function quitApp() {
  return invoke("quit_app");
}
