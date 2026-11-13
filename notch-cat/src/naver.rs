// 네이버 캘린더: 동기화 통로(CalDAV, caldav.calendar.naver.com)로 일정을 받고 넣는다.
// 로그인은 네이버 아이디 + 애플리케이션 비밀번호(기본 인증). 아이폰 기본 캘린더가 네이버를 보는 것과 같은 길.
// 아이디·비밀번호는 구글 계정과 같은 google.dat 에 윈도우 암호화(DPAPI)로 "naver:아이디" 줄로 둔다.
use windows::core::{Result, HSTRING, PCWSTR};
use windows::Foundation::Uri;
use windows::Storage::Streams::UnicodeEncoding;
use windows::Web::Http::{HttpClient, HttpMethod, HttpRequestMessage, HttpStringContent};

use crate::gcal::{self, Event};

const ROOT: &str = "https://caldav.calendar.naver.com";
const XML: &str = "application/xml";
pub const TAG: &str = "naver:"; // 계정 목록에서 네이버 줄의 앞머리

fn b64(s: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for ch in s.chunks(3) {
        let n = (ch[0] as u32) << 16 | (*ch.get(1).unwrap_or(&0) as u32) << 8 | *ch.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            out.push(if i <= ch.len() { A[(n >> (18 - 6 * i) & 63) as usize] as char } else { '=' });
        }
    }
    out
}

fn dav(c: &HttpClient, method: &str, url: &str, id: &str, pw: &str, depth: Option<&str>, body: Option<(&str, &str)>, extra: Option<(&str, &str)>) -> Result<(u16, String)> {
    let msg = HttpRequestMessage::Create(&HttpMethod::Create(&HSTRING::from(method))?, &Uri::CreateUri(&HSTRING::from(url))?)?;
    let h = msg.Headers()?;
    h.TryAppendWithoutValidation(&HSTRING::from("Authorization"), &HSTRING::from(format!("Basic {}", b64(format!("{id}:{pw}").as_bytes()))))?;
    if let Some(d) = depth {
        h.TryAppendWithoutValidation(&HSTRING::from("Depth"), &HSTRING::from(d))?;
    }
    if let Some((k, v)) = extra {
        h.TryAppendWithoutValidation(&HSTRING::from(k), &HSTRING::from(v))?;
    }
    if let Some((text, kind)) = body {
        msg.SetContent(&HttpStringContent::CreateFromStringWithEncodingAndMediaType(&HSTRING::from(text), UnicodeEncoding::Utf8, &HSTRING::from(kind))?)?;
    }
    let r = c.SendRequestAsync(&msg)?.get()?;
    Ok((r.StatusCode()?.0 as u16, r.Content()?.ReadAsStringAsync()?.get()?.to_string()))
}

// ── 아주 작은 XML 읽기: 접두어(d:, D:, cal:)는 무시하고 이름으로 요소 속을 꺼낸다 ──

fn elems<'a>(x: &'a str, local: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(p) = x[i..].find('<') {
        let s = i + p;
        i = s + 1;
        let rest = &x[s + 1..];
        let end = rest.find(|c: char| c == '>' || c == '/' || c.is_whitespace()).unwrap_or(rest.len());
        let name = &rest[..end];
        if name.is_empty() || name.starts_with(['?', '!']) || name.rsplit(':').next() != Some(local) {
            continue;
        }
        let Some(gt) = rest.find('>') else { break };
        if rest[..gt].ends_with('/') {
            out.push("");
            continue;
        }
        let from = s + 1 + gt + 1;
        let close = format!("</{name}>");
        if let Some(q) = x[from..].find(&close) {
            out.push(&x[from..from + q]);
            i = from + q + close.len();
        }
    }
    out
}

fn unxml(s: &str) -> String {
    let s = s.trim();
    let s = match (s.find("<![CDATA["), s.rfind("]]>")) {
        (Some(a), Some(b)) if b > a => return s[a + 9..b].to_string(),
        _ => s,
    };
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&#13;", "\r").replace("&#xD;", "\r").replace("&#10;", "\n").replace("&amp;", "&")
}

fn href(x: &str, prop: &str) -> Option<String> {
    let p = elems(x, prop).into_iter().next()?;
    let h = unxml(elems(p, "href").into_iter().next()?);
    Some(if h.starts_with("http") { h } else { format!("{ROOT}{h}") })
}

pub struct NCal {
    pub url: String,
    pub name: String,
    pub color: String,
    pub writable: bool,
}

