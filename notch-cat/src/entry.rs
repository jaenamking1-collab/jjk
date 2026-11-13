// 일정 넣기 도우미: 날·시각 단추 목록과 날짜·시각 칸 계산. 칸 자체는 노치 안에 그린다(main.rs draw_entry).
use crate::gcal;

pub const DAYS: [(&str, i64); 4] = [("오늘", 0), ("내일", 1), ("모레", 2), ("다음 주", 7)];
pub const TIMES: [(&str, i32); 6] = [("종일", -1), ("09:00", 540), ("12:00", 720), ("18:00", 1080), ("+1시간", 60), ("−1시간", -60)];

/// 시각 단추를 눌렀을 때의 새 (날, 분). ±1시간은 자정을 넘으면 날도 넘어간다.
pub fn pick_time(day: i64, min: i32, i: usize) -> (i64, i32) {
    let (_, v) = TIMES[i];
    if i < 4 {
        return (day, v);
    }
    let base = if min < 0 { 540 } else { min } + v;
    (day + base.div_euclid(1440) as i64, base.rem_euclid(1440))
}

/// 날짜·시각 칸 번호: 0 월, 1 일, 2 오전/오후, 3 시, 4 분
pub fn month_len(y: i32, m: u32) -> i64 {
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    gcal::days(ny, nm, 1) - gcal::days(y, m, 1)
}

fn with_month(day: i64, y: i32, m: u32) -> i64 {
    let (_, _, d) = gcal::ymd(day);
    gcal::days(y, m, (d as i64).min(month_len(y, m)) as u32)
}

/// 휠·↑↓로 칸 하나를 한 걸음(분은 5분씩). 시·분·오전오후는 그 날 안에서 돈다. 종일이면 9시에서 시작한다.
pub fn bump(day: i64, min: i32, seg: usize, d: i32) -> (i64, i32) {
    let base = if min < 0 { 540 } else { min };
    match seg {
        0 => {
            let (y, m, _) = gcal::ymd(day);
            let k = y * 12 + m as i32 - 1 + d;
            (with_month(day, k.div_euclid(12), (k.rem_euclid(12) + 1) as u32), min)
        }
        1 => (day + d as i64, min),
        2 => (day, (base + 720).rem_euclid(1440)),
        3 => (day, (base + 60 * d).rem_euclid(1440)),
        _ => {
            let mi = base % 60;
            let mi = if d > 0 { (mi / 5 + 1) * 5 } else { (mi + 4) / 5 * 5 - 5 };
            (day, base / 60 * 60 + mi.rem_euclid(60))
        }
    }
}

/// 칸에 숫자 v 를 넣은 결과. 맞지 않는 값이면 None. 시는 1~12 면 지금 오전/오후를 따르고 13~23·0 은 그대로.
pub fn put(day: i64, min: i32, seg: usize, v: u32) -> Option<(i64, i32)> {
    let base = if min < 0 { 540 } else { min };
    let (y, m, _) = gcal::ymd(day);
    match seg {
        0 if (1..=12).contains(&v) => Some((with_month(day, y, v), min)),
        1 if v >= 1 && v as i64 <= month_len(y, m) => Some((gcal::days(y, m, v), min)),
        3 if v <= 23 => {
            let pm = base >= 720;
            let h = match v {
                0 | 13.. => v,
                12 => if pm { 12 } else { 0 },
                _ => if pm { v + 12 } else { v },
            };
            Some((day, h as i32 * 60 + base % 60))
        }
        4 if v <= 59 => Some((day, base / 60 * 60 + v as i32)),
        _ => None,
    }
}

