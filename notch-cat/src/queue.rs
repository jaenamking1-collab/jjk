// 다음 곡 목록. 윈도우는 재생기의 대기열을 다른 프로그램에 알려 주지 않으므로 두 길로 모은다.
//  ① 내 음악: 지금 곡 파일이 있는 폴더의 다음 파일들. 재생기 명령줄에서 파일을 찾고,
//     없으면 음악·다운로드·바탕화면에서 곡 이름으로 찾는다. 누르면 같은 재생기로 그 파일을 연다.
//  ② 유튜브: 크롬 확장(youtube-ext)이 127.0.0.1:PORT 로 재생목록·다음 동영상을 보낸다.
//     누르면 다음 보고 때 확장에 "이 곡 틀어"를 돌려준다.
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;
use windows::core::HSTRING;
use windows::Foundation::Uri;
use windows::Storage::Streams::DataReader;
use windows::Web::Http::HttpClient;

use crate::media::Now;

pub const PORT: u16 = 47811;
const AUDIO: [&str; 9] = ["mp3", "flac", "m4a", "wav", "ogg", "opus", "aac", "wma", "ape"];
const BROWSERS: [&str; 7] = ["chrome", "msedge", "firefox", "whale", "brave", "opera", "vivaldi"];
const NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Default)]
pub struct Item {
    pub title: String,
    pub sub: String,
    pub key: String, // 내 음악: 파일 경로 · 유튜브: 확장이 찾을 열쇠(동영상 id 또는 제목)
    pub thumb: Option<Arc<Vec<u8>>>,
}

#[derive(Clone, Default)]
pub struct Queue {
    pub local: Vec<Item>,
    pub local_dir: String, // 폴더 이름(머리에 보인다)
    pub local_stamp: String,
    pub exe: String, // 내 음악을 트는 재생기(누르면 이걸로 연다)
    pub yt: Vec<Item>,
    pub yt_kind: String, // "재생목록" · "다음 동영상" · "유튜브 뮤직"
    pub yt_tab: i64,
    pub yt_seen: Option<Instant>,
    pub yt_stamp: String,
    pub yt_cmd: Option<(i64, String)>, // (탭, 열쇠) 확장에 돌려줄 "이 곡 틀어"
    pub ver: u64,
}

impl Queue {
    /// 확장이 최근 8초 안에 보고했나(탭을 닫으면 사라지게)
    pub fn yt_live(&self) -> bool {
        self.yt_seen.map_or(false, |t| t.elapsed() < Duration::from_secs(8))
    }
}

pub fn is_browser(app: &str) -> bool {
    let a = app.to_lowercase();
    BROWSERS.iter().any(|b| a.contains(b))
}

fn is_audio(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()).map_or(false, |e| AUDIO.contains(&e.to_lowercase().as_str()))
}

/// 비교용: 글자·숫자(한글 포함)만 남기고 소문자로
fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

fn same_song(stem: &str, title: &str) -> bool {
    let (s, t) = (norm(stem), norm(title));
    t.chars().count() >= 2 && (s.contains(&t) || (s.chars().count() >= 3 && t.contains(&s)))
}

/// 앱 이름(AUMID)에서 실행 파일 이름 조각: "C:\..\PotPlayerMini64.exe" → "potplayermini64"
fn app_token(app: &str) -> String {
    let last = app.rsplit(['\\', '/', '!']).next().unwrap_or(app);
    last.to_lowercase().trim_end_matches(".exe").to_string()
}

struct Proc {
    name: String,
    exe: String,
    paths: Vec<PathBuf>,
}

/// 켜져 있는 프로그램들의 명령줄에서 소리 파일 경로를 모은다(파워셸 한 번, 곡이 바뀔 때만).
fn procs() -> Vec<Proc> {
    let ps = "[Console]::OutputEncoding=[Text.Encoding]::UTF8; Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -match '\\.(mp3|flac|m4a|wav|ogg|opus|aac|wma|ape)' -or $_.Name -notmatch '^(svchost|conhost|RuntimeBroker)' } | Select-Object Name,ExecutablePath,CommandLine | ConvertTo-Json -Compress";
    let out = std::process::Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-Command", ps]).creation_flags(NO_WINDOW).output();
    let Ok(out) = out else { return vec![] };
    let v: Value = serde_json::from_slice(&out.stdout).unwrap_or_default();
    let arr = match v {
        Value::Array(a) => a,
        Value::Object(_) => vec![v],
        _ => vec![],
    };
    arr.iter()
        .map(|p| {
            let cl = p["CommandLine"].as_str().unwrap_or("");
            let mut paths = vec![];
            // 따옴표 안 조각과 빈칸으로 나눈 조각 모두 본다
            for s in cl.split('"').chain(cl.split_whitespace()) {
                let pb = PathBuf::from(s.trim());
                if is_audio(&pb) && pb.is_file() && !paths.contains(&pb) {
                    paths.push(pb);
                }
            }
            Proc { name: p["Name"].as_str().unwrap_or("").to_lowercase().trim_end_matches(".exe").to_string(), exe: p["ExecutablePath"].as_str().unwrap_or("").to_string(), paths }
        })
        .collect()
}

