use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use once_cell::sync::Lazy;

use crate::checker;
use crate::database;

static PAUSED: AtomicBool = AtomicBool::new(false);
static RUNNING: AtomicBool = AtomicBool::new(false);
static CANCEL: Lazy<Mutex<Option<tokio::sync::watch::Sender<bool>>>> =
    Lazy::new(|| Mutex::new(None));

pub fn is_paused() -> bool {
    PAUSED.load(Ordering::Relaxed)
}

pub fn toggle_pause() -> bool {
    let was = PAUSED.load(Ordering::Relaxed);
    PAUSED.store(!was, Ordering::Relaxed);
    !was
}

pub fn is_running() -> bool {
    RUNNING.load(Ordering::Relaxed)
}

/// 다음 정시 기준 실행 시점까지 남은 시간 계산
fn sleep_until_next_aligned(interval_min: i64) -> std::time::Duration {
    let now = chrono::Local::now();
    let minute = now.minute() as i64;
    let second = now.second() as i64;

    let elapsed_in_cycle = (minute % interval_min) * 60 + second;
    let cycle_total = interval_min * 60;
    let remaining = cycle_total - elapsed_in_cycle;

    let secs = if remaining <= 0 { cycle_total } else { remaining };
    std::time::Duration::from_secs(secs as u64)
}

use chrono::Timelike;

pub async fn start() {
    if RUNNING.load(Ordering::Relaxed) {
        return;
    }
    RUNNING.store(true, Ordering::Relaxed);

    let (tx, rx) = tokio::sync::watch::channel(false);
    {
        let mut lock = CANCEL.lock().await;
        *lock = Some(tx);
    }

    // 내 업무 루프
    let rx1 = rx.clone();
    tokio::spawn(async move { my_overdue_loop(rx1).await });

    let rx2 = rx.clone();
    tokio::spawn(async move { my_deadline_loop(rx2).await });

    // 관리 업무 루프
    let rx3 = rx.clone();
    tokio::spawn(async move { approval_request_loop(rx3).await });

    let rx4 = rx.clone();
    tokio::spawn(async move { overdue_task_loop(rx4).await });

    // 메일 루프
    let rx5 = rx.clone();
    tokio::spawn(async move { mail_loop(rx5).await });

    // 업데이트 체크 루프
    let rx6 = rx.clone();
    tokio::spawn(async move { update_check_loop(rx6).await });

    log::info!("Scheduler started (6 loops)");
}

pub async fn stop() {
    let lock = CANCEL.lock().await;
    if let Some(tx) = lock.as_ref() {
        let _ = tx.send(true);
    }
    RUNNING.store(false, Ordering::Relaxed);
    log::info!("Scheduler stopped");
}

// ── 내 지연업무 루프 ──

async fn my_overdue_loop(mut cancel: tokio::sync::watch::Receiver<bool>) {
    loop {
        let settings = database::get_settings();

        if settings.my_overdue_schedule_type == "times" {
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {}
                _ = cancel.changed() => { return; }
            }
            if PAUSED.load(Ordering::Relaxed) { continue; }

            let now = chrono::Local::now();
            let current_time = now.format("%H:%M").to_string();
            for time_str in settings.my_overdue_times.split(',') {
                if time_str.trim() == current_time {
                    checker::check_my_overdue().await;
                    break;
                }
            }
        } else {
            let wait = sleep_until_next_aligned(settings.my_overdue_interval_min as i64);
            tokio::select! {
                _ = tokio::time::sleep(wait) => {}
                _ = cancel.changed() => { return; }
            }
            if PAUSED.load(Ordering::Relaxed) { continue; }
            checker::check_my_overdue().await;
        }
    }
}

// ── 마감임박 루프 ──

async fn my_deadline_loop(mut cancel: tokio::sync::watch::Receiver<bool>) {
    loop {
        let settings = database::get_settings();

        if settings.my_deadline_schedule_type == "times" {
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {}
                _ = cancel.changed() => { return; }
            }
            if PAUSED.load(Ordering::Relaxed) { continue; }

            let now = chrono::Local::now();
            let current_time = now.format("%H:%M").to_string();
            for time_str in settings.my_deadline_times.split(',') {
                if time_str.trim() == current_time {
                    checker::check_my_deadline().await;
                    break;
                }
            }
        } else {
            let wait = sleep_until_next_aligned(settings.my_deadline_interval_min as i64);
            tokio::select! {
                _ = tokio::time::sleep(wait) => {}
                _ = cancel.changed() => { return; }
            }
            if PAUSED.load(Ordering::Relaxed) { continue; }
            checker::check_my_deadline().await;
        }
    }
}

// ── 승인요청 루프 (기존 rule1) ──

