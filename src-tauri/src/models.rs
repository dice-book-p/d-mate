use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Task {
    #[serde(default)]
    pub t_code: String,
    #[serde(default)]
    pub t_title: String,
    #[serde(default)]
    pub t_status: String,
    #[serde(default)]
    pub t_assignee: String,
    #[serde(default)]
    pub t_assigner: String,
    #[serde(default)]
    pub t_due_date: Option<String>,
    #[serde(default)]
    pub assigner_nickname: String,
    #[serde(default)]
    pub assignee_nickname: String,
    #[serde(default)]
    pub project_title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _days_overdue: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub task_notify_enabled: i32,

    // Rule1: 승인/검수 요청
    pub rule1_enabled: i32,
    pub rule1_schedule_type: String, // "interval" | "times"
    pub rule1_interval_min: i32,
    pub rule1_times: String,
    pub rule1_use_work_hours: i32,

    // Rule2: 지연 업무
    pub rule2_enabled: i32,
    pub rule2_schedule_type: String, // "interval" | "times"
    pub rule2_interval_min: i32,
    pub rule2_times: String,
    pub rule2_use_work_hours: i32,

    // Mail
    pub mail_notify_enabled: i32,
    pub mail_server: String,
    pub mail_port: i32,
    pub mail_use_ssl: i32,
    pub mail_account: String,
    pub mail_schedule_type: String, // "interval" | "times"
    pub mail_interval_min: i32,
    pub mail_times: String,
    pub mail_use_work_hours: i32,

    // System
    pub work_hours_enabled: i32,
    pub work_start_time: String,
    pub work_end_time: String,
    pub work_days: String,
    pub autostart: i32,
    pub error_reporting: i32,
    pub update_server_url: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            task_notify_enabled: 1,

            rule1_enabled: 1,
            rule1_schedule_type: "interval".into(),
            rule1_interval_min: 5,
            rule1_times: "08:50,13:20,17:50".into(),
            rule1_use_work_hours: 1,

            rule2_enabled: 1,
            rule2_schedule_type: "times".into(),
            rule2_interval_min: 5,
            rule2_times: "08:50,13:20,17:50".into(),
            rule2_use_work_hours: 1,

            mail_notify_enabled: 0,
            mail_server: String::new(),
            mail_port: 110,
            mail_use_ssl: 0,
            mail_account: String::new(),
            mail_schedule_type: "interval".into(),
            mail_interval_min: 2,
            mail_times: String::new(),
            mail_use_work_hours: 1,

            work_hours_enabled: 1,
            work_start_time: "09:00".into(),
            work_end_time: "18:00".into(),
            work_days: "mon-fri".into(),
            autostart: 0,
            error_reporting: 1,
            update_server_url: "http://192.168.204.53:18900".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationLog {
    pub id: i64,
    pub rule_type: String,
    pub task_code: String,
    pub task_title: String,
    pub slot_key: String,
    pub sent_at: String,
    pub success: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse {
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardData {
    pub rule1_tasks: Vec<Task>,
    pub rule2_tasks: Vec<Task>,
    pub recent_logs: Vec<NotificationLog>,
    pub error: String,
    pub settings: Settings,
    pub is_paused: bool,
}
