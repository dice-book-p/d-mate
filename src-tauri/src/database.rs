use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Mutex;
use once_cell::sync::Lazy;

use crate::models::{NotificationLog, Settings};

fn data_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("d-mate")
}

fn db_path() -> PathBuf {
    let dir = data_dir();
    std::fs::create_dir_all(&dir).ok();
    dir.join("d-mate.db")
}

static DB: Lazy<Mutex<Connection>> = Lazy::new(|| {
    let conn = Connection::open(db_path()).expect("Failed to open database");
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .unwrap();
    Mutex::new(conn)
});

pub fn init_db() {
    let conn = DB.lock().unwrap();

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            task_notify_enabled INTEGER DEFAULT 1,

            my_overdue_enabled INTEGER DEFAULT 1,
            my_overdue_schedule_type TEXT DEFAULT 'interval',
            my_overdue_interval_min INTEGER DEFAULT 5,
            my_overdue_times TEXT DEFAULT '08:50,13:20,17:50',
            my_overdue_use_work_hours INTEGER DEFAULT 1,

            my_deadline_enabled INTEGER DEFAULT 1,
            my_deadline_schedule_type TEXT DEFAULT 'times',
            my_deadline_interval_min INTEGER DEFAULT 5,
            my_deadline_times TEXT DEFAULT '08:50,13:20,17:50',
            my_deadline_use_work_hours INTEGER DEFAULT 1,

            approval_request_enabled INTEGER DEFAULT 1,
            approval_request_schedule_type TEXT DEFAULT 'interval',
            approval_request_interval_min INTEGER DEFAULT 5,
            approval_request_times TEXT DEFAULT '08:50,13:20,17:50',
            approval_request_use_work_hours INTEGER DEFAULT 1,

            overdue_task_enabled INTEGER DEFAULT 1,
            overdue_task_schedule_type TEXT DEFAULT 'times',
            overdue_task_interval_min INTEGER DEFAULT 5,
            overdue_task_times TEXT DEFAULT '08:50,13:20,17:50',
            overdue_task_use_work_hours INTEGER DEFAULT 1,

            mail_notify_enabled INTEGER DEFAULT 0,
            mail_server TEXT DEFAULT '',
            mail_port INTEGER DEFAULT 110,
            mail_use_ssl INTEGER DEFAULT 0,
            mail_account TEXT DEFAULT '',
            mail_schedule_type TEXT DEFAULT 'interval',
            mail_interval_min INTEGER DEFAULT 2,
            mail_times TEXT DEFAULT '',
            mail_use_work_hours INTEGER DEFAULT 1,

            work_hours_enabled INTEGER DEFAULT 1,
            work_start_time TEXT DEFAULT '09:00',
            work_end_time TEXT DEFAULT '18:00',
            work_days TEXT DEFAULT 'mon-fri',
            autostart INTEGER DEFAULT 0,
            error_reporting INTEGER DEFAULT 1,
            update_server_url TEXT DEFAULT 'http://192.168.204.53:18900',
            os_notification_enabled INTEGER DEFAULT 1,
            updated_at TEXT DEFAULT (datetime('now'))
        );
        INSERT OR IGNORE INTO settings (id) VALUES (1);

        CREATE TABLE IF NOT EXISTS notification_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            rule_type TEXT NOT NULL,
            task_code TEXT NOT NULL,
            task_title TEXT DEFAULT '',
            slot_key TEXT DEFAULT '',
            sent_at TEXT DEFAULT (datetime('now')),
            success INTEGER DEFAULT 1
        );
        CREATE INDEX IF NOT EXISTS idx_notif_dedup
            ON notification_log(rule_type, task_code, slot_key);",
    )
    .expect("Failed to create tables");

    // v1.0.x → v1.1.0 마이그레이션: 기존 rule1/rule2 → 새 네이밍
    let migrations = [
        // 기존 호환 마이그레이션 (v1.0.x 이전 버전 업그레이드)
        "ALTER TABLE settings ADD COLUMN rule1_schedule_type TEXT DEFAULT 'interval'",
        "ALTER TABLE settings ADD COLUMN rule1_times TEXT DEFAULT '08:50,13:20,17:50'",
        "ALTER TABLE settings ADD COLUMN rule1_use_work_hours INTEGER DEFAULT 1",
        "ALTER TABLE settings ADD COLUMN rule2_schedule_type TEXT DEFAULT 'times'",
        "ALTER TABLE settings ADD COLUMN rule2_interval_min INTEGER DEFAULT 5",
        "ALTER TABLE settings ADD COLUMN rule2_use_work_hours INTEGER DEFAULT 1",
        "ALTER TABLE settings ADD COLUMN mail_schedule_type TEXT DEFAULT 'interval'",
        "ALTER TABLE settings ADD COLUMN mail_times TEXT DEFAULT ''",
        "ALTER TABLE settings ADD COLUMN mail_use_work_hours INTEGER DEFAULT 1",
        // v1.1.0 신규 칼럼
        "ALTER TABLE settings ADD COLUMN my_overdue_enabled INTEGER DEFAULT 1",
        "ALTER TABLE settings ADD COLUMN my_overdue_schedule_type TEXT DEFAULT 'interval'",
        "ALTER TABLE settings ADD COLUMN my_overdue_interval_min INTEGER DEFAULT 5",
        "ALTER TABLE settings ADD COLUMN my_overdue_times TEXT DEFAULT '08:50,13:20,17:50'",
        "ALTER TABLE settings ADD COLUMN my_overdue_use_work_hours INTEGER DEFAULT 1",
        "ALTER TABLE settings ADD COLUMN my_deadline_enabled INTEGER DEFAULT 1",
        "ALTER TABLE settings ADD COLUMN my_deadline_schedule_type TEXT DEFAULT 'times'",
        "ALTER TABLE settings ADD COLUMN my_deadline_interval_min INTEGER DEFAULT 5",
        "ALTER TABLE settings ADD COLUMN my_deadline_times TEXT DEFAULT '08:50,13:20,17:50'",
        "ALTER TABLE settings ADD COLUMN my_deadline_use_work_hours INTEGER DEFAULT 1",
        "ALTER TABLE settings ADD COLUMN approval_request_enabled INTEGER DEFAULT 1",
        "ALTER TABLE settings ADD COLUMN approval_request_schedule_type TEXT DEFAULT 'interval'",
        "ALTER TABLE settings ADD COLUMN approval_request_interval_min INTEGER DEFAULT 5",
        "ALTER TABLE settings ADD COLUMN approval_request_times TEXT DEFAULT '08:50,13:20,17:50'",
        "ALTER TABLE settings ADD COLUMN approval_request_use_work_hours INTEGER DEFAULT 1",
        "ALTER TABLE settings ADD COLUMN overdue_task_enabled INTEGER DEFAULT 1",
        "ALTER TABLE settings ADD COLUMN overdue_task_schedule_type TEXT DEFAULT 'times'",
        "ALTER TABLE settings ADD COLUMN overdue_task_interval_min INTEGER DEFAULT 5",
        "ALTER TABLE settings ADD COLUMN overdue_task_times TEXT DEFAULT '08:50,13:20,17:50'",
        "ALTER TABLE settings ADD COLUMN overdue_task_use_work_hours INTEGER DEFAULT 1",
        "ALTER TABLE settings ADD COLUMN os_notification_enabled INTEGER DEFAULT 1",
    ];
    for sql in &migrations {
        conn.execute_batch(sql).ok();
    }

    // rule1/rule2 기존 설정값을 새 칼럼으로 복사 (한 번만 실행)
    conn.execute_batch(
        "UPDATE settings SET
            approval_request_enabled = COALESCE(rule1_enabled, 1),
            approval_request_schedule_type = COALESCE(rule1_schedule_type, 'interval'),
            approval_request_interval_min = COALESCE(rule1_interval_min, 5),
            approval_request_times = COALESCE(rule1_times, '08:50,13:20,17:50'),
            approval_request_use_work_hours = COALESCE(rule1_use_work_hours, 1),
            overdue_task_enabled = COALESCE(rule2_enabled, 1),
            overdue_task_schedule_type = COALESCE(rule2_schedule_type, 'times'),
            overdue_task_interval_min = COALESCE(rule2_interval_min, 5),
            overdue_task_times = COALESCE(rule2_times, '08:50,13:20,17:50'),
            overdue_task_use_work_hours = COALESCE(rule2_use_work_hours, 1)
         WHERE id = 1
           AND approval_request_enabled = 1
           AND approval_request_schedule_type = 'interval'
           AND overdue_task_enabled = 1
           AND overdue_task_schedule_type = 'times';"
    ).ok();

    // notification_log rule_type 마이그레이션
    conn.execute_batch(
        "UPDATE notification_log SET rule_type = 'approval_request' WHERE rule_type = 'rule1';
         UPDATE notification_log SET rule_type = 'overdue_task' WHERE rule_type = 'rule2';"
    ).ok();

    // update_server_url이 비어있으면 기본값 설정
    conn.execute(
        "UPDATE settings SET update_server_url='http://192.168.204.53:18900' WHERE id=1 AND (update_server_url IS NULL OR update_server_url='')",
        [],
    ).ok();
}