async fn approval_request_loop(mut cancel: tokio::sync::watch::Receiver<bool>) {
    loop {
        let settings = database::get_settings();

        if settings.approval_request_schedule_type == "times" {
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {}
                _ = cancel.changed() => { return; }
            }
            if PAUSED.load(Ordering::Relaxed) { continue; }

            let now = chrono::Local::now();
            let current_time = now.format("%H:%M").to_string();
            for time_str in settings.approval_request_times.split(',') {
                if time_str.trim() == current_time {
                    checker::check_approval_request().await;
                    break;
                }
            }
        } else {
            let wait = sleep_until_next_aligned(settings.approval_request_interval_min as i64);
            tokio::select! {
                _ = tokio::time::sleep(wait) => {}
                _ = cancel.changed() => { return; }
            }
            if PAUSED.load(Ordering::Relaxed) { continue; }
            checker::check_approval_request().await;
        }
    }
}

// ── 지연업무 루프 (기존 rule2) ──

async fn overdue_task_loop(mut cancel: tokio::sync::watch::Receiver<bool>) {
    loop {
        let settings = database::get_settings();

        if settings.overdue_task_schedule_type == "interval" {
            let wait = sleep_until_next_aligned(settings.overdue_task_interval_min as i64);
            tokio::select! {
                _ = tokio::time::sleep(wait) => {}
                _ = cancel.changed() => { return; }
            }
            if PAUSED.load(Ordering::Relaxed) { continue; }

            let slot_key = chrono::Local::now().format("%Y-%m-%d_%H:%M").to_string();
            checker::check_overdue_task(&slot_key).await;
        } else {
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {}
                _ = cancel.changed() => { return; }
            }
            if PAUSED.load(Ordering::Relaxed) { continue; }

            let now = chrono::Local::now();
            let current_time = now.format("%H:%M").to_string();
            for time_str in settings.overdue_task_times.split(',') {
                if time_str.trim() == current_time {
                    let slot_key = now.format("%Y-%m-%d_%H:%M").to_string();
                    checker::check_overdue_task(&slot_key).await;
                    break;
                }
            }
        }
    }
}

// ── 메일 루프 ──

async fn mail_loop(mut cancel: tokio::sync::watch::Receiver<bool>) {
    loop {
        let settings = database::get_settings();

        if settings.mail_schedule_type == "times" {
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {}
                _ = cancel.changed() => { return; }
            }
            if PAUSED.load(Ordering::Relaxed) { continue; }

            let now = chrono::Local::now();
            let current_time = now.format("%H:%M").to_string();
            for time_str in settings.mail_times.split(',') {
                if time_str.trim() == current_time {
                    checker::check_mail().await;
                    break;
                }
            }
        } else {
            let wait = sleep_until_next_aligned(settings.mail_interval_min as i64);
            tokio::select! {
                _ = tokio::time::sleep(wait) => {}
                _ = cancel.changed() => { return; }
            }
            if PAUSED.load(Ordering::Relaxed) { continue; }
            checker::check_mail().await;
        }
    }
}

// ── 업데이트 체크 루프 (6시간) ──

async fn update_check_loop(mut cancel: tokio::sync::watch::Receiver<bool>) {
    // 앱 시작 후 30초 뒤 첫 체크
    tokio::select! {
        _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {}
        _ = cancel.changed() => { return; }
    }

    loop {
        let settings = database::get_settings();
        let url = settings.update_server_url.trim().to_string();

        if !url.is_empty() {
            let check_url = format!("{}/update/version.json", url.trim_end_matches('/'));
            if let Ok(resp) = reqwest::Client::new()
                .get(&check_url)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
            {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    let latest = data["version"].as_str().unwrap_or("0.0.0");
                    let current = env!("CARGO_PKG_VERSION");
                    if latest > current {
                        let notes = data["notes"].as_str().unwrap_or("");
                        let msg = format!("v{} 사용 가능. {}", latest, notes);
                        crate::alert_hub::push(
                            "update_available",
                            crate::alert_hub::AlertLevel::Warning,
                            "system",
                            &format!("새 버전 v{} 업데이트", latest),
                            &msg,
                            "navigate:system",
                        );
                        log::info!("Update available: {} → {}", current, latest);
                    } else {
                        crate::alert_hub::resolve("update_available");
                    }
                }
            }
        }

        // 6시간 대기
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(6 * 3600)) => {}
            _ = cancel.changed() => { return; }
        }
    }
}

pub async fn trigger_now() {
    log::info!("Manual trigger: checking all rules");
    tokio::spawn(async { checker::check_my_overdue().await });
    tokio::spawn(async { checker::check_my_deadline().await });
    tokio::spawn(async { checker::check_approval_request().await });
    let slot_key = chrono::Local::now().format("%Y-%m-%d_%H:%M").to_string();
    tokio::spawn(async move { checker::check_overdue_task(&slot_key).await });
    tokio::spawn(async { checker::check_mail().await });
}
