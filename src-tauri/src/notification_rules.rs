use chrono::{Local, NaiveDate};

use crate::models::Task;

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(s, "%Y.%m.%d"))
        .or_else(|_| NaiveDate::parse_from_str(s, "%Y/%m/%d"))
        .ok()
}

pub fn filter_rule1(tasks: &[Task], username: &str) -> Vec<Task> {
    tasks
        .iter()
        .filter(|t| {
            t.t_assignee != username
                && (t.t_status == "승인요청" || t.t_status == "검수요청")
        })
        .cloned()
        .collect()
}

pub fn filter_rule2(tasks: &[Task], _username: &str) -> Vec<Task> {
    let today = Local::now().date_naive();

    tasks
        .iter()
        .filter_map(|t| {
            let status_ok =
                t.t_status == "업무승인" || t.t_status == "진행중" || t.t_status == "검수완료";
            if !status_ok {
                return None;
            }

            let due = t.t_due_date.as_deref().and_then(parse_date)?;
            if due >= today {
                return None;
            }

            let days = (today - due).num_days();
            let mut task = t.clone();
            task._days_overdue = Some(days);
            Some(task)
        })
        .collect()
}

pub fn format_rule1_message(tasks: &[Task]) -> String {
    if tasks.is_empty() {
        return String::new();
    }

    let mut msg = String::from("<b>📋 승인/검수 요청 알림</b>\n\n");

    for t in tasks {
        msg.push_str(&format!(
            "• <b>[{}]</b> {}\n  프로젝트: {}\n  담당: {} → {}\n  상태: <b>{}</b>\n\n",
            esc(&t.t_code),
            esc(&t.t_title),
            esc(&t.project_title),
            esc(&t.assignee_nickname),
            esc(&t.assigner_nickname),
            esc(&t.t_status),
        ));
    }

    msg.push_str(&format!("총 {}건", tasks.len()));
    msg
}

pub fn format_rule2_message(tasks: &[Task]) -> String {
    if tasks.is_empty() {
        return String::new();
    }

    let mut msg = String::from("<b>⏰ 지연 업무 알림</b>\n\n");

    for t in tasks {
        let overdue = t._days_overdue.unwrap_or(0);
        let due_str = t.t_due_date.as_deref().unwrap_or("-");
        msg.push_str(&format!(
            "• <b>[{}]</b> {}\n  기한: {} (<b>{}일 초과</b>)\n  담당: {} → {}\n  상태: {}\n\n",
            esc(&t.t_code),
            esc(&t.t_title),
            esc(due_str),
            overdue,
            esc(&t.assignee_nickname),
            esc(&t.assigner_nickname),
            esc(&t.t_status),
        ));
    }

    msg.push_str(&format!("총 {}건", tasks.len()));
    msg
}

pub fn format_mail_message(from: &str, subject: &str, date: &str) -> String {
    let date_line = if date.is_empty() {
        String::new()
    } else {
        format!("\n수신 시간: {}", esc(date))
    };
    format!(
        "<b>📬 새로운 메일이 도착했어요!</b>\n\n보낸 사람: {}\n제목: {}{}",
        esc(from),
        esc(subject),
        date_line
    )
}
