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
    pub group: usize, // 설정의 --- 구분선으로 나뉜 묶음 번호
}

#[derive(Default)]
pub struct Quotes {
    pub list: Vec<Quote>,
    pub stamp: String,         // 마지막으로 받은 시각 "hh:mm"
    pub seen: Option<Instant>, // 〃
    pub reload: bool,          // 설정을 방금 고쳤다 → 기다리지 말고 바로 다시 받는다
    pub tried: String,         // 마지막으로 받아 보려 한 시각(하나도 못 받았을 때 '받는 중'에 멈춰 있지 않게)
}

#[derive(PartialEq, Debug)]
pub enum Src {
    Stock, // 네이버 종목
    Index, // 네이버 지수
    Coin, // 원화 코인: 빗썸 값(막힌 망에선 코인게코가 중계하는 빗썸 값). 둘 다 안 되면 야후
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
    Some((if code { Src::Stock } else if sym.ends_with("-KRW") { Src::Coin } else { Src::Yahoo }, sym.to_string()))
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

/// 받은 한 종목: (심볼, 값 글자, 등락률, 받은 곳이 알려 준 이름).
pub type Got = (String, String, f64, String);

/// 네이버 응답.
pub fn parse_naver(body: &str, dec: usize) -> Vec<Got> {
    let v: Value = serde_json::from_str(body).unwrap_or_default();
    let mut out = Vec::new();
    for d in v["datas"].as_array().into_iter().flatten() {
        if let (Some(code), Some(p), Some(pct)) = (d["itemCode"].as_str(), num(&d["closePrice"]), num(&d["fluctuationsRatio"])) {
            out.push((code.to_string(), commas(p, dec), pct, d["stockName"].as_str().unwrap_or(code).to_string()));
        }
    }
    out
}

/// 야후 응답 → (심볼, 값 글자, 등락률). 장 전후 값(fullday…)이 있으면 그걸 쓴다(위젯과 같다).
pub fn parse_yahoo(body: &str) -> Vec<Got> {
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
        let name = m["shortName"].as_str().unwrap_or(sym).to_string();
        out.push((sym.to_string(), commas(price, if won && price >= 100.0 { 0 } else { 2 }), pct, name));
    }
    out
}

/// 코인게코는 심볼이 아니라 자기네 이름으로 묻는다(위젯과 같은 표).
const GECKO_ID: [(&str, &str); 6] = [("BTC", "bitcoin"), ("ETH", "ethereum"), ("XRP", "ripple"), ("KAIA", "kaia"), ("SOL", "solana"), ("DOGE", "dogecoin")];

fn coin_text(p: f64) -> String {
    commas(p, if p >= 100.0 { 0 } else { 2 })
}

/// 빗썸 응답(한 코인). 등락률은 빗썸 화면 '변동(당일)'과 같은 전일 종가 기준.
pub fn parse_bithumb(body: &str, sym: &str) -> Option<Got> {
    let v: Value = serde_json::from_str(body).ok()?;
    let (p, prev) = (num(&v["data"]["closing_price"])?, num(&v["data"]["prev_closing_price"])?);
    Some((sym.to_string(), coin_text(p), if prev != 0.0 { (p - prev) / prev * 100.0 } else { 0.0 }, sym.trim_end_matches("-KRW").to_string()))
}

