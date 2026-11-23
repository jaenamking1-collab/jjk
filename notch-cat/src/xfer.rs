//! 파일 전송 쪽: 같은 와이파이의 휴대폰과 PC 가 양쪽으로 파일을 주고받는 창구.
//! 휴대폰은 QR 로 연 웹 화면에서 ① PC 가 내놓은 파일을 받고 ② 휴대폰 파일을 올린다(앱 설치 없음).
//! 올린 파일은 PC 의 '다운로드' 폴더에 저장된다. 주소엔 짧은 열쇠가 붙고, 10분 동안 아무 일이 없으면 저절로 닫힌다.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const IDLE: Duration = Duration::from_secs(600);

pub struct Xfer {
    pub url: String,
    pub qr: Vec<Vec<bool>>,
    pub until: Instant,        // 이때까지 아무 일 없으면 닫는다(일이 생기면 늘어난다)
    pub out: Vec<PathBuf>,     // PC → 휴대폰(노치에 끌어다 놓은 파일)
    pub got: Vec<PathBuf>,     // 휴대폰 → PC(받은 파일, 최근 것이 앞)
    pub busy: Option<(String, u64, u64)>, // 지금 받는 중(이름, 받은 바이트, 전체)
    pub served: u32,           // 휴대폰이 받아 간 횟수
    pub relay: Option<(String, Vec<Vec<bool>>)>, // 인터넷 중계 방 주소·QR(열렸으면)
    rids: Vec<(PathBuf, u64)>, // 중계소에 올린 파일 → 그쪽 번호
    rkey: String,
    me: State,
    dir: PathBuf,
    stop: Arc<AtomicBool>,
}

impl Xfer {
    pub fn left(&self) -> Duration {
        self.until.saturating_duration_since(Instant::now())
    }
    pub fn add(&mut self, paths: &[String]) {
        for p in paths {
            let p = PathBuf::from(p);
            if p.is_file() && !self.out.contains(&p) {
                self.out.insert(0, p.clone());
                if !self.rkey.is_empty() && p.metadata().map_or(0, |m| m.len()) <= RELAY_LIMIT {
                    let key = self.rkey.clone();
                    let st = self.me.clone();
                    std::thread::spawn(move || relay_put(&key, &p, st));
                }
            }
        }
        self.until = Instant::now() + IDLE;
    }
    /// 내놓은 파일 k 를 뺀다(중계소에 올린 것도 지운다)
    pub fn remove(&mut self, k: usize) {
        if k >= self.out.len() {
            return;
        }
        let p = self.out.remove(k);
        if let Some(i) = self.rids.iter().position(|r| r.0 == p) {
            let (_, id) = self.rids.remove(i);
            let key = self.rkey.clone();
            std::thread::spawn(move || curl(&["-X", "POST", &format!("{}/{key}/del/{id}", relay_base())]));
        }
    }
}

impl Drop for Xfer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

pub type State = Arc<Mutex<Option<Xfer>>>;

/// 받은 파일을 둘 곳 = 사용자 '다운로드' 폴더
pub fn downloads() -> PathBuf {
    PathBuf::from(std::env::var("USERPROFILE").unwrap_or_else(|_| ".".into())).join("Downloads")
}

