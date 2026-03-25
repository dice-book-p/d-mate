use std::io::{BufRead, BufReader, Read as IoRead, Write as IoWrite};
use std::net::TcpStream;

use crate::database;

pub struct Mail {
    pub uid: String,
    pub from: String,
    pub subject: String,
    pub date: String,
}

pub async fn fetch_new_mails(
    server: &str,
    port: u16,
    use_ssl: bool,
    account: &str,
    password: &str,
) -> Result<Vec<Mail>, String> {
    let server = server.to_string();
    let account = account.to_string();
    let password = password.to_string();

    tokio::task::spawn_blocking(move || {
        fetch_pop3(&server, port, use_ssl, &account, &password)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn fetch_pop3(server: &str, port: u16, use_ssl: bool, account: &str, password: &str) -> Result<Vec<Mail>, String> {
    let addr = format!("{}:{}", server, port);
    let tcp = TcpStream::connect(&addr).map_err(|e| format!("연결 실패: {}", e))?;
    tcp.set_read_timeout(Some(std::time::Duration::from_secs(30))).ok();
    tcp.set_write_timeout(Some(std::time::Duration::from_secs(10))).ok();

    if use_ssl {
        let connector = native_tls::TlsConnector::builder()
            .danger_accept_invalid_certs(true) // 사내 인증서 허용
            .build()
            .map_err(|e| format!("TLS 초기화 실패: {}", e))?;
        let tls_stream = connector
            .connect(server, tcp)
            .map_err(|e| format!("TLS 연결 실패: {}", e))?;

        fetch_pop3_with_stream(tls_stream, account, password)
    } else {
        fetch_pop3_plain(tcp, account, password)
    }
}

fn fetch_pop3_plain(stream: TcpStream, account: &str, password: &str) -> Result<Vec<Mail>, String> {
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut writer = stream;

    read_line(&mut reader)?;
    send_cmd_w(&mut writer, &mut reader, &format!("USER {}", account))?;
    send_cmd_w(&mut writer, &mut reader, &format!("PASS {}", password))?;

    write_line_w(&mut writer, "UIDL")?;
    let uidl_resp = read_multiline(&mut reader)?;
    let mails = process_uidl(&uidl_resp, &mut writer, &mut reader)?;

    write_line_w(&mut writer, "QUIT").ok();
    Ok(mails)
}

fn fetch_pop3_with_stream<S: IoRead + IoWrite>(stream: S, account: &str, password: &str) -> Result<Vec<Mail>, String> {
    // For TLS we use a single buffered stream since we can't clone
    let mut buf_stream = BufStream::new(stream);

    read_line_buf(&mut buf_stream)?;
    send_cmd_buf(&mut buf_stream, &format!("USER {}", account))?;
    send_cmd_buf(&mut buf_stream, &format!("PASS {}", password))?;

    write_line_buf(&mut buf_stream, "UIDL")?;
    let uidl_resp = read_multiline_buf(&mut buf_stream)?;

    let uidl_map = parse_uidl(&uidl_resp);
    let start = if uidl_map.len() > 50 { uidl_map.len() - 50 } else { 0 };
    let mut mails = Vec::new();

    for &(msg_num, ref uid) in &uidl_map[start..] {
        if database::is_notification_sent("mail", uid, "") {
            continue;
        }
        write_line_buf(&mut buf_stream, &format!("TOP {} 0", msg_num))?;
        let headers = read_multiline_buf(&mut buf_stream)?;
        let from = extract_header(&headers, "From");
        let subject = extract_header(&headers, "Subject");
        let date = extract_header(&headers, "Date");
        mails.push(Mail {
            uid: uid.clone(),
            from: decode_mime_header(&from),
            subject: decode_mime_header(&subject),
            date: parse_mail_date(&date),
        });
    }

    write_line_buf(&mut buf_stream, "QUIT").ok();
    Ok(mails)
}

// ── BufStream: 단일 스트림에서 read + write ──────

struct BufStream<S: IoRead + IoWrite> {
    inner: S,
}

impl<S: IoRead + IoWrite> BufStream<S> {
    fn new(stream: S) -> Self {
        Self { inner: stream }
    }
}

fn write_line_buf<S: IoRead + IoWrite>(bs: &mut BufStream<S>, cmd: &str) -> Result<(), String> {
    bs.inner.write_all(format!("{}\r\n", cmd).as_bytes()).map_err(|e| e.to_string())?;
    bs.inner.flush().map_err(|e| e.to_string())
}

fn read_line_buf<S: IoRead + IoWrite>(bs: &mut BufStream<S>) -> Result<String, String> {
    let mut line = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        bs.inner.read_exact(&mut byte).map_err(|e| e.to_string())?;
        if byte[0] == b'\n' {
            break;
        }
        line.push(byte[0]);
    }
    let s = String::from_utf8_lossy(&line).trim_end_matches('\r').to_string();
    Ok(s)
}

fn send_cmd_buf<S: IoRead + IoWrite>(bs: &mut BufStream<S>, cmd: &str) -> Result<String, String> {
    write_line_buf(bs, cmd)?;
    let resp = read_line_buf(bs)?;
    if !resp.starts_with("+OK") {
        return Err(format!("POP3 error: {}", resp));
    }
    Ok(resp)
}

fn read_multiline_buf<S: IoRead + IoWrite>(bs: &mut BufStream<S>) -> Result<Vec<String>, String> {
    let first = read_line_buf(bs)?;
    if !first.starts_with("+OK") {
        return Err(format!("POP3 error: {}", first));
    }
    let mut lines = Vec::new();
    loop {
        let line = read_line_buf(bs)?;
        if line == "." { break; }
        lines.push(if line.starts_with("..") { line[1..].to_string() } else { line });
    }
    Ok(lines)
}

// ── Plain TCP helpers (use separate reader/writer) ──

fn write_line_w(writer: &mut TcpStream, cmd: &str) -> Result<(), String> {
    writer.write_all(format!("{}\r\n", cmd).as_bytes()).map_err(|e| e.to_string())
}

fn read_line(reader: &mut BufReader<TcpStream>) -> Result<String, String> {
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| e.to_string())?;
    Ok(line.trim_end().to_string())
}

fn send_cmd_w(writer: &mut TcpStream, reader: &mut BufReader<TcpStream>, cmd: &str) -> Result<String, String> {
    write_line_w(writer, cmd)?;
    let resp = read_line(reader)?;
    if !resp.starts_with("+OK") {
        return Err(format!("POP3 error: {}", resp));
    }
    Ok(resp)
}

fn read_multiline(reader: &mut BufReader<TcpStream>) -> Result<Vec<String>, String> {
    let first = read_line(reader)?;
    if !first.starts_with("+OK") {
        return Err(format!("POP3 error: {}", first));
    }
    let mut lines = Vec::new();
    loop {
        let line = read_line(reader)?;
        if line == "." { break; }
        lines.push(if line.starts_with("..") { line[1..].to_string() } else { line });
    }
    Ok(lines)
}

fn process_uidl(uidl_resp: &[String], writer: &mut TcpStream, reader: &mut BufReader<TcpStream>) -> Result<Vec<Mail>, String> {
    let uidl_map = parse_uidl(uidl_resp);
    let start = if uidl_map.len() > 50 { uidl_map.len() - 50 } else { 0 };
    let mut mails = Vec::new();

    for &(msg_num, ref uid) in &uidl_map[start..] {
        if database::is_notification_sent("mail", uid, "") {
            continue;
        }
        write_line_w(writer, &format!("TOP {} 0", msg_num))?;
        let headers = read_multiline(reader)?;
        let from = extract_header(&headers, "From");
        let subject = extract_header(&headers, "Subject");
        let date = extract_header(&headers, "Date");
        mails.push(Mail {
            uid: uid.clone(),
            from: decode_mime_header(&from),
            subject: decode_mime_header(&subject),
            date: parse_mail_date(&date),
        });
    }

    Ok(mails)
}

// ── 공통 유틸 ────────────────────────────────────

fn parse_uidl(lines: &[String]) -> Vec<(usize, String)> {
    lines.iter().filter_map(|line| {
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() == 2 {
            parts[0].parse::<usize>().ok().map(|n| (n, parts[1].to_string()))
        } else {
            None
        }
    }).collect()
}

fn extract_header(headers: &[String], name: &str) -> String {
    let prefix = format!("{}:", name);
    let mut result = String::new();
    let mut capturing = false;

    for line in headers {
        if line.to_lowercase().starts_with(&prefix.to_lowercase()) {
            result = line[prefix.len()..].trim().to_string();
            capturing = true;
        } else if capturing && (line.starts_with(' ') || line.starts_with('\t')) {
            result.push(' ');
            result.push_str(line.trim());
        } else {
            capturing = false;
        }
    }
    result
}

fn decode_mime_header(s: &str) -> String {
    let mut result = s.to_string();
    while let Some(start) = result.find("=?") {
        if let Some(end) = result[start + 2..].find("?=") {
            let end = start + 2 + end + 2;
            let encoded = &result[start..end];
            if let Some(decoded) = decode_mime_word(encoded) {
                result = format!("{}{}{}", &result[..start], decoded, &result[end..]);
            } else {
                break;
            }
        } else {
            break;
        }
    }
    result.trim().to_string()
}

fn decode_mime_word(encoded: &str) -> Option<String> {
    let inner = encoded.strip_prefix("=?")?.strip_suffix("?=")?;
    let parts: Vec<&str> = inner.splitn(3, '?').collect();
    if parts.len() != 3 { return None; }

    let encoding = parts[1].to_uppercase();
    let text = parts[2];

    let bytes = match encoding.as_str() {
        "B" => Some(base64_decode(text)?),
        "Q" => {
            let decoded = text.replace('_', " ").bytes().collect::<Vec<u8>>();
            let mut result = Vec::new();
            let mut i = 0;
            while i < decoded.len() {
                if decoded[i] == b'=' && i + 2 < decoded.len() {
                    if let Ok(byte) = u8::from_str_radix(&String::from_utf8_lossy(&decoded[i + 1..i + 3]), 16) {
                        result.push(byte);
                        i += 3;
                        continue;
                    }
                }
                result.push(decoded[i]);
                i += 1;
            }
            Some(result)
        }
        _ => None,
    }?;

    let cloned = bytes.clone();
    String::from_utf8(bytes).ok().or_else(|| Some(String::from_utf8_lossy(&cloned).to_string()))
}

/// "Tue, 25 Mar 2025 14:36:00 +0900" → "2025-03-25 14:36"
fn parse_mail_date(raw: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }
    // Try common RFC 2822 formats
    let formats = [
        "%a, %d %b %Y %H:%M:%S %z",
        "%d %b %Y %H:%M:%S %z",
        "%a, %d %b %Y %H:%M:%S",
        "%d %b %Y %H:%M:%S",
    ];
    for fmt in &formats {
        if let Ok(dt) = chrono::DateTime::parse_from_str(raw.trim(), fmt) {
            let local = dt.with_timezone(&chrono::Local);
            return local.format("%Y-%m-%d %H:%M").to_string();
        }
    }
    // Fallback: return as-is but trimmed
    raw.trim().to_string()
}

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    let table = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = Vec::new();
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;

    for &b in input.as_bytes() {
        if b == b'=' || b == b'\n' || b == b'\r' || b == b' ' { continue; }
        let val = table.iter().position(|&c| c == b)?;
        buf = (buf << 6) | val as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Some(output)
}
