// AI 사용량: 클로드·GPT(코덱스)·제미나이의 5시간·주간 한도를 5분마다 받는다(2026-11-13).
// 각 회사 공식 프로그램이 이 PC 에 남긴 로그인을 그대로 쓴다(CodexBar·claude-meter 와 같은 방식).
// - 클로드: ~/.claude/.credentials.json → api.anthropic.com/api/oauth/usage. 만료되면 갱신해 같은 파일에 저장(사용자 허락).
// - GPT: ~/.codex/auth.json → chatgpt.com/backend-api/wham/usage. 갱신은 코덱스가 한다(여기서 돌리면 코덱스 로그인이 풀릴 수 있다).
// - 제미나이: ~/.gemini/oauth_creds.json → cloudcode-pa.googleapis.com 할당량. 갱신은 제미나이 CLI 가 한다.
// 로그인이 없거나 풀리면 '연결' 단추가 공식 프로그램의 로그인을 콘솔로 띄운다. 토큰 값은 로그에 절대 남기지 않는다.
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use windows::core::{Result, HSTRING};
use windows::Foundation::Uri;
use windows::Storage::Streams::UnicodeEncoding;
use windows::Web::Http::{HttpClient, HttpMethod, HttpRequestMessage, HttpStringContent};

#[derive(Clone, Debug, Default)]
pub struct Bar {
    pub label: String, // "5시간", "주간", "프로" …
    pub pct: f32,      // 쓴 비율 0~100
    pub reset: i64,    // 초기화 시각(유닉스 초, 0 이면 모름)
}

#[derive(Clone, Debug, Default)]
pub struct Svc {
    pub name: &'static str,
    pub login: bool,     // 쓸 수 있는 로그인이 있다
    pub bars: Vec<Bar>,
    pub plan: String,    // 요금제(있으면)
    pub note: String,    // 안내·오류(비어 있으면 정상)
    pub stamp: String,   // 마지막으로 받은 시각
    pub seen: Option<Instant>,
}

#[derive(Clone, Debug, Default)]
pub struct Usage {
    pub svcs: Vec<Svc>,
    pub ver: u64,
}

pub const NAMES: [&str; 3] = ["클로드", "GPT", "제미나이"];

pub fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// "2026-11-13T18:00:00.123+00:00" / "...Z" → 유닉스 초
pub fn rfc3339(s: &str) -> Option<i64> {
    let n = |a: usize, b: usize| s.get(a..b).and_then(|x| x.parse::<i64>().ok());
    let day = crate::gcal::days(n(0, 4)? as i32, n(5, 7)? as u32, n(8, 10)? as u32);
    let mut t = day * 86400 + n(11, 13)? * 3600 + n(14, 16)? * 60 + n(17, 19)?;
    let tail = &s[19..];
    if let Some(p) = tail.rfind(['+', '-']).filter(|_| !tail.ends_with('Z')) {
        let z = &tail[p..];
        let sign = if z.starts_with('-') { -1 } else { 1 };
        let h = z.get(1..3).and_then(|x| x.parse::<i64>().ok()).unwrap_or(0);
        let m = z.get(4..6).and_then(|x| x.parse::<i64>().ok()).unwrap_or(0);
        t -= sign * (h * 3600 + m * 60);
    }
    Some(t)
}

fn req(c: &HttpClient, method: &str, url: &str, heads: &[(&str, &str)], body: Option<(&str, &str)>) -> Result<(u16, String)> {
    let msg = HttpRequestMessage::Create(&HttpMethod::Create(&HSTRING::from(method))?, &Uri::CreateUri(&HSTRING::from(url))?)?;
    let h = msg.Headers()?;
    for (k, v) in heads {
        h.TryAppendWithoutValidation(&HSTRING::from(*k), &HSTRING::from(*v))?;
    }
    if let Some((text, kind)) = body {
        msg.SetContent(&HttpStringContent::CreateFromStringWithEncodingAndMediaType(&HSTRING::from(text), UnicodeEncoding::Utf8, &HSTRING::from(kind))?)?;
    }
    let r = c.SendRequestAsync(&msg)?.get()?;
    Ok((r.StatusCode()?.0 as u16, r.Content()?.ReadAsStringAsync()?.get()?.to_string()))
}

fn home() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default())
}

// ── 클로드 ──

const CLAUDE_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e"; // 클로드 코드의 공개 앱 번호