/// 코인게코가 중계하는 빗썸 원화 값 → (코인, 값).
pub fn parse_gecko_bithumb(body: &str) -> Vec<(String, f64)> {
    let v: Value = serde_json::from_str(body).unwrap_or_default();
    v["tickers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|t| t["target"].as_str() == Some("KRW") && t["is_stale"].as_bool() != Some(true))
        .filter_map(|t| Some((t["base"].as_str()?.to_string(), t["last"].as_f64()?)))
        .collect()
}

/// 오늘 한국시간 자정의 유닉스 시각.
fn kst_midnight() -> u64 {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    (now + 9 * 3600) / 86400 * 86400 - 9 * 3600
}

/// 거래소가 막힌 망에서 쓰는 우회 길. 값은 코인게코가 중계하는 빗썸 값, 등락률은 해외 시세의 '한국시간 자정 → 지금'
/// (둘 다 코인게코 안에서 낸다 — 다른 출처와 섞어 나누면 출처 사이 시세 차이가 등락률에 실린다).
#[derive(Default)]
struct Gecko {
    at: Option<Instant>,
    vals: Vec<Got>,
    mid: std::collections::HashMap<String, (u64, f64)>, // 코인 → (그날 자정, 그때 해외 시세)
}

impl Gecko {
    fn refresh(&mut self, c: &HttpClient, coins: &[String]) {
        // 무료 등급이라 1분에 한 번만 묻는다
        if self.at.is_some_and(|t| t.elapsed() < Duration::from_secs(60)) {
            return;
        }
        self.at = Some(Instant::now());
        let ids: Vec<(&str, &str)> = coins.iter().filter_map(|k| GECKO_ID.iter().find(|g| g.0 == k.trim_end_matches("-KRW")).copied()).collect();
        if ids.is_empty() {
            return;
        }
        let list = ids.iter().map(|g| g.1).collect::<Vec<_>>().join(",");
        let Ok(tick) = get(c, &format!("https://api.coingecko.com/api/v3/exchanges/bithumb/tickers?coin_ids={list}")) else { return };
        let Ok(now) = get(c, &format!("https://api.coingecko.com/api/v3/simple/price?vs_currencies=krw&ids={list}")) else { return };
        let now: Value = serde_json::from_str(&now).unwrap_or_default();
        let (prices, day) = (parse_gecko_bithumb(&tick), kst_midnight());
        let mut vals = Vec::new();
        for (base, id) in ids {
            if self.mid.get(base).map(|m| m.0) != Some(day) {
                let url = format!("https://api.coingecko.com/api/v3/coins/{id}/market_chart/range?vs_currency=krw&from={day}&to={}", day + 3600);
                let v: Value = get(c, &url).ok().and_then(|b| serde_json::from_str(&b).ok()).unwrap_or_default();
                if let Some(p) = v["prices"][0][1].as_f64() {
                    self.mid.insert(base.to_string(), (day, p));
                }
            }
            if let (Some(p), Some(n), Some(m)) = (prices.iter().find(|x| x.0 == base), now[id]["krw"].as_f64(), self.mid.get(base).filter(|m| m.0 == day)) {
                vals.push((format!("{base}-KRW"), coin_text(p.1), (n / m.1 - 1.0) * 100.0, base.to_string()));
            }
        }
        if !vals.is_empty() {
            self.vals = vals;
        }
    }
}

pub fn get(c: &HttpClient, url: &str) -> Result<String> {
    Ok(c.GetStringAsync(&Uri::CreateUri(&HSTRING::from(url))?)?.get()?.to_string())
}

/// 종목 목록 파일. 시세 위젯과 같은 파일이다. 시험할 땐 NOTCH_CAT_TICKER_CONFIG 로 임시 파일을 가리킨다(실제 목록을 건드리지 않게).
pub fn config_path() -> std::path::PathBuf {
    match std::env::var("NOTCH_CAT_TICKER_CONFIG") {
        Ok(p) if !p.is_empty() => p.into(),
        _ => {
            // 내 PC 두 대는 시세 위젯과 같은 공용 목록. 그게 없는 PC(전산실 등)는 노치 고양이 자기 목록.
            let shared = std::path::PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default()).join("claude-memory");
            if shared.is_dir() { shared.join("ticker/ticker_config.json") } else { crate::data_dir().join("ticker_config.json") }
        }
    }
}

/// 목록이 없을 때 쓰는 기본 종목(시세 위젯 DEFAULT 와 같다).
pub const DEFAULT: [&str; 9] = ["KOSPI=코스피", "069500=코덱스200", "USDKRW=X=환율", "---", "BTC-KRW=비트코인", "XRP-KRW=XRP", "---", "AAPL", "NVDA"];

/// 목록의 줄들(적힌 그대로).
pub fn read_lines(path: &std::path::Path) -> Vec<String> {
    let v: Value = std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    v["tickers"].as_array().into_iter().flatten().filter_map(|t| t.as_str()).map(str::to_string).collect()
}

/// 설정창 글자 → 줄들. 위젯과 같은 규칙: 앞뒤 빈칸을 떼고 빈 줄은 버린다. 줄 안쪽(@목표·비율 줄)은 손대지 않는다.
pub fn text_lines(text: &str) -> Vec<String> {
    text.lines().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect()
}

