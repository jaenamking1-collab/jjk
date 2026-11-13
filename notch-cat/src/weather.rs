// 날씨: 지금 있는 곳(윈도우 위치 서비스 → 안 되면 settings.json 의 lat·lon)의 날씨를 10분마다 받는다.
// - 날씨·시간별: 노르웨이 기상청 공개 예보(api.met.no, 열쇠 없음)
// - 일출·일몰: 같은 곳의 sunrise API
// - 동네 이름: OpenStreetMap(nominatim, 한국어)
// - 미세먼지: open-meteo 대기질(학교 망에선 윈도우 통신이 인증서 오류로 막힌다 — 그땐 칸을 숨긴다, 2026-11-12)
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;
use windows::core::{Result, HSTRING};
use windows::Web::Http::HttpClient;

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Hour {
    pub hour: u32, // 한국 시각(시)
    pub kind: usize,
    pub day: bool,
    pub temp: f64,
}

#[derive(Clone, Default)]
pub struct Weather {
    pub kind: usize, // NAMES 의 번호
    pub temp: f64,
    pub day: bool,
    pub feels: f64,
    pub humid: f64,
    pub wind: f64,     // m/s
    pub wind_dir: f64, // 불어오는 쪽(도)
    pub uv: f64,
    pub rain: f64, // 앞으로 1시간 강수량(mm)
    pub pressure: f64,
    pub hi: f64, // 오늘 최고·최저(한국 날짜)
    pub lo: f64,
    pub hours: Vec<Hour>, // 지금부터 시간별
    pub sunrise: String,  // "07:08"
    pub sunset: String,
    pub pm10: Option<f64>,
    pub pm25: Option<f64>,
    pub place: String,         // "의정부시 흥선동"
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

/// 바람이 불어오는 쪽 → "남동풍"
pub fn wind_name(deg: f64) -> &'static str {
    const N: [&str; 8] = ["북풍", "북동풍", "동풍", "남동풍", "남풍", "남서풍", "서풍", "북서풍"];
    N[((deg.rem_euclid(360.0) + 22.5) / 45.0) as usize % 8]
}

/// 자외선 등급(기상청)
pub fn uv_name(v: f64) -> &'static str {
    match v {
        v if v < 3.0 => "낮음",
        v if v < 6.0 => "보통",
        v if v < 8.0 => "높음",
        v if v < 11.0 => "매우높음",
        _ => "위험",
    }
}

/// 미세먼지 등급(환경부): 0 좋음 1 보통 2 나쁨 3 매우나쁨. fine = 초미세먼지(PM2.5)
pub fn dust_grade(v: f64, fine: bool) -> usize {
    let cut = if fine { [15.0, 35.0, 75.0] } else { [30.0, 80.0, 150.0] };
    cut.iter().filter(|c| v > **c).count()
}
pub const DUST: [&str; 4] = ["좋음", "보통", "나쁨", "매우나쁨"];