/// 클로드 사용량 응답 → 막대들
pub fn parse_claude(v: &Value) -> Vec<Bar> {
    let mut out = Vec::new();
    for (k, label) in [("five_hour", "5시간"), ("seven_day", "주간"), ("seven_day_opus", "주간 오퍼스"), ("seven_day_sonnet", "주간 소넷")] {
        let w = &v[k];
        if let Some(p) = w["utilization"].as_f64() {
            out.push(Bar { label: label.into(), pct: p as f32, reset: w["resets_at"].as_str().and_then(rfc3339).unwrap_or(0) });
        }
    }
    out
}

fn claude(c: &HttpClient) -> Svc {
    let mut s = Svc { name: NAMES[0], ..Default::default() };
    let path = home().join(".claude").join(".credentials.json");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        s.note = "로그인이 없습니다".into();
        return s;
    };
    let Ok(mut full) = serde_json::from_str::<Value>(&raw) else { return s };
    s.plan = full["claudeAiOauth"]["subscriptionType"].as_str().unwrap_or("").to_string();
    for attempt in 0..2 {
        let o = &full["claudeAiOauth"];
        let exp = o["expiresAt"].as_i64().unwrap_or(0) / 1000;
        if attempt == 1 || exp < now() + 120 {
            // 갱신해서 같은 파일에 저장(다른 칸은 그대로)
            let body = json!({"grant_type": "refresh_token", "refresh_token": o["refreshToken"].as_str().unwrap_or(""), "client_id": CLAUDE_ID}).to_string();
            match req(c, "POST", "https://console.anthropic.com/v1/oauth/token", &[("User-Agent", "notch-cat")], Some((&body, "application/json"))) {
                Ok((200, b)) => {
                    let r: Value = serde_json::from_str(&b).unwrap_or_default();
                    let t = &mut full["claudeAiOauth"];
                    t["accessToken"] = r["access_token"].clone();
                    if r["refresh_token"].is_string() {
                        t["refreshToken"] = r["refresh_token"].clone();
                    }
                    t["expiresAt"] = json!((now() + r["expires_in"].as_i64().unwrap_or(3600)) * 1000);
                    let tmp = path.with_extension("json.notch");
                    if std::fs::write(&tmp, full.to_string()).is_ok() && std::fs::rename(&tmp, &path).is_ok() {
                        crate::log("ai: 클로드 로그인 갱신");
                    }
                }
                Ok((st, _)) => {
                    crate::log(&format!("ai: 클로드 갱신 실패 {st}"));
                    s.note = "로그인이 만료됐습니다. 연결을 눌러 주세요".into();
                    return s;
                }
                Err(e) => {
                    s.note = format!("닿지 않습니다: {e}");
                    return s;
                }
            }
        }
        let tok = full["claudeAiOauth"]["accessToken"].as_str().unwrap_or("").to_string();
        match req(c, "GET", "https://api.anthropic.com/api/oauth/usage", &[("Authorization", &format!("Bearer {tok}")), ("anthropic-beta", "oauth-2025-04-20"), ("User-Agent", "notch-cat")], None) {
            Ok((200, b)) => {
                let v: Value = serde_json::from_str(&b).unwrap_or_default();
                s.bars = parse_claude(&v);
                s.login = true;
                if s.bars.is_empty() {
                    // 모양이 바뀌었으면 칸 이름만 남긴다(값은 안 남김)
                    crate::log(&format!("ai: 클로드 응답 칸 {:?}", v.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>())));
                }
                return s;
            }
            Ok((401, _)) if attempt == 0 => continue,
            Ok((st, _)) => {
                s.note = format!("사용량을 못 받았습니다({st})");
                s.login = st != 401;
                return s;
            }
            Err(e) => {
                s.note = format!("닿지 않습니다: {e}");
                s.login = true;
                return s;
            }
        }
    }
    s
}

// ── GPT(코덱스) ──

pub fn parse_codex(v: &Value) -> Vec<Bar> {
    let rl = &v["rate_limit"];
    let mut out = Vec::new();
    for (k, label) in [("primary_window", "5시간"), ("secondary_window", "주간")] {
        let w = &rl[k];
        if let Some(p) = w["used_percent"].as_f64() {
            let reset = w["reset_at"].as_i64().or_else(|| w["reset_after_seconds"].as_i64().map(|x| now() + x)).unwrap_or(0);
            out.push(Bar { label: label.into(), pct: p as f32, reset });
        }
    }
    out
}

