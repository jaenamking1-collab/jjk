// 알송: 윈도우 '지금 재생'(GSMTC)에 안 나와서 창 제목과 알송 자체 목록(SQLite)을 읽는다.
// SQLite 는 윈도우에 들어 있는 winsqlite3.dll 을 빌려 쓴다(크레이트 추가 없음).
use std::ffi::c_void;
use windows::core::{s, w, PCSTR};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindowTextW, SendMessageTimeoutW, SMTO_ABORTIFHUNG};

/// 알송이 켜져 있지만 멈춘 상태(창 제목이 "알송"뿐)인가 — 다음에 틀면 처음부터라 위치를 안다.
pub fn stopped() -> bool {
    let Ok(h) = (unsafe { FindWindowW(w!("ALSong_Class"), None) }) else { return false };
    let mut b = [0u16; 64];
    let n = unsafe { GetWindowTextW(h, &mut b) } as usize;
    matches!(String::from_utf16_lossy(&b[..n]).trim(), "알송" | "ALSong")
}

/// 알송 창 제목 그대로("제목 - 가수", 가수 태그가 없으면 제목만). 알송이 없거나 멈췄으면 None.
pub fn raw() -> Option<String> {
    let h = unsafe { FindWindowW(w!("ALSong_Class"), None) }.ok()?;
    let mut b = [0u16; 512];
    let n = unsafe { GetWindowTextW(h, &mut b) } as usize;
    let t = String::from_utf16_lossy(&b[..n]).trim().to_string();
    if t.is_empty() || t.eq_ignore_ascii_case("ALSong") || t == "알송" { None } else { Some(t) }
}

/// 창 제목 → (제목, 가수). 알송 목록에 있으면 태그 그대로, 없으면 마지막 " - " 로 나눈다.
pub fn now() -> Option<(String, String)> {
    let t = raw()?;
    if let Some((rows, at)) = list(&t) {
        return Some((rows[at].title.clone(), rows[at].artist.clone()));
    }
    Some(match t.rsplit_once(" - ") {
        Some((a, b)) => (a.trim().to_string(), b.trim().to_string()),
        None => (t, String::new()),
    })
}

/// 재생 명령(WM_APPCOMMAND). 알송은 겉창이 아니라 안쪽 ALSongKernelWindow 만 받는다.
/// 11 다음 곡 · 12 이전 곡 · 14 재생/멈춤
pub fn cmd(code: u32) -> bool {
    let Ok(h) = (unsafe { FindWindowW(w!("ALSongKernelWindow"), None) }) else { return false };
    unsafe { SendMessageTimeoutW(h, 0x0319, WPARAM(h.0 as usize), LPARAM((code << 16) as isize), SMTO_ABORTIFHUNG, 1000, None).0 != 0 }
}

pub struct Row {
    pub title: String,
    pub artist: String,
    pub uri: String,
    pub secs: i64,
}

type Open = unsafe extern "system" fn(*const u8, *mut *mut c_void, i32, *const u8) -> i32;
type Prep = unsafe extern "system" fn(*mut c_void, *const u8, i32, *mut *mut c_void, *mut *const u8) -> i32;
type Step = unsafe extern "system" fn(*mut c_void) -> i32;
type Text = unsafe extern "system" fn(*mut c_void, i32) -> *const u8;
type Int = unsafe extern "system" fn(*mut c_void, i32) -> i64;
type Done = unsafe extern "system" fn(*mut c_void) -> i32;

unsafe fn f<T>(m: windows::Win32::Foundation::HMODULE, n: PCSTR) -> Option<T> {
    GetProcAddress(m, n).map(|p| std::mem::transmute_copy(&p))
}

/// 지금 곡이 든 재생목록 전체와 그 안에서 지금 곡의 자리.
/// 같은 제목이 여러 목록에 있으면 알송이 마지막으로 튼 항목(last_playitem_id)을 고른다.
pub fn list(title: &str) -> Option<(Vec<Row>, usize)> {
    let path = format!("{}\\ESTsoft\\ALSong\\ALSong3.0.0.0.db\0", std::env::var("APPDATA").ok()?);
    unsafe {
        let m = LoadLibraryW(w!("winsqlite3.dll")).ok()?;
        let open: Open = f(m, s!("sqlite3_open_v2"))?;
        let prep: Prep = f(m, s!("sqlite3_prepare_v2"))?;
        let step: Step = f(m, s!("sqlite3_step"))?;
        let text: Text = f(m, s!("sqlite3_column_text"))?;
        let int: Int = f(m, s!("sqlite3_column_int64"))?;
        let fin: Done = f(m, s!("sqlite3_finalize"))?;
        let close: Done = f(m, s!("sqlite3_close"))?;
        let mut db = std::ptr::null_mut();
        if open(path.as_ptr(), &mut db, 1 /* 읽기 전용 */, std::ptr::null()) != 0 {
            close(db);
            return None;
        }
        // 0·1 번은 '많이 들은/최근 들은 음악'(자동 목록)이라 뺀다.
        let q = "select i.list_id, i.id, ifnull(p.last_playitem_id,-1), ifnull(t.tag_title,''), ifnull(t.tag_artist,''), ifnull(f.uri,''), ifnull(t.tag_time,0) \
                 from PlaylistItem i join PlayList p on p.id=i.list_id left join TagInfo t on t.file_id=i.file_id left join FileInfo f on f.file_id=i.file_id \
                 where p.id>1 order by p.list_order, i.order_in_parent\0";
        let mut st = std::ptr::null_mut();
        let mut rows: Vec<(i64, i64, i64, Row)> = vec![];
        let rc = prep(db, q.as_ptr(), -1, &mut st, std::ptr::null_mut());
        if rc == 0 {
            let s = |i| {
                let p = text(st, i);
                if p.is_null() { String::new() } else { std::ffi::CStr::from_ptr(p as _).to_string_lossy().into_owned() }
            };
            while step(st) == 100 {
                rows.push((int(st, 0), int(st, 1), int(st, 2), Row { title: s(3), artist: s(4), uri: s(5), secs: int(st, 6) }));
            }
            fin(st);
        }
        close(db);
        let stem = |u: &str| std::path::Path::new(u).file_stem().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
        let hit = |r: &Row| r.title == title || format!("{} - {}", r.title, r.artist) == title || stem(&r.uri) == title;
        let (lid, iid) = rows.iter().find(|(_, id, last, r)| id == last && hit(r)).or_else(|| rows.iter().find(|x| hit(&x.3))).map(|x| (x.0, x.1))?;
        let mine: Vec<(i64, Row)> = rows.into_iter().filter(|x| x.0 == lid).map(|x| (x.1, x.3)).collect();
        let at = mine.iter().position(|x| x.0 == iid)?;
        Some((mine.into_iter().map(|x| x.1).collect(), at))
    }
}

