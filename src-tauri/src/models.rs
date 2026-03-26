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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _days_left: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub task_notify_enabled: i32,

    // 내 업무: 내 지연업무
    pub my_overdue_enabled: i32,
    pub my_overdue_schedule_type: String,
    pub my_overdue_interval_min: i32,
    pub my_overdue_times: String,
    pub my_overdue_use_work_hours: i32,

    // 내 업무: 마감임박
    pub my_deadline_enabled: i32,
    pub my_deadline_schedule_type: String,
    pub my_deadline_interval_min: i32,
    pub my_deadline_times: String,
    pub my_deadline_use_work_hours: i32,

    // 관리 업무: 승인요청
    pub approval_request_enabled: i32,
    pub approval_request_schedule_type: String,
    pub approval_request_interval_min: i32,
    pub approval_request_times: String,
    pub approval_request_use_work_hours: i32,

    // 관리 업무: 지연업무
    pub overdue_task_enabled: i32,
    pub overdue_task_schedule_type: String,
    pub overdue_task_interval_min: i32,
    pub overdue_task_times: String,
    pub overdue_task_use_work_hours: i32,

    // Mail
    pub mail_notify_enabled: i32,
    pub mail_server: String,
    pub mail_port: i32,
    pub mail_use_ssl: i32,
    pub mail_account: String,
    pub mail_schedule_type: String,
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
    pub os_notification_enabled: i32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            task_notify_enabled: 1,

            my_overdue_enabled: 1,
            my_overdue_schedule_type: "interval".into(),
            my_overdue_interval_min: 5,
            my_overdue_times: "08:50,13:20,17:50".into(),
            my_overdue_use_work_hours: 1,

            my_deadline_enabled: 1,
            my_deadline_schedule_type: "times".into(),
            my_deadline_interval_min: 5,
            my_deadline_times: "08:50,13:20,17:50".into(),
            my_deadline_use_work_hours: 1,

            approval_request_enabled: 1,
            approval_request_schedule_type: "interval".into(),
            approval_request_interval_min: 5,
            approval_request_times: "08:50,13:20,17:50".into(),
            approval_request_use_work_hours: 1,

            overdue_task_enabled: 1,
            overdue_task_schedule_type: "times".into(),
            overdue_task_interval_min: 5,
            overdue_task_times: "08:50,13:20,17:50".into(),
            overdue_task_use_work_hours: 1,

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
            os_notification_enabled: 1,
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
    pub my_overdue_tasks: Vec<Task>,
    pub my_deadline_tasks: Vec<Task>,
    pub approval_request_tasks: Vec<Task>,
    pub overdue_task_tasks: Vec<Task>,
    pub recent_logs: Vec<NotificationLog>,
    pub error: String,
    pub settings: Settings,
    pub is_paused: bool,
}
