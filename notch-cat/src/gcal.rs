// 구글 캘린더: 계정마다 브라우저에서 '계정 선택 → 허용'으로 연결하고(OAuth, 내 PC 안에서 받는 방식),
// 앞뒤 몇 주 일정을 5분마다 받아 오며, 노치에서 넣은 일정을 그 캘린더에 바로 넣는다.
// - 등록증(client id·secret): 빌드 때 NOTCH_GOOGLE_ID·NOTCH_GOOGLE_SECRET, 없으면 실행 파일 옆이나
//   %APPDATA%\notch-cat 의 google_client.json(구글 클라우드에서 받은 그 파일). 공개 저장소에는 두지 않는다.
// - 로그인 정보(갱신 열쇠)는 %APPDATA%\notch-cat\google.dat 에 윈도우 암호화(DPAPI)로. 이 PC·이 사용자만 풀 수 있다.
use std::io::{Read, Write};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use windows::core::{Result, HSTRING, PCWSTR};
use windows::Foundation::Uri;
use windows::Storage::Streams::UnicodeEncoding;
use windows::Web::Http::{HttpClient, HttpMethod, HttpRequestMessage, HttpStringContent};

// ── 날짜 셈(한국 시각 기준). 날짜는 1970-01-01 부터 센 날 수로 다룬다. ──

