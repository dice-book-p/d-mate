use chrono::Local;

use crate::{alert_hub, database, keyring_store, mail_checker, notification_rules, swork_client, telegram};
use crate::models::Settings;

fn is_global_work_hours(settings: &Settings) -> bool {
    if settings.work_hours_enabled == 0 {
        return true;
    }
    check_work_hours(&settings.work_days, &settings.work_start_time, &settings.work_end_time)
}

fn check_work_hours(days: &str, start: &str, end: &str) -> bool {
    let now = Local::now();
    let weekday = now.format("%a").to_string().to_lowercase();

    let days_lower = days.to_lowercase();
    let day_ok = if days_lower.contains('-') {
        let parts: Vec<&str> = days_lower.split('-').collect();
        if parts.len() == 2 {
            let order = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
            let s = order.iter().position(|&d| d == parts[0]);
            let e = order.iter().position(|&d| d == parts[1]);
            let c = order.iter().position(|&d| d == weekday.as_str());
            match (s, e, c) {
                (Some(s), Some(e), Some(c)) => c >= s && c <= e,
                _ => true,
            }
        } else {
            true
        }
    } else {
        days_lower.split(',').any(|d| d.trim() == weekday)
    };

    if !day_ok {
        return false;
    }

    let time_now = now.format("%H:%M").to_string();
    time_now.as_str() >= start && time_now.as_str() <= end
}

fn should_send(use_work_hours: i32, settings: &Settings) -> bool {
    if use_work_hours == 0 {
        return true;
    }
    is_global_work_hours(settings)
}

fn is_auth_error(err: &str) -> bool {
    let lower = err.to_lowercase();
    lower.contains("401") || lower.contains("403") || lower.contains("login")
        || lower.contains("auth") || lower.contains("password") || lower.contains("credential")
}

pub async fn check_rule1() {
    let settings = database::get_settings();
    if settings.task_notify_enabled == 0 || settings.rule1_enabled == 0 {
        return;
    }
    if !should_send(settings.rule1_use_work_hours, &settings) {
        return;
    }

    let username = keyring_store::get_swork_username();
    let token = keyring_store::get_swork_tg_token();
    let chat_id = keyring_store::get_swork_tg_chat_id();
    if username.is_empty() || token.is_empty() || chat_id.is_empty() {
        return;
    }

    let tasks = match swork_client::fetch_manager_tasks().await {
        Ok(t) => {
            alert_hub::swork_ok();
            t
        }
        Err(e) => {
            log::error!("Rule1 fetch error: {}", e);
            if is_auth_error(&e) { alert_hub::swork_auth_error(&e); }
            else { alert_hub::swork_server_error(&e); }
            return;
        }
    };

    log::info!("Rule1: fetched {} total tasks, username={}", tasks.len(), username);

    let filtered = notification_rules::filter_rule1(&tasks, &username);
    log::info!("Rule1: {} tasks after filter", filtered.len());

    if filtered.is_empty() {
        return;
    }

    // Rule1: 처리될 때까지 매 주기마다 반복 알림 (dedup 없음)
    let msg = notification_rules::format_rule1_message(&filtered);
    let ok = telegram::send_message(&token, &chat_id, &msg).await;

    if ok {
        alert_hub::swork_tg_ok();
    } else {
        alert_hub::swork_tg_error("발송 실패");
    }

    let slot = Local::now().format("%Y-%m-%d_%H:%M").to_string();
    for t in &filtered {
        database::log_notification("rule1", &t.t_code, &t.t_title, &slot, ok);
    }

    log::info!("Rule1: sent {} tasks, success={}", filtered.len(), ok);
}

pub async fn check_rule2(slot_key: &str) {
    let settings = database::get_settings();
    if settings.task_notify_enabled == 0 || settings.rule2_enabled == 0 {
        return;
    }
    if !should_send(settings.rule2_use_work_hours, &settings) {
        return;
    }

    let username = keyring_store::get_swork_username();
    let token = keyring_store::get_swork_tg_token();
    let chat_id = keyring_store::get_swork_tg_chat_id();
    if username.is_empty() || token.is_empty() || chat_id.is_empty() {
        return;
    }

    let tasks = match swork_client::fetch_manager_tasks().await {
        Ok(t) => {
            alert_hub::swork_ok();
            t
        }
        Err(e) => {
            log::error!("Rule2 fetch error: {}", e);
            if is_auth_error(&e) { alert_hub::swork_auth_error(&e); }
            else { alert_hub::swork_server_error(&e); }
            return;
        }
    };

    let filtered = notification_rules::filter_rule2(&tasks, &username);
    let new_tasks: Vec<_> = filtered
        .iter()
        .filter(|t| database::try_log_notification("rule2", &t.t_code, &t.t_title, slot_key))
        .cloned()
        .collect();

    if new_tasks.is_empty() {
        return;
    }

    let msg = notification_rules::format_rule2_message(&new_tasks);
    let ok = telegram::send_message(&token, &chat_id, &msg).await;

    if ok { alert_hub::swork_tg_ok(); }
    else { alert_hub::swork_tg_error("발송 실패"); }

    log::info!("Rule2: sent {} tasks for slot {}", new_tasks.len(), slot_key);
}

pub async fn check_mail() {
    let settings = database::get_settings();
    if settings.mail_notify_enabled == 0 {
        return;
    }
    if !should_send(settings.mail_use_work_hours, &settings) {
        return;
    }

    let token = keyring_store::get_mail_tg_token();
    let chat_id = keyring_store::get_mail_tg_chat_id();
    let password = keyring_store::get_mail_password();

    if settings.mail_account.is_empty() || password.is_empty() || token.is_empty() || chat_id.is_empty() {
        return;
    }

    let mails = match mail_checker::fetch_new_mails(
        &settings.mail_server,
        settings.mail_port as u16,
        settings.mail_use_ssl == 1,
        &settings.mail_account,
        &password,
    )
    .await
    {
        Ok(m) => {
            alert_hub::mail_ok();
            m
        }
        Err(e) => {
            log::error!("Mail check error: {}", e);
            if is_auth_error(&e) { alert_hub::mail_auth_error(&e); }
            else { alert_hub::mail_server_error(&e); }
            return;
        }
    };

    for mail in &mails {
        // atomic dedup: 이미 발송된 UID면 스킵
        if !database::try_log_notification("mail", &mail.uid, &mail.subject, "") {
            continue;
        }

        let msg = notification_rules::format_mail_message(&mail.from, &mail.subject, &mail.date);
        let ok = telegram::send_message(&token, &chat_id, &msg).await;

        if ok { alert_hub::mail_tg_ok(); }
        else { alert_hub::mail_tg_error("발송 실패"); }
    }

    if !mails.is_empty() {
        log::info!("Mail: sent {} notifications", mails.len());
    }
}