fn codex(c: &HttpClient) -> Svc {
    let mut s = Svc { name: NAMES[1], ..Default::default() };
    let Some(v) = std::fs::read_to_string(home().join(".codex").join("auth.json")).ok().and_then(|r| serde_json::from_str::<Value>(&r).ok()) else {
        s.note = "로그인이 없습니다".into();
        return s;
    };
    let t = &v["tokens"];
    let (tok, acct) = (t["access_token"].as_str().unwrap_or(""), t["account_id"].as_str().unwrap_or(""));
    if tok.is_empty() {
        s.note = "로그인이 없습니다".into();
        return s;
    }
    match req(c, "GET", "https://chatgpt.com/backend-api/wham/usage", &[("Authorization", &format!("Bearer {tok}")), ("ChatGPT-Account-Id", acct), ("User-Agent", "codex-cli")], None) {
        Ok((200, b)) => {
            let v: Value = serde_json::from_str(&b).unwrap_or_default();
            s.bars = parse_codex(&v);
            s.plan = v["plan_type"].as_str().unwrap_or("").to_string();
            s.login = true;
        }
        Ok((401 | 403, _)) => s.note = "로그인이 만료됐습니다. 연결을 눌러 주세요".into(),
        Ok((st, _)) => {
            s.note = format!("사용량을 못 받았습니다({st})");
            s.login = true;
        }
        Err(e) => {
            s.note = format!("닿지 않습니다: {e}");
            s.login = true;
        }
    }
    s
}

// ── 제미나이 ──

/// 할당량 응답 → 모델 종류별(프로·플래시) 가장 많이 쓴 것
pub fn parse_gemini(v: &Value) -> Vec<Bar> {
    let mut out: Vec<Bar> = Vec::new();
    for b in v["buckets"].as_array().into_iter().flatten() {
        let model = b["modelId"].as_str().unwrap_or("");
        let label = if model.contains("pro") { "프로" } else if model.contains("flash") { "플래시" } else { continue };
        let Some(left) = b["remainingFraction"].as_f64() else { continue };
        let bar = Bar { label: label.into(), pct: ((1.0 - left) * 100.0) as f32, reset: b["resetTime"].as_str().and_then(rfc3339).unwrap_or(0) };
        match out.iter_mut().find(|x| x.label == label) {
            Some(x) if x.pct < bar.pct => *x = bar,
            Some(_) => {}
            None => out.push(bar),
        }
    }
    out
}

fn gemini(c: &HttpClient) -> Svc {
    let mut s = Svc { name: NAMES[2], ..Default::default() };
    let Some(v) = std::fs::read_to_string(home().join(".gemini").join("oauth_creds.json")).ok().and_then(|r| serde_json::from_str::<Value>(&r).ok()) else {
        s.note = "로그인이 없습니다".into();
        return s;
    };
    let tok = v["access_token"].as_str().unwrap_or("").to_string();
    if v["expiry_date"].as_i64().unwrap_or(0) / 1000 < now() + 60 {
        s.note = "로그인이 오래됐습니다. 연결을 누르면 제미나이가 새로 받습니다".into();
        return s;
    }
    let auth = format!("Bearer {tok}");
    let heads = [("Authorization", auth.as_str()), ("User-Agent", "notch-cat")];
    let meta = r#"{"metadata":{"ideType":"IDE_UNSPECIFIED","platform":"PLATFORM_UNSPECIFIED","pluginType":"GEMINI"}}"#;
    let project = match req(c, "POST", "https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist", &heads, Some((meta, "application/json"))) {
        Ok((200, b)) => serde_json::from_str::<Value>(&b).ok().and_then(|x| x["cloudaicompanionProject"].as_str().map(str::to_string)).unwrap_or_default(),
        Ok((st, _)) => {
            s.note = format!("제미나이가 응답하지 않습니다({st})");
            s.login = st != 401;
            return s;
        }
        Err(e) => {
            s.note = format!("닿지 않습니다: {e}");
            return s;
        }
    };
    let body = json!({"project": project}).to_string();
    match req(c, "POST", "https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuota", &heads, Some((&body, "application/json"))) {
        Ok((200, b)) => {
            let v: Value = serde_json::from_str(&b).unwrap_or_default();
            s.bars = parse_gemini(&v);
            s.login = true;
            if s.bars.is_empty() {
                crate::log(&format!("ai: 제미나이 응답 칸 {:?}", v.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>())));
            }
        }
        Ok((st, _)) => {
            s.note = format!("할당량을 못 받았습니다({st})");
            s.login = true;
        }
        Err(e) => s.note = format!("닿지 않습니다: {e}"),
    }
    s
}