/// 칸에 숫자 한 자를 쳤다. buf 는 그 칸에 친 숫자들. 돌려주는 bool 은 '다음 칸으로 넘어가라'.
pub fn digit(day: i64, min: i32, seg: usize, buf: &mut String, c: char) -> (i64, i32, bool) {
    let val = |s: &str| s.parse::<u32>().unwrap_or(999);
    let fits = |s: &str| put(day, min, seg, val(s));
    let grow = |s: &str| s.len() < 2 && (0..=9).any(|k| put(day, min, seg, val(s) * 10 + k).is_some());
    let mut s = format!("{buf}{c}");
    if buf.is_empty() || (fits(&s).is_none() && !grow(&s)) {
        s = c.to_string();
    }
    if fits(&s).is_none() && !grow(&s) {
        buf.clear();
        return (day, min, false);
    }
    let (d2, m2) = fits(&s).unwrap_or((day, min));
    if grow(&s) {
        *buf = s;
        (d2, m2, false)
    } else {
        buf.clear();
        (d2, m2, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_time_buttons() {
        let d = gcal::days(2026, 11, 13);
        assert_eq!(pick_time(d, 540, 3), (d, 1080)); // 18:00
        assert_eq!(pick_time(d, 23 * 60 + 30, 4), (d + 1, 30)); // +1시간이 자정을 넘으면 다음 날
        assert_eq!(pick_time(d, 30, 5), (d - 1, 23 * 60 + 30));
        assert_eq!(pick_time(d, -1, 4), (d, 600)); // 종일에서 +1시간 → 10:00
        assert_eq!(pick_time(d, 540, 0), (d, -1)); // 종일
    }

    #[test]
    fn segments() {
        let d = gcal::days(2026, 11, 13);
        assert_eq!(bump(d, 540, 0, 1), (gcal::days(2026, 12, 13), 540));
        assert_eq!(bump(gcal::days(2026, 1, 31), 540, 0, 1), (gcal::days(2026, 2, 28), 540)); // 2월엔 31일이 없다
        assert_eq!(bump(gcal::days(2026, 12, 5), 540, 0, 1), (gcal::days(2027, 1, 5), 540));
        assert_eq!(bump(d, 540, 2, 1), (d, 1260)); // 오전 9시 → 오후 9시
        assert_eq!(bump(d, 23 * 60, 3, 1), (d, 0)); // 시는 그 날 안에서 돈다
        assert_eq!(bump(d, 547, 4, 1), (d, 550));
        assert_eq!(bump(d, 547, 4, -1), (d, 545));
        assert_eq!(bump(d, 540, 4, -1), (d, 595));
        assert_eq!(bump(d, -1, 3, 1), (d, 600)); // 종일에서 시를 올리면 10시
        // 숫자 치기: "1" 은 10~12 를 기다리고, "2" 를 더 치면 12월로 넘어가 다음 칸
        let mut b = String::new();
        assert_eq!(digit(d, 540, 0, &mut b, '1'), (gcal::days(2026, 1, 13), 540, false));
        assert_eq!(digit(d, 540, 0, &mut b, '2'), (gcal::days(2026, 12, 13), 540, true));
        assert_eq!(digit(d, 540, 0, &mut b, '0'), (d, 540, false)); // 0 은 09 를 기다린다
        assert_eq!(digit(d, 540, 0, &mut b, '9'), (gcal::days(2026, 9, 13), 540, true));
        assert_eq!(digit(d, 540, 1, &mut b, '5'), (gcal::days(2026, 11, 5), 540, true)); // 5 뒤엔 올 수 있는 날이 없다
        assert_eq!(digit(d, 1080, 3, &mut b, '7'), (d, 1140, true)); // 오후일 때 7 → 오후 7시
        assert_eq!(digit(d, 540, 3, &mut b, '1'), (d, 60, false));
        assert_eq!(digit(d, 540, 3, &mut b, '5'), (d, 900, true)); // 15 → 오후 3시
        assert_eq!(digit(d, 540, 4, &mut b, '3'), (d, 543, false));
        assert_eq!(digit(d, 540, 4, &mut b, '0'), (d, 570, true));
    }
}
