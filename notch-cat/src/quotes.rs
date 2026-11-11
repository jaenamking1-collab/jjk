// 시세: 기존 시세 위젯(tools/ticker.pyw)의 종목 목록을 읽기만 하고, 같은 곳에서 받는다
// (국내 6자리·코스피/코스닥 = 네이버, 나머지 = 야후). 총평가 금액은 다루지 않는다.
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;
use windows::core::{Result, HSTRING};
use windows::Foundation::Uri;
use windows::Web::Http::HttpClient;

#[derive(Clone, Default, PartialEq, Debug)]
pub struct Quote {
    pub name: String,
    pub price: String,
    pub pct: f64,
}

#[derive(Default)]
pub struct Quotes {
    pub list: Vec<Quote>,
    pub stamp: String,         // 마지막으로 받은 시각 "hh:mm"
    pub seen: Option<Instant>, // 〃
}

#[derive(PartialEq, Debug)]
pub enum Src {
    Stock, // 네이버 종목
    Index, // 네이버 지수
    Yahoo,
}

/// "069500=KODEX 200@목표" → (심볼, 이름). "USDKRW=X" 처럼 심볼에 = 가 있어도 안전.
pub fn split(line: &str) -> (String, String) {
    let line = line.split('@').next().unwrap_or("").trim_end();
    match line.rsplit_once('=') {
        Some((sym, name)) if name.chars().count() > 1 => (sym.to_string(), name.to_string()),
        _ => (line.to_string(), line.to_string()),
    }
}

/// 어디서 받을지. 구분선(---)과 교환비율 줄(A/B)은 None.
pub fn source(sym: &str) -> Option<(Src, String)> {
    if sym.is_empty() || sym.starts_with("---") || sym.contains('/') {
        return None;
    }
    match sym {
        "KOSPI" | "^KS11" => return Some((Src::Index, "KOSPI".into())),
        "KOSDAQ" | "^KQ11" => return Some((Src::Index, "KOSDAQ".into())),
        _ => {}
    }
    let b = sym.as_bytes();
    let code = b.len() == 6 && b[..4].iter().all(u8::is_ascii_digit) && b[4..].iter().all(|c| c.is_ascii_digit() || c.is_ascii_uppercase());
    Some((if code { Src::Stock } else { Src::Yahoo }, sym.to_string()))
}

/// 국내 ETF 브랜드는 약칭으로(위젯과 같은 표).
pub fn short(name: &str) -> String {
    const BRAND: [(&str, &str); 22] = [
        ("KODEX", "K"), ("TIGER", "T"), ("RISE", "R"), ("KBSTAR", "R"), ("PLUS", "P"), ("ARIRANG", "P"), ("SOL", "S"), ("ACE", "A"),
        ("KINDEX", "A"), ("HANARO", "H"), ("KOSEF", "KF"), ("TIMEFOLIO", "TF"), ("WOORI", "W"), ("FOCUS", "F"), ("UNICORN", "U"),
        ("TREX", "TX"), ("BNK", "B"), ("히어로즈", "HR"), ("마이티", "MT"), ("마이다스", "MD"), ("파워", "PW"), ("에셋플러스", "EP"),
    ];
    for (full, s) in BRAND {
        if let Some(rest) = name.strip_prefix(full).and_then(|r| r.strip_prefix(' ')) {
            return format!("{s} {rest}");
        }
    }
    name.to_string()
}

fn commas(v: f64, dec: usize) -> String {
    let s = format!("{:.*}", dec, v.abs());
    let (int, frac) = s.split_once('.').unwrap_or((&s, ""));
    let mut out = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if !frac.is_empty() {
        out.push('.');
        out.push_str(frac);
    }
    if v < 0.0 { format!("-{out}") } else { out }
}

fn num(v: &Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_str()?.replace(',', "").parse().ok())
}

/// 네이버 응답 → (종목코드, 값 글자, 등락률).
pub fn parse_naver(body: &str, dec: usize) -> Vec<(String, String, f64)> {
    let v: Value = serde_json::from_str(body).unwrap_or_default();
    let mut out = Vec::new();
    for d in v["datas"].as_array().into_iter().flatten() {
        if let (Some(code), Some(p), Some(pct)) = (d["itemCode"].as_str(), num(&d["closePrice"]), num(&d["fluctuationsRatio"])) {
            out.push((code.to_string(), commas(p, dec), pct));
        }
    }
    out
}

/// 야후 응답 → (심볼, 값 글자, 등락률). 장 전후 값(fullday…)이 있으면 그걸 쓴다(위젯과 같다).
pub fn parse_yahoo(body: &str) -> Vec<(String, String, f64)> {
    let v: Value = serde_json::from_str(body).unwrap_or_default();
    let mut out = Vec::new();
    for r in v["spark"]["result"].as_array().into_iter().flatten() {
        let m = &r["response"][0]["meta"];
        let Some(sym) = r["symbol"].as_str() else { continue };
        let (price, pct) = match (m["fulldayPrice"].as_f64(), m["fulldayChangePercent"].as_f64()) {
            (Some(p), Some(c)) if p != 0.0 => (p, c),
            _ => {
                let Some(p) = m["regularMarketPrice"].as_f64() else { continue };
                let prev = m["chartPreviousClose"].as_f64().filter(|x| *x != 0.0).unwrap_or(p);
                (p, (p - prev) / prev * 100.0)
            }
        };
        // 원화로 표시되는 자산은 정수, 환율·달러 종목은 소수 둘째 자리까지
        let won = m["currency"].as_str() == Some("KRW") && !sym.ends_with("=X");
        out.push((sym.to_string(), commas(price, if won && price >= 100.0 { 0 } else { 2 }), pct));
    }
    out
}