/// 5분마다(연결을 누른 뒤 5분 동안은 10초마다) 세 곳을 받는다
pub fn spawn(out: Arc<Mutex<Usage>>, poke: Arc<Mutex<Option<Instant>>>) {
    std::thread::spawn(move || {
        let Ok(c) = HttpClient::new() else { return };
        let mut last: Vec<(bool, usize, String)> = Vec::new();
        loop {
            let svcs: Vec<Svc> = vec![claude(&c), codex(&c), gemini(&c)];
            let stamp = crate::hhmm();
            let sig: Vec<(bool, usize, String)> = svcs.iter().map(|s| (s.login, s.bars.len(), s.note.clone())).collect();
            if sig != last {
                crate::log(&format!("ai: {}", svcs.iter().map(|s| format!("{} {}", s.name, if s.login { s.bars.iter().map(|b| format!("{} {:.0}%", b.label, b.pct)).collect::<Vec<_>>().join(" ") } else { s.note.clone() })).collect::<Vec<_>>().join(" / ")));
                last = sig;
            }
            {
                let mut o = out.lock().unwrap();
                o.svcs = svcs.into_iter().map(|mut s| {
                    if s.login && s.note.is_empty() {
                        s.stamp = stamp.clone();
                        s.seen = Some(Instant::now());
                    }
                    s
                }).collect();
                o.ver += 1;
            }
            let fast = poke.lock().unwrap().is_some_and(|t| t.elapsed() < Duration::from_secs(300));
            let wait = if fast { 10 } else { 300 };
            for _ in 0..wait {
                std::thread::sleep(Duration::from_secs(1));
                if poke.lock().unwrap().is_some_and(|t| t.elapsed() < Duration::from_secs(2)) {
                    break; // 방금 연결을 눌렀다
                }
            }
        }
    });
}

/// 공식 프로그램의 로그인을 콘솔 창으로 띄운다(브라우저에서 허용만 누르면 된다). 창은 맨 앞으로.
pub fn login(i: usize) {
    let cmd = match i {
        0 => "claude auth login",
        1 => "where codex >nul 2>nul || npm i -g @openai/codex & codex login",
        _ => "where gemini >nul 2>nul || npm i -g @google/gemini-cli & gemini",
    };
    let title = format!("노치 {} 연결", NAMES[i.min(2)]);
    crate::log(&format!("ai: {} 연결 창", NAMES[i.min(2)]));
    std::thread::spawn(move || {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("cmd").raw_arg(format!("/k \"title {title} & echo {title}: 브라우저가 열리면 계정을 고르고 허용을 눌러 주세요. 끝나면 이 창을 닫아도 됩니다. & timeout /t 2 >nul & {cmd}\"")).creation_flags(0x0000_0010).spawn();
        // 노치는 앞으로 못 나서는 창이라 새 창이 뒤에 숨는다 → 제목으로 찾아 잠깐 맨 위로
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::*;
            let t: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
            for _ in 0..30 {
                std::thread::sleep(Duration::from_millis(100));
                if let Ok(h) = FindWindowW(None, windows::core::PCWSTR(t.as_ptr())) {
                    if !h.0.is_null() {
                        let _ = SetWindowPos(h, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW);
                        std::thread::sleep(Duration::from_millis(300));
                        let _ = SetWindowPos(h, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW);
                        let _ = SetForegroundWindow(h);
                        break;
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_usage() {
        assert_eq!(rfc3339("1970-01-02T00:00:00Z"), Some(86400));
        assert_eq!(rfc3339("1970-01-02T09:00:00+09:00"), Some(86400));
        assert_eq!(rfc3339("1970-01-02T00:00:00.123456+00:00"), Some(86400));
        let c = parse_claude(&serde_json::json!({"five_hour": {"utilization": 12.0, "resets_at": "1970-01-02T00:00:00Z"}, "seven_day": {"utilization": 40.5, "resets_at": null}, "seven_day_opus": null}));
        assert_eq!(c.iter().map(|b| (b.label.as_str(), b.pct, b.reset)).collect::<Vec<_>>(), vec![("5시간", 12.0, 86400), ("주간", 40.5, 0)]);
        let x = parse_codex(&serde_json::json!({"rate_limit": {"primary_window": {"used_percent": 1, "reset_at": 100}, "secondary_window": {"used_percent": 17, "reset_at": 200}}}));
        assert_eq!(x.iter().map(|b| (b.pct, b.reset)).collect::<Vec<_>>(), vec![(1.0, 100), (17.0, 200)]);
        let g = parse_gemini(&serde_json::json!({"buckets": [{"modelId": "gemini-2.5-pro", "remainingFraction": 0.9}, {"modelId": "gemini-2.5-pro-preview", "remainingFraction": 0.5}, {"modelId": "gemini-2.5-flash", "remainingFraction": 1.0}]}));
        assert_eq!(g.iter().map(|b| (b.label.as_str(), b.pct.round())).collect::<Vec<_>>(), vec![("프로", 50.0), ("플래시", 0.0)]);
    }
}
