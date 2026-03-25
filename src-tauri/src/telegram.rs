const MAX_LEN: usize = 4096;
const MAX_RETRIES: u32 = 3;
const RETRY_SLEEP_MS: u64 = 1000;

pub async fn send_message(token: &str, chat_id: &str, message: &str) -> bool {
    if token.is_empty() || chat_id.is_empty() || message.is_empty() {
        return false;
    }

    let chunks = split_message(message, MAX_LEN);
    let mut all_ok = true;

    for chunk in &chunks {
        if !send_single(token, chat_id, chunk).await {
            all_ok = false;
        }
        if chunks.len() > 1 {
            tokio::time::sleep(std::time::Duration::from_millis(RETRY_SLEEP_MS)).await;
        }
    }

    all_ok
}

async fn send_single(token: &str, chat_id: &str, text: &str) -> bool {
    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
    let client = reqwest::Client::new();

    for attempt in 0..MAX_RETRIES {
        let resp = client
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "text": text,
                "parse_mode": "HTML"
            }))
            .send()
            .await;

        match resp {
            Ok(r) => {
                let status = r.status().as_u16();
                if status == 200 {
                    return true;
                }
                if status == 401 || status == 403 {
                    log::error!("Telegram auth error ({})", status);
                    return false;
                }
                if status == 429 {
                    let body: serde_json::Value = r.json().await.unwrap_or_default();
                    let retry_after = body["parameters"]["retry_after"].as_u64().unwrap_or(5);
                    log::warn!("Telegram rate limited, waiting {}s", retry_after);
                    tokio::time::sleep(std::time::Duration::from_secs(retry_after)).await;
                    continue;
                }
                log::warn!("Telegram error {} (attempt {}/{})", status, attempt + 1, MAX_RETRIES);
            }
            Err(e) => {
                log::warn!("Telegram request error: {} (attempt {}/{})", e, attempt + 1, MAX_RETRIES);
            }
        }

        if attempt < MAX_RETRIES - 1 {
            tokio::time::sleep(std::time::Duration::from_millis(RETRY_SLEEP_MS)).await;
        }
    }

    false
}

fn split_message(text: &str, max_len: usize) -> Vec<String> {
    if text.len() <= max_len {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if remaining.len() <= max_len {
            chunks.push(remaining.to_string());
            break;
        }

        let boundary = &remaining[..max_len];
        let split_at = boundary.rfind('\n').unwrap_or(max_len);
        let split_at = if split_at == 0 { max_len } else { split_at };

        chunks.push(remaining[..split_at].to_string());
        remaining = &remaining[split_at..].trim_start_matches('\n');
    }

    chunks
}