fn pct_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Some(v) = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(if b[i] == b'+' { b' ' } else { b[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// 휴대폰이 보낸 이름 → 윈도우에 저장할 수 있는 이름(폴더 경로·금지 글자 제거)
pub fn safe_name(raw: &str) -> String {
    let last = raw.rsplit(['/', '\\']).next().unwrap_or("");
    let s: String = last.chars().filter(|c| !c.is_control() && !"<>:\"|?*".contains(*c)).collect();
    let s = s.trim().trim_matches('.').trim().to_string();
    if s.is_empty() { "휴대폰 파일".into() } else { s }
}

/// 같은 이름이 있으면 '이름 (1).확장자' 꼴로 비켜 간다
fn unique(dir: &Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    if !p.exists() {
        return p;
    }
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    (1..).map(|k| dir.join(format!("{stem} ({k}){ext}"))).find(|p| !p.exists()).unwrap()
}

fn reply(c: &mut TcpStream, code: &str, ctype: &str, body: &[u8]) {
    let head = format!("HTTP/1.1 {code}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n", body.len());
    let _ = c.write_all(head.as_bytes());
    let _ = c.write_all(body);
}

fn json_str(s: &str) -> String {
    serde_json::Value::String(s.into()).to_string()
}

fn handle(mut c: TcpStream, key: &str, st: &State) {
    let _ = c.set_read_timeout(Some(Duration::from_secs(30)));
    // 머리(헤더)를 다 읽는다
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let at = loop {
        if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break p;
        }
        match c.read(&mut chunk) {
            Ok(n) if n > 0 && buf.len() < 16 * 1024 => buf.extend_from_slice(&chunk[..n]),
            _ => return,
        }
    };
    let head = String::from_utf8_lossy(&buf[..at]).into_owned();
    let mut rest = buf[at + 4..].to_vec();
    let mut first = head.lines().next().unwrap_or("").split_whitespace();
    let (method, target) = (first.next().unwrap_or(""), first.next().unwrap_or(""));
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let Some(sub) = path.strip_prefix(&format!("/{key}")) else {
        return reply(&mut c, "404 Not Found", "text/plain", b"");
    };
    let touch = |st: &State| {
        if let Some(x) = st.lock().unwrap().as_mut() {
            x.until = Instant::now() + IDLE;
        }
    };
    touch(st);
    match (method, sub) {
        ("GET", "" | "/") => reply(&mut c, "200 OK", "text/html; charset=utf-8", PAGE.replace("{KEY}", key).as_bytes()),
        ("GET", "/list") => {
            let g = st.lock().unwrap();
            let Some(x) = g.as_ref() else { return };
            let out: Vec<String> = x.out.iter().enumerate().filter(|(_, p)| p.is_file()).map(|(i, p)| {
                let n = p.file_name().map_or(String::new(), |n| n.to_string_lossy().into_owned());
                format!("{{\"i\":{i},\"name\":{},\"size\":{}}}", json_str(&n), p.metadata().map_or(0, |m| m.len()))
            }).collect();
            let got: Vec<String> = x.got.iter().map(|p| json_str(&p.file_name().map_or(String::new(), |n| n.to_string_lossy().into_owned()))).collect();
            let body = format!("{{\"out\":[{}],\"got\":[{}]}}", out.join(","), got.join(","));
            drop(g);
            reply(&mut c, "200 OK", "application/json", body.as_bytes());
        }
        ("GET", s) if s.starts_with("/d/") => {
            let p = s[3..].parse::<usize>().ok().and_then(|i| st.lock().unwrap().as_ref().and_then(|x| x.out.get(i).cloned()));
            let Some(p) = p else { return reply(&mut c, "404 Not Found", "text/plain", b"") };
            let Ok(mut f) = std::fs::File::open(&p) else { return reply(&mut c, "410 Gone", "text/plain", b"") };
            let name = p.file_name().map_or("file".into(), |n| n.to_string_lossy().into_owned());
            let len = f.metadata().map_or(0, |m| m.len());
            let head = format!("HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {len}\r\nContent-Disposition: {}\r\nConnection: close\r\n\r\n", crate::send::disposition(&name));
            if c.write_all(head.as_bytes()).is_ok() && std::io::copy(&mut f, &mut c).is_ok() {
                if let Some(x) = st.lock().unwrap().as_mut() {
                    x.served += 1;
                }
                crate::log("xfer: 휴대폰이 받아 감");
            }
        }
        ("POST", "/up") => {
            let raw = query.split('&').find_map(|kv| kv.strip_prefix("n=")).unwrap_or("");
            let name = safe_name(&pct_decode(raw));
            let len: u64 = head.lines().find_map(|l| {
                let (k, v) = l.split_once(':')?;
                k.trim().eq_ignore_ascii_case("content-length").then(|| v.trim().parse().ok())?
            }).unwrap_or(0);
            let dir = st.lock().unwrap().as_ref().map(|x| x.dir.clone()).unwrap_or_else(downloads);
            let _ = std::fs::create_dir_all(&dir);
            let dest = unique(&dir, &name);
            let part = dest.with_file_name(format!("{}.받는중", dest.file_name().unwrap().to_string_lossy()));
            let Ok(mut f) = std::fs::File::create(&part) else { return reply(&mut c, "500 Internal Server Error", "text/plain", b"") };
            rest.truncate(len as usize);
            let mut done = rest.len() as u64;
            let mut ok = f.write_all(&rest).is_ok();
            let mut big = vec![0u8; 256 * 1024];
            while ok && done < len {
                match c.read(&mut big) {
                    Ok(n) if n > 0 => {
                        let n = n.min((len - done) as usize);
                        ok = f.write_all(&big[..n]).is_ok();
                        done += n as u64;
                        if let Some(x) = st.lock().unwrap().as_mut() {
                            x.busy = Some((name.clone(), done, len));
                            x.until = Instant::now() + IDLE;
                        }
                    }
                    _ => ok = false,
                }
            }
            drop(f);
            if let Some(x) = st.lock().unwrap().as_mut() {
                x.busy = None;
            }
            if ok && std::fs::rename(&part, &dest).is_ok() {
                crate::log("xfer: 휴대폰에서 받음");
                if let Some(x) = st.lock().unwrap().as_mut() {
                    x.got.insert(0, dest.clone());
                }
                reply(&mut c, "200 OK", "application/json", b"{\"ok\":true}");
            } else {
                let _ = std::fs::remove_file(&part); // 끊기면 반쪽 파일을 남기지 않는다
                reply(&mut c, "400 Bad Request", "text/plain", b"");
            }
        }
        _ => reply(&mut c, "404 Not Found", "text/plain", b""),
    }
}

/// 창구를 연다. lan = false 면 이 PC 안에서만(시험용). dir = 받은 파일을 둘 곳.
pub fn start(lan: bool, relay: bool, dir: PathBuf, st: State) -> std::io::Result<()> {
    let ip = if lan { crate::send::lan_ip().ok_or_else(|| std::io::Error::other("와이파이 주소 없음"))? } else { "127.0.0.1".into() };
    let l = TcpListener::bind((if lan { "0.0.0.0" } else { "127.0.0.1" }, 0))?;
    l.set_nonblocking(true)?;
    let port = l.local_addr()?.port();
    let key = crate::gcal::random(6);
    let url = format!("http://{ip}:{port}/{key}");
    let stop = Arc::new(AtomicBool::new(false));
    let keep = st.lock().unwrap().take().map(|x| x.out.clone()).unwrap_or_default(); // 다시 열어도 내놓은 파일은 그대로
    let rkey = if relay { crate::gcal::random(16).replace(['.', '~'], "_") } else { String::new() };
    *st.lock().unwrap() = Some(Xfer { qr: crate::send::qr(&url), url, until: Instant::now() + IDLE, out: vec![], got: vec![], busy: None, served: 0, relay: None, rids: vec![], rkey: rkey.clone(), me: st.clone(), dir, stop: stop.clone() });
    if relay {
        let (st, stop) = (st.clone(), stop.clone());
        std::thread::spawn(move || relay_loop(rkey, st, stop));
    }
    if let Some(x) = st.lock().unwrap().as_mut() {
        x.add(&keep.iter().map(|p| p.display().to_string()).collect::<Vec<_>>());
    }
    std::thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) {
            if st.lock().unwrap().as_ref().map_or(true, |x| x.left().is_zero() && x.busy.is_none()) {
                *st.lock().unwrap() = None; // 오래 쉬었다: 닫는다
                break;
            }
            match l.accept() {
                Ok((c, _)) => {
                    let _ = c.set_nonblocking(false);
                    let (key, st) = (key.clone(), st.clone());
                    std::thread::spawn(move || handle(c, &key, &st));
                }
                Err(_) => std::thread::sleep(Duration::from_millis(150)),
            }
        }
    });
    Ok(())
}