pub fn days(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

pub fn ymd(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    ((yoe + era * 400 + (m <= 2) as i64) as i32, m, d)
}

/// 0 = 일요일
pub fn weekday(z: i64) -> usize {
    (z + 4).rem_euclid(7) as usize
}

pub const WEEK: [&str; 7] = ["일", "월", "화", "수", "목", "금", "토"];

/// "2026-11-12T10:00:00+09:00" / "2026-11-12T01:00:00Z" → 한국 시각 (날, 분). "2026-11-12" → (날, -1) = 종일
pub fn parse_when(s: &str) -> Option<(i64, i32)> {
    let num = |a: usize, b: usize| s.get(a..b)?.parse::<i64>().ok();
    let day = days(num(0, 4)? as i32, num(5, 7)? as u32, num(8, 10)? as u32);
    if s.len() == 10 {
        return Some((day, -1));
    }
    let mut min = num(11, 13)? * 60 + num(14, 16)?;
    let tz = s.get(19..).unwrap_or("").trim_start_matches(|c: char| c == '.' || c.is_ascii_digit());
    let off = match tz {
        "" | "Z" => 0,
        t => {
            let sign = if t.starts_with('-') { -1 } else { 1 };
            sign * (t.get(1..3)?.parse::<i64>().ok()? * 60 + t.get(4..6)?.parse::<i64>().ok()?)
        }
    };
    min += 540 - off; // 한국 시각으로
    Some((day + min.div_euclid(1440), min.rem_euclid(1440) as i32))
}

pub fn rfc3339(day: i64, min: i32) -> String {
    let (y, m, d) = ymd(day);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:00+09:00", min / 60, min % 60)
}

pub fn date_str(day: i64) -> String {
    let (y, m, d) = ymd(day);
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn today() -> (i64, i32) {
    let t = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    (days(t.wYear as i32, t.wMonth as u32, t.wDay as u32), t.wHour as i32 * 60 + t.wMinute as i32)
}

// ── 받은 것 ──

#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub title: String,
    pub place: String,
    pub day: i64,     // 시작 날
    pub start: i32,   // 시작 분(종일이면 -1)
    pub end_day: i64, // 끝 날(종일이면 마지막 날 포함)
    pub end: i32,
    pub color: (f32, f32, f32),
    pub memo: String, // 설명(태그를 걷어 낸 글)
}

#[derive(Clone, Debug)]
pub struct CalInfo {
    pub account: usize,
    pub id: String,
    pub name: String,
    pub color: (f32, f32, f32),
    pub writable: bool,
}

#[derive(Default, Clone)]
pub struct Cal {
    pub ready: bool,           // 등록증이 있다
    pub accounts: Vec<String>, // 연결한 계정(메일 주소)
    pub cals: Vec<CalInfo>,
    pub events: Vec<Event>,
    pub note: String, // 오류·안내(비어 있으면 정상)
    pub stamp: String,
    pub seen: Option<Instant>,
    pub ver: u64,
}

impl Cal {
    /// 그날 걸친 일정(종일 먼저, 그다음 시각순)
    pub fn on(&self, day: i64) -> Vec<&Event> {
        let mut v: Vec<&Event> = self.events.iter().filter(|e| e.day <= day && day <= e.end_day).collect();
        v.sort_by_key(|e| (e.start >= 0, e.start, e.title.clone()));
        v
    }
}

pub enum Cmd {
    Login,
    Refresh,
    Add { cal: usize, title: String, day: i64, start: i32, end: i32 }, // start -1 = 종일
    Naver { id: String, pw: String }, // 네이버 캘린더 연결(아이디·애플리케이션 비밀번호)
}

fn hex_color(s: &str) -> (f32, f32, f32) {
    let h = |i: usize| u8::from_str_radix(s.get(i..i + 2).unwrap_or("88"), 16).unwrap_or(0x88) as f32 / 255.0;
    if s.len() == 7 { (h(1), h(3), h(5)) } else { (0.53, 0.6, 0.7) }
}

/// 구글 일정 설명(HTML 이 섞여 온다) → 보통 글. <br>·</p> 는 줄바꿈, 나머지 태그는 지운다.
pub fn plain(s: &str) -> String {
    let s = s.replace("<br>", "
").replace("<br/>", "
").replace("<br />", "
").replace("</p>", "
").replace("</li>", "
");
    let mut out = String::new();
    let mut tag = false;
    for c in s.chars() {
        match c {
            '<' => tag = true,
            '>' if tag => tag = false,
            _ if !tag => out.push(c),
            _ => {}
        }
    }
    let out = out.replace("&nbsp;", " ").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'").replace("&amp;", "&");
    out.lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("
")
}

/// 일정 목록 응답 → 일정들
pub fn parse_events(body: &str, color: (f32, f32, f32)) -> Vec<Event> {
    let v: Value = serde_json::from_str(body).unwrap_or_default();
    let mut out = Vec::new();
    for e in v["items"].as_array().into_iter().flatten() {
        if e["status"].as_str() == Some("cancelled") {
            continue;
        }
        let when = |k: &str| e[k]["dateTime"].as_str().or(e[k]["date"].as_str()).and_then(parse_when);
        let (Some((d0, s0)), Some((mut d1, e1))) = (when("start"), when("end")) else { continue };
        if s0 < 0 {
            d1 -= 1; // 종일 일정의 끝 날짜는 '다음 날'로 온다
        }
        out.push(Event { title: e["summary"].as_str().unwrap_or("(제목 없음)").to_string(), place: e["location"].as_str().unwrap_or("").to_string(), day: d0, start: s0, end_day: d1.max(d0), end: e1, color, memo: plain(e["description"].as_str().unwrap_or("")) });
    }
    out
}

// ── 통신 ──

fn send(c: &HttpClient, method: &str, url: &str, bearer: Option<&str>, body: Option<(&str, &str)>) -> Result<(u16, String)> {
    let m = if method == "POST" { HttpMethod::Post()? } else { HttpMethod::Get()? };
    let msg = HttpRequestMessage::Create(&m, &Uri::CreateUri(&HSTRING::from(url))?)?;
    if let Some(t) = bearer {
        msg.Headers()?.TryAppendWithoutValidation(&HSTRING::from("Authorization"), &HSTRING::from(format!("Bearer {t}")))?;
    }
    if let Some((text, kind)) = body {
        msg.SetContent(&HttpStringContent::CreateFromStringWithEncodingAndMediaType(&HSTRING::from(text), UnicodeEncoding::Utf8, &HSTRING::from(kind))?)?;
    }
    let r = c.SendRequestAsync(&msg)?.get()?;
    Ok((r.StatusCode()?.0 as u16, r.Content()?.ReadAsStringAsync()?.get()?.to_string()))
}

pub fn enc(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

fn dec(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' if i + 2 < b.len() => {
                out.push(u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap_or(b'?'));
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn b64url(s: &str) -> Vec<u8> {
    let val = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'-' | b'+' => Some(62),
        b'_' | b'/' => Some(63),
        _ => None,
    };
    let (mut acc, mut bits, mut out) = (0u32, 0, Vec::new());
    for v in s.bytes().filter_map(val) {
        acc = acc << 6 | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    out
}

pub fn random(n: usize) -> String {
    let mut buf = vec![0u8; n];
    unsafe {
        let _ = windows::Win32::Security::Cryptography::BCryptGenRandom(None, &mut buf, windows::Win32::Security::Cryptography::BCRYPT_USE_SYSTEM_PREFERRED_RNG);
    }
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    buf.iter().map(|b| A[*b as usize % A.len()] as char).collect()
}

// ── 등록증·로그인 정보 ──

pub fn client() -> Option<(String, String)> {
    if let (Some(id), Some(sec)) = (option_env!("NOTCH_GOOGLE_ID"), option_env!("NOTCH_GOOGLE_SECRET")) {
        // 비밀값을 붙여 넣을 때 끝에 줄바꿈이 따라 들어가면 구글이 '그런 앱 없음(invalid_client)'으로 거절한다
        let (id, sec) = (id.trim(), sec.trim());
        if !id.is_empty() {
            return Some((id.into(), sec.into()));
        }
    }
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    for d in [exe_dir, crate::data_dir()] {
        let v: Value = std::fs::read_to_string(d.join("google_client.json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        let c = if v["installed"].is_object() { &v["installed"] } else { &v };
        if let (Some(id), Some(sec)) = (c["client_id"].as_str(), c["client_secret"].as_str()) {
            return Some((id.into(), sec.into()));
        }
    }
    None
}

fn blob(b: &[u8]) -> windows::Win32::Security::Cryptography::CRYPT_INTEGER_BLOB {
    windows::Win32::Security::Cryptography::CRYPT_INTEGER_BLOB { cbData: b.len() as u32, pbData: b.as_ptr() as *mut u8 }
}

/// 계정들: [{email, refresh}]
pub fn load_accounts() -> Vec<(String, String)> {
    use windows::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};
    let Ok(raw) = std::fs::read(crate::data_dir().join("google.dat")) else { return Vec::new() };
    let mut out = CRYPT_INTEGER_BLOB::default();
    let plain = unsafe {
        if CryptUnprotectData(&blob(&raw), None, None, None, None, 0, &mut out).is_err() {
            return Vec::new();
        }
        let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
        let _ = windows::Win32::Foundation::LocalFree(windows::Win32::Foundation::HLOCAL(out.pbData as _));
        v
    };
    let v: Value = serde_json::from_slice(&plain).unwrap_or_default();
    v.as_array().into_iter().flatten().filter_map(|a| Some((a["email"].as_str()?.to_string(), a["refresh"].as_str()?.to_string()))).collect()
}

fn save_accounts(list: &[(String, String)]) {
    use windows::Win32::Security::Cryptography::{CryptProtectData, CRYPT_INTEGER_BLOB};
    let plain = json!(list.iter().map(|(e, r)| json!({"email": e, "refresh": r})).collect::<Vec<_>>()).to_string();
    let mut out = CRYPT_INTEGER_BLOB::default();
    unsafe {
        if CryptProtectData(&blob(plain.as_bytes()), PCWSTR::null(), None, None, None, 0, &mut out).is_ok() {
            let _ = std::fs::write(crate::data_dir().join("google.dat"), std::slice::from_raw_parts(out.pbData, out.cbData as usize));
            let _ = windows::Win32::Foundation::LocalFree(windows::Win32::Foundation::HLOCAL(out.pbData as _));
        }
    }
}

/// 브라우저에서 계정을 골라 허용하면 (메일, 갱신 열쇠)
fn login(c: &HttpClient, id: &str, secret: &str, rx: &mpsc::Receiver<Cmd>, later: &mut Vec<Cmd>) -> std::result::Result<(String, String), String> {
    let l = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = l.local_addr().map_err(|e| e.to_string())?.port();
    let redirect = format!("http://127.0.0.1:{port}");
    let (verifier, state) = (random(64), random(24));
    let url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt={}&code_challenge={verifier}&code_challenge_method=plain&state={state}",
        enc(id), enc(&redirect), enc("openid email https://www.googleapis.com/auth/calendar"), enc("select_account consent")
    );
    // 시험용: 실행 파일 옆에 login_url.txt 가 있으면 브라우저를 열지 않고 주소를 거기에 적는다
    let hook = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join("login_url.txt"))).filter(|f| f.exists());
    let open = || match &hook {
        Some(f) => {
            let _ = std::fs::write(f, &url);
        }
        None => unsafe {
            windows::Win32::UI::Shell::ShellExecuteW(None, &HSTRING::from("open"), &HSTRING::from(url.as_str()), None, None, windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL);
        },
    };
    open();
    // 5분 기다린다. 그 사이 '계정 추가'를 또 누르면(창을 닫아 버렸을 때) 로그인 창을 다시 띄운다.
    l.set_nonblocking(true).map_err(|e| e.to_string())?;
    let mut started = Instant::now();
    let mut sock = loop {
        while let Ok(m) = rx.try_recv() {
            if let Cmd::Login = m {
                crate::log("gcal: 로그인 창 다시 띄움");
                open();
                started = Instant::now();
            } else {
                later.push(m);
            }
        }
        match l.accept() {
            Ok((s, _)) => break s,
            Err(_) if started.elapsed() < Duration::from_secs(300) => std::thread::sleep(Duration::from_millis(200)),
            Err(_) => return Err("로그인을 5분 안에 끝내지 않아 멈췄습니다".into()),
        }
    };
    let _ = sock.set_nonblocking(false);
    let _ = sock.set_read_timeout(Some(Duration::from_secs(5)));
    let mut buf = [0u8; 4096];
    let n = sock.read(&mut buf).unwrap_or(0);
    let req = String::from_utf8_lossy(&buf[..n]).to_string();
    let query = req.split_whitespace().nth(1).and_then(|p| p.split_once('?')).map(|x| x.1.to_string()).unwrap_or_default();
    let param = |k: &str| query.split('&').find_map(|kv| kv.strip_prefix(&format!("{k}="))).map(dec);
    let ok = param("state").as_deref() == Some(state.as_str()) && param("code").is_some();
    let page = if ok { "노치 고양이에 구글 캘린더를 연결했습니다. 이 창은 닫으셔도 됩니다." } else { "연결하지 못했습니다. 노치 고양이에서 다시 시도해 주세요." };
    let html = format!("<!doctype html><meta charset=utf-8><title>노치 고양이</title><body style='background:#000;color:#f3f4f6;font:16px Malgun Gothic;display:grid;place-items:center;height:90vh'><p>{page}</p>");
    let _ = sock.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}", html.len()).as_bytes());
    if !ok {
        return Err(param("error").unwrap_or_else(|| "허용하지 않았습니다".into()));
    }
    let form = format!("code={}&client_id={}&client_secret={}&redirect_uri={}&grant_type=authorization_code&code_verifier={verifier}", enc(&param("code").unwrap()), enc(id), enc(secret), enc(&redirect));
    let (st, body) = send(c, "POST", "https://oauth2.googleapis.com/token", None, Some((&form, "application/x-www-form-urlencoded"))).map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&body).unwrap_or_default();
    let refresh = v["refresh_token"].as_str().ok_or(format!("열쇠를 못 받았습니다({st})"))?.to_string();
    let idt = v["id_token"].as_str().unwrap_or("");
    let claims: Value = serde_json::from_slice(&b64url(idt.split('.').nth(1).unwrap_or(""))).unwrap_or_default();
    Ok((claims["email"].as_str().unwrap_or("구글 계정").to_string(), refresh))
}

fn access(c: &HttpClient, id: &str, secret: &str, refresh: &str) -> Option<String> {
    let form = format!("client_id={}&client_secret={}&refresh_token={}&grant_type=refresh_token", enc(id), enc(secret), enc(refresh));
    let (_, body) = send(c, "POST", "https://oauth2.googleapis.com/token", None, Some((&form, "application/x-www-form-urlencoded"))).ok()?;
    serde_json::from_str::<Value>(&body).ok()?["access_token"].as_str().map(str::to_string)
}

pub fn spawn(out: Arc<Mutex<Cal>>) -> mpsc::Sender<Cmd> {
    let (tx, rx) = mpsc::channel::<Cmd>();
    std::thread::spawn(move || {
        // 비밀번호가 틀려도 윈도우가 따로 로그인 창을 띄우지 않게
        let Ok(c) = windows::Web::Http::Filters::HttpBaseProtocolFilter::new().and_then(|f| { f.SetAllowUI(false)?; HttpClient::Create(&f) }) else { return };
        let mut tokens: Vec<Option<(String, Instant)>> = Vec::new();
        let mut cmd: Option<Cmd> = Some(Cmd::Refresh);
        let mut seen_counts = (usize::MAX, 0);
        let mut later: Vec<Cmd> = Vec::new(); // 로그인을 기다리는 동안 들어온 다른 명령
        loop {
            let Some((id, secret)) = client() else {
                let mut o = out.lock().unwrap();
                o.ready = false;
                o.ver += 1;
                drop(o);
                std::thread::sleep(Duration::from_secs(30));
                continue;
            };
            let mut accts = load_accounts();
            let mut note = String::new();
            match cmd.take() {
                Some(Cmd::Login) => match login(&c, &id, &secret, &rx, &mut later) {
                    Ok((email, refresh)) => {
                        accts.retain(|a| a.0 != email);
                        accts.push((email.clone(), refresh));
                        save_accounts(&accts);
                        crate::log(&format!("gcal: 계정 연결 {}", accts.len()));
                    }
                    Err(e) => {
                        crate::log(&format!("gcal: 구글 연결 실패 {e}"));
                        note = format!("연결 실패: {e}");
                    }
                },
                Some(Cmd::Naver { id: nid, pw }) => match crate::naver::calendars(&c, &nid, &pw) {
                    Ok(list) => {
                        let key = format!("{}{nid}", crate::naver::TAG);
                        accts.retain(|a| a.0 != key);
                        accts.push((key, pw));
                        save_accounts(&accts);
                        crate::log(&format!("gcal: 네이버 연결, 캘린더 {}", list.len()));
                    }
                    Err(e) => {
                        crate::log(&format!("gcal: 네이버 연결 실패 {e}"));
                        note = format!("네이버 연결 실패: {e}");
                    }
                },
                Some(Cmd::Add { cal, title, day, start, end }) => {
                    let info = out.lock().unwrap().cals.get(cal).cloned();
                    let tok = info.as_ref().and_then(|i| accts.get(i.account)).filter(|a| !a.0.starts_with(crate::naver::TAG)).and_then(|a| access(&c, &id, &secret, &a.1));
                    let body = if start < 0 {
                        json!({"summary": title, "start": {"date": date_str(day)}, "end": {"date": date_str(day + 1)}})
                    } else {
                        let (ed, em) = if end > start { (day, end) } else { (day + (start + 60) as i64 / 1440, (start + 60) % 1440) };
                        json!({"summary": title, "start": {"dateTime": rfc3339(day, start), "timeZone": "Asia/Seoul"}, "end": {"dateTime": rfc3339(ed, em), "timeZone": "Asia/Seoul"}})
                    };
                    let naver = info.as_ref().and_then(|i| accts.get(i.account)).and_then(|a| a.0.strip_prefix(crate::naver::TAG).map(|n| (n.to_string(), a.1.clone())));
                    if let (Some(i), Some((nid, pw))) = (info.as_ref(), naver) {
                        match crate::naver::add(&c, &nid, &pw, &i.id, &title, day, start, end) {
                            Ok(_) => crate::log("gcal: 네이버에 일정 넣음"),
                            Err(e) => {
                                crate::log(&format!("gcal: 네이버에 못 넣음 {e}"));
                                note = format!("일정을 못 넣었습니다{e}");
                            }
                        }
                    } else { match (info, tok) {
                        (Some(i), Some(t)) => {
                            let u = format!("https://www.googleapis.com/calendar/v3/calendars/{}/events", enc(&i.id));
                            match send(&c, "POST", &u, Some(&t), Some((&body.to_string(), "application/json"))) {
                                Ok((200, _)) => crate::log("gcal: 일정 넣음"),
                                Ok((st, b)) => note = format!("일정을 못 넣었습니다({st}) {}", b.chars().take(80).collect::<String>()),
                                Err(e) => note = format!("일정을 못 넣었습니다: {e}"),
                            }
                        }
                        _ => note = "일정을 넣을 캘린더에 연결되어 있지 않습니다".into(),
                    } }
                }
                _ => {}
            }
            // 받기: 지난달 1일부터 6주 뒤까지
            tokens.resize(accts.len(), None);
            let (now, _) = today();
            let (y, m, _) = ymd(now);
            let from = days(y, m, 1) - 35;
            let to = now + 70;
            let (mut cals, mut events) = (Vec::new(), Vec::new());
            let mut failed = false; // 하나라도 못 받으면 지난번 것을 그대로 두고 1분 뒤 다시
            for (ai, (email, refresh)) in accts.iter().enumerate() {
                if let Some(nid) = email.strip_prefix(crate::naver::TAG) {
                    match crate::naver::calendars(&c, nid, refresh) {
                        Ok(list) => {
                            for k in list {
                                let info = CalInfo { account: ai, id: k.url, name: k.name, color: hex_color(&k.color), writable: k.writable };
                                match crate::naver::events(&c, nid, refresh, &info.id, from, to, info.color) {
                                    Ok(v) => events.extend(v),
                                    Err(e) => {
                                        crate::log(&format!("gcal: 네이버 일정 받기 실패 {e}"));
                                        failed = true;
                                        note = "네이버 일정을 못 받았습니다. 곧 다시 받습니다".into();
                                    }
                                }
                                cals.push(info);
                            }
                        }
                        Err(e) => {
                            crate::log(&format!("gcal: 네이버 캘린더 목록 실패 {e}"));
                            failed = true;
                            note = e;
                        }
                    }
                    continue;
                }
                // 열쇠(액세스 토큰)는 1시간이면 끝난다. 받은 시각은 새로 받을 때만 적는다
                // (전엔 쓸 때마다 적어서 끝난 열쇠를 계속 썼다 → 1시간 뒤 일정 0개, 2026-11-13).
                let mut got: Option<(String, Value)> = None;
                let mut gone = false;
                for attempt in 0..2 {
                    let fresh = if attempt == 0 { tokens[ai].as_ref().filter(|t| t.1.elapsed() < Duration::from_secs(3000)).map(|t| t.0.clone()) } else { None };
                    let tok = match fresh.or_else(|| access(&c, &id, &secret, refresh).inspect(|t| tokens[ai] = Some((t.clone(), Instant::now())))) {
                        Some(t) => t,
                        None => {
                            gone = true;
                            break;
                        }
                    };
                    match send(&c, "GET", "https://www.googleapis.com/calendar/v3/users/me/calendarList?minAccessRole=reader", Some(&tok), None) {
                        Ok((200, b)) => {
                            got = serde_json::from_str(&b).ok().map(|v| (tok, v));
                            break;
                        }
                        r => {
                            crate::log(&format!("gcal: 캘린더 목록 실패 {}", r.map(|x| x.0.to_string()).unwrap_or_else(|e| e.to_string())));
                            tokens[ai] = None; // 열쇠를 버리고 새로 받아 한 번 더
                        }
                    }
                }
                let Some((tok, list)) = got else {
                    failed = true;
                    note = if gone { format!("{email} 연결이 풀렸습니다. 다시 연결해 주세요") } else { "일정을 못 받았습니다. 곧 다시 받습니다".into() };
                    continue;
                };
                for k in list["items"].as_array().into_iter().flatten() {
                    if k["selected"].as_bool() != Some(true) && k["primary"].as_bool() != Some(true) {
                        continue; // 구글 캘린더에서 꺼 둔 캘린더는 안 본다
                    }
                    let info = CalInfo {
                        account: ai,
                        id: k["id"].as_str().unwrap_or("").to_string(),
                        name: k["summaryOverride"].as_str().or(k["summary"].as_str()).unwrap_or("").to_string(),
                        color: hex_color(k["backgroundColor"].as_str().unwrap_or("")),
                        writable: matches!(k["accessRole"].as_str(), Some("owner" | "writer")),
                    };
                    let u = format!(
                        "https://www.googleapis.com/calendar/v3/calendars/{}/events?singleEvents=true&orderBy=startTime&maxResults=250&timeZone=Asia%2FSeoul&timeMin={}&timeMax={}",
                        enc(&info.id), enc(&rfc3339(from, 0)), enc(&rfc3339(to, 0))
                    );
                    match send(&c, "GET", &u, Some(&tok), None) {
                        Ok((200, b)) => events.extend(parse_events(&b, info.color)),
                        r => {
                            crate::log(&format!("gcal: 일정 받기 실패 {}", r.map(|x| x.0.to_string()).unwrap_or_else(|e| e.to_string())));
                            failed = true;
                            note = "일정을 못 받았습니다. 곧 다시 받습니다".into();
                        }
                    }
                    cals.push(info);
                }
            }
            if !failed && (cals.len(), events.len()) != seen_counts {
                seen_counts = (cals.len(), events.len());
                // 날짜만 남긴다(제목은 안 남김): 사용자가 본 달력과 노치가 받은 것을 맞춰 볼 수 있게
                let mut ds: Vec<String> = events.iter().map(|e| { let (_, m, d) = ymd(e.day); format!("{m}/{d}") }).collect();
                ds.dedup();
                crate::log(&format!("gcal: 계정 {} 캘린더 {} 일정 {} ({})", accts.len(), cals.len(), events.len(), ds.join(" ")));
            }
            {
                let mut o = out.lock().unwrap();
                o.ready = true;
                o.accounts = accts.iter().map(|a| a.0.strip_prefix(crate::naver::TAG).map(|n| format!("네이버 {n}")).unwrap_or_else(|| a.0.clone())).collect();
                if !accts.is_empty() && !failed {
                    o.cals = cals;
                    o.events = events;
                    o.stamp = crate::hhmm();
                    o.seen = Some(Instant::now());
                }
                o.note = note;
                o.ver += 1;
            }
            cmd = if !later.is_empty() { Some(later.remove(0)) } else { match rx.recv_timeout(Duration::from_secs(if failed { 60 } else { 300 })) {
                Ok(c) => Some(c),
                Err(mpsc::RecvTimeoutError::Timeout) => Some(Cmd::Refresh),
                Err(_) => return,
            } };
        }
    });
    tx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memo_plain() {
        assert_eq!(plain("준비물:<br>노트북 &amp; 충전기<br><b>꼭</b>"), "준비물:
노트북 & 충전기
꼭");
        assert_eq!(plain(""), "");
    }

    #[test]
    fn dates() {
        assert_eq!(days(1970, 1, 1), 0);
        let d = days(2026, 11, 12);
        assert_eq!(ymd(d), (2026, 11, 12));
        assert_eq!(WEEK[weekday(d)], "목");
        assert_eq!(ymd(days(2024, 2, 28) + 1), (2024, 2, 29));
        assert_eq!(parse_when("2026-11-12T10:00:00+09:00"), Some((d, 600)));
        assert_eq!(parse_when("2026-11-12T16:30:00Z"), Some((d + 1, 90))); // 한국은 다음 날 01:30
        assert_eq!(parse_when("2026-11-12T10:00:00.000-05:00"), Some((d + 1, 0)));
        assert_eq!(parse_when("2026-11-12"), Some((d, -1)));
        assert_eq!(rfc3339(d, 545), "2026-11-12T09:05:00+09:00");
    }

    #[test]
    fn events_and_codes() {
        let b = r#"{"items":[
            {"summary":"회의","location":"3층","start":{"dateTime":"2026-11-12T10:00:00+09:00"},"end":{"dateTime":"2026-11-12T10:30:00+09:00"}},
            {"summary":"여행","start":{"date":"2026-11-13"},"end":{"date":"2026-11-15"}},
            {"status":"cancelled","start":{"date":"2026-11-13"},"end":{"date":"2026-11-14"}}]}"#;
        let e = parse_events(b, (1.0, 0.0, 0.0));
        let d = days(2026, 11, 12);
        assert_eq!(e.len(), 2);
        assert_eq!((e[0].day, e[0].start, e[0].end, e[0].place.as_str()), (d, 600, 630, "3층"));
        assert_eq!((e[1].day, e[1].start, e[1].end_day), (d + 1, -1, d + 2)); // 13~14일(15일은 빠짐)
        let cal = Cal { events: e, ..Default::default() };
        assert_eq!(cal.on(d + 2).len(), 1);
        assert_eq!(enc("a b/가"), "a%20b%2F%EA%B0%80");
        assert_eq!(dec("a%20b%2F%EA%B0%80+c"), "a b/가 c");
        assert_eq!(b64url("eyJlbWFpbCI6ImFAYi5jb20ifQ"), br#"{"email":"a@b.com"}"#.to_vec());
        assert_eq!(hex_color("#ff0000"), (1.0, 0.0, 0.0));
    }
}