/// 목록만 바꿔 저장한다. 파일에 있던 다른 값은 그대로 둔다. 위젯이 쓰는 모양(들여쓰기 2칸·한글 그대로)과 같다.
pub fn save_lines(path: &std::path::Path, lines: &[String]) -> std::result::Result<(), String> {
    if !lines.iter().any(|l| !l.starts_with("---")) {
        return Err("종목을 하나 이상 입력하세요".into());
    }
    let mut v = match std::fs::read_to_string(path) {
        // 읽히지 않는 파일을 덮어쓰면 그 안의 것을 잃는다 → 멈춘다
        Ok(s) => serde_json::from_str::<Value>(&s).ok().filter(Value::is_object).ok_or("설정 파일을 읽지 못해 저장하지 않았습니다")?,
        Err(_) => serde_json::json!({}),
    };
    v["tickers"] = lines.iter().map(|l| Value::from(l.as_str())).collect();
    let body = serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?;
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    // 옆에 다 쓴 뒤 바꿔치기: 쓰다 끊겨도 원래 목록이 남는다
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, body).and_then(|_| std::fs::rename(&tmp, path)).map_err(|e| format!("저장 실패: {e}"))
}

/// 설정창 지수 버튼. 시세 위젯(tools/ticker.pyw)·폰(ticker.html)의 IDX_BTNS 와 같은 값.
pub const IDX_BTNS: [&[(&str, &str)]; 2] = [
    &[("코스피", "KOSPI=코스피"), ("코스닥", "KOSDAQ=코스닥")],
    &[("S&P500", "^GSPC=S&P500"), ("나스닥", "^IXIC=나스닥"), ("다우", "^DJI=다우"), ("반도체", "^SOX=필라델피아반도체")],
];

pub fn has_idx(lines: &[String], row: &str) -> bool {
    lines.iter().any(|l| split(l).0 == split(row).0)
}

/// 지수 버튼: 없으면 목록 맨 위에 넣고, 있으면 뺀다(위젯과 같다).
pub fn toggle_idx(lines: &mut Vec<String>, row: &str) {
    if has_idx(lines, row) {
        lines.retain(|l| split(l).0 != split(row).0);
    } else {
        lines.insert(0, row.to_string());
    }
}

/// (심볼, 이름, 묶음 번호). 구분선(---)을 만날 때마다 묶음이 바뀐다.
fn config() -> Vec<(String, String, usize)> {
    let mut group = 0;
    let mut out = Vec::new();
    let mut lines = read_lines(&config_path());
    if lines.is_empty() {
        lines = DEFAULT.iter().map(|s| s.to_string()).collect();
    }
    for line in lines {
        if line.starts_with("---") {
            group += 1;
        } else {
            let (sym, name) = split(&line);
            out.push((sym, name, group));
        }
    }
    out
}

