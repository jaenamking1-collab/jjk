//! 새 판 스스로 받기(2026-11-17 사용자 "받으라고 해줘"): 전산실·집 PC 에서 cat.html 을 다시 열지 않아도 되게.
//! 켜고 1분 뒤·6시간마다 릴리스 `notch` 의 NotchCat.exe 지문(sha256)을 내 실행 파일과 비교해, 다르면 받아서
//! 옆에 두고 바꿔 끼운 뒤 새 판을 실행한다(새 판이 켜지며 지금 것을 닫는다 — 단일 실행 규칙).
use std::sync::{Arc, Mutex};
use std::time::Duration;
use serde_json::Value;
use windows::core::{Result, HSTRING};
use windows::Foundation::Uri;
use windows::Storage::Streams::DataReader;
use windows::Web::Http::HttpClient;

const API: &str = "https://api.github.com/repos/jaenamking1-collab/jjk/releases/tags/notch";

pub fn sha256(data: &[u8]) -> String {
    use windows::Win32::Security::Cryptography::{BCryptHash, BCRYPT_SHA256_ALG_HANDLE};
    let mut out = [0u8; 32];
    let _ = unsafe { BCryptHash(BCRYPT_SHA256_ALG_HANDLE, None, data, &mut out) };
    out.iter().map(|b| format!("{b:02x}")).collect()
}

fn bytes(c: &HttpClient, url: &str) -> Result<(u32, Vec<u8>)> {
    let r = c.GetAsync(&Uri::CreateUri(&HSTRING::from(url))?)?.get()?;
    let code = r.StatusCode()?.0 as u32;
    let buf = r.Content()?.ReadAsBufferAsync()?.get()?;
    let mut v = vec![0u8; buf.Length()? as usize];
    if !v.is_empty() {
        DataReader::FromBuffer(&buf)?.ReadBytes(&mut v)?;
    }
    Ok((code, v))
}

/// 릴리스에서 (지문, 받을 주소). 지문이 없으면 None.
pub fn latest(json: &str) -> Option<(String, String)> {
    let v: Value = serde_json::from_str(json).ok()?;
    let a = v["assets"].as_array()?.iter().find(|a| a["name"] == "NotchCat.exe")?;
    Some((a["digest"].as_str()?.strip_prefix("sha256:")?.to_lowercase(), a["browser_download_url"].as_str()?.to_string()))
}

/// 설치한 정식 판일 때만(손으로 빌드한 것·시험판은 건드리지 않는다).
fn installed() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let p = exe.to_string_lossy().to_lowercase();
    let test = std::env::vars().any(|(k, _)| k.starts_with("NOTCH_CAT_"));
    (exe.file_name()? == "NotchCat.exe" && !p.contains("\\target\\") && !test).then_some(exe)
}

fn check(c: &HttpClient, exe: &std::path::Path, notes: &Arc<Mutex<crate::notif::Note>>) -> Option<()> {
    let (code, body) = bytes(c, API).ok()?;
    if code != 200 {
        crate::log(&format!("update: 릴리스 확인 실패 {code}"));
        return None;
    }
    let (want, url) = latest(&String::from_utf8_lossy(&body))?;
    let mine = sha256(&std::fs::read(exe).ok()?);
    if mine == want {
        return None;
    }
    let (code, data) = bytes(c, &url).ok()?;
    if code != 200 || sha256(&data) != want {
        crate::log(&format!("update: 받기 실패 {code} {}바이트", data.len()));
        return None;
    }
    let new = exe.with_file_name("NotchCat.new.exe");
    let old = exe.with_file_name("NotchCat.old.exe");
    let _ = std::fs::remove_file(&old);
    std::fs::write(&new, &data).ok()?;
    // 실행 중인 파일은 지울 순 없어도 이름은 바꿀 수 있다
    if std::fs::rename(exe, &old).is_err() || std::fs::rename(&new, exe).is_err() {
        let _ = std::fs::rename(&old, exe);
        crate::log("update: 바꿔 끼우기 실패");
        return None;
    }
    crate::log(&format!("update: 새 판으로 바꿈 {}", &want[..12]));
    {
        let mut n = notes.lock().unwrap();
        n.ver += 1;
        n.app = "노치 고양이".into();
        n.title = "새 판으로 바꿉니다".into();
        n.body = "잠깐 꺼졌다 다시 켜집니다".into();
        n.icon = None;
    }
    std::thread::sleep(Duration::from_secs(4)); // 알림을 보여 줄 틈
    unsafe {
        windows::Win32::UI::Shell::ShellExecuteW(None, &HSTRING::from("open"), &HSTRING::from(exe.as_os_str()), None, None, windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL);
    }
    Some(())
}

pub fn spawn(notes: Arc<Mutex<crate::notif::Note>>) {
    let Some(exe) = installed() else { return };
    let _ = std::fs::remove_file(exe.with_file_name("NotchCat.old.exe")); // 지난번에 바꾸고 남은 것
    std::thread::spawn(move || {
        let Ok(c) = crate::quotes::fresh_client() else { return };
        if let Ok(h) = c.DefaultRequestHeaders() {
            let _ = h.UserAgent().and_then(|u| u.TryParseAdd(&HSTRING::from("notch-cat")));
        }
        std::thread::sleep(Duration::from_secs(60));
        loop {
            if check(&c, &exe, &notes).is_some() {
                return;
            }
            std::thread::sleep(Duration::from_secs(6 * 3600));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_release_digest() {
        let j = r#"{"assets":[{"name":"x.txt"},{"name":"NotchCat.exe","digest":"sha256:ABCdef","browser_download_url":"https://g/n.exe"}]}"#;
        assert_eq!(latest(j), Some(("abcdef".into(), "https://g/n.exe".into())));
        assert_eq!(latest(r#"{"assets":[{"name":"NotchCat.exe"}]}"#), None);
        assert_eq!(sha256(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    /// 지금 릴리스 지문이 실제로 받은 파일 지문과 같은지. cargo test release_digest_live -- --ignored --nocapture
    #[test]
    #[ignore]
    fn release_digest_live() {
        let c = crate::quotes::fresh_client().unwrap();
        c.DefaultRequestHeaders().unwrap().UserAgent().unwrap().TryParseAdd(&HSTRING::from("notch-cat")).unwrap();
        let (code, body) = bytes(&c, API).unwrap();
        assert_eq!(code, 200);
        let (want, url) = latest(&String::from_utf8_lossy(&body)).expect("지문");
        let (_, data) = bytes(&c, &url).unwrap();
        assert_eq!(sha256(&data), want);
        println!("{} {}바이트", &want[..12], data.len());
    }
}