pub fn get_settings() -> Settings {
    let conn = DB.lock().unwrap();
    conn.query_row(
        "SELECT
            task_notify_enabled,
            my_overdue_enabled, my_overdue_schedule_type, my_overdue_interval_min, my_overdue_times, my_overdue_use_work_hours,
            my_deadline_enabled, my_deadline_schedule_type, my_deadline_interval_min, my_deadline_times, my_deadline_use_work_hours,
            approval_request_enabled, approval_request_schedule_type, approval_request_interval_min, approval_request_times, approval_request_use_work_hours,
            overdue_task_enabled, overdue_task_schedule_type, overdue_task_interval_min, overdue_task_times, overdue_task_use_work_hours,
            mail_notify_enabled, mail_server, mail_port, mail_use_ssl, mail_account,
            mail_schedule_type, mail_interval_min, mail_times, mail_use_work_hours,
            work_hours_enabled, work_start_time, work_end_time, work_days,
            autostart, error_reporting, update_server_url, os_notification_enabled
         FROM settings WHERE id=1",
        [],
        |row| {
            Ok(Settings {
                task_notify_enabled: row.get(0)?,
                my_overdue_enabled: row.get(1)?,
                my_overdue_schedule_type: row.get(2)?,
                my_overdue_interval_min: row.get(3)?,
                my_overdue_times: row.get(4)?,
                my_overdue_use_work_hours: row.get(5)?,
                my_deadline_enabled: row.get(6)?,
                my_deadline_schedule_type: row.get(7)?,
                my_deadline_interval_min: row.get(8)?,
                my_deadline_times: row.get(9)?,
                my_deadline_use_work_hours: row.get(10)?,
                approval_request_enabled: row.get(11)?,
                approval_request_schedule_type: row.get(12)?,
                approval_request_interval_min: row.get(13)?,
                approval_request_times: row.get(14)?,
                approval_request_use_work_hours: row.get(15)?,
                overdue_task_enabled: row.get(16)?,
                overdue_task_schedule_type: row.get(17)?,
                overdue_task_interval_min: row.get(18)?,
                overdue_task_times: row.get(19)?,
                overdue_task_use_work_hours: row.get(20)?,
                mail_notify_enabled: row.get(21)?,
                mail_server: row.get(22)?,
                mail_port: row.get(23)?,
                mail_use_ssl: row.get(24)?,
                mail_account: row.get(25)?,
                mail_schedule_type: row.get(26)?,
                mail_interval_min: row.get(27)?,
                mail_times: row.get(28)?,
                mail_use_work_hours: row.get(29)?,
                work_hours_enabled: row.get(30)?,
                work_start_time: row.get(31)?,
                work_end_time: row.get(32)?,
                work_days: row.get(33)?,
                autostart: row.get(34)?,
                error_reporting: row.get(35)?,
                update_server_url: row.get(36)?,
                os_notification_enabled: row.get(37)?,
            })
        },
    )
    .unwrap_or_default()
}