/// 곡 이름으로 파일 찾기: 전에 찾은 폴더 → 음악 → 다운로드 → 바탕화면 (깊이 4, 3만 개까지)
fn search(title: &str, known: &[PathBuf]) -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default());
    let mut roots: Vec<PathBuf> = known.to_vec();
    for r in ["Music", "Downloads", "Desktop", "OneDrive\\Music", "OneDrive\\음악", "OneDrive\\Desktop", "OneDrive\\바탕 화면", "OneDrive\\문서"] {
        roots.push(home.join(r));
    }
    let mut seen = 0usize;
    for root in roots {
        let mut stack = vec![(root, 0u8)];
        while let Some((d, depth)) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.flatten() {
                seen += 1;
                if seen > 30_000 {
                    return None;
                }
                let p = e.path();
                if p.is_dir() {
                    if depth < 4 && !e.file_name().to_string_lossy().starts_with('.') {
                        stack.push((p, depth + 1));
                    }
                } else if is_audio(&p) && p.file_stem().map_or(false, |s| same_song(&s.to_string_lossy(), title)) {
                    return Some(p);
                }
            }
        }
    }
    None
}

/// 사람 눈 순서(숫자는 숫자로): "2. a" < "10. b"
fn nat_key(s: &str) -> Vec<(u8, u64, String)> {
    let mut out = vec![];
    let mut num = String::new();
    let mut txt = String::new();
    for c in s.to_lowercase().chars() {
        if c.is_ascii_digit() {
            if !txt.is_empty() {
                out.push((1, 0, std::mem::take(&mut txt)));
            }
            num.push(c);
        } else {
            if !num.is_empty() {
                out.push((0, num.parse().unwrap_or(0), String::new()));
                num.clear();
            }
            txt.push(c);
        }
    }
    if !num.is_empty() {
        out.push((0, num.parse().unwrap_or(0), String::new()));
    }
    if !txt.is_empty() {
        out.push((1, 0, txt));
    }
    out
}

/// 폴더 안 곡들을 이름 순으로, 지금 곡 다음 것부터
fn folder_list(cur: &Path, title: &str) -> (Vec<Item>, String) {
    let Some(dir) = cur.parent() else { return (vec![], String::new()) };
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_file() && is_audio(p)).collect();
    files.sort_by_cached_key(|p| nat_key(&p.file_name().unwrap_or_default().to_string_lossy()));
    // 재생기가 자기 목록으로 넘어갔으면 명령줄 파일과 지금 곡이 다르다 → 폴더에서 지금 곡 이름을 다시 찾는다
    let i = files.iter().position(|p| p.file_stem().map_or(false, |s| same_song(&s.to_string_lossy(), title))).or_else(|| files.iter().position(|p| p == cur));
    let start = i.map_or(0, |i| i + 1);
    let cover = ["cover.jpg", "folder.jpg", "front.jpg", "cover.png", "folder.png", "albumart.jpg"].iter().find_map(|f| std::fs::read(dir.join(f)).ok()).map(Arc::new);
    let items = files[start.min(files.len())..]
        .iter()
        .take(200)
        .map(|p| {
            let stem = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
            // "가수 - 제목" 꼴이면 나눠 보인다. 앞의 "01. " 같은 번호는 뗀다.
            let bare = stem.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == '-' || c == ' ' || c == '_').to_string();
            let bare = if bare.is_empty() { stem.clone() } else { bare };
            let (title, sub) = match bare.split_once(" - ") {
                Some((a, t)) => (t.trim().to_string(), a.trim().to_string()),
                None => (bare, String::new()),
            };
            Item { title, sub, key: p.to_string_lossy().to_string(), thumb: cover.clone() }
        })
        .collect();
    (items, dir.file_name().unwrap_or_default().to_string_lossy().to_string())
}