// ── 인터넷 중계(Cloudflare, workers/notch-drop). 와이파이가 달라도 된다. ──

pub const RELAY_LIMIT: u64 = 95 * 1024 * 1024;

/// 중계소 주소. NOTCH_CAT_RELAY 로 바꿀 수 있다(시험용·나중에 집 서버로 옮길 때).
pub fn relay_base() -> String {
    std::env::var("NOTCH_CAT_RELAY").unwrap_or_else(|_| "https://notch-drop.distributionjn.workers.dev".into())
}

/// 윈도우에 들어 있는 curl 로 요청한다(창 없이). (상태 코드, 받은 글)
fn curl(args: &[&str]) -> Option<(u16, String)> {
    use std::os::windows::process::CommandExt;
    let o = std::process::Command::new("curl.exe").args(["-s", "-m", "900", "-w", "\n%{http_code}"]).args(args).creation_flags(0x0800_0000).output().ok()?;
    let t = String::from_utf8_lossy(&o.stdout).into_owned();
    let (body, code) = t.rsplit_once('\n')?;
    Some((code.trim().parse().ok()?, body.to_string()))
}

fn relay_put(key: &str, p: &Path, st: State) {
    let name = p.file_name().map_or("file".into(), |n| n.to_string_lossy().into_owned());
    let url = format!("{}/{key}/put?n={}", relay_base(), crate::gcal::enc(&name));
    let at = format!("@{}", p.display());
    match curl(&["-X", "POST", "-H", "Content-Type: application/octet-stream", "--data-binary", &at, &url]) {
        Some((200, b)) => {
            let id = serde_json::from_str::<serde_json::Value>(&b).ok().and_then(|v| v["id"].as_u64());
            if let (Some(id), Some(x)) = (id, st.lock().unwrap().as_mut()) {
                x.rids.push((p.to_path_buf(), id));
            }
        }
        r => crate::log(&format!("xfer: 중계 올리기 실패 {:?}", r.map(|r| r.0))),
    }
}