fn fetch(c: &HttpClient, gecko: &mut Gecko, bithumb_off: &mut Option<Instant>) -> Vec<Quote> {
    // 설정에 이름이 없으면(심볼만 적혀 있으면) 받은 곳의 이름을 쓴다
    let want: Vec<(Src, String, Option<String>, usize)> = config().into_iter().filter_map(|(sym, name, grp)| source(&sym).map(|(s, key)| (s, key, (name != sym).then_some(name), grp))).collect();
    let keys = |src: Src| want.iter().filter(|w| w.0 == src).map(|w| w.1.clone()).collect::<Vec<_>>();
    let mut got: Vec<Got> = Vec::new();
    for (src, kind, dec) in [(Src::Stock, "stock", 0), (Src::Index, "index", 2)] {
        let k = keys(src);
        if !k.is_empty() {
            if let Ok(b) = get(c, &format!("https://polling.finance.naver.com/api/realtime/domestic/{kind}/{}", k.join(","))) {
                got.extend(parse_naver(&b, dec));
            }
        }
    }
    // 코인은 국내 값이 해외 값과 달라서 빗썸에서 받는다. 빗썸이 막힌 망(학교)에선 10분 쉬었다 다시 두드리고, 그동안은 코인게코로.
    let mut k = keys(Src::Yahoo);
    let mut rest = Vec::new();
    for coin in keys(Src::Coin) {
        let open = bithumb_off.map_or(true, |t| t.elapsed() > Duration::from_secs(600));
        let one = if open { get(c, &format!("https://api.bithumb.com/public/ticker/{}_KRW", coin.trim_end_matches("-KRW"))).ok().and_then(|b| parse_bithumb(&b, &coin)) } else { None };
        match one {
            Some(g) => {
                *bithumb_off = None;
                got.push(g);
            }
            None => {
                if open {
                    *bithumb_off = Some(Instant::now());
                }
                rest.push(coin);
            }
        }
    }
    if !rest.is_empty() {
        gecko.refresh(c, &rest);
        for coin in rest {
            match gecko.vals.iter().find(|g| g.0 == coin) {
                Some(g) => got.push(g.clone()),
                None => k.push(coin), // 마지막 길: 야후(해외 값)
            }
        }
    }
    if !k.is_empty() {
        let q = k.join(",").replace('^', "%5E").replace('=', "%3D");
        if let Ok(b) = get(c, &format!("https://query1.finance.yahoo.com/v7/finance/spark?range=1d&interval=1d&symbols={q}")) {
            got.extend(parse_yahoo(&b));
        }
    }
    // 설정에 적힌 순서대로. 못 받은 종목은 빠진다.
    want.iter().filter_map(|(_, key, name, grp)| got.iter().find(|g| &g.0 == key).map(|g| Quote { name: short(name.as_ref().unwrap_or(&g.3)), price: g.1.clone(), pct: g.2, group: *grp })).collect()
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
        let (mut gecko, mut bithumb_off) = (Gecko::default(), None);
        loop {
            let list = fetch(&c, &mut gecko, &mut bithumb_off);
            out.lock().unwrap().tried = crate::hhmm();
            if !list.is_empty() {
                let mut o = out.lock().unwrap();
                o.list = list;
                o.stamp = crate::hhmm();
                o.seen = Some(Instant::now());
            }
            // 15초 쉰다. 설정을 고치면 바로 깬다.
            for _ in 0..30 {
                std::thread::sleep(Duration::from_millis(500));
                if std::mem::take(&mut out.lock().unwrap().reload) {
                    break;
                }
            }
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
    fn saving_keeps_every_line_and_other_keys() {
        let p = std::env::temp_dir().join(format!("notch-cat-test-{}.json", std::process::id()));
        std::fs::write(&p, r#"{"tickers": ["069500=코덱스200"], "refresh_sec": 5, "coin_prem": {"XRP": 1.01}}"#).unwrap();
        let lines = text_lines("^KS11=코스피
  069500=코덱스200 

USDKRW=X=환율
---
XRP-KRW/KAIA-KRW=XRP당 카이아 @49,50,v41
---
0005A0
");
        save_lines(&p, &lines).unwrap();
        assert_eq!(read_lines(&p), lines);
        assert_eq!(lines[4], "XRP-KRW/KAIA-KRW=XRP당 카이아 @49,50,v41");
        let body = std::fs::read_to_string(&p).unwrap();
        let v: Value = serde_json::from_str(&body).unwrap();
        assert_eq!((v["refresh_sec"].as_i64(), v["coin_prem"]["XRP"].as_f64()), (Some(5), Some(1.01)));
        assert!(body.contains("코덱스200") && body.contains("
  \"tickers\": [
    \""), "한글 그대로·들여쓰기 2칸: {body}");
        // 종목이 없으면 저장하지 않고, 깨진 파일은 덮어쓰지 않는다
        assert!(save_lines(&p, &["---".to_string()]).is_err());
        std::fs::write(&p, "{깨진").unwrap();
        assert!(save_lines(&p, &lines).is_err());
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "{깨진");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn index_buttons_match_the_widget() {
        let mut l = text_lines("069500
^GSPC=S&P500");
        assert!(has_idx(&l, "^GSPC=S&P500") && !has_idx(&l, "KOSPI=코스피"));
        toggle_idx(&mut l, "KOSPI=코스피");
        assert_eq!(l, ["KOSPI=코스피", "069500", "^GSPC=S&P500"]);
        toggle_idx(&mut l, "^GSPC=S&P500");
        assert_eq!(l, ["KOSPI=코스피", "069500"]);
        // 위젯 코드에 같은 줄이 그대로 있어야 한다(한쪽만 바뀌면 여기서 걸린다)
        if let Ok(py) = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../tools/ticker.pyw")) {
            for (label, row) in IDX_BTNS.iter().flat_map(|g| g.iter()) {
                assert!(py.contains(&format!("(\"{label}\", [\"{row}\"])")), "위젯에 없는 지수 버튼: {label} {row}");
            }
        }
    }

    #[test]
    fn picks_source() {
        assert_eq!(source("069500"), Some((Src::Stock, "069500".into())));
        assert_eq!(source("0005A0"), Some((Src::Stock, "0005A0".into())));
        assert_eq!(source("^KS11"), Some((Src::Index, "KOSPI".into())));
        assert_eq!(source("JEPQ"), Some((Src::Yahoo, "JEPQ".into())));
        assert_eq!(source("XRP-KRW"), Some((Src::Coin, "XRP-KRW".into())));
        let b = parse_bithumb(r#"{"status":"0000","data":{"closing_price":"1431","prev_closing_price":"1444"}}"#, "XRP-KRW").unwrap();
        assert_eq!((b.0.as_str(), b.1.as_str()), ("XRP-KRW", "1,431"));
        assert!((b.2 + 0.9003).abs() < 0.001);
        let g = parse_gecko_bithumb(r#"{"tickers":[{"base":"KAIA","target":"KRW","last":43.77,"is_stale":false},{"base":"KAIA","target":"BTC","last":0.1}]}"#);
        assert_eq!(g, vec![("KAIA".to_string(), 43.77)]);
    }

    #[test]
    fn parses_naver_and_yahoo() {
        let n = r#"{"datas":[{"itemCode":"069500","closePrice":"81,950","compareToPreviousClosePrice":"-50","fluctuationsRatio":"-0.06","stockName":"KODEX 200"}]}"#;
        assert_eq!(parse_naver(n, 0), vec![("069500".to_string(), "81,950".to_string(), -0.06, "KODEX 200".to_string())]);
        let y = r#"{"spark":{"result":[
            {"symbol":"^GSPC","response":[{"meta":{"currency":"USD","regularMarketPrice":7500.0,"fulldayPrice":7520.93,"fulldayChangePercent":-0.384,"chartPreviousClose":7549.94}}]},
            {"symbol":"BTC-KRW","response":[{"meta":{"currency":"KRW","regularMarketPrice":150000000.0,"chartPreviousClose":148500000.0}}]}]}}"#;
        let got = parse_yahoo(y);
        assert_eq!(got[0], ("^GSPC".to_string(), "7,520.93".to_string(), -0.384, "^GSPC".to_string()));
        assert_eq!(got[1].1, "150,000,000");
        assert!((got[1].2 - 1.0101).abs() < 0.001);
    }
}


#[cfg(test)]
mod live {
    use super::*;

    // 손으로 돌리는 확인용(통신이 필요해서 평소 시험에선 뺀다): cargo test live -- --ignored --nocapture
    #[test]
    #[ignore]
    fn fetch_prints_what_the_notch_shows() {
        let c = HttpClient::new().unwrap();
        let h = c.DefaultRequestHeaders().unwrap();
        h.UserAgent().unwrap().TryParseAdd(&HSTRING::from("Mozilla/5.0")).unwrap();
        h.Append(&HSTRING::from("Referer"), &HSTRING::from("https://finance.naver.com/")).unwrap();
        let (mut g, mut off) = (Gecko::default(), None);
        for q in fetch(&c, &mut g, &mut off) {
            println!("{:<24} {:>14} {:+.2}%", q.name, q.price, q.pct);
        }
        println!("bithumb blocked: {}", off.is_some());
        // 1분 뒤 두 번째 물음을 흉내 낸다(첫 번에 못 받은 자정 값을 채운다)
        std::thread::sleep(Duration::from_secs(5));
        g.at = None;
        for q in fetch(&c, &mut g, &mut off).iter().skip(5).take(3) {
            println!("2nd {:<20} {:>14} {:+.2}%", q.name, q.price, q.pct);
        }
    }
}

#[cfg(test)]
mod no_shared {
    use super::*;

    // 공용 목록(claude-memory)이 없는 PC: 기본 목록으로 실제로 받아지는지(통신 필요): cargo test no_shared -- --ignored --nocapture
    #[test]
    #[ignore]
    fn fresh_pc_gets_default_quotes() {
        let home = std::env::temp_dir().join(format!("notch-cat-fresh-{}", std::process::id()));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("USERPROFILE", &home);
        assert!(!config_path().starts_with(&home), "공용 목록 자리가 아니라 노치 고양이 폴더여야 한다: {:?}", config_path());
        assert_eq!(config().len(), 7);
        let c = HttpClient::new().unwrap();
        let h = c.DefaultRequestHeaders().unwrap();
        h.UserAgent().unwrap().TryParseAdd(&HSTRING::from("Mozilla/5.0")).unwrap();
        h.Append(&HSTRING::from("Referer"), &HSTRING::from("https://finance.naver.com/")).unwrap();
        let got = fetch(&c, &mut Gecko::default(), &mut None);
        for q in &got {
            println!("{} {:<12} {:>14} {:+.2}%", q.group, q.name, q.price, q.pct);
        }
        assert!(got.len() >= 5, "기본 목록 7개 중 {}개만", got.len());
        let _ = std::fs::remove_dir(&home);
    }
}