/// 내 음악: 곡이 바뀔 때마다 폴더 목록을 다시 만든다.
pub fn spawn(now: Arc<Mutex<Now>>, q: Arc<Mutex<Queue>>) {
    std::thread::spawn(move || {
        let mut key = String::new();
        let mut known: Vec<PathBuf> = vec![];
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let (has, app, title, artist) = {
                let n = now.lock().unwrap();
                (n.has, n.app.clone(), n.title.clone(), n.artist.clone())
            };
            let k = format!("{app}|{artist}|{title}");
            if k == key {
                continue;
            }
            key = k;
            if !has || is_browser(&app) {
                let mut g = q.lock().unwrap();
                if !g.local.is_empty() || !g.local_dir.is_empty() {
                    g.local.clear();
                    g.local_dir.clear();
                    g.ver += 1;
                }
                continue;
            }
            if app == "ALSong" {
                // 알송 자체 목록에서 지금 곡 다음부터(끝이면 처음부터 이어서)
                let mut g = q.lock().unwrap();
                match crate::alsong::list(&title) {
                    Some((rows, at)) => {
                        let n = rows.len();
                        g.local = (1..n).map(|d| &rows[(at + d) % n]).map(|r| Item { title: r.title.clone(), sub: r.artist.clone(), key: r.uri.clone(), thumb: None }).collect();
                        g.local_dir = "알송 재생목록".into();
                        crate::log(&format!("다음 곡: '{title}' → 알송 목록 {}곡 중 {}번째, 다음 {}곡", n, at + 1, g.local.len()));
                    }
                    None => {
                        g.local.clear();
                        g.local_dir.clear();
                        crate::log(&format!("다음 곡: '{title}' 알송 목록에서 못 찾음"));
                    }
                }
                g.exe = r"C:\Program Files (x86)\ESTsoft\ALSong\ALSong.exe".into();
                g.local_stamp = crate::hhmm();
                g.ver += 1;
                continue;
            }
            let tok = app_token(&app);
            let ps = procs();
            let mine = |p: &Proc| tok.len() >= 3 && !p.name.is_empty() && (tok.contains(&p.name) || p.name.contains(&tok));
            // 지금 곡 이름과 맞는 파일이 명령줄에 있으면 그게 제일 확실하다. 다음은 재생기 자신의 명령줄.
            let by_title = ps.iter().find_map(|p| p.paths.iter().find(|f| f.file_stem().map_or(false, |s| same_song(&s.to_string_lossy(), &title))).map(|f| (f.clone(), p.exe.clone())));
            let by_app = ps.iter().filter(|p| mine(p)).find_map(|p| p.paths.first().map(|f| (f.clone(), p.exe.clone())));
            let exe = ps.iter().find(|p| mine(p)).map(|p| p.exe.clone()).unwrap_or_default();
            let found = by_title.or(by_app).or_else(|| search(&title, &known).map(|f| (f, exe.clone())));
            let mut g = q.lock().unwrap();
            match found {
                Some((f, ex)) => {
                    if let Some(d) = f.parent() {
                        if !known.iter().any(|k| k == d) {
                            known.insert(0, d.to_path_buf());
                            known.truncate(5);
                        }
                    }
                    let (items, dir) = folder_list(&f, &title);
                    crate::log(&format!("다음 곡: '{title}' → {} 폴더 다음 {}곡 (재생기 {app})", dir, items.len()));
                    g.local = items;
                    g.local_dir = dir;
                    g.exe = if ex.is_empty() { exe } else { ex };
                }
                None => {
                    g.local.clear();
                    g.local_dir.clear();
                    crate::log(&format!("다음 곡: '{title}' 파일을 못 찾음 (재생기 {app})"));
                }
            }
            g.local_stamp = crate::hhmm();
            g.ver += 1;
        }
    });
}

/// 내 음악 곡을 같은 재생기로 연다(재생기를 모르거나 스토어 앱이면 윈도우 기본 프로그램으로).
pub fn play_local(path: &str, exe: &str) {
    let r = if !exe.is_empty() && !exe.contains("WindowsApps") && Path::new(exe).is_file() {
        std::process::Command::new(exe).arg(path).spawn().map(|_| ())
    } else {
        std::process::Command::new("cmd").args(["/c", "start", "", path]).creation_flags(NO_WINDOW).spawn().map(|_| ())
    };
    if let Err(e) = r {
        crate::log(&format!("WARN 다음 곡 열기 실패: {e}"));
    }
}

fn get_bytes(c: &HttpClient, url: &str) -> windows::core::Result<Vec<u8>> {
    let buf = c.GetBufferAsync(&Uri::CreateUri(&HSTRING::from(url))?)?.get()?;
    let rd = DataReader::FromBuffer(&buf)?;
    let mut v = vec![0u8; buf.Length()? as usize];
    rd.ReadBytes(&mut v)?;
    Ok(v)
}