fn relay_loop(key: String, st: State, stop: Arc<AtomicBool>) {
    let base = relay_base();
    let open = curl(&["-X", "POST", &format!("{base}/{key}/open")]);
    if open.as_ref().map(|r| r.0) != Some(200) {
        crate::log(&format!("xfer: 중계 못 엶 {:?}", open.map(|r| r.0)));
        return;
    }
    let url = format!("{base}/{key}");
    if let Some(x) = st.lock().unwrap().as_mut() {
        x.relay = Some((url.clone(), crate::send::qr(&url)));
    }
    crate::log("xfer: 중계 열림");
    while !stop.load(Ordering::Relaxed) {
        // 휴대폰이 올린 파일을 받아 다운로드 폴더에 둔다
        if let Some((200, b)) = curl(&[&format!("{url}/inbox")]) {
            let items: Vec<serde_json::Value> = serde_json::from_str(&b).unwrap_or_default();
            for it in items {
                let (Some(id), Some(raw)) = (it["id"].as_u64(), it["name"].as_str()) else { continue };
                let name = safe_name(raw);
                let Some(dir) = st.lock().unwrap().as_mut().map(|x| {
                    x.busy = Some((name.clone(), 0, it["size"].as_u64().unwrap_or(0)));
                    x.until = Instant::now() + IDLE;
                    x.dir.clone()
                }) else { return };
                let _ = std::fs::create_dir_all(&dir);
                let dest = unique(&dir, &name);
                let part = dest.with_file_name(format!("{}.받는중", dest.file_name().unwrap().to_string_lossy()));
                let ok = curl(&["-o", &part.display().to_string(), &format!("{url}/take/{id}")]).is_some_and(|r| r.0 == 200) && std::fs::rename(&part, &dest).is_ok();
                if let Some(x) = st.lock().unwrap().as_mut() {
                    x.busy = None;
                }
                if ok {
                    curl(&["-X", "POST", &format!("{url}/took/{id}")]);
                    crate::log("xfer: 중계로 휴대폰에서 받음");
                    if let Some(x) = st.lock().unwrap().as_mut() {
                        x.got.insert(0, dest);
                    }
                } else {
                    let _ = std::fs::remove_file(&part);
                }
            }
        }
        for _ in 0..20 {
            if stop.load(Ordering::Relaxed) {
                return;
            }
            std::thread::sleep(Duration::from_millis(150));
        }
    }
}

