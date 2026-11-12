// 날씨: 노르웨이 기상청 공개 예보(api.met.no, 열쇠 없음)에서 지금 하늘 상태와 기온을 10분마다 받는다.
// (open-meteo 는 학교 망에서 윈도우 통신이 인증서 오류로 막혀 쓰지 못했다 — 2026-11-11)
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;
use windows::core::{Result, HSTRING};
use windows::Web::Http::HttpClient;

#[derive(Clone, Default)]
pub struct Weather {
    pub kind: usize, // NAMES 의 번호
    pub temp: f64,
    pub day: bool,
    pub stamp: String,         // 마지막으로 받은 시각 "hh:mm"
    pub seen: Option<Instant>, // 〃
}

pub const NAMES: [&str; 7] = ["맑음", "구름 조금", "흐림", "안개", "비", "눈", "천둥번개"];

/// 기상청 그림 이름("partlycloudy_night", "heavyrainandthunder" …) → NAMES 번호.
pub fn kind_of(symbol: &str) -> usize {
    let has = |w: &str| symbol.contains(w);
    if has("thunder") {
        6
    } else if has("snow") || has("sleet") {
        5
    } else if has("rain") {
        4
    } else if has("fog") {
        3
    } else if has("partlycloudy") || has("fair") {
        1
    } else if has("cloudy") {
        2
    } else {
        0
    }
}

/// 응답 → (종류, 기온, 낮인가).
pub fn parse(body: &str) -> Option<(usize, f64, bool)> {
    let v: Value = serde_json::from_str(body).ok()?;
    let d = &v["properties"]["timeseries"][0]["data"];
    let sym = d["next_1_hours"]["summary"]["symbol_code"].as_str()?;
    Some((kind_of(sym), d["instant"]["details"]["air_temperature"].as_f64()?, !sym.ends_with("_night")))
}

fn client() -> Result<HttpClient> {
    let c = HttpClient::new()?;
    c.DefaultRequestHeaders()?.UserAgent()?.TryParseAdd(&HSTRING::from("NotchCat/0.1"))?; // 기상청이 프로그램 이름을 밝히라고 한다
    Ok(c)
}

fn url(lat: f64, lon: f64) -> String {
    format!("https://api.met.no/weatherapi/locationforecast/2.0/compact?lat={lat:.2}&lon={lon:.2}")
}

pub fn spawn(out: Arc<Mutex<Weather>>, lat: f64, lon: f64) {
    std::thread::spawn(move || {
        let Ok(c) = client() else { return };
        loop {
            let got = crate::quotes::get(&c, &url(lat, lon)).ok().and_then(|b| parse(&b));
            if let Some((kind, temp, day)) = got {
                *out.lock().unwrap() = Weather { kind, temp, day, stamp: crate::hhmm(), seen: Some(Instant::now()) };
            }
            // 못 받았으면 1분 뒤 다시, 받았으면 10분 뒤
            std::thread::sleep(Duration::from_secs(if got.is_some() { 600 } else { 60 }));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_sky_and_temperature() {
        let b = r#"{"properties":{"timeseries":[{"time":"2026-11-11T05:00:00Z","data":{"instant":{"details":{"air_temperature":7.4}},"next_1_hours":{"summary":{"symbol_code":"heavyrainandthunder"}}}}]}}"#;
        assert_eq!(parse(b), Some((6, 7.4, true)));
        let k = ["clearsky_night", "fair_day", "partlycloudy_day", "cloudy", "fog", "lightrainshowers_day", "rain", "heavysnow", "sleet", "snowandthunder"].map(kind_of);
        assert_eq!(k, [0, 1, 1, 2, 3, 4, 4, 5, 5, 6]);
        assert_eq!(parse("{}"), None);
        assert!(!parse(&b.replace("heavyrainandthunder", "clearsky_night")).unwrap().2);
    }
}

#[cfg(test)]
mod live {
    // 손으로 돌리는 확인용(통신 필요): cargo test live -- --ignored --nocapture
    #[test]
    #[ignore]
    fn weather_fetch_prints() {
        let r = crate::quotes::get(&super::client().unwrap(), &super::url(37.57, 126.98));
        println!("weather: {:?}", r.as_ref().map(|b| super::parse(b)));
    }

    // 여러 주소를 윈도우 통신으로 시험(WX_URLS 공백 구분)
    #[test]
    #[ignore]
    fn probe_urls() {
        let c = super::client().unwrap();
        for u in std::env::var("WX_URLS").unwrap_or_default().split(' ').filter(|u| !u.is_empty()) {
            match crate::quotes::get(&c, u) {
                Ok(b) => println!("OK  {u}\n    {}", b.chars().take(300).collect::<String>().replace('\n', " ")),
                Err(e) => println!("ERR {u}\n    {e:?}"),
            }
        }
    }
}
