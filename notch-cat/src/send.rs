//! 휴대폰으로 보내기: 같은 와이파이에서 노치가 작은 내려받기 창구를 열고 QR 을 띄운다.
//! 휴대폰 카메라로 QR 을 찍으면 브라우저가 그 파일을 받는다(앱 설치 없음).
//! 주소에는 한 번 쓰는 짧은 열쇠가 붙고, 5분 뒤 창구가 저절로 닫힌다. 처음 한 번 윈도우 방화벽 '허용'은 사용자가 누른다.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const OPEN_FOR: Duration = Duration::from_secs(300);

pub struct Send {
    pub url: String,
    pub name: String,
    pub until: Instant,
    pub served: u32,        // 휴대폰이 받아 간 횟수
    pub qr: Vec<Vec<bool>>, // QR 칸(검정 = true)
    stop: Arc<AtomicBool>,
}

impl Send {
    pub fn left(&self) -> Duration {
        self.until.saturating_duration_since(Instant::now())
    }
}

impl Drop for Send {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed); // 새로 보내거나 닫으면 창구도 닫는다
    }
}

/// 이 PC 의 와이파이(사설망) 주소. 밖으로 패킷은 보내지 않는다(경로만 묻는다).
pub fn lan_ip() -> Option<String> {
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("8.8.8.8:80").ok()?;
    Some(s.local_addr().ok()?.ip().to_string())
}

pub fn qr(url: &str) -> Vec<Vec<bool>> {
    use qrcodegen::{QrCode, QrCodeEcc};
    let Ok(q) = QrCode::encode_text(url, QrCodeEcc::Medium) else { return vec![] };
    (0..q.size()).map(|y| (0..q.size()).map(|x| q.get_module(x, y)).collect()).collect()
}

/// 파일 이름을 내려받기 머리에 넣을 꼴로(한글은 UTF-8 % 꼴)
fn disposition(name: &str) -> String {
    let ascii: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || ".-_".contains(c) { c } else { '_' }).collect();
    let pct: String = name.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect();
    format!("attachment; filename=\"{ascii}\"; filename*=UTF-8''{pct}")
}

fn serve(mut c: TcpStream, key: &str, path: &Path, name: &str) -> bool {
    let _ = c.set_read_timeout(Some(Duration::from_secs(5)));
    let mut buf = [0u8; 2048];
    let n = c.read(&mut buf).unwrap_or(0);
    let req = String::from_utf8_lossy(&buf[..n]);
    let target = req.split_whitespace().nth(1).unwrap_or("");
    if target != format!("/{key}") {
        let _ = c.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return false;
    }
    let Ok(mut f) = std::fs::File::open(path) else {
        let _ = c.write_all(b"HTTP/1.1 410 Gone\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return false;
    };
    let len = f.metadata().map_or(0, |m| m.len());
    let head = format!("HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {len}\r\nContent-Disposition: {}\r\nConnection: close\r\n\r\n", disposition(name));
    c.write_all(head.as_bytes()).is_ok() && std::io::copy(&mut f, &mut c).is_ok()
}

/// 파일 하나를 보낼 창구를 연다. lan = false 면 이 PC 안에서만(시험용).
pub fn start(path: &Path, lan: bool, state: Arc<Mutex<Option<Send>>>) -> std::io::Result<()> {
    let ip = if lan { lan_ip().ok_or_else(|| std::io::Error::other("와이파이 주소 없음"))? } else { "127.0.0.1".into() };
    let l = TcpListener::bind((if lan { "0.0.0.0" } else { "127.0.0.1" }, 0))?;
    l.set_nonblocking(true)?;
    let port = l.local_addr()?.port();
    let key = crate::gcal::random(6);
    let url = format!("http://{ip}:{port}/{key}");
    let name = path.file_name().map_or_else(|| "file".into(), |n| n.to_string_lossy().into_owned());
    let stop = Arc::new(AtomicBool::new(false));
    *state.lock().unwrap() = Some(Send { qr: qr(&url), url, name: name.clone(), until: Instant::now() + OPEN_FOR, served: 0, stop: stop.clone() });
    let path: PathBuf = path.to_path_buf();
    std::thread::spawn(move || {
        let end = Instant::now() + OPEN_FOR;
        while Instant::now() < end && !stop.load(Ordering::Relaxed) {
            match l.accept() {
                Ok((c, _)) => {
                    let _ = c.set_nonblocking(false);
                    let (key, path, name, st, stop) = (key.clone(), path.clone(), name.clone(), state.clone(), stop.clone());
                    std::thread::spawn(move || {
                        if serve(c, &key, &path, &name) && !stop.load(Ordering::Relaxed) {
                            if let Some(s) = st.lock().unwrap().as_mut() {
                                s.served += 1;
                            }
                            crate::log("send: 휴대폰이 받아 감");
                        }
                    });
                }
                Err(_) => std::thread::sleep(Duration::from_millis(150)),
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(url: &str) -> (String, Vec<u8>) {
        let rest = url.strip_prefix("http://").unwrap();
        let (host, path) = rest.split_once('/').unwrap();
        let mut c = TcpStream::connect(host).unwrap();
        c.write_all(format!("GET /{path} HTTP/1.1\r\nHost: {host}\r\n\r\n").as_bytes()).unwrap();
        let mut b = Vec::new();
        c.read_to_end(&mut b).unwrap();
        let at = b.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        (String::from_utf8_lossy(&b[..at]).into_owned(), b[at + 4..].to_vec())
    }

    #[test]
    fn serves_file_only_with_key() {
        let f = std::env::temp_dir().join("노치 보내기 시험.txt");
        std::fs::write(&f, "안녕 휴대폰").unwrap();
        let st = Arc::new(Mutex::new(None));
        start(&f, false, st.clone()).unwrap();
        let url = st.lock().unwrap().as_ref().unwrap().url.clone();
        let (head, body) = get(&url);
        assert!(head.starts_with("HTTP/1.1 200"), "{head}");
        assert!(head.contains("filename*=UTF-8''%EB%85%B8%EC%B9%98"), "{head}");
        assert_eq!(body, "안녕 휴대폰".as_bytes());
        let (head, _) = get(&format!("{}x", url)); // 열쇠가 틀리면 못 받는다
        assert!(head.starts_with("HTTP/1.1 404"), "{head}");
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(st.lock().unwrap().as_ref().unwrap().served, 1);
        assert!(!st.lock().unwrap().as_ref().unwrap().qr.is_empty());
        *st.lock().unwrap() = None; // 닫기
        let _ = std::fs::remove_file(f);
    }
}