const PAGE: &str = r##"<!doctype html><html lang="ko"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>노치 파일 전송</title>
<style>
body{margin:0;background:#0b0b0c;color:#f3f4f6;font:16px/1.5 -apple-system,"Apple SD Gothic Neo","Malgun Gothic",sans-serif;padding:20px 18px 40px}
h1{font-size:20px;margin:0 0 18px}h2{font-size:15px;margin:26px 0 10px;color:#a3e635}
.up{display:block;background:#a3e635;color:#111;text-align:center;border-radius:16px;padding:22px 0;font-weight:700;font-size:18px}
input{display:none}
.row{display:flex;align-items:center;gap:10px;padding:12px 14px;border-radius:12px;background:#1a1b1e;margin:6px 0;color:#f3f4f6;text-decoration:none}
.row b{flex:1;font-weight:500;word-break:break-all}.row small{color:#9ca3af}
.bar{height:6px;background:#2a2b2f;border-radius:3px;margin-top:8px;overflow:hidden}.bar i{display:block;height:100%;background:#a3e635;width:0}
.empty{color:#6b7280;font-size:14px}
</style></head><body>
<h1>🐱 노치 파일 전송</h1>
<label class="up">휴대폰 → PC 보내기<input id="f" type="file" multiple></label>
<div id="prog"></div>
<h2>PC → 휴대폰 (누르면 받기)</h2><div id="out" class="empty">PC 노치의 '파일 전송' 쪽에 파일을 끌어다 놓으면 여기 나옵니다.</div>
<h2>PC 가 받은 파일</h2><div id="got" class="empty">아직 없습니다.</div>
<script>
const K='/{KEY}';
const sz=n=>n>1e9?(n/1e9).toFixed(1)+'GB':n>1e6?(n/1e6).toFixed(1)+'MB':Math.max(1,Math.round(n/1e3))+'KB';
const esc=s=>s.replace(/[&<>"]/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c]));
async function load(){try{const j=await (await fetch(K+'/list')).json();
const o=document.getElementById('out'),g=document.getElementById('got');
if(j.out.length){o.className='';o.innerHTML=j.out.map(f=>`<a class="row" href="${K}/d/${f.i}" download><b>${esc(f.name)}</b><small>${sz(f.size)}</small></a>`).join('')}
if(j.got.length){g.className='';g.innerHTML=j.got.map(n=>`<div class="row"><b>${esc(n)}</b><small>✓</small></div>`).join('')}}catch(e){}}
function send(file){return new Promise(res=>{const d=document.createElement('div');d.className='row';d.style.display='block';
d.innerHTML=`<b>${esc(file.name)}</b> <small>${sz(file.size)}</small><div class="bar"><i></i></div>`;document.getElementById('prog').appendChild(d);
const x=new XMLHttpRequest();x.open('POST',K+'/up?n='+encodeURIComponent(file.name));
x.upload.onprogress=e=>{if(e.lengthComputable)d.querySelector('i').style.width=(100*e.loaded/e.total)+'%'};
x.onload=()=>{d.querySelector('small').textContent=x.status==200?'보냄 ✓':'실패';if(x.status==200)setTimeout(()=>d.remove(),2500);load();res()};
x.onerror=()=>{d.querySelector('small').textContent='실패 — 같은 와이파이인지 확인';res()};x.send(file)})}
document.getElementById('f').onchange=async e=>{for(const f of e.target.files)await send(f);e.target.value=''};
load();setInterval(load,3000);
</script></body></html>"##;

#[cfg(test)]
mod tests {
    use super::*;

    fn http(url: &str, method: &str, body: &[u8]) -> (String, Vec<u8>) {
        let rest = url.strip_prefix("http://").unwrap();
        let (host, path) = rest.split_once('/').unwrap();
        let mut c = TcpStream::connect(host).unwrap();
        c.write_all(format!("{method} /{path} HTTP/1.1\r\nHost: {host}\r\nContent-Length: {}\r\n\r\n", body.len()).as_bytes()).unwrap();
        c.write_all(body).unwrap();
        let mut b = Vec::new();
        c.read_to_end(&mut b).unwrap();
        let at = b.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        (String::from_utf8_lossy(&b[..at]).into_owned(), b[at + 4..].to_vec())
    }

    #[test]
    fn names_are_safe() {
        assert_eq!(safe_name("../../Windows/evil.exe"), "evil.exe");
        assert_eq!(safe_name("C:\\a\\사진:1?.jpg"), "사진1.jpg");
        assert_eq!(safe_name(".."), "휴대폰 파일");
        assert_eq!(pct_decode("%EC%82%AC%EC%A7%84%20a.jpg"), "사진 a.jpg");
    }

    #[test]
    fn both_ways() {
        let dir = std::env::temp_dir().join(format!("노치전송시험{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("PC 파일.txt");
        std::fs::write(&src, "PC 에서 폰으로").unwrap();
        let st: State = Arc::new(Mutex::new(None));
        start(false, false, dir.join("받음"), st.clone()).unwrap();
        st.lock().unwrap().as_mut().unwrap().add(&[src.display().to_string()]);
        let url = st.lock().unwrap().as_ref().unwrap().url.clone();
        // 화면
        let (h, b) = http(&url, "GET", b"");
        assert!(h.starts_with("HTTP/1.1 200") && String::from_utf8_lossy(&b).contains("노치 파일 전송"), "{h}");
        // 목록·받기(PC → 폰)
        let (_, b) = http(&format!("{url}/list"), "GET", b"");
        assert!(String::from_utf8_lossy(&b).contains("PC 파일.txt"));
        let (h, b) = http(&format!("{url}/d/0"), "GET", b"");
        assert!(h.contains("filename*=UTF-8''PC%20"), "{h}");
        assert_eq!(b, "PC 에서 폰으로".as_bytes());
        // 올리기(폰 → PC), 같은 이름 두 번이면 (1) 로 비켜 간다
        let big: Vec<u8> = (0..700_000u32).map(|i| i as u8).collect();
        for _ in 0..2 {
            let (h, _) = http(&format!("{url}/up?n=%EC%82%AC%EC%A7%84.jpg"), "POST", &big);
            assert!(h.starts_with("HTTP/1.1 200"), "{h}");
        }
        assert_eq!(std::fs::read(dir.join("받음").join("사진.jpg")).unwrap(), big);
        assert!(dir.join("받음").join("사진 (1).jpg").exists());
        assert_eq!(st.lock().unwrap().as_ref().unwrap().got.len(), 2);
        // 열쇠가 틀리면 아무것도 못 한다
        let (h, _) = http(&format!("{url}x/list"), "GET", b"");
        assert!(h.starts_with("HTTP/1.1 404"), "{h}");
        assert_eq!(st.lock().unwrap().as_ref().unwrap().served, 1);
        *st.lock().unwrap() = None;
        let _ = std::fs::remove_dir_all(&dir);
    }
}
