// 가사 받기: LRCLIB /api/get → 없으면 /api/search. 받은 건 %APPDATA%\notch-cat\lyrics\ 에 둔다.
use std::sync::{Arc, Mutex};
use windows::core::{Result, HSTRING};
use windows::Foundation::Uri;
use windows::Storage::Streams::DataReader;
use windows::Web::Http::HttpClient;

use crate::lrc;

#[derive(Default, Clone, Copy, PartialEq)]
pub enum St {
    #[default]
    Idle,
    Loading,
    Found,
    Missing,
    Error,
}

#[derive(Default)]
pub struct Lyr {
    pub key: String,
    pub lines: Vec<(u64, String)>,
    pub st: St,
    pub stamp: String, // 마지막으로 답을 받은 시각(생존 표시)
    pub ver: u64,
}

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect()
}

fn http(url: &str) -> Result<(u32, String)> {
    let c = HttpClient::new()?;
    c.DefaultRequestHeaders()?.UserAgent()?.TryParseAdd(&HSTRING::from("notch-cat/0.1"))?;
    let r = c.GetAsync(&Uri::CreateUri(&HSTRING::from(url))?)?.get()?;
    let code = r.StatusCode()?.0 as u32;
    let buf = r.Content()?.ReadAsBufferAsync()?.get()?;
    let mut v = vec![0u8; buf.Length()? as usize];
    if !v.is_empty() {
        DataReader::FromBuffer(&buf)?.ReadBytes(&mut v)?;
    }
    Ok((code, String::from_utf8_lossy(&v).into_owned()))
}

/// /api/get 응답(객체) 또는 /api/search 응답(배열)에서 시간 맞춘 가사를 고른다.
/// 배열이면 길이가 가장 가까운 것.
pub fn pick(json: &str, dur_s: f64) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let synced = |o: &serde_json::Value| o["syncedLyrics"].as_str().filter(|s| !s.trim().is_empty()).map(String::from);
    match v.as_array() {
        None => synced(&v),
        Some(a) => a
            .iter()
            .filter_map(|o| synced(o).map(|s| ((o["duration"].as_f64().unwrap_or(0.0) - dur_s).abs(), s)))
            .min_by(|x, y| if dur_s > 0.0 { x.0.total_cmp(&y.0) } else { std::cmp::Ordering::Equal })
            .map(|x| x.1),
    }
}

/// 검색 결과에서 길이가 tol 초 안으로 맞는 것만 남긴다.
pub fn only_near(json: &str, dur_s: f64, tol: f64) -> String {
    let v: Vec<serde_json::Value> = serde_json::from_str(json).unwrap_or_default();
    let keep: Vec<_> = v.into_iter().filter(|o| (o["duration"].as_f64().unwrap_or(-99.0) - dur_s).abs() <= tol).collect();
    serde_json::Value::Array(keep).to_string()
}

fn download(title: &str, artist: &str, dur_ms: i64) -> Result<Option<String>> {
    let (a, t) = lrc::clean(title, artist);
    let d = dur_ms as f64 / 1000.0;
    let base = "https://lrclib.net/api";
    let mut urls = Vec::new();
    if dur_ms > 0 {
        urls.push(format!("{base}/get?artist_name={}&track_name={}&duration={}", enc(&a), enc(&t), d.round() as i64));
    }
    urls.push(format!("{base}/search?track_name={}&artist_name={}", enc(&t), enc(&a)));
    urls.push(format!("{base}/search?q={}", enc(&format!("{a} {t}"))));
    for u in urls {
        let (code, body) = http(&u)?;
        if code == 200 {
            if let Some(s) = pick(&body, d) {
                return Ok(Some(s));
            }
        }
    }
    // 유튜브는 가수 칸에 채널 이름이 오는 일이 많다. 곡명만으로 찾되, 길이가 3초 안으로 맞는 것만 믿는다.
    if dur_ms > 0 {
        let (code, body) = http(&format!("{base}/search?track_name={}", enc(&t)))?;
        if code == 200 {
            return Ok(pick(&only_near(&body, d, 3.0), d));
        }
    }
    Ok(None)
}

pub fn fetch(lyr: Arc<Mutex<Lyr>>, key: String, title: String, artist: String, dur_ms: i64) {
    {
        let mut l = lyr.lock().unwrap();
        l.key = key.clone();
        l.lines.clear();
        l.st = St::Loading;
        l.ver += 1;
    }
    std::thread::spawn(move || {
        let name: String = key.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { '_' }).take(120).collect();
        let path = crate::data_dir().join("lyrics").join(format!("{name}.lrc"));
        // 알송 곡은 알송이 그 파일에 맞춰 받은 가사가 제일 정확하다(LRCLIB 은 다른 판본일 수 있다).
        let als = key.strip_prefix("ALSong|").and_then(crate::alsong::lyc);
        let (text, st) = if let Some(t) = als {
            (Some(t), St::Found)
        } else { match std::fs::read_to_string(&path) {
            Ok(t) => (Some(t), St::Found),
            Err(_) => match download(&title, &artist, dur_ms) {
                Ok(Some(t)) => {
                    let _ = std::fs::create_dir_all(path.parent().unwrap());
                    let _ = std::fs::write(&path, &t);
                    (Some(t), St::Found)
                }
                Ok(None) => (None, St::Missing),
                Err(e) => {
                    crate::log(&format!("WARN 가사 받기 실패({title}): {e}"));
                    (None, St::Error)
                }
            },
        } };
        let mut l = lyr.lock().unwrap();
        if l.key != key {
            return; // 그 사이 곡이 바뀌었다
        }
        l.lines = text.map(|t| lrc::parse(&t)).unwrap_or_default();
        l.st = if st == St::Found && l.lines.is_empty() { St::Missing } else { st };
        if st != St::Error {
            l.stamp = crate::hhmm();
        }
        l.ver += 1;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_closest_duration_with_synced() {
        let j = r#"[{"duration":200,"syncedLyrics":null},{"duration":230,"syncedLyrics":"[00:01.00]far"},{"duration":212,"syncedLyrics":"[00:01.00]near"}]"#;
        assert_eq!(pick(j, 210.0).unwrap(), "[00:01.00]near");
        assert_eq!(pick(j, 0.0).unwrap(), "[00:01.00]far");
        assert_eq!(pick(r#"{"syncedLyrics":"[00:02.00]x"}"#, 1.0).unwrap(), "[00:02.00]x");
        assert!(pick(r#"{"syncedLyrics":null}"#, 1.0).is_none());
        assert!(pick(&only_near(j, 205.0, 3.0), 205.0).is_none());
        assert_eq!(pick(&only_near(j, 211.0, 3.0), 211.0).unwrap(), "[00:01.00]near");
    }

    #[test]
    fn encodes_korean_and_spaces() {
        assert_eq!(enc("a b&가"), "a%20b%26%EA%B0%80");
    }
}