pub fn update_settings(data: &serde_json::Value) {
    let allowed = [
        "task_notify_enabled",
        "my_overdue_enabled", "my_overdue_schedule_type", "my_overdue_interval_min", "my_overdue_times", "my_overdue_use_work_hours",
        "my_deadline_enabled", "my_deadline_schedule_type", "my_deadline_interval_min", "my_deadline_times", "my_deadline_use_work_hours",
        "approval_request_enabled", "approval_request_schedule_type", "approval_request_interval_min", "approval_request_times", "approval_request_use_work_hours",
        "overdue_task_enabled", "overdue_task_schedule_type", "overdue_task_interval_min", "overdue_task_times", "overdue_task_use_work_hours",
        "mail_notify_enabled", "mail_server", "mail_port", "mail_use_ssl", "mail_account",
        "mail_schedule_type", "mail_interval_min", "mail_times", "mail_use_work_hours",
        "work_hours_enabled", "work_start_time", "work_end_time", "work_days",
        "autostart", "error_reporting", "os_notification_enabled", "update_server_url",
    ];

    let obj = match data.as_object() {
        Some(o) => o,
        None => return,
    };

    let conn = DB.lock().unwrap();
    for (key, val) in obj {
        if !allowed.contains(&key.as_str()) {
            continue;
        }
        let sql = format!(
            "UPDATE settings SET {}=?, updated_at=datetime('now') WHERE id=1",
            key
        );
        match val {
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    conn.execute(&sql, params![i]).ok();
                }
            }
            serde_json::Value::String(s) => {
                conn.execute(&sql, params![s]).ok();
            }
            serde_json::Value::Bool(b) => {
                conn.execute(&sql, params![*b as i32]).ok();
            }
            _ => {}
        }
    }
}

