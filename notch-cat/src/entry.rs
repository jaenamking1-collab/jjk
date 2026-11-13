// 일정 넣기 도우미: 날·시각 단추 목록과 "11월 13일 (금) 오전 9:00" 글. 칸 자체는 노치 안에 그린다(main.rs draw_entry).
use crate::gcal;

pub const DAYS: [(&str, i64); 4] = [("오늘", 0), ("내일", 1), ("모레", 2), ("다음 주", 7)];
pub const TIMES: [(&str, i32); 6] = [("종일", -1), ("09:00", 540), ("12:00", 720), ("18:00", 1080), ("+1시간", 60), ("−1시간", -60)];

/// "11월 13일 (금) 오전 9:00" / "11월 13일 (금) 종일"
pub fn when_text(day: i64, min: i32) -> String {
    let (_, m, d) = gcal::ymd(day);
    let head = format!("{m}월 {d}일 ({})", gcal::WEEK[gcal::weekday(day)]);
    if min < 0 {
        return format!("{head} 종일");
    }
    let (h, mi) = (min / 60, min % 60);
    format!("{head} {} {}:{mi:02}", if h < 12 { "오전" } else { "오후" }, if h % 12 == 0 { 12 } else { h % 12 })
}

/// 시각 단추를 눌렀을 때의 새 (날, 분). ±1시간은 자정을 넘으면 날도 넘어간다.
pub fn pick_time(day: i64, min: i32, i: usize) -> (i64, i32) {
    let (_, v) = TIMES[i];
    if i < 4 {
        return (day, v);
    }
    let base = if min < 0 { 540 } else { min } + v;
    (day + base.div_euclid(1440) as i64, base.rem_euclid(1440))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_time_buttons() {
        let d = gcal::days(2026, 11, 13);
        assert_eq!(when_text(d, 540), "11월 13일 (금) 오전 9:00");
        assert_eq!(when_text(d, 780), "11월 13일 (금) 오후 1:00");
        assert_eq!(when_text(d, 0), "11월 13일 (금) 오전 12:00");
        assert_eq!(when_text(d, -1), "11월 13일 (금) 종일");
        assert_eq!(pick_time(d, 540, 3), (d, 1080)); // 18:00
        assert_eq!(pick_time(d, 23 * 60 + 30, 4), (d + 1, 30)); // +1시간이 자정을 넘으면 다음 날
        assert_eq!(pick_time(d, 30, 5), (d - 1, 23 * 60 + 30));
        assert_eq!(pick_time(d, -1, 4), (d, 600)); // 종일에서 +1시간 → 10:00
        assert_eq!(pick_time(d, 540, 0), (d, -1)); // 종일
    }
}