/// "2026-11-12T03:00:00Z" → 한국 (날짜, 시)
fn kst(t: &str) -> Option<(String, u32)> {
    let (d, rest) = t.split_once('T')?;
    let h: u32 = rest.get(..2)?.parse().ok()?;
    let k = h + 9;
    if k < 24 {
        return Some((d.to_string(), k));
    }
    // 날짜를 하루 넘긴다(달 끝도 맞게)
    let mut p = d.split('-').map(|x| x.parse::<u32>().unwrap_or(1));
    let (y, m, day) = (p.next()?, p.next()?, p.next()?);
    let last = match m {
        2 if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    let (y, m, day) = if day < last { (y, m, day + 1) } else if m < 12 { (y, m + 1, 1) } else { (y + 1, 1, 1) };
    Some((format!("{y:04}-{m:02}-{day:02}"), k - 24))
}

/// 예보 응답(complete) → Weather(지금·오늘 최고최저·시간별 12칸). 일출·동네·미세먼지는 따로 채운다.
pub fn parse(body: &str) -> Option<Weather> {
    let v: Value = serde_json::from_str(body).ok()?;
    let ts = v["properties"]["timeseries"].as_array()?;
    let first = &ts.first()?["data"];
    let d = &first["instant"]["details"];
    let sym = first["next_1_hours"]["summary"]["symbol_code"].as_str().or(first["next_6_hours"]["summary"]["symbol_code"].as_str())?;
    let (today, _) = kst(ts[0]["time"].as_str()?)?;
    let mut w = Weather {
        kind: kind_of(sym),
        day: !sym.ends_with("_night"),
        temp: d["air_temperature"].as_f64()?,
        humid: d["relative_humidity"].as_f64().unwrap_or(0.0),
        wind: d["wind_speed"].as_f64().unwrap_or(0.0),
        wind_dir: d["wind_from_direction"].as_f64().unwrap_or(0.0),
        uv: d["ultraviolet_index_clear_sky"].as_f64().unwrap_or(0.0),
        pressure: d["air_pressure_at_sea_level"].as_f64().unwrap_or(0.0),
        rain: first["next_1_hours"]["details"]["precipitation_amount"].as_f64().unwrap_or(0.0),
        ..Default::default()
    };
    w.feels = d["apparent_air_temperature"].as_f64().unwrap_or(w.temp);
    let (mut hi, mut lo) = (w.temp, w.temp);
    for t in ts {
        let Some((day, hour)) = t["time"].as_str().and_then(kst) else { continue };
        let x = &t["data"];
        let temp = x["instant"]["details"]["air_temperature"].as_f64();
        if day == today {
            if let Some(tp) = temp {
                hi = hi.max(tp);
                lo = lo.min(tp);
            }
        }
        if w.hours.len() < 12 {
            if let (Some(tp), Some(s)) = (temp, x["next_1_hours"]["summary"]["symbol_code"].as_str()) {
                w.hours.push(Hour { hour, kind: kind_of(s), day: !s.ends_with("_night"), temp: tp });
            }
        }
    }
    (w.hi, w.lo) = (hi, lo);
    Some(w)
}

/// 일출·일몰 응답 → ("07:08", "17:23")
pub fn parse_sun(body: &str) -> Option<(String, String)> {
    let v: Value = serde_json::from_str(body).ok()?;
    let p = &v["properties"];
    let hm = |k: &str| p[k]["time"].as_str().and_then(|t| t.split_once('T')).map(|(_, r)| r.chars().take(5).collect::<String>());
    Some((hm("sunrise")?, hm("sunset")?))
}

/// 동네 이름 응답 → "의정부시 흥선동"
pub fn parse_place(body: &str) -> Option<String> {
    let v: Value = serde_json::from_str(body).ok()?;
    let a = &v["address"];
    let city = ["city", "county", "town", "province"].iter().find_map(|k| a[*k].as_str()).unwrap_or("");
    let dong = ["suburb", "quarter", "city_district", "village", "neighbourhood"].iter().find_map(|k| a[*k].as_str()).or(v["name"].as_str()).unwrap_or("");
    let s = format!("{city} {}", if dong == city { "" } else { dong }).trim().to_string();
    (!s.is_empty()).then_some(s)
}

fn client() -> Result<HttpClient> {
    let c = HttpClient::new()?;
    c.DefaultRequestHeaders()?.UserAgent()?.TryParseAdd(&HSTRING::from("NotchCat/0.1 (github.com/jaenamking1-collab/jjk)"))?; // 기상청·지도가 프로그램 이름을 밝히라고 한다
    Ok(c)
}

fn url(lat: f64, lon: f64) -> String {
    format!("https://api.met.no/weatherapi/locationforecast/2.0/complete?lat={lat:.3}&lon={lon:.3}")
}

/// 윈도우 위치 서비스(와이파이로 수백 m 안쪽). 꺼져 있으면 None. 세 번째 값은 오차(m).
fn here() -> Option<(f64, f64, f64)> {
    use windows::Devices::Geolocation::{GeolocationAccessStatus, Geolocator};
    if Geolocator::RequestAccessAsync().ok()?.get().ok()? != GeolocationAccessStatus::Allowed {
        return None;
    }
    let c = Geolocator::new().ok()?.GetGeopositionAsync().ok()?.get().ok()?.Coordinate().ok()?;
    let p = c.Point().ok()?.Position().ok()?;
    Some((p.Latitude, p.Longitude, c.Accuracy().unwrap_or(0.0)))
}

pub fn spawn(out: Arc<Mutex<Weather>>, lat: f64, lon: f64) {
    std::thread::spawn(move || {
        let Ok(c) = client() else { return };
        let mut spot: Option<((f64, f64), Instant)> = None;
        let mut place: (String, (f64, f64)) = Default::default();
        let mut sun: (String, (String, String)) = Default::default(); // (그날, 일출·일몰)
        let mut warned = false;
        loop {
            // 위치는 30분마다 다시 본다(노트북을 들고 옮기면 따라간다)
            if spot.as_ref().map_or(true, |s| s.1.elapsed() > Duration::from_secs(1800)) {
                // 와이파이 없는 데스크톱은 인터넷 주소로 짐작해 몇 km 틀린다(집 PC: 의정부 호원동인데 서울 방학동, 2026-11-13).
                // 오차가 1.5km 넘고 settings.json 에 동네를 적어 두었으면 그쪽을 쓴다.
                let set = (lat - 37.57).abs() + (lon - 126.98).abs() > 0.001;
                let p = here();
                let use_win = p.filter(|w| !(set && w.2 > 1500.0));
                if !warned {
                    crate::log(&match (p, use_win) {
                        (Some(w), Some(_)) => format!("weather: 윈도우 위치 씀(오차 {:.0}m)", w.2),
                        (Some(w), None) => format!("weather: 윈도우 위치 오차 {:.0}m 라 설정 위치를 씀", w.2),
                        _ => "weather: 윈도우 위치를 못 받아 설정 위치를 씀".to_string(),
                    });
                    warned = true;
                }
                spot = Some((use_win.map(|w| (w.0, w.1)).unwrap_or((lat, lon)), Instant::now()));
            }
            let (la, lo) = spot.as_ref().unwrap().0;
            let got = crate::quotes::get(&c, &url(la, lo)).ok().and_then(|b| parse(&b));
            if let Some(mut w) = got.clone() {
                // 동네 이름: 1km 넘게 옮겼을 때만 다시 묻는다
                if place.0.is_empty() || (place.1 .0 - la).abs() + (place.1 .1 - lo).abs() > 0.01 {
                    let u = format!("https://nominatim.openstreetmap.org/reverse?lat={la:.4}&lon={lo:.4}&format=json&accept-language=ko&zoom=14");
                    if let Some(n) = crate::quotes::get(&c, &u).ok().and_then(|b| parse_place(&b)) {
                        place = (n, (la, lo));
                    }
                }
                let today = crate::ymd();
                if sun.0 != today {
                    let u = format!("https://api.met.no/weatherapi/sunrise/3.0/sun?lat={la:.2}&lon={lo:.2}&date={today}&offset=+09:00");
                    if let Some(s) = crate::quotes::get(&c, &u).ok().and_then(|b| parse_sun(&b)) {
                        sun = (today, s);
                    }
                }
                let aq = format!("https://air-quality-api.open-meteo.com/v1/air-quality?latitude={la:.2}&longitude={lo:.2}&current=pm10,pm2_5");
                let aq: Value = crate::quotes::get(&c, &aq).ok().and_then(|b| serde_json::from_str(&b).ok()).unwrap_or_default();
                w.pm10 = aq["current"]["pm10"].as_f64();
                w.pm25 = aq["current"]["pm2_5"].as_f64();
                w.place = place.0.clone();
                (w.sunrise, w.sunset) = sun.1.clone();
                w.stamp = crate::hhmm();
                w.seen = Some(Instant::now());
                *out.lock().unwrap() = w;
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
    fn reads_forecast() {
        let b = r#"{"properties":{"timeseries":[
            {"time":"2026-11-12T14:00:00Z","data":{"instant":{"details":{"air_temperature":7.4,"apparent_air_temperature":5.1,"relative_humidity":60.2,"wind_speed":2.4,"wind_from_direction":135,"ultraviolet_index_clear_sky":0.0,"air_pressure_at_sea_level":1026.2}},"next_1_hours":{"summary":{"symbol_code":"heavyrainandthunder"},"details":{"precipitation_amount":1.2}}}},
            {"time":"2026-11-12T15:00:00Z","data":{"instant":{"details":{"air_temperature":3.0}},"next_1_hours":{"summary":{"symbol_code":"clearsky_night"}}}},
            {"time":"2026-11-12T16:00:00Z","data":{"instant":{"details":{"air_temperature":9.5}},"next_6_hours":{"summary":{"symbol_code":"cloudy"}}}}]}}"#;
        let w = parse(b).unwrap();
        assert_eq!((w.kind, w.temp, w.feels, w.rain, w.day), (6, 7.4, 5.1, 1.2, true));
        // 14시Z = 23시 KST(12일), 15시Z = 0시(13일) → 오늘 최고·최저는 23시 하나뿐
        assert_eq!((w.hi, w.lo), (7.4, 7.4));
        assert_eq!(w.hours, vec![Hour { hour: 23, kind: 6, day: true, temp: 7.4 }, Hour { hour: 0, kind: 0, day: false, temp: 3.0 }]);
        let k = ["clearsky_night", "fair_day", "partlycloudy_day", "cloudy", "fog", "lightrainshowers_day", "rain", "heavysnow", "sleet", "snowandthunder"].map(kind_of);
        assert_eq!(k, [0, 1, 1, 2, 3, 4, 4, 5, 5, 6]);
        assert!(parse("{}").is_none());
    }

    #[test]
    fn korean_labels() {
        assert_eq!([0.0, 44.0, 135.0, 180.0, 350.0].map(wind_name), ["북풍", "북동풍", "남동풍", "남풍", "북풍"]);
        assert_eq!([0.5, 3.0, 7.0, 9.0, 12.0].map(uv_name), ["낮음", "보통", "높음", "매우높음", "위험"]);
        assert_eq!([dust_grade(25.0, false), dust_grade(81.0, false), dust_grade(16.0, true), dust_grade(80.0, true)], [0, 2, 1, 3]);
        assert_eq!(kst("2026-11-30T15:00:00Z"), Some(("2026-12-01".into(), 0)));
        assert_eq!(kst("2026-12-31T20:00:00Z"), Some(("2027-01-01".into(), 5)));
        let s = r#"{"properties":{"sunrise":{"time":"2026-11-12T07:08+09:00"},"sunset":{"time":"2026-11-12T17:23+09:00"}}}"#;
        assert_eq!(parse_sun(s), Some(("07:08".into(), "17:23".into())));
        let p = r#"{"name":"흥선동","address":{"city_district":"흥선동","city":"의정부시","province":"경기도"}}"#;
        assert_eq!(parse_place(p).as_deref(), Some("의정부시 흥선동"));
    }
}

#[cfg(test)]
mod live {
    // 손으로 돌리는 확인용(통신 필요): cargo test live -- --ignored --nocapture
    #[test]
    #[ignore]
    fn weather_fetch_prints() {
        let here = super::here();
        println!("위치: {here:?}");
        let (la, lo) = here.map(|w| (w.0, w.1)).unwrap_or((37.74, 127.05));
        let c = super::client().unwrap();
        let w = crate::quotes::get(&c, &super::url(la, lo)).ok().and_then(|b| super::parse(&b)).unwrap();
        println!("{} {:.1}° 체감 {:.1}° 습도 {:.0}% {} {:.1}m/s 자외선 {:.1} 강수 {:.1}mm 기압 {:.0} 최고 {:.1} 최저 {:.1}", super::NAMES[w.kind], w.temp, w.feels, w.humid, super::wind_name(w.wind_dir), w.wind, w.uv, w.rain, w.pressure, w.hi, w.lo);
        println!("{:?}", w.hours.iter().map(|h| format!("{}시 {} {:.0}°", h.hour, super::NAMES[h.kind], h.temp)).collect::<Vec<_>>());
        let u = format!("https://nominatim.openstreetmap.org/reverse?lat={la:.4}&lon={lo:.4}&format=json&accept-language=ko&zoom=14");
        println!("동네: {:?}", crate::quotes::get(&c, &u).ok().and_then(|b| super::parse_place(&b)));
    }
}