pub fn log_notification(
    rule_type: &str,
    task_code: &str,
    task_title: &str,
    slot_key: &str,
    success: bool,
) {
    let conn = DB.lock().unwrap();
    conn.execute(
        "INSERT INTO notification_log (rule_type, task_code, task_title, slot_key, success)
         VALUES (?,?,?,?,?)",
        params![rule_type, task_code, task_title, slot_key, success as i32],
    )
    .ok();
}

pub fn is_notification_sent(rule_type: &str, task_code: &str, slot_key: &str) -> bool {
    let conn = DB.lock().unwrap();
    conn.query_row(
        "SELECT 1 FROM notification_log
         WHERE rule_type=? AND task_code=? AND slot_key=? AND success=1 LIMIT 1",
        params![rule_type, task_code, slot_key],
        |_| Ok(true),
    )
    .unwrap_or(false)
}

pub fn try_log_notification(
    rule_type: &str,
    task_code: &str,
    task_title: &str,
    slot_key: &str,
) -> bool {
    if is_notification_sent(rule_type, task_code, slot_key) {
        return false;
    }
    log_notification(rule_type, task_code, task_title, slot_key, true);
    true
}

pub fn get_recent_notifications(limit: i32) -> Vec<NotificationLog> {
    let conn = DB.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT id, rule_type, task_code, task_title, slot_key, sent_at, success
             FROM notification_log ORDER BY sent_at DESC LIMIT ?",
        )
        .unwrap();

    stmt.query_map(params![limit], |row| {
        Ok(NotificationLog {
            id: row.get(0)?,
            rule_type: row.get(1)?,
            task_code: row.get(2)?,
            task_title: row.get(3)?,
            slot_key: row.get(4)?,
            sent_at: row.get(5)?,
            success: row.get(6)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

pub fn clear_all_data() {
    let conn = DB.lock().unwrap();
    conn.execute_batch(
        "DELETE FROM notification_log;
         DELETE FROM settings;
         INSERT OR IGNORE INTO settings (id) VALUES (1);",
    )
    .ok();
}