/// 알송 가사 지문: ID3 머리 뒤 첫 MP3 프레임부터 163840바이트의 MD5. 알송 캐시 파일 이름이 이것이다.
fn md5_of(path: &str) -> Option<String> {
    use std::io::Read;
    let mut b = vec![];
    let mut f = std::fs::File::open(path).ok()?;
    // 머리(ID3)만 먼저 읽어 크기를 알고, 필요한 만큼만 더 읽는다(구글 드라이브 파일이면 읽기가 느리다)
    f.by_ref().take(10).read_to_end(&mut b).ok()?;
    let head = if b.len() == 10 && &b[..3] == b"ID3" { 10 + (((b[6] as usize & 0x7f) << 21) | ((b[7] as usize & 0x7f) << 14) | ((b[8] as usize & 0x7f) << 7) | (b[9] as usize & 0x7f)) + 10 } else { 0 };
    f.take((head + 163_840 + 8_192) as u64).read_to_end(&mut b).ok()?;
    let mut i = 0;
    if b.len() > 10 && &b[..3] == b"ID3" {
        let sz = ((b[6] as usize & 0x7f) << 21) | ((b[7] as usize & 0x7f) << 14) | ((b[8] as usize & 0x7f) << 7) | (b[9] as usize & 0x7f);
        i = 10 + sz + if b[5] & 0x10 != 0 { 10 } else { 0 };
    }
    while i + 1 < b.len() && !(b[i] == 0xFF && b[i + 1] & 0xE0 == 0xE0) {
        i += 1;
    }
    let part = &b[i.min(b.len())..(i + 163_840).min(b.len())];
    let mut out = [0u8; 16];
    use windows::Win32::Security::Cryptography::{BCryptHash, BCRYPT_MD5_ALG_HANDLE};
    unsafe { BCryptHash(BCRYPT_MD5_ALG_HANDLE, None, part, &mut out) }.ok().ok()?;
    Some(out.iter().map(|x| format!("{x:02x}")).collect())
}

/// 알송이 이 파일에 맞춰 받아 둔 싱크 가사(LRC 글). 알송이 곡을 튼 뒤 받으므로 몇 초 기다려 본다.
/// 같은 시각 줄이 여럿이면(원문·번역) 첫 줄만 쓴다.
pub fn lyc(raw: &str) -> Option<String> {
    let (rows, at) = list(raw)?;
    let md5 = md5_of(&rows[at].uri)?;
    let f = std::env::temp_dir().join("ALSongLyric").join("cache").join(format!("{md5}.lyc"));
    for _ in 0..80 {
        if let Ok(b) = std::fs::read(&f) {
            let u: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            let t = String::from_utf16_lossy(&u);
            let body = t.split("[lyrics]").nth(1)?;
            let mut seen = std::collections::HashSet::new();
            let lines: Vec<&str> = body.split("<br>").map(|l| l.trim()).filter(|l| l.starts_with('[') && l.get(..10).map_or(false, |k| seen.insert(k.to_string()))).collect();
            return if lines.is_empty() { None } else { Some(lines.join("
")) };
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    None
}

#[cfg(test)]
mod t {
    #[test]
    fn reads_live_alsong() {
        // 알송 목록이 있는 PC 에서만 의미가 있다: cargo test reads_live -- --nocapture
        let t = super::raw().unwrap_or_default();
        if let Some((rows, at)) = super::list(&t) {
            println!("'{t}' → {}곡 중 {}번째, 길이 {}초, 다음 '{}'", rows.len(), at + 1, rows[at].secs, rows[(at + 1) % rows.len()].title);
            assert!(t.starts_with(&rows[at].title));
            if let Some(l) = super::lyc(&t) {
                println!("알송 가사 {}줄, 첫 줄 {:?}", l.lines().count(), l.lines().next());
            }
        }
    }
}
