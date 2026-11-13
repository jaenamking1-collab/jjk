// 알송: 윈도우 '지금 재생'(GSMTC)에 안 나와서 창 제목과 알송 자체 목록(SQLite)을 읽는다.
// SQLite 는 윈도우에 들어 있는 winsqlite3.dll 을 빌려 쓴다(크레이트 추가 없음).
use std::ffi::c_void;
use windows::core::{s, w, PCSTR};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindowTextW};

/// 창 제목 "제목 - 가수" → (제목, 가수). 알송이 없으면 None.
pub fn now() -> Option<(String, String)> {
    let h = unsafe { FindWindowW(w!("ALSong_Class"), None) }.ok()?;
    let mut b = [0u16; 512];
    let n = unsafe { GetWindowTextW(h, &mut b) } as usize;
    let t = String::from_utf16_lossy(&b[..n]);
    let t = t.trim();
    if t.is_empty() || t.eq_ignore_ascii_case("ALSong") || t == "알송" {
        return None;
    }
    Some(match t.rsplit_once(" - ") {
        Some((a, b)) => (a.trim().to_string(), b.trim().to_string()),
        None => (t.to_string(), String::new()),
    })
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
        let hit = |r: &Row| r.title == title || stem(&r.uri) == title;
        let (lid, iid) = rows.iter().find(|(_, id, last, r)| id == last && hit(r)).or_else(|| rows.iter().find(|x| hit(&x.3))).map(|x| (x.0, x.1))?;
        let mine: Vec<(i64, Row)> = rows.into_iter().filter(|x| x.0 == lid).map(|x| (x.1, x.3)).collect();
        let at = mine.iter().position(|x| x.0 == iid)?;
        Some((mine.into_iter().map(|x| x.1).collect(), at))
    }
}

#[cfg(test)]
mod t {
    #[test]
    fn reads_live_alsong() {
        // 알송 목록이 있는 PC 에서만 의미가 있다: cargo test reads_live -- --nocapture
        let t = super::now().map(|x| x.0).unwrap_or_else(|| "가만히 눈을 감고 정재욱".into());
        if let Some((rows, at)) = super::list(&t) {
            println!("'{t}' → {}곡 중 {}번째, 길이 {}초, 다음 '{}'", rows.len(), at + 1, rows[at].secs, rows[(at + 1) % rows.len()].title);
            assert_eq!(rows[at].title, t);
        }
    }
}