/// 유튜브: 확장의 보고를 받는 작은 문(이 PC 안에서만 열린다).
pub fn serve(q: Arc<Mutex<Queue>>) {
    std::thread::spawn(move || {
        let l = match TcpListener::bind(("127.0.0.1", PORT)) {
            Ok(l) => l,
            Err(e) => {
                crate::log(&format!("WARN 유튜브 목록 받는 문을 못 열었다: {e}"));
                return;
            }
        };
        let client = HttpClient::new().ok();
        let mut thumbs: HashMap<String, Arc<Vec<u8>>> = HashMap::new();
        for mut s in l.incoming().flatten() {
            let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
            let mut buf = vec![];
            let mut chunk = [0u8; 8192];
            let mut need: Option<usize> = None;
            while let Ok(n) = s.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&chunk[..n]);
                if need.is_none() {
                    if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&buf[..i]).to_lowercase();
                        let len = head.lines().find_map(|l| l.strip_prefix("content-length:").and_then(|v| v.trim().parse::<usize>().ok())).unwrap_or(0);
                        need = Some(i + 4 + len);
                    }
                }
                if need.map_or(false, |n| buf.len() >= n) || buf.len() > 2_000_000 {
                    break;
                }
            }
            let body = buf.windows(4).position(|w| w == b"\r\n\r\n").map(|i| &buf[i + 4..]).unwrap_or(&[]);
            let v: Value = serde_json::from_slice(body).unwrap_or_default();
            let tab = v["tab"].as_i64().unwrap_or(0);
            let playing = v["playing"].as_bool().unwrap_or(false);
            let mut want: Vec<String> = vec![];
            let reply = {
                let mut g = q.lock().unwrap();
                // 재생 중인 탭이 주인. 멈춘 탭은 자기가 주인일 때만 갱신(다른 탭이 목록을 덮지 않게)
                if v.is_object() && (playing || g.yt_tab == tab || !g.yt_live()) {
                    g.yt_tab = tab;
                    g.yt_kind = v["kind"].as_str().unwrap_or("").to_string();
                    g.yt = v["items"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .take(200)
                        .map(|it| {
                            let img = it["img"].as_str().unwrap_or("").to_string();
                            let thumb = thumbs.get(&img).cloned();
                            if thumb.is_none() && !img.is_empty() && want.len() < 12 {
                                want.push(img.clone());
                            }
                            Item { title: it["t"].as_str().unwrap_or("").to_string(), sub: it["c"].as_str().unwrap_or("").to_string(), key: format!("{}\n{}", it["k"].as_str().unwrap_or(""), img), thumb }
                        })
                        .collect();
                    g.yt_seen = Some(Instant::now());
                    g.yt_stamp = crate::hhmm();
                    g.ver += 1;
                }
                match g.yt_cmd.take() {
                    Some((t, k)) if t == tab => serde_json::json!({ "play": k }),
                    Some(other) => {
                        g.yt_cmd = Some(other);
                        serde_json::json!({})
                    }
                    None => serde_json::json!({}),
                }
            };
            let rb = reply.to_string();
            let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{rb}", rb.len()).as_bytes());
            drop(s);
            // 썸네일은 답한 뒤에 받는다(유튜브 그림 주소만)
            if let Some(c) = &client {
                let mut got = false;
                for url in want {
                    if !(url.starts_with("https://i.ytimg.com/") || url.starts_with("https://lh3.googleusercontent.com/") || url.starts_with("https://yt3.ggpht.com/")) {
                        continue;
                    }
                    if let Ok(b) = get_bytes(c, &url) {
                        thumbs.insert(url, Arc::new(b));
                        got = true;
                    }
                }
                if thumbs.len() > 600 {
                    thumbs.clear();
                }
                if got {
                    let mut g = q.lock().unwrap();
                    for it in g.yt.iter_mut() {
                        if it.thumb.is_none() {
                            if let Some(b) = it.key.split_once('\n').and_then(|(_, img)| thumbs.get(img)) {
                                it.thumb = Some(b.clone());
                            }
                        }
                    }
                    g.ver += 1;
                }
            }
        }
    });
}

/// 확장 파일을 데이터 폴더에 풀어 둔다(실행 파일이 바뀌면 같이 바뀐다). 크롬에서 이 폴더를 한 번 '로드'하면 된다.
pub fn write_ext() -> PathBuf {
    let d = crate::data_dir().join("youtube-ext");
    let _ = std::fs::create_dir_all(&d);
    for (name, body) in [("manifest.json", include_str!("../youtube-ext/manifest.json")), ("background.js", include_str!("../youtube-ext/background.js")), ("content.js", include_str!("../youtube-ext/content.js"))] {
        let p = d.join(name);
        if std::fs::read_to_string(&p).ok().as_deref() != Some(body) {
            let _ = std::fs::write(&p, body);
        }
    }
    d
}
