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

            rule1_enabled INTEGER DEFAULT 1,
            rule1_schedule_type TEXT DEFAULT 'interval',
            rule1_interval_min INTEGER DEFAULT 5,
            rule1_times TEXT DEFAULT '08:50,13:20,17:50',
            rule1_use_work_hours INTEGER DEFAULT 1,

            rule2_enabled INTEGER DEFAULT 1,
            rule2_schedule_type TEXT DEFAULT 'times',
            rule2_interval_min INTEGER DEFAULT 5,
            rule2_times TEXT DEFAULT '08:50,13:20,17:50',
            rule2_use_work_hours INTEGER DEFAULT 1,

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
            update_server_url TEXT DEFAULT '',
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

    // Migration: add new columns if missing (upgrade from old schema)
    let migrations = [
        "ALTER TABLE settings ADD COLUMN rule1_schedule_type TEXT DEFAULT 'interval'",
        "ALTER TABLE settings ADD COLUMN rule1_times TEXT DEFAULT '08:50,13:20,17:50'",
        "ALTER TABLE settings ADD COLUMN rule1_use_work_hours INTEGER DEFAULT 1",
        "ALTER TABLE settings ADD COLUMN rule2_schedule_type TEXT DEFAULT 'times'",
        "ALTER TABLE settings ADD COLUMN rule2_interval_min INTEGER DEFAULT 5",
        "ALTER TABLE settings ADD COLUMN rule2_use_work_hours INTEGER DEFAULT 1",
        "ALTER TABLE settings ADD COLUMN mail_schedule_type TEXT DEFAULT 'interval'",
        "ALTER TABLE settings ADD COLUMN mail_times TEXT DEFAULT ''",
        "ALTER TABLE settings ADD COLUMN mail_use_work_hours INTEGER DEFAULT 1",
    ];
    for sql in &migrations {
        conn.execute_batch(sql).ok(); // ignore "duplicate column" errors
    }
}

pub fn get_settings() -> Settings {
    let conn = DB.lock().unwrap();
    conn.query_row(
        "SELECT
            task_notify_enabled,
            rule1_enabled, rule1_schedule_type, rule1_interval_min, rule1_times, rule1_use_work_hours,
            rule2_enabled, rule2_schedule_type, rule2_interval_min, rule2_times, rule2_use_work_hours,
            mail_notify_enabled, mail_server, mail_port, mail_use_ssl, mail_account,
            mail_schedule_type, mail_interval_min, mail_times, mail_use_work_hours,
            work_hours_enabled, work_start_time, work_end_time, work_days,
            autostart, error_reporting, update_server_url
         FROM settings WHERE id=1",
        [],
        |row| {
            Ok(Settings {
                task_notify_enabled: row.get(0)?,
                rule1_enabled: row.get(1)?,
                rule1_schedule_type: row.get(2)?,
                rule1_interval_min: row.get(3)?,
                rule1_times: row.get(4)?,
                rule1_use_work_hours: row.get(5)?,
                rule2_enabled: row.get(6)?,
                rule2_schedule_type: row.get(7)?,
                rule2_interval_min: row.get(8)?,
                rule2_times: row.get(9)?,
                rule2_use_work_hours: row.get(10)?,
                mail_notify_enabled: row.get(11)?,
                mail_server: row.get(12)?,
                mail_port: row.get(13)?,
                mail_use_ssl: row.get(14)?,
                mail_account: row.get(15)?,
                mail_schedule_type: row.get(16)?,
                mail_interval_min: row.get(17)?,
                mail_times: row.get(18)?,
                mail_use_work_hours: row.get(19)?,
                work_hours_enabled: row.get(20)?,
                work_start_time: row.get(21)?,
                work_end_time: row.get(22)?,
                work_days: row.get(23)?,
                autostart: row.get(24)?,
                error_reporting: row.get(25)?,
                update_server_url: row.get(26)?,
            })
        },
    )
    .unwrap_or_default()
}

pub fn update_settings(data: &serde_json::Value) {
    let allowed = [
        "task_notify_enabled",
        "rule1_enabled", "rule1_schedule_type", "rule1_interval_min", "rule1_times", "rule1_use_work_hours",
        "rule2_enabled", "rule2_schedule_type", "rule2_interval_min", "rule2_times", "rule2_use_work_hours",
        "mail_notify_enabled", "mail_server", "mail_port", "mail_use_ssl", "mail_account",
        "mail_schedule_type", "mail_interval_min", "mail_times", "mail_use_work_hours",
        "work_hours_enabled", "work_start_time", "work_end_time", "work_days",
        "autostart", "error_reporting",
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
