// LRC 가사 해석과 제목 정리(검색어 만들기).

/// "[01:02.34]가사" → (62340ms, "가사"). 한 줄에 시각이 여러 개면 각각 넣는다. 시각순 정렬.
pub fn parse(text: &str) -> Vec<(u64, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut rest = line.trim();
        let mut times = Vec::new();
        while let Some(s) = rest.strip_prefix('[') {
            let Some(end) = s.find(']') else { break };
            match parse_time(&s[..end]) {
                Some(t) => times.push(t),
                None => break, // [ar:..] 같은 머리말
            }
            rest = s[end + 1..].trim_start();
        }
        for t in times {
            out.push((t, rest.trim().to_string()));
        }
    }
    out.sort_by_key(|x| x.0);
    out
}

fn parse_time(s: &str) -> Option<u64> {
    let (m, sec) = s.split_once(':')?;
    let m: u64 = m.trim().parse().ok()?;
    let sec: f64 = sec.trim().replace(':', ".").parse().ok()?;
    Some(m * 60_000 + (sec * 1000.0).round() as u64)
}

/// 지금 위치에서 불러야 할 줄 번호(아직 첫 줄 전이면 None).
pub fn current(lines: &[(u64, String)], pos_ms: u64) -> Option<usize> {
    let n = lines.partition_point(|l| l.0 <= pos_ms);
    n.checked_sub(1)
}

/// 유튜브식 제목 정리: "가수 - 곡 (Official Video) [MV]" → ("가수", "곡").
/// 가수 칸이 비었거나 채널 이름일 수 있어서, 제목에 " - " 가 있으면 그쪽을 믿는다.
pub fn clean(title: &str, artist: &str) -> (String, String) {
    let mut t = strip_brackets(title);
    let mut a = artist.trim().trim_end_matches(" - Topic").to_string();
    if let Some((l, r)) = t.split_once(" - ") {
        a = l.trim().to_string();
        t = r.trim().to_string();
    }
    for cut in [" ft.", " feat.", " Feat.", " FT."] {
        if let Some(i) = t.find(cut) { t.truncate(i); }
    }
    (a, t.trim().to_string())
}

fn strip_brackets(s: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for c in s.chars() {
        match c {
            '(' | '[' | '【' | '「' => depth += 1,
            ')' | ']' | '】' | '」' => depth = (depth - 1).max(0),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_sorts() {
        let l = parse("[ar:IU]\n[00:12.50]둘째\n[00:01.00][00:20.00]첫째\n[01:02.345]셋째\n");
        assert_eq!(l.iter().map(|x| x.0).collect::<Vec<_>>(), vec![1000, 12500, 20000, 62345]);
        assert_eq!(l[0].1, "첫째");
        assert_eq!(l[2].1, "첫째");
    }

    #[test]
    fn current_line() {
        let l = parse("[00:01.00]a\n[00:05.00]b\n");
        assert_eq!(current(&l, 500), None);
        assert_eq!(current(&l, 1000), Some(0));
        assert_eq!(current(&l, 4999), Some(0));
        assert_eq!(current(&l, 9000), Some(1));
    }

    #[test]
    fn cleans_youtube_titles() {
        assert_eq!(clean("IU - Love wins all (Official MV)", "이지금 [IU Official]"), ("IU".into(), "Love wins all".into()));
        assert_eq!(clean("Ditto", "NewJeans - Topic"), ("NewJeans".into(), "Ditto".into()));
        assert_eq!(clean("Song ft. Someone [Lyrics]", "A"), ("A".into(), "Song".into()));
    }
}