fn get(c: &HttpClient, url: &str) -> Result<String> {
    Ok(c.GetStringAsync(&Uri::CreateUri(&HSTRING::from(url))?)?.get()?.to_string())
}

fn config() -> Vec<(String, String)> {
    let path = std::path::PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default()).join("claude-memory/ticker/ticker_config.json");
    let v: Value = std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    v["tickers"].as_array().into_iter().flatten().filter_map(|t| t.as_str()).map(split).collect()
}

fn fetch(c: &HttpClient) -> Vec<Quote> {
    let want: Vec<(Src, String, String)> = config().into_iter().filter_map(|(sym, name)| source(&sym).map(|(s, key)| (s, key, short(&name)))).collect();
    let keys = |src: Src| want.iter().filter(|w| w.0 == src).map(|w| w.1.clone()).collect::<Vec<_>>();
    let mut got: Vec<(String, String, f64)> = Vec::new();
    for (src, kind, dec) in [(Src::Stock, "stock", 0), (Src::Index, "index", 2)] {
        let k = keys(src);
        if !k.is_empty() {
            if let Ok(b) = get(c, &format!("https://polling.finance.naver.com/api/realtime/domestic/{kind}/{}", k.join(","))) {
                got.extend(parse_naver(&b, dec));
            }
        }
    }
    let k = keys(Src::Yahoo);
    if !k.is_empty() {
        let q = k.join(",").replace('^', "%5E").replace('=', "%3D");
        if let Ok(b) = get(c, &format!("https://query1.finance.yahoo.com/v7/finance/spark?range=1d&interval=1d&symbols={q}")) {
            got.extend(parse_yahoo(&b));
        }
    }
    // 설정에 적힌 순서대로. 못 받은 종목은 빠진다.
    want.iter().filter_map(|(_, key, name)| got.iter().find(|g| &g.0 == key).map(|g| Quote { name: name.clone(), price: g.1.clone(), pct: g.2 })).collect()
}

pub fn spawn(out: Arc<Mutex<Quotes>>) {
    std::thread::spawn(move || {
        let client = || -> Result<HttpClient> {
            let c = HttpClient::new()?;
            let h = c.DefaultRequestHeaders()?;
            h.UserAgent()?.TryParseAdd(&HSTRING::from("Mozilla/5.0"))?;
            h.Append(&HSTRING::from("Referer"), &HSTRING::from("https://finance.naver.com/"))?;
            Ok(c)
        };
        let Ok(c) = client() else { return };
        loop {
            let list = fetch(&c);
            if !list.is_empty() {
                let mut o = out.lock().unwrap();
                o.list = list;
                o.stamp = crate::hhmm();
                o.seen = Some(Instant::now());
            }
            std::thread::sleep(Duration::from_secs(15));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_lines_like_the_widget() {
        assert_eq!(split("069500=KODEX 200"), ("069500".into(), "KODEX 200".into()));
        assert_eq!(split("USDKRW=X=달러"), ("USDKRW=X".into(), "달러".into()));
        assert_eq!(split("USDKRW=X"), ("USDKRW=X".into(), "USDKRW=X".into()));
        assert_eq!(split("XRP-KRW/KAIA-KRW=비율@49,50,v41").0, "XRP-KRW/KAIA-KRW");
    }

    #[test]
    fn picks_source() {
        assert_eq!(source("069500"), Some((Src::Stock, "069500".into())));
        assert_eq!(source("0005A0"), Some((Src::Stock, "0005A0".into())));
        assert_eq!(source("^KS11"), Some((Src::Index, "KOSPI".into())));
        assert_eq!(source("JEPQ"), Some((Src::Yahoo, "JEPQ".into())));
        assert_eq!(source("---"), None);
        assert_eq!(source("XRP-KRW/KAIA-KRW"), None);
        assert_eq!(short("KODEX 200"), "K 200");
    }

    #[test]
    fn parses_naver_and_yahoo() {
        let n = r#"{"datas":[{"itemCode":"069500","closePrice":"81,950","compareToPreviousClosePrice":"-50","fluctuationsRatio":"-0.06"}]}"#;
        assert_eq!(parse_naver(n, 0), vec![("069500".to_string(), "81,950".to_string(), -0.06)]);
        let y = r#"{"spark":{"result":[
            {"symbol":"^GSPC","response":[{"meta":{"currency":"USD","regularMarketPrice":7500.0,"fulldayPrice":7520.93,"fulldayChangePercent":-0.384,"chartPreviousClose":7549.94}}]},
            {"symbol":"BTC-KRW","response":[{"meta":{"currency":"KRW","regularMarketPrice":150000000.0,"chartPreviousClose":148500000.0}}]}]}}"#;
        let got = parse_yahoo(y);
        assert_eq!(got[0], ("^GSPC".to_string(), "7,520.93".to_string(), -0.384));
        assert_eq!(got[1].1, "150,000,000");
        assert!((got[1].2 - 1.0101).abs() < 0.001);
    }
}