/// 캘린더 찾기: 나 → 캘린더 집 → 그 안의 캘린더들. 틀린 비밀번호면 사람이 읽을 이유를 돌려준다.
pub fn calendars(c: &HttpClient, id: &str, pw: &str) -> std::result::Result<Vec<NCal>, String> {
    let ask = |url: &str, depth: &str, q: &str| -> std::result::Result<String, String> {
        match dav(c, "PROPFIND", url, id, pw, Some(depth), Some((q, XML)), None) {
            Ok((207, b)) => Ok(b),
            Ok((401 | 403, _)) => Err("네이버 아이디나 비밀번호가 맞지 않습니다".into()),
            Ok((st, _)) => Err(format!("네이버가 응답하지 않습니다({st})")),
            Err(e) => Err(format!("네이버에 닿지 않습니다: {e}")),
        }
    };
    let b = ask(&format!("{ROOT}/"), "0", r#"<?xml version="1.0" encoding="utf-8"?><d:propfind xmlns:d="DAV:"><d:prop><d:current-user-principal/></d:prop></d:propfind>"#)?;
    let me = href(&b, "current-user-principal").ok_or("네이버 캘린더 주소를 못 찾았습니다")?;
    let b = ask(&me, "0", r#"<?xml version="1.0" encoding="utf-8"?><d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav"><d:prop><c:calendar-home-set/></d:prop></d:propfind>"#)?;
    let home = href(&b, "calendar-home-set").ok_or("네이버 캘린더 목록 주소를 못 찾았습니다")?;
    let b = ask(&home, "1", r#"<?xml version="1.0" encoding="utf-8"?><d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" xmlns:x="http://apple.com/ns/ical/"><d:prop><d:resourcetype/><d:displayname/><x:calendar-color/><c:supported-calendar-component-set/><d:current-user-privilege-set/></d:prop></d:propfind>"#)?;
    Ok(parse_cals(&b))
}

fn parse_cals(b: &str) -> Vec<NCal> {
    let mut out = Vec::new();
    for r in elems(b, "response") {
        let is_cal = elems(r, "resourcetype").iter().any(|t| !elems(t, "calendar").is_empty());
        let comps = elems(r, "supported-calendar-component-set").concat();
        if !is_cal || (!comps.trim().is_empty() && !comps.contains("VEVENT")) {
            continue; // 할 일 목록 등은 뺀다
        }
        let Some(h) = elems(r, "href").into_iter().next() else { continue };
        let h = unxml(h);
        let privs = elems(r, "current-user-privilege-set").concat();
        out.push(NCal {
            url: if h.starts_with("http") { h } else { format!("{ROOT}{h}") },
            name: elems(r, "displayname").iter().map(|s| unxml(s)).find(|s| !s.is_empty()).unwrap_or_else(|| "네이버".into()),
            color: elems(r, "calendar-color").iter().map(|s| unxml(s)).find(|s| s.starts_with('#')).map(|s| s.chars().take(7).collect()).unwrap_or_default(),
            writable: privs.trim().is_empty() || privs.contains("write") || privs.contains(":all") || privs.contains("<all"),
        });
    }
    out
}

fn zstamp(day: i64, min: i32) -> String {
    let t = min - 540; // 한국 → 세계 표준시
    let (y, m, d) = gcal::ymd(day + t.div_euclid(1440) as i64);
    let t = t.rem_euclid(1440);
    format!("{y:04}{m:02}{d:02}T{:02}{:02}00Z", t / 60, t % 60)
}

/// 한 캘린더의 from~to 일정
pub fn events(c: &HttpClient, id: &str, pw: &str, url: &str, from: i64, to: i64, color: (f32, f32, f32)) -> std::result::Result<Vec<Event>, String> {
    let (a, b) = (zstamp(from, 0), zstamp(to + 1, 0));
    let q = format!(
        r#"<?xml version="1.0" encoding="utf-8"?><c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav"><d:prop><c:calendar-data/></d:prop><c:filter><c:comp-filter name="VCALENDAR"><c:comp-filter name="VEVENT"><c:time-range start="{a}" end="{b}"/></c:comp-filter></c:comp-filter></c:filter></c:calendar-query>"#
    );
    match dav(c, "REPORT", url, id, pw, Some("1"), Some((&q, XML)), None) {
        Ok((207, body)) => Ok(elems(&body, "calendar-data").iter().flat_map(|d| parse_ics(&unxml(d), color, from, to)).collect()),
        Ok((st, _)) => Err(format!("{st}")),
        Err(e) => Err(e.to_string()),
    }
}

/// 일정 하나 넣기
pub fn add(c: &HttpClient, id: &str, pw: &str, url: &str, title: &str, day: i64, start: i32, end: i32) -> std::result::Result<(), String> {
    let uid = format!("{}@notch-cat", gcal::random(24).replace(['.', '~'], "x"));
    let esc = title.replace('\\', "\\\\").replace(';', "\\;").replace(',', "\\,").replace('\n', "\\n");
    let (now_d, now_m) = gcal::today();
    let when = if start < 0 {
        let ds = |z: i64| { let (y, m, d) = gcal::ymd(z); format!("{y:04}{m:02}{d:02}") };
        format!("DTSTART;VALUE=DATE:{}\r\nDTEND;VALUE=DATE:{}", ds(day), ds(day + 1))
    } else {
        let (ed, em) = if end > start { (day, end) } else { (day + (start + 60) as i64 / 1440, (start + 60) % 1440) };
        format!("DTSTART:{}\r\nDTEND:{}", zstamp(day, start), zstamp(ed, em))
    };
    let ics = format!("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//notch-cat//KO\r\nBEGIN:VEVENT\r\nUID:{uid}\r\nDTSTAMP:{}\r\n{when}\r\nSUMMARY:{esc}\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n", zstamp(now_d, now_m));
    let u = format!("{}{}.ics", if url.ends_with('/') { url.to_string() } else { format!("{url}/") }, gcal::enc(&uid));
    match dav(c, "PUT", &u, id, pw, None, Some((&ics, "text/calendar")), Some(("If-None-Match", "*"))) {
        Ok((200 | 201 | 204, _)) => Ok(()),
        Ok((st, b)) => Err(format!("({st}) {}", b.chars().take(80).collect::<String>())),
        Err(e) => Err(e.to_string()),
    }
}

// ── 일정 파일(iCalendar) 읽기 ──

/// "20261115" / "20261115T140000" / "...Z"(세계 표준시) → (날, 분). 날짜만이면 분 -1. TZID 는 한국으로 본다.
fn dt(v: &str) -> Option<(i64, i32)> {
    let v = v.trim();
    let n = |a: usize, b: usize| v.get(a..b).and_then(|s| s.parse::<u32>().ok());
    let day = gcal::days(n(0, 4)? as i32, n(4, 6)?, n(6, 8)?);
    if v.len() < 13 {
        return Some((day, -1));
    }
    let mut t = (n(9, 11)? * 60 + n(11, 13)?) as i32;
    if v.ends_with('Z') {
        t += 540;
    }
    Some((day + t.div_euclid(1440) as i64, t.rem_euclid(1440)))
}

fn dur(v: &str) -> i32 {
    let (mut total, mut num, mut neg) = (0i32, 0i32, false);
    let mut in_time = false;
    for ch in v.chars() {
        match ch {
            '-' => neg = true,
            'T' => in_time = true,
            '0'..='9' => num = num * 10 + ch as i32 - '0' as i32,
            'W' => (total, num) = (total + num * 7 * 1440, 0),
            'D' => (total, num) = (total + num * 1440, 0),
            'H' => (total, num) = (total + num * 60, 0),
            'M' if in_time => (total, num) = (total + num, 0),
            'S' => num = 0,
            _ => {}
        }
    }
    if neg { -total } else { total }
}

fn untext(s: &str) -> String {
    s.replace("\\n", "\n").replace("\\N", "\n").replace("\\,", ",").replace("\\;", ";").replace("\\\\", "\\")
}

const WD: [&str; 7] = ["SU", "MO", "TU", "WE", "TH", "FR", "SA"];

/// 되풀이(RRULE)를 from~to 사이 시작 날들로 편다. 처음 날 d0 도 셈에 든다.
fn expand(rule: &str, d0: i64, from: i64, to: i64, ex: &[i64]) -> Vec<i64> {
    let get = |k: &str| rule.split(';').find_map(|p| p.strip_prefix(k).and_then(|r| r.strip_prefix('=')));
    let freq = get("FREQ").unwrap_or("");
    let iv = get("INTERVAL").and_then(|s| s.parse::<i64>().ok()).unwrap_or(1).max(1);
    let count = get("COUNT").and_then(|s| s.parse::<usize>().ok());
    let until = get("UNTIL").and_then(dt).map(|d| d.0);
    let byday: Vec<(i32, usize)> = get("BYDAY").unwrap_or("").split(',').filter_map(|s| {
        let s = s.trim();
        let w = WD.iter().position(|d| s.ends_with(d))?;
        Some((s[..s.len() - 2].parse::<i32>().unwrap_or(0), w))
    }).collect();
    let bymd: Vec<i64> = get("BYMONTHDAY").unwrap_or("").split(',').filter_map(|s| s.trim().parse().ok()).collect();
    let (y0, m0, dd0) = gcal::ymd(d0);
    let mut out = Vec::new();
    let mut n = 0usize;
    let mut take = |c: i64, out: &mut Vec<i64>| -> bool {
        // 거짓이면 그만
        if c < d0 {
            return true;
        }
        if c > to || until.is_some_and(|u| c > u) {
            return false;
        }
        n += 1;
        if count.is_some_and(|k| n > k) {
            return false;
        }
        if c >= from && !ex.contains(&c) {
            out.push(c);
        }
        true
    };
    'outer: for k in 0..2000i64 {
        let mut days: Vec<i64> = Vec::new();
        match freq {
            "DAILY" => days.push(d0 + k * iv),
            "WEEKLY" => {
                let ws = d0 - gcal::weekday(d0) as i64 + 7 * iv * k;
                if byday.is_empty() {
                    days.push(d0 + 7 * iv * k);
                } else {
                    let mut w: Vec<usize> = byday.iter().map(|b| b.1).collect();
                    w.sort();
                    days.extend(w.iter().map(|&x| ws + x as i64));
                }
            }
            "MONTHLY" | "YEARLY" => {
                let step = if freq == "MONTHLY" { iv } else { 12 * iv };
                let mi = (y0 * 12 + m0 as i32 - 1) as i64 + step * k;
                let (y, m) = (mi.div_euclid(12) as i32, (mi.rem_euclid(12) + 1) as u32);
                let len = crate::entry::month_len(y, m);
                let first = gcal::days(y, m, 1);
                if !byday.is_empty() && freq == "MONTHLY" {
                    for &(nth, w) in &byday {
                        let all: Vec<i64> = (0..len).map(|i| first + i).filter(|&z| gcal::weekday(z) == w).collect();
                        let pick = if nth > 0 { all.get(nth as usize - 1) } else if nth < 0 { all.len().checked_sub((-nth) as usize).and_then(|i| all.get(i)) } else { None };
                        if let Some(&z) = pick {
                            days.push(z);
                        } else if nth == 0 {
                            days.extend(all);
                        }
                    }
                } else {
                    let mds = if bymd.is_empty() || freq == "YEARLY" { vec![dd0 as i64] } else { bymd.clone() };
                    for md in mds {
                        let md = if md < 0 { len + 1 + md } else { md };
                        if (1..=len).contains(&md) {
                            days.push(first + md - 1);
                        }
                    }
                }
                days.sort();
            }
            _ => break,
        }
        for c in days {
            if !take(c, &mut out) {
                break 'outer;
            }
        }
    }
    out
}

/// iCalendar 글 → from~to 에 걸친 일정들(되풀이는 펴고, 따로 고친 회차는 그것으로 바꾼다)
pub fn parse_ics(ics: &str, color: (f32, f32, f32), from: i64, to: i64) -> Vec<Event> {
    let mut lines: Vec<String> = Vec::new();
    for raw in ics.split('\n') {
        let l = raw.trim_end_matches('\r');
        match lines.last_mut() {
            Some(last) if l.starts_with([' ', '\t']) => last.push_str(&l[1..]),
            _ => lines.push(l.to_string()),
        }
    }
    let mut blocks: Vec<Vec<(String, String, String)>> = Vec::new();
    let (mut cur, mut inner): (Option<Vec<(String, String, String)>>, i32) = (None, 0);
    for l in &lines {
        if l == "BEGIN:VEVENT" {
            cur = Some(Vec::new());
        } else if l == "END:VEVENT" {
            blocks.extend(cur.take());
        } else if l.starts_with("BEGIN:") && cur.is_some() {
            inner += 1;
        } else if l.starts_with("END:") && cur.is_some() {
            inner -= 1;
        } else if let (Some(p), 0) = (cur.as_mut(), inner) {
            // 이름;매개변수:값 (따옴표 안의 ':' 는 건너뛴다)
            let mut q = false;
            let Some(ci) = l.char_indices().find(|&(_, ch)| { if ch == '"' { q = !q; } ch == ':' && !q }).map(|(i, _)| i) else { continue };
            let (head, val) = (&l[..ci], &l[ci + 1..]);
            let (name, params) = head.split_once(';').unwrap_or((head, ""));
            p.push((name.to_ascii_uppercase(), params.to_string(), val.to_string()));
        }
    }
    let mut out: Vec<(Event, String, bool)> = Vec::new(); // (일정, uid, 되풀이로 만든 것)
    let mut moved: Vec<(String, i64)> = Vec::new();
    for b in &blocks {
        let get = |k: &str| b.iter().find(|p| p.0 == k).map(|p| p.2.as_str());
        if get("STATUS") == Some("CANCELLED") {
            continue;
        }
        let Some((d0, s0)) = get("DTSTART").and_then(dt) else { continue };
        let (mut d1, e1) = match get("DTEND").and_then(dt) {
            Some(e) => e,
            None => {
                let m = get("DURATION").map(dur).unwrap_or(if s0 < 0 { 1440 } else { 0 });
                let t = if s0 < 0 { m } else { s0 + m };
                (d0 + t.div_euclid(1440) as i64, if s0 < 0 { -1 } else { t.rem_euclid(1440) })
            }
        };
        if s0 < 0 {
            d1 -= 1; // 종일 일정의 끝 날짜는 '다음 날'로 온다
        }
        let uid = get("UID").unwrap_or("").to_string();
        let ev = Event {
            title: get("SUMMARY").map(untext).filter(|s| !s.is_empty()).unwrap_or_else(|| "(제목 없음)".into()),
            place: get("LOCATION").map(untext).unwrap_or_default(),
            day: d0,
            start: s0,
            end_day: d1.max(d0),
            end: e1,
            color,
            memo: get("DESCRIPTION").map(untext).unwrap_or_default(),
        };
        if let Some(r) = get("RECURRENCE-ID").and_then(dt) {
            moved.push((uid.clone(), r.0));
        }
        match get("RRULE").filter(|_| get("RECURRENCE-ID").is_none()) {
            Some(rule) => {
                let ex: Vec<i64> = b.iter().filter(|p| p.0 == "EXDATE").flat_map(|p| p.2.split(',').filter_map(dt).map(|d| d.0).collect::<Vec<_>>()).collect();
                let span = ev.end_day - ev.day;
                for z in expand(rule, d0, from - span, to, &ex) {
                    out.push((Event { day: z, end_day: z + span, ..ev.clone() }, uid.clone(), true));
                }
            }
            None => {
                if ev.end_day >= from && ev.day <= to {
                    out.push((ev, uid, false));
                }
            }
        }
    }
    out.into_iter().filter(|(e, u, gen)| !*gen || !moved.contains(&(u.clone(), e.day))).map(|x| x.0).collect()
}

/// 아이디·비밀번호 묻는 창(윈도우 기본 창). 취소하면 None.
pub fn ask(owner: isize) -> Option<(String, String)> {
    use windows::Win32::Security::Credentials::*;
    let msg: Vec<u16> = "네이버 아이디와 애플리케이션 비밀번호를 넣어 주세요.\n(네이버 → 내정보 → 보안설정 → 2단계 인증 → 애플리케이션 비밀번호. 2단계 인증을 안 쓰면 로그인 비밀번호)\0".encode_utf16().collect();
    let cap: Vec<u16> = "노치 고양이 - 네이버 캘린더 연결\0".encode_utf16().collect();
    let info = CREDUI_INFOW { cbSize: std::mem::size_of::<CREDUI_INFOW>() as u32, hwndParent: windows::Win32::Foundation::HWND(owner as _), pszMessageText: PCWSTR(msg.as_ptr()), pszCaptionText: PCWSTR(cap.as_ptr()), ..Default::default() };
    let (mut user, mut pass) = ([0u16; 514], [0u16; 257]);
    let target: Vec<u16> = "caldav.calendar.naver.com\0".encode_utf16().collect();
    let r = unsafe {
        CredUIPromptForCredentialsW(Some(&info), PCWSTR(target.as_ptr()), None, 0, &mut user, &mut pass, None, CREDUI_FLAGS_GENERIC_CREDENTIALS | CREDUI_FLAGS_ALWAYS_SHOW_UI | CREDUI_FLAGS_DO_NOT_PERSIST | CREDUI_FLAGS_EXCLUDE_CERTIFICATES)
    };
    let s = |b: &[u16]| String::from_utf16_lossy(&b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())]).trim().to_string();
    let (u, p) = (s(&user), s(&pass));
    pass.fill(0);
    crate::log(&format!("naver: 입력 창 결과 {} (아이디 {}자, 비밀번호 {}자)", r.0, u.chars().count(), p.chars().count()));
    (r.0 == 0 && !u.is_empty() && !p.is_empty()).then(|| (u.trim_end_matches("@naver.com").to_string(), p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_dav_and_ics() {
        assert_eq!(b64(b"ab:c"), "YWI6Yw==");
        let x = r#"<d:multistatus xmlns:d="DAV:"><d:response><d:href>/calendar/u/</d:href><d:propstat><d:prop><d:resourcetype><d:collection/></d:resourcetype></d:prop></d:propstat></d:response><D:response><D:href>/calendar/u/1/</D:href><D:propstat><D:prop><D:resourcetype><D:collection/><C:calendar/></D:resourcetype><D:displayname>내 캘린더</D:displayname><x:calendar-color>#FF5500FF</x:calendar-color></D:prop></D:propstat></D:response></d:multistatus>"#;
        let c = parse_cals(x);
        assert_eq!(c.len(), 1);
        assert_eq!((c[0].url.as_str(), c[0].name.as_str(), c[0].color.as_str()), ("https://caldav.calendar.naver.com/calendar/u/1/", "내 캘린더", "#FF5500"));
        let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:a\r\nDTSTART:20261115T050000Z\r\nDTEND:20261115T060000Z\r\nSUMMARY:치과\\, 예약\r\nDESCRIPTION:줄1\\n줄\r\n 2\r\nBEGIN:VALARM\r\nDESCRIPTION:알림\r\nEND:VALARM\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:b\r\nDTSTART;VALUE=DATE:20261101\r\nDTEND;VALUE=DATE:20261102\r\nRRULE:FREQ=WEEKLY;BYDAY=SU,WE;COUNT=4\r\nEXDATE;VALUE=DATE:20261104\r\nSUMMARY:운동\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:b\r\nRECURRENCE-ID;VALUE=DATE:20261108\r\nDTSTART;VALUE=DATE:20261109\r\nDTEND;VALUE=DATE:20261110\r\nSUMMARY:운동(옮김)\r\nEND:VEVENT\r\nEND:VCALENDAR";
        let (from, to) = (gcal::days(2026, 10, 1), gcal::days(2026, 12, 31));
        let ev = parse_ics(ics, (0.0, 0.0, 0.0), from, to);
        let d = |m, d| gcal::days(2026, m, d);
        assert_eq!((ev[0].title.as_str(), ev[0].day, ev[0].start, ev[0].end, ev[0].memo.as_str()), ("치과, 예약", d(11, 15), 840, 900, "줄1\n줄2"));
        let days: Vec<(i64, &str)> = ev[1..].iter().map(|e| (e.day, e.title.as_str())).collect();
        // 11/1(일) 11/4(수, 뺌) 11/8(일, 11/9로 옮김) 11/11(수) → 4회째에서 끝
        assert_eq!(days, vec![(d(11, 1), "운동"), (d(11, 11), "운동"), (d(11, 9), "운동(옮김)")]);
        // 매달 둘째 화요일, 매년
        assert_eq!(expand("FREQ=MONTHLY;BYDAY=2TU;COUNT=3", d(11, 10), from, to, &[]), vec![d(11, 10), d(12, 8)]);
        assert_eq!(expand("FREQ=YEARLY", gcal::days(2020, 11, 20), from, to, &[]), vec![d(11, 20)]);
        assert_eq!(dur("PT1H30M"), 90);
    }

    #[test]
    #[ignore] // 화면: 입력 창이 뜨는지(밖에서 창을 닫으면 None)
    fn dialog_opens() {
        assert_eq!(ask(0), None);
    }

    #[test]
    #[ignore] // 네트워크: 틀린 비밀번호면 '맞지 않습니다'가 나와야 한다(통로 자체는 열림)
    fn live_wrong_password() {
        let c = windows::Web::Http::Filters::HttpBaseProtocolFilter::new().and_then(|f| { f.SetAllowUI(false)?; HttpClient::Create(&f) }).unwrap();
        let r = calendars(&c, "nobody_test_x", "wrong");
        println!("{:?}", r.as_ref().err());
        assert_eq!(r.err().as_deref(), Some("네이버 아이디나 비밀번호가 맞지 않습니다"));
    }
}
