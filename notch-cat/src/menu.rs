//! 쪽(메뉴) 순서·켜짐. 설정 물방울의 '메뉴 정리'에서 누르면 켜고 끄고, 끌어서 순서를 바꾼다.
//! 쪽 번호는 그리는 코드가 쓰는 번호 그대로(0 음악 1 시세 2 날씨 3 달력 4 AI …). 순서는 넘기는 차례다.

use serde_json::{json, Value};

/// 쪽 이름(번호 순)
pub const NAMES: [&str; 6] = ["음악", "시세", "날씨", "달력", "AI", "클립보드"];
pub const N: usize = NAMES.len();

/// (쪽 번호, 켜짐) 을 넘기는 차례대로
pub type Order = Vec<(usize, bool)>;

/// 저장된 값 → 순서. 없거나 틀린 번호는 빼고, 새로 생긴 쪽은 맨 뒤에 켜진 채로 붙인다.
pub fn parse(v: &Value) -> Order {
    let mut out: Order = Vec::new();
    for p in v.as_array().into_iter().flatten() {
        let (Some(i), on) = (p[0].as_u64().map(|i| i as usize), p[1].as_bool().unwrap_or(true)) else { continue };
        if i < N && !out.iter().any(|o| o.0 == i) {
            out.push((i, on));
        }
    }
    for i in 0..N {
        if !out.iter().any(|o| o.0 == i) {
            out.push((i, true));
        }
    }
    if !out.iter().any(|o| o.1) {
        out[0].1 = true; // 다 꺼져 있으면 맨 앞은 켠다
    }
    out
}

pub fn save(o: &Order) -> Value {
    json!(o.iter().map(|(i, on)| json!([i, on])).collect::<Vec<_>>())
}

/// 켜진 쪽들(차례대로)
pub fn enabled(o: &Order) -> Vec<usize> {
    o.iter().filter(|p| p.1).map(|p| p.0).collect()
}

/// 처음 보일 쪽 = 켜진 것 중 맨 앞
pub fn first(o: &Order) -> usize {
    enabled(o).first().copied().unwrap_or(0)
}

/// 지금 쪽에서 d(+1 다음, -1 앞) 만큼 넘긴 쪽. 꺼진 쪽은 건너뛰고 끝에서 돌아간다.
/// 지금 쪽이 꺼져 있으면(방금 껐다) 차례상 그 자리에서 센다.
pub fn step(o: &Order, cur: usize, d: i32) -> usize {
    let n = o.len() as i32;
    let at = o.iter().position(|p| p.0 == cur).unwrap_or(0) as i32;
    for k in 1..=n {
        let j = (at + d.signum() * k).rem_euclid(n) as usize;
        if o[j].1 {
            return o[j].0;
        }
    }
    cur
}

/// 차례 k 번째 칸을 켜고 끈다. 마지막 남은 하나는 끄지 않는다(false 를 돌려준다).
pub fn toggle(o: &mut Order, k: usize) -> bool {
    let Some(p) = o.get(k) else { return false };
    if p.1 && enabled(o).len() <= 1 {
        return false;
    }
    o[k].1 = !o[k].1;
    true
}

/// 차례 from 칸을 to 자리로 옮긴다
pub fn move_to(o: &mut Order, from: usize, to: usize) {
    if from >= o.len() {
        return;
    }
    let p = o.remove(from);
    o.insert(to.min(o.len()), p);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_fills_new_pages() {
        let o = parse(&json!([[3, true], [0, false], [9, true], [3, false]]));
        assert_eq!(o, vec![(3, true), (0, false), (1, true), (2, true), (4, true), (5, true)]);
        assert_eq!(parse(&Value::Null), (0..N).map(|i| (i, true)).collect::<Vec<_>>());
        assert_eq!(parse(&save(&o)), o);
    }

    #[test]
    fn step_skips_off_pages() {
        let o = vec![(3, true), (0, false), (1, true), (2, false), (4, true)];
        assert_eq!(first(&o), 3);
        assert_eq!(step(&o, 3, 1), 1);
        assert_eq!(step(&o, 1, 1), 4);
        assert_eq!(step(&o, 4, 1), 3); // 끝에서 처음으로
        assert_eq!(step(&o, 3, -1), 4);
        assert_eq!(step(&o, 0, 1), 1); // 꺼진 쪽에 있었으면 그 자리에서 센다
    }

    #[test]
    fn last_one_stays_on() {
        let mut o = vec![(0, true), (1, false)];
        assert!(!toggle(&mut o, 0));
        assert_eq!(o[0], (0, true));
        assert!(toggle(&mut o, 1));
        assert!(toggle(&mut o, 0));
        assert_eq!(enabled(&o), vec![1]);
    }

    #[test]
    fn move_reorders() {
        let mut o: Order = (0..5).map(|i| (i, true)).collect();
        move_to(&mut o, 4, 0);
        assert_eq!(o.iter().map(|p| p.0).collect::<Vec<_>>(), vec![4, 0, 1, 2, 3]);
        move_to(&mut o, 0, 9);
        assert_eq!(o.iter().map(|p| p.0).collect::<Vec<_>>(), vec![0, 1, 2, 3, 4]);
    }
}
