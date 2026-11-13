// 노치 고양이 — 화면 위 가운데 노치. 1단계: 틀·움직임·음악·가사.
// 설계: docs/superpowers/specs/2026-11-10-notch-cat-design.md
#![windows_subsystem = "windows"]

mod face;
mod entry;
mod gcal;
mod gfx;
mod lrc;
mod lyrics;
mod naver;
mod media;
mod meter;
mod notif;
mod queue;
mod alsong;
mod wmp;
mod quotes;
mod settings;
mod spring;
mod weather;

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use windows::core::{w, Result, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct2D::ID2D1Bitmap;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Registry::*;
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{SetFocus, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT};
use windows::Win32::UI::Input::Ime::{ImmGetCompositionStringW, ImmGetContext, ImmReleaseContext, GCS_COMPSTR, GCS_RESULTSTR};
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use gfx::{Align, Gfx, Rgb};
use lyrics::{Lyr, St};
use media::{Cmd, Now};
use spring::Spring;

const VERSION: &str = env!("CARGO_PKG_VERSION");

// 창(=그릴 수 있는 최대 넓이), 논리 단위
const WIN_W: f32 = 920.0; // 음악 화면 + 다음 곡 목록까지 들어가게
const QW: f32 = 290.0; // 다음 곡 목록 칸 너비
const WIN_H: f32 = 800.0; // 큰 달력 + 일정 칸 + 알림 물방울까지
// 일정 넣기 칸(노치 아래로 물방울처럼 나온다)
const ENT_W: f32 = 560.0;
const ENT_H: f32 = 112.0;
const ENT_GAP: f32 = 18.0;
const NOTE_H: f32 = 54.0; // 적는 동안 칸 아래로 나오는 알림 물방울 높이

/// 날짜·시각 칸에서 옆 칸으로(d = +1 오른쪽, -1 왼쪽). 종일이면 월·일만 있다. 끝이면 None.
fn seg_step(i: usize, d: i32, min: i32) -> Option<usize> {
    let last = if min < 0 { 1 } else { 4 };
    let j = i as i32 + d;
    (0..=last as i32).contains(&j).then_some(j as usize)
}

struct Ent {
    text: String,
    comp: String, // 조합 중인 한글
    day: i64,
    min: i32, // -1 종일
    cals: Vec<usize>, // 넣을 캘린더들(계정마다 하나까지, 여러 계정이면 골라서 같이)
    prev: HWND, // 칸을 열기 전 앞에 있던 창(닫으면 돌려준다)
    focused: bool,
    closing: bool,
    caret: Instant,
    seg: Option<usize>, // 고른 날짜·시각 칸(0 월 1 일 2 오전오후 3 시 4 분). None 이면 글자 칸
    buf: String,        // 그 칸에 친 숫자
    parked: bool,       // 다른 창으로 가서 접어 둔 상태(적던 내용은 남는다)
    cur: usize,         // 글자 칸 커서(글자 수 기준)
}

impl Ent {
    /// 커서의 바이트 자리
    fn at(&self) -> usize {
        self.text.char_indices().nth(self.cur).map_or(self.text.len(), |(i, _)| i)
    }
    fn insert(&mut self, t: &str) {
        let i = self.at();
        self.text.insert_str(i, t);
        self.cur += t.chars().count();
    }
    /// 화면에 보일 글: 커서 자리에 조합 중인 한글을 끼운다
    fn shown(&self) -> String {
        let i = self.at();
        format!("{}{}{}", &self.text[..i], self.comp, &self.text[i..])
    }
}
// 노치 크기
const IDLE: (f32, f32) = (150.0, 28.0);
const BAR_H: f32 = 28.0;
const FACE_ZOOM: f32 = 1.15;
const QUIET: (f32, f32) = (250.0, 104.0); // 음악이 없을 때 펼친 크기(시계)
const NOTE_W: f32 = 300.0; // 알림이 왔을 때 펼친 너비
const TINY: (f32, f32) = (48.0, 9.0); // 일할 때 줄어든 크기
const OPEN: (f32, f32) = (460.0, 246.0);
// 음악 화면 모양: 0 기본(가사 아래) · 1 와이드(소리 막대) · 2 가사 옆
const MUSIC: [(f32, f32); 3] = [OPEN, (540.0, 128.0), (590.0, 184.0)];
const WX: [(f32, f32); 2] = [(600.0, 168.0), (600.0, 262.0)]; // 날씨 쪽: 작은 판 · 큰 판
const CAL_BIG: (f32, f32) = (880.0, 580.0); // 달력을 눌러 크게 편 판
const CALV: [(f32, f32); 3] = [(560.0, 186.0), (500.0, 322.0), (600.0, 282.0)]; // 달력 쪽: 한 주 줄 · 한 달 · 달력+목록
const MUSIC_NAMES: [&str; 3] = ["기본 (가사 아래)", "와이드 (소리 막대)", "가사 옆"];
const SHADOW: f32 = 20.0; // 펼쳤을 때 그림자가 번지는 폭

// 색: 디자인.md '대비 강한 조합' — 미드나잇 네이비 · 쿨 화이트 · 일렉트릭 라임
const BG: Rgb = (0.0, 0.0, 0.0);
const FG: Rgb = (0.953, 0.957, 0.965);
const EYE: Rgb = (0.639, 0.902, 0.208); // 고양이 눈(라임)
const PUPIL: Rgb = (0.063, 0.086, 0.043);
const TAIL: Rgb = (0.075, 0.095, 0.075);
const NOSE: Rgb = (0.757, 0.784, 0.741);
const UP: Rgb = (1.0, 0.361, 0.4); // 국내식: 오르면 빨강, 내리면 파랑
const DOWN: Rgb = (0.353, 0.659, 1.0);
const WHISK: Rgb = (0.81, 0.83, 0.81);
const GLOW: [Rgb; 3] = [(0.176, 0.831, 0.749), (0.3, 0.8, 0.45), (0.55, 0.85, 0.2)]; // 아래 빛: 청록 → 라임
const EAR: f32 = 6.0; // 위쪽 모서리가 바깥으로 휘는 크기
const TAIL_ROOM: f32 = 22.0; // 꼬리가 나올 옆 여백
const LIME: Rgb = (0.639, 0.902, 0.208);
const WARN: Rgb = (1.0, 0.416, 0.102);
const NAVER: Rgb = (0.012, 0.78, 0.353); // 네이버 초록
const SUN: Rgb = (1.0, 0.8, 0.28);

const WM_TRAY: u32 = WM_APP + 1;
const WM_MOUSELEAVE: u32 = 0x02A3;

// ── 공용: 데이터 폴더·로그·시각 ─────────────────────────────

pub fn data_dir() -> PathBuf {
    let d = PathBuf::from(std::env::var("APPDATA").unwrap_or_else(|_| ".".into())).join("notch-cat");
    let _ = std::fs::create_dir_all(&d);
    d
}

fn local() -> SYSTEMTIME {
    unsafe { GetLocalTime() }
}

pub fn ymd() -> String {
    let t = local();
    format!("{:04}-{:02}-{:02}", t.wYear, t.wMonth, t.wDay)
}

pub fn hhmm() -> String {
    let t = local();
    format!("{:02}:{:02}", t.wHour, t.wMinute)
}

pub fn log(msg: &str) {
    let t = local();
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(data_dir().join("log.txt")) {
        let _ = writeln!(f, "{:02}-{:02} {:02}:{:02}:{:02} {msg}", t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond);
    }
}

fn trim_log() {
    let p = data_dir().join("log.txt");
    if let Ok(s) = std::fs::read_to_string(&p) {
        let lines: Vec<&str> = s.lines().collect();
        if lines.len() > 500 {
            let _ = std::fs::write(&p, lines[lines.len() - 500..].join("\n") + "\n");
        }
    }
}

// ── 설정 ────────────────────────────────────────────────

struct Cfg {
    hide_fullscreen: bool,
    autostart: bool,
    tiny: bool,  // 창을 꽉 채워 일할 때 작게 줄이기
    bar: bool,   // 닫힘에 시세 한 줄
    aside: bool, // 가운데 대신 오른쪽으로 비켜 놓기(다른 노치 앱과 나란히 볼 때)
    lnk: String, // 바탕화면 바로가기를 마지막으로 만든 실행 파일 경로(같으면 다시 만들지 않는다 — 지운 바로가기가 되살아나지 않게)
    two: bool,   // 시세를 세로 두 칸으로(끄면 시세 위젯처럼 한 칸)
    ml: usize,   // 음악 화면 모양(MUSIC 번호)
    wbig: bool,  // 날씨 쪽을 큰 판으로
    cv: usize,   // 달력 쪽 모양(CALV 번호)
    place: (f64, f64), // 날씨를 볼 곳(위도, 경도). settings.json 의 lat·lon, 없으면 서울
}

impl Cfg {
    /// 시세 칸 수. 시험할 땐 NOTCH_CAT_COLS=1 또는 2 로 바꿔 본다.
    fn cols(&self) -> usize {
        match std::env::var("NOTCH_CAT_COLS").as_deref() {
            Ok("1") => 1,
            Ok("2") => 2,
            _ => 1 + self.two as usize,
        }
    }

    fn load() -> Cfg {
        let v: serde_json::Value = std::fs::read_to_string(data_dir().join("settings.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Cfg { hide_fullscreen: v["hide_fullscreen"].as_bool().unwrap_or(true), autostart: v["autostart"].as_bool().unwrap_or(true), aside: v["aside"].as_bool().unwrap_or(false), tiny: v["tiny"].as_bool().unwrap_or(true), bar: v["bar"].as_bool().unwrap_or(false), two: v["two"].as_bool().unwrap_or(true), ml: (v["ml"].as_u64().unwrap_or(0) as usize).min(2), wbig: v["wbig"].as_bool().unwrap_or(true), cv: (v["cv"].as_u64().unwrap_or(2) as usize).min(2), lnk: v["lnk"].as_str().unwrap_or("").into(), place: (v["lat"].as_f64().unwrap_or(37.57), v["lon"].as_f64().unwrap_or(126.98)) }
    }

    fn save(&self) {
        let v = serde_json::json!({ "hide_fullscreen": self.hide_fullscreen, "autostart": self.autostart, "aside": self.aside, "tiny": self.tiny, "bar": self.bar, "two": self.two, "ml": self.ml, "wbig": self.wbig, "cv": self.cv, "lnk": self.lnk, "lat": self.place.0, "lon": self.place.1 });
        let _ = std::fs::write(data_dir().join("settings.json"), v.to_string());
    }

    /// 바탕화면에 '노치 고양이' 바로가기. 이 실행 파일로 처음 켤 때 한 번 만든다(파일을 옮기거나 새로 받으면 다시).
    unsafe fn ensure_shortcut(&mut self) {
        use windows::core::{Interface, HSTRING};
        use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, IPersistFile, CLSCTX_INPROC_SERVER};
        if cfg!(debug_assertions) || std::env::var("NOTCH_CAT_TEST").is_ok() {
            return;
        }
        let Ok(exe) = std::env::current_exe() else { return };
        let path = exe.display().to_string();
        if self.lnk == path {
            return;
        }
        let made = (|| -> Result<()> {
            let desk = SHGetKnownFolderPath(&FOLDERID_Desktop, KNOWN_FOLDER_FLAG(0), None)?;
            let lnk = format!("{}\\노치 고양이.lnk", desk.to_string().unwrap_or_default());
            CoTaskMemFree(Some(desk.0 as _));
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
            link.SetPath(&HSTRING::from(&path))?;
            if let Some(dir) = exe.parent() {
                link.SetWorkingDirectory(&HSTRING::from(dir.display().to_string()))?;
            }
            link.SetIconLocation(&HSTRING::from(&path), 0)?;
            link.cast::<IPersistFile>()?.Save(&HSTRING::from(lnk), true)
        })();
        log(&format!("shortcut {}", if made.is_ok() { "ok" } else { "ERR" }));
        if made.is_ok() {
            self.lnk = path;
            self.save();
        }
    }

    /// 윈도우 시작할 때 자동 실행(HKCU Run). 시험 중·개발 빌드에선 건드리지 않는다.
    unsafe fn apply_autostart(&self) {
        if cfg!(debug_assertions) || std::env::var("NOTCH_CAT_TEST").is_ok() {
            return;
        }
        let key = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        if self.autostart {
            let Ok(exe) = std::env::current_exe() else { return };
            let val: Vec<u16> = format!("\"{}\"", exe.display()).encode_utf16().chain([0]).collect();
            let _ = RegSetKeyValueW(HKEY_CURRENT_USER, key, w!("NotchCat"), REG_SZ.0, Some(val.as_ptr() as _), (val.len() * 2) as u32);
        } else {
            let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, key, w!("NotchCat"));
        }
    }
}

// ── 창 메시지 → 사건 목록 ───────────────────────────────

enum Ev {
    Move(f32, f32),
    Leave,
    Click(f32, f32),
    Tick,
    Tray,
    Wheel(i32), // 휠: -1 왼쪽 쪽, +1 오른쪽 쪽
    Char(u32),      // 일정 넣기: 글자(영문·숫자·지운 글자)
    Key(u32),       // 일정 넣기: Enter·Esc 등
    Text(String),   // 일정 넣기: 한글 조합이 끝난 글자
    Comp(String),   // 일정 넣기: 조합 중인 한글(밑줄로 보인다)
    Blur,           // 일정 넣기: 다른 창을 눌러 자판을 잃었다
}

thread_local! {
    static TYPING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; // 일정 넣기 칸이 열려 자판을 받는 중
}

/// IME 조합 글자 읽기
unsafe fn ime_str(hwnd: HWND, kind: windows::Win32::UI::Input::Ime::IME_COMPOSITION_STRING) -> String {
    let c = ImmGetContext(hwnd);
    let n = ImmGetCompositionStringW(c, kind, None, 0);
    let mut buf = vec![0u16; (n.max(0) as usize) / 2];
    if n > 0 {
        ImmGetCompositionStringW(c, kind, Some(buf.as_mut_ptr() as _), n as u32);
    }
    let _ = ImmReleaseContext(hwnd, c);
    String::from_utf16_lossy(&buf)
}

thread_local! {
    static EVENTS: RefCell<Vec<Ev>> = const { RefCell::new(Vec::new()) };
}

fn push(e: Ev) {
    EVENTS.with(|v| v.borrow_mut().push(e));
}

/// 노치 바로 아래(마우스 자리)의 창에 '마우스가 나갔다'를 알린다.
/// 크롬은 탭 위에 잠깐 머물면 미리보기 카드를 띄우는데, 마우스가 노치로 넘어와도 나간 줄 몰라 카드가 노치 밑에 남는다.
unsafe fn release_below(me: HWND) {
    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);
    let mut h = GetWindow(me, GW_HWNDNEXT).unwrap_or_default();
    for _ in 0..200 {
        if h.0.is_null() {
            return;
        }
        let mut r = RECT::default();
        if IsWindowVisible(h).as_bool() && GetWindowRect(h, &mut r).is_ok() && pt.x >= r.left && pt.x < r.right && pt.y >= r.top && pt.y < r.bottom
            && GetWindowLongW(h, GWL_EXSTYLE) as u32 & WS_EX_TRANSPARENT.0 == 0
        {
            let _ = PostMessageW(h, 0x02A3 /* WM_MOUSELEAVE */, WPARAM(0), LPARAM(0));
            return;
        }
        h = GetWindow(h, GW_HWNDNEXT).unwrap_or_default();
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_MOUSEMOVE => push(Ev::Move((lp.0 & 0xFFFF) as i16 as f32, ((lp.0 >> 16) & 0xFFFF) as i16 as f32)),
        WM_MOUSELEAVE => push(Ev::Leave),
        // 휠을 위로(또는 왼쪽으로) 굴리면 왼쪽 쪽, 아래로(또는 오른쪽으로) 굴리면 오른쪽 쪽
        WM_MOUSEWHEEL => push(Ev::Wheel(if ((wp.0 >> 16) as i16) > 0 { -1 } else { 1 })),
        WM_MOUSEHWHEEL => push(Ev::Wheel(if ((wp.0 >> 16) as i16) > 0 { 1 } else { -1 })),
        WM_LBUTTONUP => push(Ev::Click((lp.0 & 0xFFFF) as i16 as f32, ((lp.0 >> 16) & 0xFFFF) as i16 as f32)),
        WM_TIMER => push(Ev::Tick),
        WM_MOUSEACTIVATE if !TYPING.with(|t| t.get()) => return LRESULT(MA_NOACTIVATE as isize),
        WM_CHAR if TYPING.with(|t| t.get()) => {
            push(Ev::Char(wp.0 as u32));
            return LRESULT(0);
        }
        WM_KEYDOWN if TYPING.with(|t| t.get()) => push(Ev::Key(wp.0 as u32)),
        WM_KILLFOCUS if TYPING.with(|t| t.get()) => push(Ev::Blur),
        // 한글 조합: 윈도우 기본 조합 창(화면 구석의 작은 창)을 띄우지 않고 노치에 직접 그린다
        0x010D /* WM_IME_STARTCOMPOSITION */ if TYPING.with(|t| t.get()) => return LRESULT(0),
        0x010F /* WM_IME_COMPOSITION */ if TYPING.with(|t| t.get()) => {
            if lp.0 as u32 & GCS_RESULTSTR.0 != 0 {
                push(Ev::Text(ime_str(hwnd, GCS_RESULTSTR)));
            }
            push(Ev::Comp(if lp.0 as u32 & GCS_COMPSTR.0 != 0 { ime_str(hwnd, GCS_COMPSTR) } else { String::new() }));
            return LRESULT(0);
        }
        0x010E /* WM_IME_ENDCOMPOSITION */ if TYPING.with(|t| t.get()) => push(Ev::Comp(String::new())),
        WM_TRAY => {
            let m = lp.0 as u32 & 0xFFFF;
            if m == WM_RBUTTONUP || m == WM_LBUTTONUP {
                push(Ev::Tray);
            }
        }
        WM_DESTROY => PostQuitMessage(0),
        _ => {}
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}

/// 앞에 있는 창이 이 모니터를 꼭 맞게 덮고 있나(게임·영상·슬라이드).
unsafe fn fullscreen_in_front(own: HWND, mon: &RECT) -> bool {
    let fg = GetForegroundWindow();
    if fg.0.is_null() || fg == own {
        return false;
    }
    let mut cls = [0u16; 64];
    let n = GetClassNameW(fg, &mut cls) as usize;
    let cls = String::from_utf16_lossy(&cls[..n]);
    if cls == "Progman" || cls == "WorkerW" {
        return false; // 바탕화면
    }
    let mut r = RECT::default();
    GetWindowRect(fg, &mut r).is_ok() && r == *mon
}

/// 노치 바로 아래에 보이는 창이 최대화돼 있나(그 창의 탭·제목줄을 노치가 가린다). 내 창이 걸리면 None.
unsafe fn maximized_under(own: HWND, x: i32, y: i32) -> Option<bool> {
    let w = WindowFromPoint(POINT { x, y });
    if w.0.is_null() {
        return Some(false);
    }
    let root = GetAncestor(w, GA_ROOT);
    if root == own {
        return None;
    }
    Some(IsZoomed(root).as_bool())
}

/// 시세 쪽의 자리. 설정의 --- 구분선으로 나뉜 구역마다 버튼 하나, 그 안에 종목을 시세 위젯처럼 한 줄에 하나씩 내려 적는다.
/// cols: 1 = 위젯처럼 한 칸, 2 = 세로 두 칸(구역째로 왼쪽·오른쪽에 나눠 담는다). k: 0 작은 판(이름·등락률) … 1 큰 판(값까지).
/// 돌려주는 것: 구역 버튼들, 종목 줄들(각 [왼쪽(몸통 기준), 위, 너비, 높이]), 몸통 전체 (너비, 높이).
fn q_layout(groups: &[usize], k: f32, cols: usize) -> (Vec<[f32; 4]>, Vec<[f32; 4]>, (f32, f32)) {
    let w = if cols == 1 { 270.0 + 80.0 * k } else { 500.0 + 80.0 * k };
    if groups.is_empty() {
        return (Vec::new(), Vec::new(), (w, 104.0));
    }
    let (side, inx, iny, gap, rh) = (16.0, 12.0, 7.0, 8.0, 20.0 + if cols == 1 { 2.0 } else { 4.0 } * k);
    // 구역 크기(줄 수)
    let mut sizes = Vec::new();
    let mut i = 0;
    while i < groups.len() {
        let n = groups[i..].iter().take_while(|g| **g == groups[i]).count();
        sizes.push(n);
        i += n;
    }
    let tall = |r: &[usize]| r.iter().map(|n| *n as f32 * rh + iny * 2.0 + gap).sum::<f32>();
    // 두 칸이면 두 칸의 키가 가장 비슷해지는 구역 경계에서 나눈다
    let cut = if cols == 1 { sizes.len() } else { (1..=sizes.len()).min_by(|a, b| tall(&sizes[..*a]).max(tall(&sizes[*a..])).total_cmp(&tall(&sizes[..*b]).max(tall(&sizes[*b..])))).unwrap_or(sizes.len()) };
    let pw = if cut < sizes.len() { (w - side * 2.0 - gap) / 2.0 } else { w - side * 2.0 };
    let (mut panels, mut cells) = (Vec::new(), Vec::with_capacity(groups.len()));
    let (mut y, mut bottom) = (44.0f32, 0.0f32);
    for (gi, n) in sizes.iter().enumerate() {
        if gi == cut {
            y = 44.0;
        }
        let x = side + if gi >= cut { pw + gap } else { 0.0 };
        let ph = *n as f32 * rh + iny * 2.0;
        panels.push([x, y, pw, ph]);
        for j in 0..*n {
            cells.push([x + inx, y + iny + j as f32 * rh, pw - inx * 2.0, rh]);
        }
        y += ph + gap;
        bottom = bottom.max(y - gap);
    }
    // ponytail: 종목이 아주 많으면 아래가 잘린다(한 칸 24줄·두 칸 48줄쯤). 그때는 쪽을 나누거나 넘겨 보게.
    (panels, cells, (w, (bottom + 14.0).min(WIN_H - SHADOW - 4.0)))
}

/// 소리 막대: 실제 소리 크기를 따라 오르내린다(아래에서 위로, 끝은 둥글게). base 는 막대 바닥.
unsafe fn sound_bars(g: &gfx::Gfx, bars: &[f32], x: f32, base: f32, w: f32, hmax: f32, a: f32) {
    let step = w / bars.len() as f32;
    for (i, b) in bars.iter().enumerate() {
        let bh = 4.0 + hmax * b.clamp(0.0, 1.0);
        g.rrect(x + i as f32 * step, base - bh, step * 0.62, bh, step * 0.31, mix(FG, LIME, 0.25 + 0.6 * b), a * (0.35 + 0.6 * b));
    }
}

/// 가사 없이 소리만 나는 때인가: 첫 줄 전(전주) · 줄 사이가 길게 빈 곳(간주) · 마지막 줄 뒤(후주).
fn instr_part(lines: &[(u64, String)], cur: Option<usize>, pos: u64) -> Option<&'static str> {
    let first = lines.first()?.0;
    match cur {
        None => (pos + 300 < first && first > 3000).then_some("전주"),
        Some(i) => {
            let at = lines[i].0;
            match lines.get(i + 1) {
                // 다음 줄이 10초 넘게 남았고 지금 줄을 5초 넘게 띄웠으면, 다음 줄 1.5초 전까지
                Some(nx) => (nx.0 - at > 10_000 && pos > at + 5_000 && nx.0 > pos + 1_500).then_some("간주"),
                None => (pos > at + 6_000).then_some("후주"),
            }
        }
    }
}

#[test]
fn instr_parts() {
    let l: Vec<(u64, String)> = [(12_000, "a"), (15_000, "b"), (40_000, "c")].iter().map(|x| (x.0, x.1.to_string())).collect();
    assert_eq!(instr_part(&l, None, 5_000), Some("전주"));
    assert_eq!(instr_part(&l, Some(0), 13_000), None);
    assert_eq!(instr_part(&l, Some(1), 25_000), Some("간주"));
    assert_eq!(instr_part(&l, Some(1), 39_000), None); // 다음 줄 바로 앞은 가사로 돌아온다
    assert_eq!(instr_part(&l, Some(2), 50_000), Some("후주"));
}

fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, a.2 + (b.2 - a.2) * t)
}

/// 날씨 그림(가로 24쯤). kind 는 weather::NAMES 번호.
unsafe fn wx_icon(g: &Gfx, kind: usize, day: bool, cx: f32, cy: f32, a: f32) {
    let cloud = |cx: f32, cy: f32, a: f32| {
        g.ellipse(cx - 5.0, cy + 1.0, 5.0, 4.5, FG, a);
        g.ellipse(cx + 1.0, cy - 2.5, 6.5, 6.0, FG, a);
        g.ellipse(cx + 7.0, cy + 1.5, 4.5, 4.0, FG, a);
        g.rrect(cx - 10.0, cy + 0.5, 21.5, 5.5, 2.75, FG, a);
    };
    // 해(낮) 또는 초승달(밤)
    let light = |cx: f32, cy: f32, r: f32| {
        if day {
            g.ellipse(cx, cy, r, r, SUN, a);
            for i in 0..8 {
                let (s, c) = (i as f32 * std::f32::consts::FRAC_PI_4).sin_cos();
                ribbon(g, &[(cx + c * (r + 2.6), cy + s * (r + 2.6)), (cx + c * (r + 5.0), cy + s * (r + 5.0))], |_| 1.7, SUN, a);
            }
        } else {
            g.ellipse(cx, cy, r + 1.0, r + 1.0, SUN, a);
            g.ellipse(cx + r * 0.55, cy - r * 0.35, r * 0.95, r * 0.95, BG, a);
        }
    };
    match kind {
        0 => light(cx, cy, 6.0),
        1 => {
            light(cx + 5.0, cy - 4.0, 4.0);
            cloud(cx - 2.0, cy + 3.0, a);
        }
        2 => {
            cloud(cx + 4.0, cy - 4.0, a * 0.4);
            cloud(cx - 1.0, cy + 2.0, a);
        }
        3 => {
            for (i, wd) in [20.0f32, 14.0, 20.0].into_iter().enumerate() {
                g.rrect(cx - 10.0 + (i % 2) as f32 * 6.0, cy - 7.0 + i as f32 * 6.0, wd, 2.6, 1.3, FG, a * 0.8);
            }
        }
        _ => {
            cloud(cx, cy - 4.0, a);
            for i in 0..3 {
                let dx = cx - 6.0 + i as f32 * 6.0;
                match kind {
                    4 => ribbon(g, &[(dx + 1.5, cy + 5.0), (dx - 0.5, cy + 10.5)], |_| 1.8, DOWN, a),
                    5 => g.ellipse(dx, cy + 6.5 + (i % 2) as f32 * 3.5, 1.6, 1.6, FG, a),
                    _ => {}
                }
            }
            if kind == 6 {
                g.poly(&[(cx + 2.5, cy + 2.0), (cx - 3.5, cy + 8.5), (cx - 0.2, cy + 8.5), (cx - 2.0, cy + 14.0), (cx + 4.5, cy + 6.3), (cx + 1.2, cy + 6.3), (cx + 4.0, cy + 2.0)], SUN, a);
            }
        }
    }
}

/// 굽은 선 위의 점들(2차 곡선).
fn quad(p0: (f32, f32), c: (f32, f32), p1: (f32, f32), n: usize) -> Vec<(f32, f32)> {
    (0..=n)
        .map(|i| {
            let u = i as f32 / n as f32;
            let v = 1.0 - u;
            (v * v * p0.0 + 2.0 * v * u * c.0 + u * u * p1.0, v * v * p0.1 + 2.0 * v * u * c.1 + u * u * p1.1)
        })
        .collect()
}

/// 점들을 따라가는 띠. 굵기는 wf(0…1)로 정하고 양끝은 둥글다. 한 덩어리라 반투명해도 겹친 자국이 없다.
/// 목록 단추: 둥근 바탕 + 점 셋·줄 셋. 펼쳐 두면 라임.
unsafe fn queue_btn(g: &Gfx, cx: f32, cy: f32, r: f32, on: bool, a: f32) {
    let c = if on { LIME } else { FG };
    g.ellipse(cx, cy, r, r, c, a * if on { 0.22 } else { 0.12 });
    for i in -1..=1 {
        let y = cy + i as f32 * 5.0;
        g.ellipse(cx - 6.0, y, 1.4, 1.4, c, a);
        g.rrect(cx - 3.0, y - 0.9, 10.0, 1.8, 0.9, c, a);
    }
}

unsafe fn ribbon(g: &Gfx, p: &[(f32, f32)], wf: impl Fn(f32) -> f32, c: Rgb, a: f32) {
    if a <= 0.01 || p.len() < 2 {
        return;
    }
    let n = p.len() - 1;
    let frame = |i: usize| {
        let (a0, b0) = (p[i.saturating_sub(1)], p[(i + 1).min(n)]);
        let (dx, dy) = (b0.0 - a0.0, b0.1 - a0.1);
        let l = (dx * dx + dy * dy).sqrt().max(1e-4);
        (dx / l, dy / l, wf(i as f32 / n as f32) / 2.0)
    };
    let mut out = Vec::with_capacity(n * 2 + 14);
    for i in 0..=n {
        let (tx, ty, hw) = frame(i);
        out.push((p[i].0 - ty * hw, p[i].1 + tx * hw));
    }
    let (tx, ty, hw) = frame(n);
    for k in 1..6 {
        let (s, co) = (std::f32::consts::PI * k as f32 / 6.0).sin_cos();
        out.push((p[n].0 + (-ty * co + tx * s) * hw, p[n].1 + (tx * co + ty * s) * hw));
    }
    for i in (0..=n).rev() {
        let (tx, ty, hw) = frame(i);
        out.push((p[i].0 + ty * hw, p[i].1 - tx * hw));
    }
    let (tx, ty, hw) = frame(0);
    for k in 1..6 {
        let (s, co) = (std::f32::consts::PI * k as f32 / 6.0).sin_cos();
        out.push((p[0].0 + (ty * co - tx * s) * hw, p[0].1 + (-tx * co - ty * s) * hw));
    }
    g.poly(&out, c, a);
}

fn mmss(ms: i64) -> String {
    let s = ms.max(0) / 1000;
    format!("{}:{:02}", s / 60, s % 60)
}

#[derive(Clone, Copy)]
enum Hit {
    Prev,
    Toggle,
    Next,
    Bar(f32, f32), // x 시작, 너비
    Line(u64),
    Page(i32), // 쪽 넘기기(-1 왼쪽, +1 오른쪽)
    Layout,    // 음악 화면 모양 바꾸기
    CalNav(i32), // 달력: 달 넘기기(0 = 오늘로)
    CalDay(i64), // 달력: 날 고르기
    CalEv(i64, usize), // 큰 달력: 그날 j 번째 일정 내용 보기(usize::MAX = 닫기)
    CalAdd,      // 달력: 일정 넣기 창
    CalLogin,    // 달력: 구글 계정 연결
    CalNaver,    // 달력: 네이버 캘린더 연결
    CalAcct,     // 달력: '＋ 계정' 고르기 판 열고 닫기
    NaverOpen,   // 네이버 안내: 보안설정 페이지 열기
    NaverInput,  // 네이버 안내: 아이디·비밀번호 창
    NaverClose,  // 네이버 안내 닫기
    EntField,    // 일정 넣기: 글자 칸(자판을 다시 잡는다)
    EntDay(usize),
    EntTime(usize),
    EntCal,      // 일정 넣기: 넣을 캘린더 바꾸기
    EntAcct(usize), // 일정 넣기: 이 계정에도 넣기 켜고 끄기(휠: 그 계정 안 캘린더 바꾸기)
    EntSeg(usize), // 일정 넣기: 날짜·시각 칸 하나
    EntSave,
    EntClose,
    Shuffle,
    Repeat,
    Queue,        // 다음 곡 목록 열고 닫기
    QItem(usize), // 다음 곡 목록의 한 줄(qrows 번호)
}

struct App {
    hwnd: HWND,
    g: Gfx,
    scale: f32, // 논리 1 = 화면 점 몇 개
    mon: RECT,
    cfg: Cfg,
    now: Arc<Mutex<Now>>,
    lyr: Arc<Mutex<Lyr>>,
    tx: mpsc::Sender<Cmd>,
    // 움직임
    w: Spring,
    h: Spring,
    fade: Spring,   // 닫힘 글자 바뀜 0→1
    scroll: Spring, // 가사 줄 넘어감
    tm: Spring,     // 0 = 얼굴만, 100 = 얼굴은 왼쪽 + 글자
    face: face::Face,
    born: Instant,
    bump_until: Instant,
    was_title: String,
    was_playing: bool,
    // 상태
    hover: bool,
    showing_lyric: bool,
    mouse: (f32, f32), // 논리 좌표. 밖이면 음수
    tracking: bool,
    hidden: bool,
    maxed: bool, // 앞 창이 화면을 꽉 채웠다(일하는 중)
    shy: bool,   // 〃 + 마우스가 가까이 왔다 → 작게 줄어든다
    note_shy: bool, // 알림이 떠 있는데 마우스가 다가와 접었다(멀어질 때까지 작게)
    last_pt: (i32, i32),
    quotes: Arc<Mutex<quotes::Quotes>>,
    q: Vec<quotes::Quote>,
    q_stamp: String,
    q_tried: String,
    q_seen: Option<Instant>,
    pct: Option<(String, Rgb)>, // 닫힘 시세 한 줄의 등락률(색을 따로 칠한다)
    typing: bool,
    page: usize,       // 펼친 화면의 쪽: 0 기본(음악·시계), 1 시세
    paged: Instant,    // 마지막으로 쪽을 넘긴 때(휠이 한 번에 여러 쪽 넘어가지 않게)
    arr: [Spring; 2],  // 좌우 손잡이에 마우스가 올라간 정도 0…1(살짝 튕긴다)
    ent: Option<Ent>,  // 노치에서 튀어나온 일정 넣기 칸
    drop: Spring,      // 그 칸이 물방울처럼 나오는 정도 0…1
    nb: Spring,        // 적는 동안 온 알림이 칸 아래로 볼록 나온 정도 0…1
    pgf: Spring,       // 쪽이 바뀔 때 내용이 다시 떠오르는 정도 0…1
    big: bool,         // 시세 쪽: 값까지 나오는 큰 판
    bg: Spring,        // 〃 0 작은 판 … 1 큰 판(몸통 높이와 같은 박자)
    meter: meter::Meter,
    bars: [f32; 28], // 소리 막대 높이 0…1
    qs: Vec<Spring>,   // 시세 구역 버튼마다 마우스가 올라간 정도 0…1
    wx: Arc<Mutex<weather::Weather>>,
    wxn: weather::Weather, // 지금 보여 주는 날씨
    cal: Arc<Mutex<gcal::Cal>>,
    caln: gcal::Cal, // 지금 보여 주는 달력
    cal_tx: mpsc::Sender<gcal::Cmd>,
    cal_month: i32,    // 이번 달에서 몇 달 넘겼나
    cal_big: bool,     // 달력을 눌러 크게 편 판(칸마다 일정 제목)
    acct_menu: bool, // '＋ 계정' 고르기 판이 열려 있다
    naver_guide: bool, // 네이버 연결 안내 판(연결되면 저절로 닫힘)
    naver_n: usize,    // 안내를 열 때 연결돼 있던 네이버 계정 수
    cal_pop: Option<(i64, usize)>, // 큰 달력에서 눌러 연 일정(날, 그날 몇 번째)
    cal_sel: i64,      // 고른 날
    cal_wait: Instant, // 브라우저에서 로그인하는 중(이때까지)
    notes: Arc<Mutex<notif::Note>>,
    note: notif::Note, // 지금 보여 주는(마지막) 알림
    note_until: Instant,
    note_icon: Option<ID2D1Bitmap>,
    open_h: f32, // 마지막으로 펼친 높이
    mode: u8,    // 펼친 내용: 0 음악, 1 시계, 2 알림
    last_input: u32,
    moved: Instant, // 마우스가 마지막으로 움직인 때
    keyed: Instant, // 마지막으로 자판을 치거나 클릭한 때
    ck: Spring,     // 0 = 고양이, 100 = 시계
    ticks: u32,
    n: Now,
    lines: Vec<(u64, String)>,
    lyr_st: St,
    lyr_stamp: String,
    lyr_ver: u64,
    cur: Option<usize>,
    instr: Option<&'static str>, // 가사 없이 소리만 나는 때(전주·간주·후주) — 가사 자리에 소리 막대
    text: String,
    prev_text: String,
    art: Option<ID2D1Bitmap>,
    art_ptr: usize,
    hits: Vec<([f32; 4], Hit)>,
    rgn: [i32; 4],
    hold: (f32, f32), // 펼쳐 둔 동안 가장 컸던 몸통 크기
    view: String,
    dirty: bool,
    first_frame: bool,
    queue: Arc<Mutex<queue::Queue>>,
    qn: queue::Queue,     // 지금 보여 주는 다음 곡 목록
    qopen: bool,          // 목록을 펼쳐 두었나
    qtop: usize,          // 목록을 휠로 몇 줄 내렸나
    qrect: [f32; 4],      // 목록 칸 자리(휠이 여기 있으면 목록만 넘긴다)
    qrows: Vec<(bool, String)>, // 그린 줄: (유튜브인가, 열쇠)
    qbmp: HashMap<usize, ID2D1Bitmap>, // 썸네일(바이트 주소별)
}

impl App {
    fn animating(&self) -> bool {
        !(self.w.settled() && self.h.settled() && self.fade.settled() && self.scroll.settled() && self.tm.settled() && self.ck.settled() && self.pgf.settled() && self.arr[0].settled() && self.arr[1].settled() && self.drop.settled() && self.nb.settled() && self.bg.settled() && self.qs.iter().all(Spring::settled) && self.face.settled())
    }

    /// 0.05초마다: 음악·가사 상태를 읽어 목표 크기와 보일 글자를 정한다.
    unsafe fn tick(&mut self) {
        self.ticks += 1;
        self.n = self.now.lock().unwrap().clone();
        if std::env::var("NOTCH_CAT_NOMEDIA").is_ok() {
            self.n.has = false; // 시험용: 음악이 없는 척(시계·날씨 쪽을 본다)
        }
        if std::env::var("NOTCH_CAT_SHOT").is_ok() {
            // 시험용: 곡이 흐르는 척(재생기가 없는 PC에서 음악 화면을 찍어 보려고)
            let t = self.born.elapsed().as_millis() as i64;
            self.n = media::Now { has: true, title: "GOOD DAY".into(), artist: "Forrest Frank · CHILD OF GOD".into(), playing: true, pos_ms: 11_000 + t, pos_at: None, dur_ms: 171_000, shuffle: Some(false), repeat: Some(2), seen: Some(Instant::now()), stamp: hhmm(), art: None, app: "test".into() };
            if self.lines.is_empty() {
                self.lines = [(0, "they say)"), (8_000, "The sun is shinin' down on me, birds are singing praise"), (60_000, "I'm 'bout to have a good day (good day) in every single way"), (70_000, "Tell me what you need"), (80_000, "끝")].iter().map(|(t, l)| (*t, l.to_string())).collect();
                self.lyr_st = St::Found;
            }
        }
        {
            // 다음 곡 목록: 바뀌었을 때만 옮기고 썸네일을 그림으로 만든다
            let q = self.queue.lock().unwrap();
            if q.ver != self.qn.ver || (std::env::var("NOTCH_CAT_SHOT").is_ok() && self.qn.local.is_empty()) {
                self.qn = q.clone();
                drop(q);
                if std::env::var("NOTCH_CAT_SHOT").is_ok() && std::env::var("NOTCH_CAT_QUEUE").is_ok() {
                    // 시험용 목록(재생기·유튜브가 없는 PC에서 그림을 찍어 보려고)
                    let it = |t: &str, c: &str| queue::Item { title: t.into(), sub: c.into(), key: t.into(), thumb: None };
                    self.qn.yt = vec![it("honeybee", "Olivia Rodrigo"), it("maggots for brains", "Olivia Rodrigo"), it("u + me = <3", "Olivia Rodrigo")];
                    self.qn.yt_kind = "재생목록".into();
                    self.qn.yt_seen = Some(Instant::now());
                    self.qn.yt_stamp = hhmm();
                    self.qn.local = (1..=9).map(|i| it(&format!("내 노래 {i}"), "가수")).collect();
                    self.qn.local_dir = "Music".into();
                    self.qn.local_stamp = hhmm();
                }
                for it in self.qn.yt.iter().chain(self.qn.local.iter()) {
                    if let Some(b) = &it.thumb {
                        let k = Arc::as_ptr(b) as usize;
                        if !self.qbmp.contains_key(&k) {
                            if let Ok(bm) = self.g.bitmap(b) {
                                self.qbmp.insert(k, bm);
                            }
                        }
                    }
                }
                if self.qbmp.len() > 400 {
                    self.qbmp.clear();
                }
                self.dirty = true;
            }
        }
        {
            let l = self.lyr.lock().unwrap();
            if l.ver != self.lyr_ver {
                self.lyr_ver = l.ver;
                self.lines = l.lines.clone();
                self.lyr_st = l.st;
                self.lyr_stamp = l.stamp.clone();
                self.cur = None;
            }
        }
        let ptr = self.n.art.as_ref().map(|a| Arc::as_ptr(a) as usize).unwrap_or(0);
        if ptr != self.art_ptr {
            self.art_ptr = ptr;
            self.art = self.n.art.as_ref().and_then(|b| self.g.bitmap(b).ok());
        }
        let pos = self.n.pos();
        // 노치를 켜기 전부터 나오던 알송 곡은 위치를 몰라 줄을 짚지 않는다(틀린 줄을 짚느니 안 짚는다).
        let unknown = self.n.app == "ALSong" && self.n.dur_ms == 0;
        let cur = if self.n.has && !unknown { lrc::current(&self.lines, pos.max(0) as u64) } else { None };
        self.instr = if unknown || !self.n.playing || self.lyr_st != St::Found || self.lines.is_empty() { None } else {
            instr_part(&self.lines, cur, pos.max(0) as u64)
        };
        if cur != self.cur {
            if let (Some(a), Some(b)) = (self.cur, cur) {
                if b == a + 1 {
                    self.scroll.pos += 1.0; // 한 줄 아래에서 올라온다
                }
            }
            self.cur = cur;
        }
        // 닫힘에 보일 한 줄: 재생 중이고 가사가 있으면 가사, 아니면 곡명
        let line = cur.map(|i| self.lines[i].1.as_str()).filter(|s| !s.is_empty());
        {
            let q = self.quotes.lock().unwrap();
            if q.tried != self.q_tried {
                self.q_tried = q.tried.clone();
                self.dirty = true;
            }
            if q.seen != self.q_seen {
                self.q = q.list.clone();
                self.q_stamp = q.stamp.clone();
                self.q_seen = q.seen;
                self.qs.resize(self.q.len(), Spring::new(0.0, 320.0, 0.5));
                self.dirty = true;
            }
        }
        if self.ticks % 20 == 1 {
            self.wxn = self.wx.lock().unwrap().clone();
            let c = self.cal.lock().unwrap();
            if c.ver != self.caln.ver {
                self.caln = c.clone();
                self.dirty = true;
            }
            drop(c);
            if std::env::var("NOTCH_CAT_SHOT").is_ok() && self.caln.accounts.is_empty() {
                // 시험용: 연결된 척, 일정이 있는 척
                let t = gcal::today().0;
                let ev = |d: i64, s: i32, e: i32, title: &str, place: &str, c: Rgb| gcal::Event { title: title.into(), place: place.into(), day: t + d, start: s, end_day: t + d, end: e, color: c, memo: if place.is_empty() { String::new() } else { "준비물: 노트북, 지난 회의록
끝나고 점심".into() } };
                let (blue, green, red) = ((0.26, 0.52, 0.96), (0.2, 0.71, 0.47), (0.92, 0.3, 0.26));
                let ci = |ac: usize, id: &str, name: &str, color: Rgb| gcal::CalInfo { account: ac, id: id.into(), name: name.into(), color, writable: true };
                self.caln = gcal::Cal { ready: true, accounts: vec!["demo@example.com".into(), "네이버 demo".into()],
                    cals: vec![ci(0, "demo@example.com", "demo@example.com", blue), ci(0, "x", "가족", red), ci(1, "n1", "내 캘린더", green), ci(1, "n2", "배당알림", red)], stamp: hhmm(), seen: Some(Instant::now()), ver: self.caln.ver,
                    events: vec![ev(0, -1, 0, "분배금 지급일", "", green), ev(0, 600, 630, "주간 회의", "3층 회의실", blue), ev(0, 840, 900, "정보부 협의회", "교무실", blue), ev(0, 1110, 1200, "저녁 약속", "의정부역", red), ev(1, 540, 600, "전산실 점검", "", blue), ev(3, -1, 0, "은경 생일", "", red), ev(6, 780, 840, "치과", "", green), ev(-2, 600, 660, "지난 일정", "", blue), ev(12, 600, 660, "학부모 상담", "", blue)],
                    ..Default::default() };
            }
        }
        if self.hover && self.page == 0 && self.mode == 0 && (self.cfg.ml == 1 || self.instr.is_some()) && self.n.playing {
            let t = self.born.elapsed().as_secs_f32();
            let lvl = self.meter.peak().filter(|_| std::env::var("NOTCH_CAT_SHOT").is_err()).map_or(0.55 + 0.2 * (t * 2.1).sin(), |v| (v * 1.8).min(1.0).sqrt()); // 소리를 못 읽으면 잔잔히 흔들기만
            let n = self.bars.len() as f32;
            for (i, b) in self.bars.iter_mut().enumerate() {
                let f = i as f32;
                let wob = 0.5 + 0.5 * (t * (2.3 + f * 0.37) + f * 1.7).sin() * (t * 1.3 + f * 0.9).cos();
                let want = lvl * (0.25 + 0.75 * wob) * (1.0 - 0.35 * f / n);
                *b = want.max(*b * 0.82); // 오를 땐 바로, 내릴 땐 천천히
            }
            self.dirty = true;
        } else if self.bars[0] != 0.0 {
            self.bars = [0.0; 28];
        }
        let mut text = match (self.n.has, self.n.playing, line) {
            (false, _, _) => String::new(),
            (true, true, Some(l)) => l.to_string(),
            (true, true, None) => self.n.title.clone(),
            _ => String::new(), // 멈춰 있으면 얼굴만
        };
        self.showing_lyric = self.n.has && self.n.playing && line.is_some();
        // 음악 글자가 없을 땐 시세를 한 종목씩 4초마다 돌려 보여 준다(자판을 치는 동안엔 시계가 먼저)
        self.pct = None;
        if text.is_empty() && self.cfg.bar && !self.typing && !self.q.is_empty() {
            let q = &self.q[(self.born.elapsed().as_secs() / 4) as usize % self.q.len()];
            text = format!("{}  {}", q.name, q.price);
            self.pct = Some((format!("{:+.2}%", q.pct), if q.pct > 0.0 { UP } else if q.pct < 0.0 { DOWN } else { FG }));
        }
        if text != self.text {
            self.prev_text = std::mem::replace(&mut self.text, text);
            self.fade.pos = 0.0;
            self.fade.target = 1.0;
        }
        if self.ticks % 10 == 0 {
            self.hidden = self.cfg.hide_fullscreen && fullscreen_in_front(self.hwnd, &self.mon);
            // 앞에 나온 창이 아니라 노치 밑에 실제로 깔린 창을 본다(다른 모니터에서 일해도 이 화면 탭은 가려지므로)
            let nx = self.mon.left + ((self.mon.right - self.mon.left) as f32 / 2.0 + if self.cfg.aside { 460.0 * self.scale } else { 0.0 }) as i32;
            let ny = self.mon.top + ((self.h.pos.max(BAR_H) + 30.0) * self.scale) as i32;
            let maxed = self.cfg.tiny && std::env::var("NOTCH_CAT_TINY").map_or(true, |v| v != "0") && maximized_under(self.hwnd, nx, ny).unwrap_or(self.maxed);
            self.maxed = maxed;
        }
        // ── 얼굴 ──
        let playing = self.n.has && self.n.playing;
        if self.n.has && self.n.title != self.was_title {
            self.was_title = self.n.title.clone();
            self.face.blink(); // 곡이 바뀌면 눈을 한 번 깜빡이고 살짝 커졌다 돌아온다
            self.bump_until = Instant::now() + Duration::from_millis(260);
        }
        if playing != self.was_playing {
            self.was_playing = playing;
            if playing { self.face.happy(650) } else { self.face.slow_blink(350) }
        }
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        if let Ok(dir) = std::env::var("NOTCH_CAT_SHOT") {
            // 시험용: 마우스 위치를 파일(mouse.txt "x,y")에서, 알림을 note.txt 가 생기면 하나 띄운다(원격이 끊겨 마우스를 못 움직이는 PC)
            let d = PathBuf::from(dir);
            if let Some((a, b)) = std::fs::read_to_string(d.join("mouse.txt")).ok().and_then(|t| t.trim().split_once(',').map(|(a, b)| (a.parse().unwrap_or(0), b.parse().unwrap_or(0)))) {
                pt = POINT { x: a, y: b };
            }
            if std::fs::remove_file(d.join("note.txt")).is_ok() {
                let mut n = self.notes.lock().unwrap();
                n.ver += 1;
                n.app = "캡처 도구".into();
                n.title = "캡처가 클립보드에 복사됨".into();
            }
        }
        let wpx = (WIN_W * self.scale).round();
        let dxw = if self.cfg.aside { 460.0 * self.scale } else { 0.0 };
        let left = self.mon.left as f32 + ((self.mon.right - self.mon.left) as f32 - wpx) / 2.0 + dxw;
        let (mx, my) = ((pt.x as f32 - left) / self.scale, (pt.y - self.mon.top) as f32 / self.scale); // 창 안 논리 좌표
        let (pw, ph) = (self.w.pos, self.h.pos);
        let px0 = (WIN_W - pw) / 2.0;
        let fcx = px0 + pw / 2.0 + (28.0 - pw / 2.0) * self.tm.pos / 100.0;
        let near = mx > px0 - 100.0 && mx < px0 + pw + 100.0 && my < ph + 100.0;
        self.face.tick((mx - fcx, my - 13.5), near, self.hover, playing);
        // 창을 꽉 채워 일하는 동안엔 아주 작게 줄어 있다가, 마우스가 가까이 오면 돌아온다
        // 자판을 치면(입력은 있는데 마우스는 제자리·단추도 안 눌림) 시계, 마우스를 쓰면 고양이로 돌아온다
        let mut li = windows::Win32::UI::Input::KeyboardAndMouse::LASTINPUTINFO { cbSize: std::mem::size_of::<windows::Win32::UI::Input::KeyboardAndMouse::LASTINPUTINFO>() as u32, dwTime: 0 };
        let _ = windows::Win32::UI::Input::KeyboardAndMouse::GetLastInputInfo(&mut li);
        use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
        let clicked = [1, 2, 4].iter().any(|&vk| GetAsyncKeyState(vk) != 0); // 마우스 단추(누르는 중이거나 방금 눌렀다)
        if (pt.x, pt.y) != self.last_pt || clicked {
            self.moved = Instant::now();
        } else if li.dwTime != self.last_input && self.moved.elapsed() > Duration::from_millis(400) {
            self.keyed = Instant::now(); // 마우스가 미세하게 떨린 것은 치지 않는다
            self.face.touch();
        }
        self.last_input = li.dwTime;
        self.last_pt = (pt.x, pt.y);
        let quiet = Duration::from_secs(8);
        self.typing = self.keyed.elapsed() < quiet && self.keyed > self.moved && std::env::var("NOTCH_CAT_SLEEP").is_err();
        let typing = self.typing;
        self.ck.target = if typing && self.text.is_empty() { 100.0 } else { 0.0 };
        // 꽉 찬 창에서 일할 때도 고양이는 보인다. 마우스가 탭 쪽으로 올라와 노치에서 5cm쯤(190) 안에 들어오면
        // 탭을 가리지 않게 아주 작게 줄고, 멀어지면(230) 돌아온다. 작은 노치에 마우스를 올리면 펼쳐진다.
        let far = ((mx - WIN_W / 2.0).powi(2) + my * my).sqrt();
        // 알림이 떠 있을 때도 같다: 마우스가 다가오면 알림을 접고 비켜 준다(화면 캡처 알림 등이 탭을 가리지 않게)
        let near = far < if self.shy { 230.0 } else { 190.0 };
        if near && !self.hover && Instant::now() < self.note_until {
            self.note_until = Instant::now();
            self.note_shy = true; // 마우스가 멀어질 때까지 작게 있는다
        }
        if !near {
            self.note_shy = false;
        }
        self.shy = (self.maxed || self.note_shy) && near;
        let tiny = self.shy && !self.hover;
        if tiny {
            self.face.tail.target = 0.0;
        }
        self.tm.target = if self.text.is_empty() || tiny { 0.0 } else { 100.0 };

        // ── 알림: 새로 오면 5초 동안 펼쳐 보여 준다 ──
        let fresh = {
            let n = self.notes.lock().unwrap();
            if n.ver != self.note.ver { Some(n.clone()) } else { None }
        };
        if let Some(n) = fresh {
            self.note_icon = n.icon.as_ref().and_then(|b| self.g.bitmap(b).ok());
            self.note = n;
            self.note_until = Instant::now() + Duration::from_secs(5);
            self.face.touch();
            self.face.blink();
            self.dirty = true;
        }
        let noting = Instant::now() < self.note_until;
        // 일정을 적는 동안엔 몸통을 그대로 두고(칸 자리가 틀어지지 않게) 알림은 칸 아래로 볼록 나와 알린다
        let up = self.ent_up();
        self.nb.target = if noting && up { 1.0 } else { 0.0 };
        let noting = noting && !up;

        let bump = if Instant::now() < self.bump_until { (6.0, 2.0) } else { (0.0, 0.0) };
        let (tw, th) = if self.hidden {
            (IDLE.0 * 0.6, 0.0)
        } else if noting {
            (NOTE_W, 54.0)
        } else if tiny {
            TINY
        } else if self.hover {
            if self.page == 3 {
                if self.naver_guide && self.caln.accounts.iter().filter(|a| a.starts_with("네이버 ")).count() > self.naver_n {
                    self.naver_guide = false; // 연결됐다
                }
                if self.naver_guide { (600.0, 262.0) } else if self.caln.accounts.is_empty() { (440.0, 140.0) } else if self.cal_big { CAL_BIG } else { CALV[self.cfg.cv] }
            } else if self.page == 2 {
                WX[self.cfg.wbig as usize]
            } else if self.page == 1 {
                q_layout(&self.q.iter().map(|q| q.group).collect::<Vec<_>>(), self.big as u8 as f32, self.cfg.cols()).2
            } else if self.n.has {
                (MUSIC[self.cfg.ml].0 + if self.qopen { QW } else { 0.0 }, MUSIC[self.cfg.ml].1)
            } else {
                (QUIET.0, QUIET.1 + if self.wxn.seen.is_some() { 34.0 } else { 0.0 }) // 날씨 한 줄
            }
        } else if self.text.is_empty() {
            (IDLE.0 + bump.0, IDLE.1 + bump.1)
        } else {
            // 닫힘은 작게: 곡명은 짧게 자르고, 가사 줄일 때만 조금 더 넓힌다
            let max = if self.showing_lyric || self.pct.is_some() { 300.0 } else { 230.0 };
            let pw = self.pct.as_ref().map_or(0.0, |p| self.g.measure(&p.0, &self.g.f_bar) + 8.0);
            ((self.g.measure(&self.text, &self.g.f_bar) + pw + 70.0).clamp(IDLE.0, max) + bump.0, BAR_H + bump.1)
        };
        // 펼칠 때는 끝에서 살짝 출렁이며 모이고(가로·세로 박자가 달라 물결처럼), 접힐 때는 차분하게
        if th > BAR_H + 10.0 {
            // 시세 쪽은 작은 판 높이를 '다 펼친 높이'로 삼는다(큰 판으로 자라는 동안 내용이 흐려지지 않게)
            self.open_h = if self.page == 1 && !noting { q_layout(&self.q.iter().map(|q| q.group).collect::<Vec<_>>(), 1.0, self.cfg.cols()).2 .1 } else { th };
            self.mode = if noting { 2 } else if self.page == 3 { 5 } else if self.page == 2 { 4 } else if self.page == 1 { 3 } else if self.n.has { 0 } else { 1 };
            self.w.tune(250.0, 0.62);
            self.h.tune(190.0, 0.68);
        } else {
            self.w.tune(240.0, 0.8);
            self.h.tune(240.0, 0.85);
        }
        self.w.target = tw;
        self.h.target = th;
        // 보이는 것이 달라졌을 때만 다시 그린다(쉴 때 CPU 를 안 쓴다)
        let open = self.h.target > BAR_H;
        let view = format!(
            "{}|{}|{}|{}|{}|{}|{:?}|{}|{}|{}|{}",
            self.wxn.stamp, local().wMinute, self.text, self.n.title, self.n.playing, self.art_ptr, self.cur, self.lyr_st as u8,
            if open { pos / 500 } else { 0 },
            if open { self.n.stamp.as_str() } else { "" },
            self.lyr_stamp
        );
        // 닫혀 있는 동안엔 빛·꼬리가 천천히 움직이므로 초당 20장으로 계속 그린다
        if view != self.view || (!open && !self.hidden) {
            self.view = view;
            self.dirty = true;
        }
    }

    unsafe fn draw(&mut self) -> Result<()> {
        let g = &self.g;
        let (w, h) = (self.w.pos.max(0.0), self.h.pos.max(0.0));
        let x = (WIN_W - w) / 2.0;
        // 노치 밖은 클릭이 아래 창으로 가도록 창 영역을 노치 크기로 맞춘다
        let s = self.scale;
        let p = ((h - BAR_H) / (self.open_h - BAR_H)).clamp(0.0, 1.0); // 0 닫힘 … 1 펼침
                let m = (SHADOW * p).max(EAR + 1.0).max(if self.face.tail.pos > 0.3 { TAIL_ROOM } else { 0.0 }); // 그림자·휜 모서리·꼬리 자리만큼 영역을 넓힌다
        // 펼쳐 둔 동안엔 영역을 줄이지 않는다: 쪽을 넘겨 몸통이 좁아지거나 낮아져도 마우스가 '밖'으로 떨어져 닫히지 않게
        self.hold = if self.hover { (self.hold.0.max(w), self.hold.1.max(h)) } else { (0.0, 0.0) };
        let (rw, rh) = (w.max(self.hold.0), h.max(self.hold.1));
        let (rw, rh) = if self.ent_up() || (self.ent.is_some() && self.drop.pos > 0.02) {
            let d = if self.ent_up() { self.drop.pos.max(1.0) } else { self.drop.pos };
            let nb = (ENT_GAP + NOTE_H) * self.nb.pos.max(0.0);
            (rw.max(ENT_W + 4.0), rh.max(h + ENT_GAP + ENT_H * d + nb + 6.0))
        } else {
            (rw, rh)
        };
        let rx = (WIN_W - rw) / 2.0;
        let rgn = if h < 0.5 { [0; 4] } else { [((rx - m) * s).floor() as i32, 0, ((rx + rw + m) * s).ceil() as i32, ((rh + m) * s).ceil() as i32] };
        if rgn != self.rgn {
            self.rgn = rgn;
            SetWindowRgn(self.hwnd, CreateRectRgn(rgn[0], rgn[1], rgn[2], rgn[3]), false);
        }
        self.hits.clear();
        g.begin();
        if h >= 0.5 {
            let r = (14.0 + 16.0 * p * ((self.open_h - BAR_H) / 90.0).clamp(0.2, 1.0)).min(h); // 낮게 펼칠수록 모서리도 작게
            for i in 0..8 {
                let gr = (8 - i) as f32 * 2.2; // 겹겹이 옅게 → 부드러운 그림자
                g.rrect(x - gr, -r, w + gr * 2.0, h + r + gr + 3.0, r + gr, (0.0, 0.0, 0.0), 0.045 * p);
            }
            let t = self.born.elapsed().as_secs_f32();
            let ca = (1.0 - p * 3.0).clamp(0.0, 1.0) * (h / BAR_H).min(1.0); // 닫힘 내용의 진하기
            let fa = ((h - 12.0) / 12.0).clamp(0.0, 1.0); // 아주 작게 줄면 얼굴은 사라진다
            let fk = 1.0 - (self.ck.pos / 100.0).clamp(0.0, 1.0); // 시계가 나오면 얼굴이 비킨다
            // 꼬리: 잠들면 옆에서 슬그머니 나오는 짧고 도톰한 물음표. 몸보다 먼저 그려서 뿌리가 몸 뒤에 숨는다.
            let reveal = (self.face.tail.pos / 100.0).clamp(0.0, 1.0);
            if reveal > 0.003 && ca * fa * fk > 0.0 {
                use std::f32::consts::{PI, TAU};
                let f = &self.face;
                let sd = f.tail_side;
                let edge = if sd > 0.0 { x + w } else { x };
                let (sw, fl, cu) = (f.tw[0].pos / 100.0, f.tw[1].pos / 100.0, f.tw[2].pos / 100.0);
                const P: [(f32, f32); 7] = [(140.0, 20.0), (151.0, 24.0), (158.0, 21.0), (158.0, 14.0), (157.0, 4.0), (171.0, 3.0), (168.0, 11.0)];
                let (wave, lag) = ((t * TAU / 5.6).sin(), (t * TAU / 5.6 - 0.7).sin()); // 끝이 중간보다 늦게 따라온다
                let ph = (t % 5.3) / 5.3 / 0.38;
                let flick = if ph < 1.0 { (ph * PI * 4.0).sin() * (ph * PI).sin().powi(2) } else { 0.0 }; // 두 번 까딱하고 한참 쉰다
                let curl = (t * TAU / 7.3).sin();
                let shift = (1.0 - reveal) * 45.0;
                let c: Vec<(f32, f32)> = P
                    .iter()
                    .enumerate()
                    .map(|(i, &(px, py))| {
                        let tt = i as f32 / 6.0;
                        let end = ((tt - 0.45) / 0.55).max(0.0).powi(2);
                        let (k5, k6) = (if i == 5 { 1.0 } else { 0.0 }, if i == 6 { 1.0 } else { 0.0 });
                        let xx = px + sw * (wave * tt * tt * 3.0 + lag * end * 0.7) + fl * flick * end * 1.8 + cu * curl * (1.8 * k5 - 2.2 * k6);
                        let yy = (py + sw * lag * end * 0.6 + fl * flick * end * 1.6 + cu * curl * (0.3 * k5 + 2.4 * k6)).max(3.0);
                        let xx = (if xx > 150.0 { 150.0 + (xx - 150.0) * 0.65 } else { xx }) - shift; // 밖으로 나온 길이만 줄인다(굵기는 그대로)
                        (edge + sd * (xx - 150.0), yy)
                    })
                    .collect();
                let mut pts = Vec::with_capacity(34);
                for seg in 0..2 {
                    let q = &c[seg * 3..seg * 3 + 4];
                    for i in seg..=16 {
                        let u = i as f32 / 16.0;
                        let v = 1.0 - u;
                        pts.push((
                            v * v * v * q[0].0 + 3.0 * v * v * u * q[1].0 + 3.0 * v * u * u * q[2].0 + u * u * u * q[3].0,
                            v * v * v * q[0].1 + 3.0 * v * v * u * q[1].1 + 3.0 * v * u * u * q[2].1 + u * u * u * q[3].1,
                        ));
                    }
                }
                ribbon(g, &pts, |u| 2.0 * (3.1 - 0.9 * u * u * (3.0 - 2.0 * u)), TAIL, ca * fa * fk);
            }
            g.rrect(x, -r, w, h + r, r, BG, 1.0); // 위쪽은 화면 밖으로 넘겨 위 모서리를 각지게
            // 위쪽 모서리가 화면 가장자리로 부드럽게 이어지게(바깥으로 휜다)
            let e = EAR * (h / BAR_H).min(1.0);
            for side in [-1.0f32, 1.0] {
                let ex = if side < 0.0 { x } else { x + w };
                let mut pts = vec![(ex + side * e, 0.0), (ex, 0.0)];
                for k in 0..=6 {
                    let th = k as f32 / 6.0 * std::f32::consts::FRAC_PI_2;
                    pts.push((ex + side * e - side * e * th.cos(), e - e * th.sin()));
                }
                g.poly(&pts, BG, 1.0);
            }
            g.clip(x, 0.0, w, h);
            let n = &self.n;
            let tm = (self.tm.pos / 100.0).clamp(0.0, 1.0);

            // ── 닫힘: 고양이 얼굴 ──
            if ca * fa * fk > 0.0 {
                use std::f32::consts::PI;
                let ca = ca * fa * fk;
                let f = &self.face;
                let st = &face::STYLES[f.style];
                let fcx = x + w / 2.0 + (28.0 - w / 2.0) * tm;
                // 아래 빛: 넓고 옅은 것부터 세 겹. 숨 쉬듯 아주 조금만 변한다.
                let gk = f.glow.pos / 100.0 * (1.0 + 0.12 * (t * 1.31).sin()) * (1.0 - 0.45 * tm) * 2.4;
                for (i, (gw, gh, ga)) in [(72.0, 14.0, 0.035), (52.0, 8.0, 0.06), (28.0, 4.0, 0.1)].into_iter().enumerate() {
                    g.ellipse(fcx, h - 2.0 + gh * 0.1, gw / 2.0, gh / 2.0, GLOW[i], ca * ga * gk);
                }
                g.zoom(FACE_ZOOM, fcx, 14.0); // 눈·코·입·수염을 한꺼번에 키운다
                let breath = if f.asleep { (t * 1.208).sin() * 0.7 } else { 0.0 };
                let (ox, oy) = (f.ex.pos, f.ey.pos + breath);
                let cy = 12.5 + oy;
                let smile = (f.ear.pos / 10.0).clamp(0.0, 1.0);
                let mix = (f.mix.pos / 100.0).clamp(0.0, 1.0);
                let (sx, sy) = (ox * 0.65, oy * 0.65); // 코·입·수염은 눈보다 덜 움직인다
                // 귀: 반가울 때만 잠깐
                let ea = (f.ear.pos / 10.0).clamp(0.0, 1.2);
                if ea > 0.02 {
                    for side in [-1.0f32, 1.0] {
                        let bx = fcx + side * 19.0 + ox * 0.5;
                        g.poly(&[(bx - 3.5, 6.0), (bx + side * 0.5, 6.0 - 4.0 * ea), (bx + 3.5, 6.0)], EYE, ca * 0.3 * ea.min(1.0));
                    }
                }
                // 수염: 이전 모양에서 지금 모양으로 가닥째 옮겨 간다. 웃으면 끝이 살짝 올라간다.
                let (wa, wb) = (&face::WHISKS[f.prev_whisk], &face::WHISKS[f.whisk]);
                let lime = wa.1 + (wb.1 - wa.1) * mix;
                let wc = (WHISK.0 + (EYE.0 - WHISK.0) * lime, WHISK.1 + (EYE.1 - WHISK.1) * lime, WHISK.2 + (EYE.2 - WHISK.2) * lime);
                for side in [-1.0f32, 1.0] {
                    for j in 0..3 {
                        let v: [f32; 6] = std::array::from_fn(|k| wa.0[j][k] + (wb.0[j][k] - wa.0[j][k]) * mix);
                        let p0 = (fcx + side * v[0] + sx, v[1] + sy);
                        let p1 = (fcx + side * v[2] + sx, v[3] + sy - 0.9 * smile);
                        let c = ((p0.0 + p1.0) / 2.0, (p0.1 + p1.1) / 2.0 - v[4]);
                        ribbon(g, &quad(p0, c, p1, 8), |u| 1.25 - 0.75 * u, wc, ca * v[5] * (0.6 + 0.12 * lime) * (1.0 - tm));
                    }
                }
                // 코와 입: 얼굴이 바뀔 땐 겹쳐서 넘어간다
                for (si, a) in [(f.prev_style, 1.0 - mix), (f.style, mix)] {
                    if a < 0.02 {
                        continue;
                    }
                    let s2 = &face::STYLES[si];
                    let (nx, a) = (fcx + sx, ca * 0.7 * a);
                    if s2.dot_nose {
                        g.ellipse(nx, 17.2 + sy, 1.2, 1.0, NOSE, a);
                    } else {
                        g.poly(&[(nx - 1.7, 16.4 + sy), (nx + 1.7, 16.4 + sy), (nx, 18.3 + sy)], NOSE, a);
                    }
                    let my = 18.4 + sy;
                    let m = |p0, c, p1, a: f32| ribbon(g, &quad(p0, c, p1, 8), |_| 1.05, NOSE, a);
                    if s2.mouth == 3 {
                        let (hw, y) = (3.3 + 0.6 * smile, my + 1.2 - 0.5 * smile);
                        m((nx - hw, y), (nx, my + 4.6 + 0.6 * smile), (nx + hw, y), a); // 한 줄 미소
                    } else {
                        for side in [-1.0f32, 1.0] {
                            let (sp, dip, end, aa) = match s2.mouth {
                                1 => (4.3, 3.6, 1.2, 1.0),
                                2 if side < 0.0 => (2.2, 2.4, 1.6, 0.35 + 0.65 * smile), // 한쪽 입꼬리: 웃으면 반대쪽도 나온다
                                2 => (4.6, 3.8, 0.8, 1.0),
                                _ => (3.2, 3.2, 1.6, 1.0),
                            };
                            m((nx, my), (nx + side * sp * 0.42, my + dip), (nx + side * (sp + 0.6 * smile), my + end - 0.8 * smile), a * aa);
                        }
                    }
                }
                for side in [-1.0f32, 1.0] {
                    let ex = fcx + side * 13.0 + ox;
                    if f.happy_shape {
                        // 웃는 눈 ∩: 보는 쪽으로 살짝 쏠리고, 그쪽 눈이 조금 넓다
                        let rise = (f.lid.pos / face::EYE_H).clamp(0.0, 1.2) * 3.4;
                        let hx = ex + f.px.pos * 0.6;
                        let hw = st.rx.max(5.5) - 0.5 + side * f.px.pos / 2.2 * 0.4;
                        let by = cy + 1.8 + f.py.pos * 0.5;
                        ribbon(g, &quad((hx - hw, by), (hx, by - 2.0 * rise), (hx + hw, by), 10), |_| 1.9, EYE, ca);
                    } else {
                        // 눈꺼풀이 위에서 덮는다(눈을 찌그러뜨리지 않는다)
                        let o = (f.lid.pos / face::EYE_H).clamp(0.0, 1.2);
                        let shut = (1.0 - o * st.open).max(0.0);
                        let full = 2.0 * st.ry + st.tilt;
                        let top = cy - st.ry - st.tilt * 0.5 + shut * full * 0.72;
                        let band = full * (1.0 - shut);
                        let ea = ca * (0.55 + 0.37 * o.min(1.0));
                        if band < 1.8 {
                            // 감은 눈 ∪. 잠들면 더 크고 진해지고, 바깥쪽에 속눈썹이 위로 돋고, 숨 쉬듯 조금씩 움직인다.
                            let sl = ((100.0 - f.glow.pos) / 60.0).clamp(0.0, 1.0); // 0 깜빡임 … 1 잠
                            let wob = (t * 1.208).sin();
                            let y = cy + 0.6;
                            let hw = st.rx - 0.5 + sl;
                            let dip = 3.2 + (1.8 + 0.6 * wob) * sl;
                            let a = ea.max(ca * 0.95 * sl);
                            ribbon(g, &quad((ex - hw, y), (ex, y + dip), (ex + hw, y), 10), |_| 1.6 + 0.6 * sl, EYE, a);
                            if sl > 0.05 {
                                for k in 0..3 {
                                    let kf = k as f32;
                                    let u = 0.5 + side * (0.14 + 0.16 * kf);
                                    let p = (ex - hw + 2.0 * hw * u, y + 2.0 * u * (1.0 - u) * dip);
                                    let (dx, len) = (side * (0.25 + 0.3 * kf) + 0.12 * wob, (4.0 - 0.4 * kf) * sl);
                                    let n = (dx * dx + 1.0).sqrt();
                                    ribbon(g, &[p, (p.0 + dx / n * len, p.1 - len / n)], |v| 1.0 - 0.5 * v, EYE, a);
                                }
                            }
                        } else {
                            g.clip(ex - st.rx - 1.0, top, st.rx * 2.0 + 2.0, band);
                            let e = 0.5 + 0.5 * st.point; // 0.5 = 타원, 1 = 양끝이 뾰족
                            let n = 14;
                            let mut pts = Vec::with_capacity(2 * n + 2);
                            for half in [-1.0f32, 1.0] {
                                for k in 0..=n {
                                    let u = half * (PI * k as f32 / n as f32).cos();
                                    pts.push((ex + u * st.rx, cy + half * st.ry * (1.0 - u * u).max(0.0).powf(e) - st.tilt * u * side));
                                }
                            }
                            g.poly(&pts, EYE, ea);
                            // 세로 눈동자가 눈보다 먼저 움직인다. 마우스를 올리면 동그래진다.
                            let pw = st.pw + (o - 1.0).max(0.0) * 4.0;
                            g.ellipse(ex + f.px.pos - ox - side * st.pin, cy + f.py.pos - oy, pw, st.ph, PUPIL, ea);
                            g.unclip();
                        }
                    }
                }
                g.zoom(1.0, 0.0, 0.0);
            }
            // 아주 작게 줄었을 땐 눈 두 점만 남는다
            let da = ca * (1.0 - fa) * (h / 6.0).min(1.0);
            if da > 0.02 {
                let f = &self.face;
                for side in [-1.0f32, 1.0] {
                    g.ellipse(x + w / 2.0 + side * 7.0 + f.px.pos * 0.5, h / 2.0 + 0.2, 1.4, 1.4 * (f.lid.pos / face::EYE_H).clamp(0.15, 1.0), EYE, da * 0.85);
                }
            }

            // ── 닫힘: 자판을 치는 동안엔 시계 ──
            let cka = ca * fa * (1.0 - fk);
            if cka > 0.01 {
                let t = local();
                let h12 = if t.wHour % 12 == 0 { 12 } else { t.wHour % 12 };
                let time = format!("{} {}:{:02}", if t.wHour < 12 { "오전" } else { "오후" }, h12, t.wMinute);
                let wx = self.wxn.seen.is_some();
                let sh = if wx { 10.0 } else { 0.0 };
                g.text(&time, &g.f_time, x + sh, 5.0 + fk * 4.0, w, FG, cka, Align::Center);
                if wx {
                    let ix = x + w / 2.0 + sh - g.measure(&time, &g.f_time) / 2.0 - 13.0;
                    g.zoom(0.62, ix, 14.0 + fk * 4.0);
                    wx_icon(g, self.wxn.kind, self.wxn.day, ix, 14.0 + fk * 4.0, cka);
                    g.zoom(1.0, 0.0, 0.0);
                }
            }

            // ── 닫힘: 한 줄(얼굴 오른쪽) ──
            let a = ca * fa * tm;
            if a > 0.0 {
                let dim = if n.has && !n.playing && self.pct.is_none() { 0.55 } else { 1.0 };
                let f = self.fade.pos.clamp(0.0, 1.0);
                let tw = (w - 68.0).max(1.0);
                let lw = (self.w.target - 68.0).max(1.0); // 목표 너비 기준으로 말줄임(움직이는 동안 글자가 흔들리지 않게)
                g.clip(x + 56.0, 0.0, tw, BAR_H + 4.0);
                g.text(&self.prev_text, &g.f_bar, x + 56.0, 5.5 - f * 6.0, lw, FG, a * dim * (1.0 - f), Align::Left);
                g.text(&self.text, &g.f_bar, x + 56.0, 5.5 + (1.0 - f) * 6.0, lw, FG, a * dim * f, Align::Left);
                if let Some((pct, c)) = &self.pct {
                    let px = x + 56.0 + g.measure(&self.text, &g.f_bar).min(lw) + 8.0;
                    g.text(pct, &g.f_bar, px, 5.5 + (1.0 - f) * 6.0, 80.0, *c, a * f, Align::Left);
                }
                g.unclip();
            }

            // ── 펼침: 음악 쪽 ──
            let a = ((p - 0.35) / 0.65).clamp(0.0, 1.0) * self.pgf.pos.clamp(0.0, 1.0);
            // 쪽 표시(위 가운데 점). 넘기는 손잡이는 몸통 옆에 따로 그린다.
            if p > 0.5 && self.mode != 2 {
                let pa = ((p - 0.5) * 2.0).clamp(0.0, 1.0);
                for i in 0..4 {
                    g.ellipse(x + w / 2.0 + (i as f32 - 1.5) * 12.0, 8.0, 2.5, 2.5, FG, pa * if i == self.page { 0.9 } else { 0.3 });
                }
            }
            // 쪽 넘김: 몸통 안쪽 양끝의 얇은 화살표. 평소엔 거의 안 보이고, 끝에 마우스를 대면 라임색으로 또렷해진다.
            if p > 0.5 && self.mode != 2 {
                let pa = ((p - 0.5) * 2.0).clamp(0.0, 1.0);
                let (mx, my) = self.mouse;
                let (ty, th) = (h * 0.5 - 34.0, 68.0);
                for (i, d) in [-1.0f32, 1.0].into_iter().enumerate() {
                    let hit = if d < 0.0 { [x, ty, x + 16.0, ty + th] } else { [x + w - 16.0, ty, x + w, ty + th] };
                    let on = p > 0.9 && mx >= hit[0] && mx < hit[2] && my >= hit[1] && my < hit[3];
                    self.arr[i].target = if on { 1.0 } else { 0.0 };
                    let k = self.arr[i].pos.clamp(0.0, 1.2);
                    if k > 0.02 {
                        g.rrect(hit[0] + 2.0, ty, 12.0, th, 6.0, FG, 0.07 * k * pa);
                    }
                    let cx = if d < 0.0 { x + 8.0 } else { x + w - 8.0 } + d * 1.5 * k;
                    let cy = ty + th / 2.0;
                    ribbon(g, &[(cx - d * 2.2, cy - 5.5), (cx + d * 2.2, cy), (cx - d * 2.2, cy + 5.5)], |_| 1.8, mix(FG, LIME, k.min(1.0)), (0.22 + 0.75 * k.min(1.0)) * pa);
                    if p > 0.9 {
                        self.hits.push((hit, Hit::Page(d as i32)));
                    }
                }
            }
            // 다음 곡 목록이 펼쳐지는 만큼 오른쪽을 떼어 둔다(음악 화면은 제 너비 그대로)
            let qw = if self.mode == 0 && n.has && self.page == 0 { (w - MUSIC[self.cfg.ml].0).clamp(0.0, QW) } else { 0.0 };
            let w = w - qw;
            if a > 0.0 && ((self.mode == 0 && n.has) || self.mode == 4 || (self.mode == 5 && !self.caln.accounts.is_empty() && !self.naver_guide)) {
                // 모양 바꾸기 단추(오른쪽 아래): 글자가 들어간 알약. 음악·날씨·달력 모두 같은 자리·같은 모양.
                let (mx, my) = self.mouse;
                let (bw, bh) = (64.0, 24.0);
                let (bx, by) = (x + w - bw - 12.0, h - bh - 10.0);
                let on = p > 0.9 && mx >= bx && mx < bx + bw && my >= by && my < by + bh;
                g.rrect(bx, by, bw, bh, bh / 2.0, LIME, a * if on { 0.32 } else { 0.16 });
                for d in [-1.0f32, 1.0] {
                    // 바깥을 가리키는 두 화살표(⤢)
                    let (cx, cy) = (bx + 15.0 + d * 3.5, by + bh / 2.0 - d * 3.5);
                    ribbon(g, &[(cx - d * 3.5, cy), (cx, cy), (cx, cy + d * 3.5)], |_| 1.6, LIME, a);
                }
                g.text(if self.mode == 5 && self.cal_big { "작게" } else { "모양" }, &g.f_bar, bx + 26.0, by + 4.0, bw - 30.0, LIME, a, Align::Left);
                if p > 0.9 {
                    self.hits.push(([bx, by, bx + bw, by + bh], Hit::Layout));
                }
            }
            if a > 0.0 && self.mode == 0 && n.has && self.cfg.ml != 0 {
                let live = p > 0.9;
                let (mx, my) = self.mouse;
                let wide = self.cfg.ml == 1;
                // 왼쪽: 앨범 그림 · 곡명 · 가수
                let (art, tx) = if wide { (68.0, x + 98.0) } else { (82.0, x + 116.0) };
                match &self.art {
                    Some(b) => g.art(b, x + 18.0, 16.0, art, 12.0, a),
                    None => g.rrect(x + 18.0, 16.0, art, art, 12.0, FG, a * 0.12),
                }
                let left = if wide { 250.0 } else { 300.0 }; // 왼쪽 묶음 너비(그림 포함)
                let cw = x + 18.0 + left - tx;
                g.text(&n.title, &g.f_title, tx, if wide { 14.0 } else { 26.0 }, cw, FG, a, Align::Left);
                g.text(&n.artist, &g.f_sub, tx, if wide { 38.0 } else { 52.0 }, cw, FG, a * 0.6, Align::Left);
                // 진행 막대와 시각
                let pos = n.pos();
                let (b0, bw, by) = if wide { (tx, cw, 64.0) } else { (x + 18.0, left, 112.0) };
                g.rrect(b0, by, bw, 4.0, 2.0, FG, a * 0.22);
                if n.dur_ms > 0 {
                    g.rrect(b0, by, (bw * pos as f32 / n.dur_ms as f32).clamp(4.0, bw), 4.0, 2.0, LIME, a);
                    if live {
                        self.hits.push(([b0, by - 8.0, b0 + bw, by + 10.0], Hit::Bar(b0, bw)));
                    }
                }
                g.text(&if n.app == "ALSong" && n.dur_ms == 0 { "–:––".into() } else { mmss(pos) }, &g.f_tiny, b0, by + 7.0, 50.0, FG, a * 0.6, Align::Left);
                g.text(&if n.dur_ms > 0 { mmss(n.dur_ms) } else { "–:––".into() }, &g.f_tiny, b0 + bw - 50.0, by + 7.0, 50.0, FG, a * 0.6, Align::Right);
                // 단추: (섞기) 이전 재생 다음 (반복)
                let cy = if wide { 102.0 } else { 152.0 };
                let mid = b0 + bw / 2.0;
                let gap: f32 = if wide { 40.0 } else { 52.0 };
                let mut row: Vec<(f32, Hit)> = vec![(-1.0, Hit::Prev), (0.0, Hit::Toggle), (1.0, Hit::Next)];
                if n.shuffle.is_some() {
                    row.insert(0, (-2.0, Hit::Shuffle));
                }
                if n.repeat.is_some() {
                    row.push((2.0, Hit::Repeat));
                }
                if wide {
                    // 와이드는 단추 줄이 좁다 → 목록 단추만 앨범 그림 아래 빈자리에
                    queue_btn(g, x + 18.0 + 34.0, cy, 15.0, self.qopen, a);
                    if live {
                        self.hits.push(([x + 37.0, cy - 15.0, x + 67.0, cy + 15.0], Hit::Queue));
                    }
                } else {
                    row.insert(0, (row[0].0 - 1.0, Hit::Queue));
                }
                // 단추가 늘면 간격을 줄여 왼쪽 묶음 안에 넣는다
                let gap = gap.min((bw - 24.0) / (row.len() as f32 - 1.0));
                let (k0, k1) = (row[0].0, row[row.len() - 1].0);
                let mid = mid - (k0 + k1) / 2.0 * gap;
                for (k, hit) in row {
                    let cx = mid + k * gap;
                    let rr = if matches!(hit, Hit::Toggle) { 19.0 } else { 15.0 };
                    if matches!(hit, Hit::Queue) {
                        queue_btn(g, cx, cy, rr, self.qopen, a);
                        if live {
                            self.hits.push(([cx - rr, cy - rr, cx + rr, cy + rr], hit));
                        }
                        continue;
                    }
                    let r = [cx - rr, cy - rr, cx + rr, cy + rr];
                    let over = live && mx >= r[0] && mx < r[2] && my >= r[1] && my < r[3];
                    if matches!(hit, Hit::Toggle) || over {
                        g.ellipse(cx, cy, rr, rr, FG, a * if over { 0.2 } else { 0.1 }); // 재생 단추는 늘 둥근 바탕
                    }
                    let tri = |x0: f32, d: f32, k: f32| g.poly(&[(x0, cy - k), (x0 + d * k * 1.6, cy), (x0, cy + k)], FG, a);
                    match hit {
                        Hit::Toggle if n.playing => {
                            g.rrect(cx - 6.0, cy - 7.0, 4.5, 14.0, 1.5, FG, a);
                            g.rrect(cx + 1.5, cy - 7.0, 4.5, 14.0, 1.5, FG, a);
                        }
                        Hit::Toggle => tri(cx - 4.5, 1.0, 7.5),
                        Hit::Next => {
                            tri(cx - 8.5, 1.0, 5.5);
                            tri(cx, 1.0, 5.5);
                        }
                        Hit::Prev => {
                            tri(cx + 8.5, -1.0, 5.5);
                            tri(cx, -1.0, 5.5);
                        }
                        Hit::Shuffle => {
                            // 엇갈린 두 화살표. 켜져 있으면 라임.
                            let c = if n.shuffle == Some(true) { LIME } else { FG };
                            ribbon(g, &quad((cx - 7.0, cy - 4.5), (cx, cy - 4.5), (cx + 6.0, cy + 4.5), 8), |_| 1.5, c, a);
                            ribbon(g, &quad((cx - 7.0, cy + 4.5), (cx, cy + 4.5), (cx + 6.0, cy - 4.5), 8), |_| 1.5, c, a);
                            for d in [-1.0f32, 1.0] {
                                g.poly(&[(cx + 8.0, cy + d * 4.5), (cx + 4.5, cy + d * 4.5 - 2.6), (cx + 4.5, cy + d * 4.5 + 2.6)], c, a);
                            }
                        }
                        Hit::Repeat => {
                            // 둥근 고리 + 화살표. 한 곡 반복이면 가운데 1.
                            let c = if n.repeat.unwrap_or(0) != 0 { LIME } else { FG };
                            let ring: Vec<(f32, f32)> = (0..=22).map(|i| {
                                let t = 0.35 + i as f32 / 22.0 * (std::f32::consts::TAU - 0.9);
                                (cx + 7.0 * t.cos(), cy + 5.0 * t.sin())
                            }).collect();
                            ribbon(g, &ring, |_| 1.5, c, a);
                            g.poly(&[(cx + 8.5, cy - 4.0), (cx + 3.5, cy - 7.2), (cx + 3.5, cy - 0.8)], c, a);
                            if n.repeat == Some(1) {
                                g.text("1", &g.f_tiny, cx - 4.0, cy - 6.5, 8.0, c, a, Align::Center);
                            }
                        }
                        _ => {}
                    }
                    if live {
                        self.hits.push((r, hit));
                    }
                }
                let rx = x + 18.0 + left + 26.0; // 오른쪽 칸
                let rw = x + w - 40.0 - rx;
                if wide {
                    // 소리 막대: 실제 소리 크기를 따라 오르내린다(아래에서 위로, 끝은 둥글게)
                    sound_bars(g, &self.bars, rx, 108.0, rw - 50.0, 74.0, a);
                } else {
                    // 가사: 지난 줄 · 지금 줄(크게) · 다음 줄 두 개. 긴 줄은 줄을 바꾼다.
                    g.text("가사", &g.f_tiny, rx, 14.0, 60.0, FG, a * 0.45, Align::Left);
                    g.clip(rx, 30.0, rw, h - 62.0);
                    if let Some(part) = self.instr {
                        g.text(part, &g.f_tiny, rx, 44.0, rw, LIME, a * 0.7, Align::Center);
                        sound_bars(g, &self.bars[..16], rx + rw * 0.15, 118.0, rw * 0.7, 52.0, a);
                    } else if self.lyr_st == St::Found && !self.lines.is_empty() {
                        let c = self.cur.map(|i| i as i32).unwrap_or(-1);
                        let mut y = 66.0 + self.scroll.pos * 26.0;
                        if c > 0 {
                            let prev = &self.lines[c as usize - 1].1;
                            let ph = g.para(prev, &g.f_body, rx, 0.0, rw, FG, 0.0);
                            g.para(prev, &g.f_body, rx, y - ph - 6.0, rw, FG, a * 0.3);
                        }
                        for i in c.max(0)..(c + 3).min(self.lines.len() as i32) {
                            let (t, l) = &self.lines[i as usize];
                            let now = i == c;
                            let lh = g.para(l, if now { &g.f_lyric } else { &g.f_body }, rx, y, rw, if now { FG } else { FG }, a * if now { 1.0 } else { 0.45 });
                            if live {
                                self.hits.push(([rx, y, rx + rw, y + lh], Hit::Line(*t)));
                            }
                            y += lh + 6.0;
                        }
                    } else {
                        let m = match self.lyr_st {
                            St::Missing => "시간 맞춘 가사가 없는 곡입니다",
                            St::Error => "가사를 못 받았습니다",
                            _ => "가사 찾는 중…",
                        };
                        g.text(m, &g.f_body, rx, 70.0, rw, FG, a * 0.5, Align::Left);
                    }
                    g.unclip();
                }
            } else if a > 0.0 && self.mode == 0 && n.has {
                let live = p > 0.9; // 다 펼쳐졌을 때만 눌린다
                let (mx, my) = self.mouse;
                match &self.art {
                    Some(b) => g.art(b, x + 18.0, 16.0, 60.0, 12.0, a),
                    None => g.rrect(x + 18.0, 16.0, 60.0, 60.0, 12.0, FG, a * 0.12),
                }
                g.text(&n.title, &g.f_title, x + 92.0, 22.0, w - 110.0, FG, a, Align::Left);
                g.text(&n.artist, &g.f_sub, x + 92.0, 48.0, w - 110.0, FG, a * 0.6, Align::Left);
                // 진행 막대
                let pos = n.pos();
                let (b0, bw) = (x + 66.0, w - 132.0);
                g.text(&if n.app == "ALSong" && n.dur_ms == 0 { "–:––".into() } else { mmss(pos) }, &g.f_time, x + 18.0, 87.0, 44.0, FG, a, Align::Left);
                g.text(&if n.dur_ms > 0 { mmss(n.dur_ms) } else { "–:––".into() }, &g.f_time, x + w - 62.0, 87.0, 44.0, FG, a, Align::Right);
                g.rrect(b0, 94.0, bw, 5.0, 2.5, FG, a * 0.22);
                if n.dur_ms > 0 {
                    g.rrect(b0, 94.0, (bw * pos as f32 / n.dur_ms as f32).clamp(5.0, bw), 5.0, 2.5, LIME, a);
                    if live {
                        self.hits.push(([b0, 84.0, b0 + bw, 108.0], Hit::Bar(b0, bw)));
                    }
                }
                // 목록 단추(왼쪽 끝)
                queue_btn(g, x + 40.0, 132.0, 20.0, self.qopen, a);
                if live {
                    self.hits.push(([x + 20.0, 112.0, x + 60.0, 152.0], Hit::Queue));
                }
                // 버튼 세 개(가운데). 마우스를 올리면 동그란 바탕이 켜진다.
                for (i, hit) in [Hit::Prev, Hit::Toggle, Hit::Next].into_iter().enumerate() {
                    let cx = x + w / 2.0 + (i as f32 - 1.0) * 62.0;
                    let cy = 132.0;
                    let r = [cx - 21.0, cy - 21.0, cx + 21.0, cy + 21.0];
                    if live && mx >= r[0] && mx < r[2] && my >= r[1] && my < r[3] {
                        g.rrect(r[0], r[1], 42.0, 42.0, 21.0, FG, a * 0.14);
                    }
                    let tri = |x0: f32, d: f32, k: f32| g.poly(&[(x0, cy - k), (x0 + d * k * 1.6, cy), (x0, cy + k)], FG, a);
                    match hit {
                        Hit::Toggle if n.playing => {
                            g.rrect(cx - 7.0, cy - 8.0, 5.0, 16.0, 1.5, FG, a);
                            g.rrect(cx + 2.0, cy - 8.0, 5.0, 16.0, 1.5, FG, a);
                        }
                        Hit::Toggle => tri(cx - 5.5, 1.0, 8.5),
                        Hit::Next => {
                            tri(cx - 10.0, 1.0, 6.5);
                            tri(cx, 1.0, 6.5);
                        }
                        _ => {
                            tri(cx + 10.0, -1.0, 6.5);
                            tri(cx, -1.0, 6.5);
                        }
                    }
                    if live {
                        self.hits.push((r, hit));
                    }
                }
                // 가사 세 줄(가운데가 지금 줄)
                const LH: f32 = 22.0;
                let (ly, lw) = (160.0, w - 36.0);
                g.clip(x + 18.0, ly, lw, LH * 3.0);
                if let Some(part) = self.instr {
                    g.text(part, &g.f_tiny, x + 18.0, ly + 2.0, lw, LIME, a * 0.7, Align::Center);
                    sound_bars(g, &self.bars[..16], x + 18.0 + lw * 0.3, ly + LH * 3.0 - 2.0, lw * 0.4, LH * 2.0 - 6.0, a);
                } else if self.lyr_st == St::Found {
                    let c = self.cur.map(|i| i as i32).unwrap_or(-1);
                    for i in (c - 2).max(0)..(c + 3).min(self.lines.len() as i32) {
                        let y = ly + LH + (i - c) as f32 * LH + self.scroll.pos * LH;
                        let (t, l) = &self.lines[i as usize];
                        let near = 1.0 - ((y - (ly + LH)).abs() / LH).min(1.0); // 가운데에 가까울수록 밝게
                        if i == c {
                            g.text(l, &g.f_lyric, x + 18.0, y, lw, LIME, a * (0.4 + 0.6 * near), Align::Center);
                        } else {
                            g.text(l, &g.f_body, x + 18.0, y + 1.0, lw, FG, a * (0.4 + 0.3 * near), Align::Center);
                        }
                        if live && y >= ly - 1.0 && y <= ly + LH * 2.0 + 1.0 {
                            self.hits.push(([x + 18.0, y, x + 18.0 + lw, y + LH], Hit::Line(*t)));
                        }
                    }
                } else {
                    let m = match self.lyr_st {
                        St::Missing => "시간 맞춘 가사가 없는 곡입니다",
                        St::Error => "가사를 못 받았습니다",
                        _ => "가사 찾는 중…",
                    };
                    g.text(m, &g.f_body, x + 18.0, ly + LH + 1.0, lw, FG, a * 0.5, Align::Center);
                }
                g.unclip();
                // 생존 표시: 마지막으로 받은 시각. 10분 넘게 못 받으면 경고색.
                let stale = n.seen.map(|t| t.elapsed() > Duration::from_secs(600)).unwrap_or(true) || self.lyr_st == St::Error;
                let alive = format!("음악 {} · 가사 {}", n.stamp, if self.lyr_stamp.is_empty() { "–" } else { &self.lyr_stamp });
                g.text(&alive, &g.f_tiny, x + 18.0, 227.0, w - 104.0, if stale { WARN } else { FG }, a * if stale { 1.0 } else { 0.4 }, Align::Right);
            }
            if qw > 40.0 && a > 0.0 {
                self.draw_queue(x + w, qw, h, a * ((qw - 40.0) / (QW - 40.0)).clamp(0.0, 1.0), p > 0.9);
            } else {
                self.qrect = [0.0; 4];
            }
            let g = &self.g;
            if a > 0.0 && self.mode == 2 {
                // 알림: 앱 그림 · 앱 이름 · 제목(· 내용)
                let nt = &self.note;
                match &self.note_icon {
                    Some(b) => g.art(b, x + 16.0, 12.0, 30.0, 7.0, a),
                    None => g.ellipse(x + 31.0, 27.0, 5.0, 5.0, LIME, a),
                }
                let tw = w - 76.0;
                g.text(&nt.app, &g.f_tiny, x + 58.0, 9.0, tw, FG, a * 0.55, Align::Left);
                g.text(if nt.title.is_empty() { &nt.body } else { &nt.title }, &g.f_lyric, x + 58.0, 22.0, tw, FG, a, Align::Left);
            } else if a > 0.0 && self.mode == 1 {
                // 음악이 없으면 시계와 날짜
                let t = local();
                let h12 = if t.wHour % 12 == 0 { 12 } else { t.wHour % 12 };
                let time = format!("{} {}:{:02}", if t.wHour < 12 { "오전" } else { "오후" }, h12, t.wMinute);
                let day = format!("{}월 {}일 {}요일", t.wMonth, t.wDay, ["일", "월", "화", "수", "목", "금", "토"][t.wDayOfWeek as usize % 7]);
                g.text(&time, &g.f_clock, x, 26.0, w, FG, a, Align::Center);
                g.text(&day, &g.f_sub, x, 66.0, w, FG, a * 0.6, Align::Center);
                let wx = &self.wxn;
                if let Some(seen) = wx.seen {
                    let label = format!("{}  {:.0}°", weather::NAMES[wx.kind], wx.temp);
                    let tw = g.measure(&label, &g.f_lyric);
                    let x0 = x + (w - tw - 34.0) / 2.0;
                    wx_icon(g, wx.kind, wx.day, x0 + 12.0, 106.0, a);
                    g.text(&label, &g.f_lyric, x0 + 34.0, 95.0, tw + 4.0, FG, a, Align::Left);
                    // 생존 표시: 마지막으로 받은 시각. 1시간 넘게 못 받으면 경고색.
                    let stale = seen.elapsed() > Duration::from_secs(3600);
                    g.text(&format!("날씨 {}", wx.stamp), &g.f_tiny, x + 14.0, 120.0, w - 28.0, if stale { WARN } else { FG }, a * if stale { 1.0 } else { 0.4 }, Align::Right);
                }
            } else if a > 0.0 && self.mode == 5 {
                // ── 달력 쪽: 구글 캘린더(한 주 줄 · 한 달 · 달력+목록) ──
                let cal = &self.caln;
                let live = p > 0.9;
                let (mx, my) = self.mouse;
                let (l, r) = (x + 20.0, x + w - 20.0);
                let (today, now) = gcal::today();
                let over = |b: [f32; 4]| live && mx >= b[0] && mx < b[2] && my >= b[1] && my < b[3];
                if self.naver_guide {
                    // 네이버 연결 안내: 처음 하는 사람도 따라 할 수 있게 순서대로
                    g.ellipse(l + 9.0, 28.0, 9.0, 9.0, NAVER, a);
                    g.text("N", &g.f_small, l, 19.0, 18.0, FG, a, Align::Center);
                    g.text("네이버 캘린더 연결", &g.f_title, l + 26.0, 15.0, 300.0, FG, a, Align::Left);
                    let steps = [
                        ("1", "네이버 보안설정에서 '2단계 인증'을 켭니다", "아래 '보안설정 열기' → 2단계 인증 관리 (이미 켜져 있으면 넘어가세요)"),
                        ("2", "'애플리케이션 비밀번호'를 하나 만듭니다", "2단계 인증 관리 → 애플리케이션 비밀번호 관리 → 기기 '기타' → 생성 → 나온 비밀번호 복사"),
                        ("3", "노치에 아이디와 그 비밀번호를 넣습니다", "'아이디·비밀번호 넣기' → 사용자 이름에 네이버 아이디, 암호에 복사한 비밀번호"),
                    ];
                    for (i, (n, head, sub)) in steps.iter().enumerate() {
                        let y = 54.0 + i as f32 * 42.0;
                        g.ellipse(l + 9.0, y + 9.0, 9.0, 9.0, NAVER, a * 0.9);
                        g.text(n, &g.f_tiny, l, y + 2.0, 18.0, FG, a, Align::Center);
                        g.text(head, &g.f_body, l + 26.0, y - 1.0, r - l - 26.0, FG, a, Align::Left);
                        g.text(sub, &g.f_tiny, l + 26.0, y + 19.0, r - l - 26.0, FG, a * 0.55, Align::Left);
                    }
                    g.text("로그인 비밀번호로는 연결되지 않습니다. 넣은 비밀번호는 이 PC 안에만 잠가 둡니다.", &g.f_tiny, l, 182.0, r - l, LIME, a * 0.8, Align::Left);
                    let by = 204.0;
                    let mut bx = l;
                    for (label, wdt, c, hit) in [("보안설정 열기", 128.0, NAVER, Hit::NaverOpen), ("아이디·비밀번호 넣기", 170.0, LIME, Hit::NaverInput), ("닫기", 64.0, FG, Hit::NaverClose)] {
                        let b = [bx, by, bx + wdt, by + 32.0];
                        g.rrect(b[0], b[1], wdt, 32.0, 16.0, c, a * if over(b) { 0.45 } else { 0.22 });
                        g.text(label, &g.f_small, b[0], b[1] + 7.0, wdt, FG, a, Align::Center);
                        if live {
                            self.hits.push((b, hit));
                        }
                        bx += wdt + 8.0;
                    }
                    if cal.note.starts_with("네이버") {
                        g.text(&cal.note, &g.f_tiny, l, h - 18.0, r - l, WARN, a, Align::Left);
                    }
                } else if cal.accounts.is_empty() {
                    g.text("달력", &g.f_title, l, 15.0, 120.0, FG, a, Align::Left);
                    if !cal.ready {
                        g.text("구글 캘린더 연결을 준비하는 중입니다 (등록증이 아직 없습니다)", &g.f_body, l, 60.0, w - 40.0, FG, a * 0.6, Align::Center);
                    } else if Instant::now() < self.cal_wait {
                        g.text("브라우저에서 계정을 고르고 '허용'을 눌러 주세요", &g.f_body, l, 60.0, w - 40.0, LIME, a, Align::Center);
                    } else {
                        let b = [x + w / 2.0 - 196.0, 56.0, x + w / 2.0 - 6.0, 92.0];
                        g.rrect(b[0], b[1], 190.0, 36.0, 18.0, LIME, a * if over(b) { 1.0 } else { 0.88 });
                        g.text("구글 캘린더 연결", &g.f_lyric, b[0], b[1] + 7.0, 190.0, PUPIL, a, Align::Center);
                        let nb = [x + w / 2.0 + 6.0, 56.0, x + w / 2.0 + 196.0, 92.0];
                        g.rrect(nb[0], nb[1], 190.0, 36.0, 18.0, NAVER, a * if over(nb) { 1.0 } else { 0.88 });
                        g.text("네이버 캘린더 연결", &g.f_lyric, nb[0], nb[1] + 7.0, 190.0, FG, a, Align::Center);
                        if live {
                            self.hits.push((b, Hit::CalLogin));
                            self.hits.push((nb, Hit::CalNaver));
                        }
                    }
                    if !cal.note.is_empty() {
                        g.text(&cal.note, &g.f_tiny, l, 108.0, w - 40.0, WARN, a, Align::Center);
                    }
                } else {
                    let mut menu_at: Option<f32> = None; // '＋ 계정' 단추의 왼쪽 끝(눌러 연 고르기 판 자리)
                    let view = if self.cal_big { 1 } else { self.cfg.cv }; // 큰 판은 머리를 한 달 모양처럼(달 넘기기)
                    // 머리: 고른 날 · (달력 모양이면) 보고 있는 달과 넘기는 단추 · 일정 넣기
                    let (sy, sm, sd) = gcal::ymd(self.cal_sel);
                    let head = format!("{sm}월 {sd}일 ({})", gcal::WEEK[gcal::weekday(self.cal_sel)]);
                    g.text(&head, &g.f_title, l, 15.0, 200.0, FG, a, Align::Left);
                    let hw = g.measure(&head, &g.f_title);
                    if self.cal_sel == today {
                        g.ellipse(l + hw + 9.0, 28.0, 3.0, 3.0, LIME, a);
                    }
                    if live {
                        self.hits.push(([l, 12.0, l + hw + 16.0, 40.0], Hit::CalNav(0))); // 날짜를 누르면 오늘로
                    }
                    // 계정 추가: 집·학교 계정을 같이 볼 수 있게. 마우스를 올리면 연결된 계정이 보인다.
                    let ax = l + hw + 22.0;
                    let ab = [ax, 16.0, ax + 66.0, 36.0];
                    let right_free = if view == 0 { r - 200.0 } else { r - 250.0 };
                    if ab[2] < right_free {
                        let ov = over(ab);
                        g.rrect(ab[0], ab[1], 66.0, 20.0, 10.0, FG, a * if ov || self.acct_menu { 0.2 } else { 0.09 });
                        g.text("＋ 계정", &g.f_tiny, ab[0], ab[1] + 3.0, 66.0, FG, a * 0.8, Align::Center);
                        if ov && !self.acct_menu {
                            g.text(&format!("연결됨: {}", cal.accounts.join(", ")), &g.f_tiny, l, 40.0, r - l, LIME, a, Align::Left);
                        }
                        if live {
                            self.hits.push((ab, Hit::CalAcct));
                        }
                        menu_at = Some(ab[0]);
                    }
                    // 보고 있는 달
                    let (ty, tm, _) = gcal::ymd(today);
                    let mi = ty * 12 + tm as i32 - 1 + self.cal_month;
                    let (vy, vm) = (mi.div_euclid(12), mi.rem_euclid(12) as u32 + 1);
                    let add = [r - 30.0, 13.0, r, 39.0];
                    g.ellipse(r - 15.0, 26.0, 13.0, 13.0, LIME, a * if over(add) { 1.0 } else { 0.88 });
                    g.rrect(r - 21.0, 25.0, 12.0, 2.0, 1.0, PUPIL, a);
                    g.rrect(r - 16.0, 20.0, 2.0, 12.0, 1.0, PUPIL, a);
                    if live {
                        self.hits.push((add, Hit::CalAdd));
                    }
                    let stale = cal.seen.map_or(true, |t| t.elapsed() > Duration::from_secs(1800));
                    if view == 0 {
                        g.text(&format!("{sy}년 · 달력 {}", cal.stamp), &g.f_tiny, r - 200.0, 20.0, 160.0, if stale { WARN } else { FG }, a * if stale { 1.0 } else { 0.45 }, Align::Right);
                    } else {
                        for (i, d) in [-1i32, 1].into_iter().enumerate() {
                            let b = [r - 96.0 + i as f32 * 28.0, 14.0, r - 72.0 + i as f32 * 28.0, 38.0];
                            g.ellipse(b[0] + 12.0, 26.0, 12.0, 12.0, FG, a * if over(b) { 0.2 } else { 0.09 });
                            let cx = b[0] + 12.0 + d as f32 * 1.5;
                            ribbon(g, &[(cx - d as f32 * 3.0, 21.0), (cx + d as f32 * 2.0, 26.0), (cx - d as f32 * 3.0, 31.0)], |_| 1.6, FG, a * 0.9);
                            if live {
                                self.hits.push((b, Hit::CalNav(d)));
                            }
                        }
                        g.text(&format!("{vy}년 {vm}월"), &g.f_lyric, r - 250.0, 17.0, 146.0, FG, a, Align::Right);
                    }
                    // 날 하나: 숫자(오늘은 라임 동그라미, 고른 날은 테두리) + 일정 점
                    let day_cell = |hits: &mut Vec<([f32; 4], Hit)>, z: i64, cx: f32, cy: f32, cw: f32, big: bool, dimmed: bool| {
                        let (_, _, d) = gcal::ymd(z);
                        let wd = gcal::weekday(z);
                        let rr = if big { 15.0 } else { 12.0 };
                        let c = if z == today { PUPIL } else if wd == 0 { mix(FG, UP, 0.6) } else if wd == 6 { mix(FG, DOWN, 0.6) } else { FG };
                        if z == today {
                            g.ellipse(cx, cy, rr, rr, LIME, a);
                        } else if z == self.cal_sel {
                            g.ellipse(cx, cy, rr, rr, FG, a * 0.9);
                            g.ellipse(cx, cy, rr - 1.5, rr - 1.5, BG, a);
                        }
                        g.text(&d.to_string(), if big { &g.f_lyric } else { &g.f_body }, cx - cw / 2.0, cy - if big { 10.5 } else { 9.0 }, cw, c, a * if dimmed && z != today { 0.3 } else { 1.0 }, Align::Center);
                        let evs = cal.on(z);
                        let n = evs.len().min(3);
                        for (i, e) in evs.iter().take(3).enumerate() {
                            g.ellipse(cx + (i as f32 - (n as f32 - 1.0) / 2.0) * 6.0, cy + rr + 4.5, 2.0, 2.0, e.color, a * if dimmed { 0.4 } else { 1.0 });
                        }
                        if live {
                            hits.push(([cx - cw / 2.0, cy - rr - 2.0, cx + cw / 2.0, cy + rr + 8.0], Hit::CalDay(z)));
                        }
                    };
                    // 일정 한 줄: 색 점 · (날) 시각 · 제목 · 곳. 끝난 일정은 흐리게 하고 제목에 줄을 긋는다.
                    let row_at = |e: &gcal::Event, x0: f32, y: f32, rw: f32, tall: bool, with_day: bool| {
                        let mut time = if e.start < 0 { "종일".to_string() } else { format!("{:02}:{:02}", e.start / 60, e.start % 60) };
                        if with_day {
                            time = format!("{} {} {time}", gcal::ymd(e.day).2, gcal::WEEK[gcal::weekday(e.day)]);
                        }
                        let tw = if with_day { 86.0 } else { 47.0 };
                        let gone = e.end_day < today || (e.end_day == today && e.start >= 0 && e.end <= now);
                        let k = if gone { 0.45 } else { 1.0 };
                        g.rrect(x0, y + 3.0, 3.0, if tall { 30.0 } else { 13.0 }, 1.5, e.color, a * k);
                        g.text(&time, &g.f_time, x0 + 11.0, y, tw, FG, a * k * 0.9, Align::Left);
                        g.text(&e.title, &g.f_body, x0 + 11.0 + tw, y - 0.5, rw - 11.0 - tw, FG, a * k, Align::Left);
                        if gone {
                            let lw = g.measure(&e.title, &g.f_body).min(rw - 11.0 - tw);
                            g.rrect(x0 + 11.0 + tw - 1.0, y + 9.5, lw + 2.0, 1.2, 0.6, FG, a * 0.7);
                        }
                        if tall {
                            let sub = if e.start < 0 { e.place.clone() } else { format!("{:02}:{:02}{}", e.end / 60, e.end % 60, if e.place.is_empty() { String::new() } else { format!(" · {}", e.place) }) };
                            g.text(&sub, &g.f_tiny, x0 + 11.0, y + 19.0, rw - 11.0, FG, a * k * 0.55, Align::Left);
                        }
                    };
                    let row = |e: &gcal::Event, x0: f32, y: f32, rw: f32, tall: bool| row_at(e, x0, y, rw, tall, false);
                    let mut hits = Vec::new();
                    let mut pop_hit = None;
                    if self.cal_big {
                        // 큰 판: 한 달 여섯 줄, 칸마다 숫자 아래 일정 제목(넘치면 '+n')
                        let cw = (r - l) / 7.0;
                        for (i, name) in gcal::WEEK.iter().enumerate() {
                            let c = if i == 0 { mix(FG, UP, 0.6) } else if i == 6 { mix(FG, DOWN, 0.6) } else { FG };
                            g.text(name, &g.f_tiny, l + cw * i as f32, 46.0, cw, c, a * 0.6, Align::Center);
                        }
                        let (y0, bottom) = (64.0, h - 44.0);
                        let rh = (bottom - y0) / 6.0;
                        let first = gcal::days(vy, vm, 1);
                        let start = first - gcal::weekday(first) as i64;
                        for i in 0..42 {
                            let z = start + i as i64;
                            let (cx0, cy0) = (l + cw * (i % 7) as f32, y0 + rh * (i / 7) as f32);
                            let cell = [cx0 + 1.5, cy0 + 1.5, cx0 + cw - 1.5, cy0 + rh - 1.5];
                            let other = gcal::ymd(z).1 != vm;
                            let k = if other { 0.35 } else { 1.0 };
                            let sel = z == self.cal_sel;
                            g.rrect(cell[0], cell[1], cell[2] - cell[0], cell[3] - cell[1], 8.0, if sel { LIME } else { FG }, a * if sel { 0.14 } else if over(cell) { 0.09 } else { 0.04 });
                            let wd = gcal::weekday(z);
                            let (_, _, d) = gcal::ymd(z);
                            let nc = if z == today { PUPIL } else if wd == 0 { mix(FG, UP, 0.6) } else if wd == 6 { mix(FG, DOWN, 0.6) } else { FG };
                            if z == today {
                                g.ellipse(cell[0] + 14.0, cell[1] + 12.0, 10.0, 10.0, LIME, a);
                            }
                            g.text(&d.to_string(), &g.f_small, cell[0] + 2.0, cell[1] + 4.0, 24.0, nc, a * if z == today { 1.0 } else { k }, Align::Center);
                            let evs = cal.on(z);
                            let lh = 14.5;
                            let fit = (((cell[3] - cell[1]) - 24.0) / lh).floor().max(0.0) as usize;
                            let more = evs.len() > fit;
                            let take = if more { fit.saturating_sub(1) } else { evs.len() };
                            for (j, e) in evs.iter().take(take).enumerate() {
                                let ey = cell[1] + 22.0 + j as f32 * lh;
                                let gone = e.end_day < today || (e.end_day == today && e.start >= 0 && e.end <= now);
                                let ek = k * if gone { 0.5 } else { 1.0 };
                                g.rrect(cell[0] + 4.0, ey + 1.0, cell[2] - cell[0] - 8.0, lh - 2.0, 3.0, e.color, a * 0.22 * ek);
                                g.rrect(cell[0] + 4.0, ey + 1.0, 2.5, lh - 2.0, 1.2, e.color, a * ek);
                                let t = if e.start >= 0 && e.day == z { format!("{}:{:02} {}", e.start / 60, e.start % 60, e.title) } else { e.title.clone() };
                                g.text(&t, &g.f_tiny, cell[0] + 9.0, ey + 0.5, cell[2] - cell[0] - 13.0, FG, a * ek, Align::Left);
                                if live {
                                    hits.push(([cell[0] + 4.0, ey, cell[2] - 4.0, ey + lh], Hit::CalEv(z, j)));
                                }
                            }
                            if more {
                                let my_ = cell[1] + 22.0 + take as f32 * lh;
                                g.text(&format!("+{}", evs.len() - take), &g.f_tiny, cell[0] + 9.0, my_, 40.0, FG, a * 0.55 * k, Align::Left);
                                if live {
                                    hits.push(([cell[0] + 4.0, my_, cell[2] - 4.0, my_ + lh], Hit::CalEv(z, take)));
                                }
                            }
                            if live {
                                hits.push((cell, Hit::CalDay(z)));
                            }
                        }
                        // 누른 일정의 내용: 칸 옆에 카드(제목 전체·날짜와 시각·장소·설명)
                        if let Some((pz, pj)) = self.cal_pop {
                            let evs = cal.on(pz);
                            let i = pz - start;
                            if let (Some(e), true) = (evs.get(pj), (0..42).contains(&i)) {
                                let (cx0, cy0) = (l + cw * (i % 7) as f32, y0 + rh * (i / 7) as f32);
                                let pw = 300.0;
                                let px = if cx0 + cw + pw + 6.0 <= r { cx0 + cw + 6.0 } else { cx0 - pw - 6.0 };
                                let px = px.max(l).min(r - pw);
                                let iw = pw - 36.0;
                                let memo: String = e.memo.chars().take(300).collect();
                                let th = g.para(&e.title, &g.f_body, 0.0, 0.0, iw, FG, 0.0).max(18.0);
                                let mh = if memo.is_empty() { 0.0 } else { g.para(&memo, &g.f_small, 0.0, 0.0, iw, FG, 0.0) + 8.0 };
                                let ph = (14.0 + th + 6.0 + 20.0 + if e.place.is_empty() { 0.0 } else { 20.0 } + mh + if evs.len() > 1 { 20.0 } else { 0.0 } + 12.0).min(bottom - y0);
                                let py = cy0.max(y0).min(bottom - ph);
                                let dl = |z: i64| { let (_, m, d) = gcal::ymd(z); format!("{m}월 {d}일 ({})", gcal::WEEK[gcal::weekday(z)]) };
                                let hm = |m: i32| format!("{}:{:02}", m / 60, m % 60);
                                let when = if e.start < 0 {
                                    if e.end_day > e.day { format!("{} – {} · 종일", dl(e.day), dl(e.end_day)) } else { format!("{} · 종일", dl(e.day)) }
                                } else if e.end_day > e.day {
                                    format!("{} {} – {} {}", dl(e.day), hm(e.start), dl(e.end_day), hm(e.end))
                                } else {
                                    format!("{} {} – {}", dl(e.day), hm(e.start), hm(e.end))
                                };
                                g.rrect(px - 1.5, py - 1.5, pw + 3.0, ph + 3.0, 13.5, e.color, a * 0.7);
                                g.rrect(px, py, pw, ph, 12.0, BG, a);
                                g.rrect(px, py, pw, ph, 12.0, e.color, a * 0.12);
                                g.rrect(px + 13.0, py + 15.0, 4.0, th - 2.0, 2.0, e.color, a);
                                let mut yy = py + 12.0;
                                g.para(&e.title, &g.f_body, px + 24.0, yy, iw, FG, a);
                                yy += th + 6.0;
                                g.text(&when, &g.f_small, px + 24.0, yy, iw, LIME, a, Align::Left);
                                yy += 20.0;
                                if !e.place.is_empty() {
                                    g.text(&format!("장소  {}", e.place), &g.f_small, px + 24.0, yy, iw, FG, a * 0.8, Align::Left);
                                    yy += 20.0;
                                }
                                if !memo.is_empty() {
                                    g.para(&memo, &g.f_small, px + 24.0, yy + 4.0, iw, FG, a * 0.7);
                                    yy += mh;
                                }
                                if evs.len() > 1 {
                                    g.text(&format!("{}/{} · 누르면 다음 일정", pj + 1, evs.len()), &g.f_tiny, px + 24.0, yy + 2.0, iw, FG, a * 0.45, Align::Left);
                                }
                                pop_hit = Some(([px, py, px + pw, py + ph], Hit::CalEv(pz, if evs.len() > 1 { (pj + 1) % evs.len() } else { usize::MAX })));
                            }
                        }
                        g.text(&format!("달력 {}", cal.stamp), &g.f_tiny, l, h - 20.0, 120.0, if stale { WARN } else { FG }, a * if stale { 1.0 } else { 0.4 }, Align::Left);
                    } else if view == 0 {
                        // 한 주 줄: 고른 날을 가운데 두고 앞뒤 나흘
                        let cw = (r - l) / 9.0;
                        for i in 0..9 {
                            let z = self.cal_sel - 4 + i as i64;
                            let cx = l + cw * (i as f32 + 0.5);
                            day_cell(&mut hits, z, cx, 66.0, cw, true, false);
                            g.text(gcal::WEEK[gcal::weekday(z)], &g.f_tiny, cx - cw / 2.0, 91.0, cw, FG, a * 0.5, Align::Center);
                        }
                        // 그날 일정 세 줄
                        let evs = cal.on(self.cal_sel);
                        if evs.is_empty() {
                            g.text(if self.cal_sel == today { "오늘은 일정이 없습니다" } else { "일정이 없는 날입니다" }, &g.f_body, l, 122.0, r - l, FG, a * 0.5, Align::Left);
                        }
                        // 오늘이면 아직 안 끝난 일정부터
                        let skip = if self.cal_sel == today && evs.len() > 3 { evs.iter().position(|e| e.start < 0 || e.end > now).unwrap_or(0).min(evs.len() - 3) } else { 0 };
                        for (i, e) in evs.iter().skip(skip).take(3).enumerate() {
                            row(e, l, 114.0 + i as f32 * 21.0, r - l - 70.0, false);
                        }
                        if evs.len() > 3 {
                            g.text(&format!("모두 {}개", evs.len()), &g.f_tiny, r - 110.0, 160.0, 76.0, FG, a * 0.5, Align::Right);
                        }
                    } else {
                        // 한 달(여섯 줄 고정). 달력+목록 모양은 왼쪽에 작게, 오른쪽에 고른 날 일정.
                        let (gw, rh, y0) = if view == 1 { (r - l, 42.0, 78.0) } else { (290.0, 33.0, 76.0) };
                        let cw = gw / 7.0;
                        for (i, name) in gcal::WEEK.iter().enumerate() {
                            let c = if i == 0 { mix(FG, UP, 0.6) } else if i == 6 { mix(FG, DOWN, 0.6) } else { FG };
                            g.text(name, &g.f_tiny, l + cw * i as f32, 48.0, cw, c, a * 0.6, Align::Center);
                        }
                        let first = gcal::days(vy, vm, 1);
                        let start = first - gcal::weekday(first) as i64;
                        for i in 0..42 {
                            let z = start + i as i64;
                            let other = gcal::ymd(z).1 != vm;
                            day_cell(&mut hits, z, l + cw * ((i % 7) as f32 + 0.5), y0 + (i / 7) as f32 * rh, cw, view == 1, other);
                        }
                        if view == 2 {
                            let lx = l + gw + 22.0;
                            g.rrect(lx - 12.0, 50.0, 1.0, h - 70.0, 0.5, FG, a * 0.12);
                            // 위: 고른 날 일정 · 아래: 그 주(일~토)의 나머지 날 일정
                            let rw = r - lx - 8.0;
                            let bottom = h - 28.0;
                            let mut y = 50.0;
                            let evs = cal.on(self.cal_sel);
                            g.text(if self.cal_sel == today { "오늘" } else { "고른 날" }, &g.f_tiny, lx, y, 80.0, LIME, a * 0.8, Align::Left);
                            y += 18.0;
                            if evs.is_empty() {
                                g.text("일정이 없습니다", &g.f_body, lx, y, rw, FG, a * 0.5, Align::Left);
                                y += 22.0;
                            }
                            let mut shown = 0;
                            for e in evs.iter() {
                                if y + 21.0 > bottom - 40.0 {
                                    break;
                                }
                                row_at(e, lx, y, rw, false, false);
                                y += 21.0;
                                shown += 1;
                            }
                            if evs.len() > shown {
                                g.text(&format!("외 {}개", evs.len() - shown), &g.f_tiny, lx + 11.0, y, 120.0, FG, a * 0.5, Align::Left);
                                y += 16.0;
                            }
                            let wk0 = self.cal_sel - gcal::weekday(self.cal_sel) as i64;
                            let week: Vec<&gcal::Event> = (wk0..wk0 + 7).filter(|z| *z != self.cal_sel).flat_map(|z| cal.on(z).into_iter().filter(move |e| e.day == z || z == wk0)).collect();
                            y += 8.0;
                            g.rrect(lx, y, rw, 1.0, 0.5, FG, a * 0.1);
                            y += 8.0;
                            g.text("이번 주", &g.f_tiny, lx, y, 80.0, LIME, a * 0.8, Align::Left);
                            y += 18.0;
                            if week.is_empty() {
                                g.text("다른 날 일정은 없습니다", &g.f_body, lx, y, rw, FG, a * 0.5, Align::Left);
                            }
                            let mut shown = 0;
                            for e in week.iter() {
                                if y + 21.0 > bottom {
                                    break;
                                }
                                row_at(e, lx, y, rw, false, true);
                                y += 21.0;
                                shown += 1;
                            }
                            if week.len() > shown {
                                g.text(&format!("외 {}개", week.len() - shown), &g.f_tiny, r - 170.0, h - 24.0, 96.0, FG, a * 0.5, Align::Right);
                            }
                            g.text(&format!("달력 {}", cal.stamp), &g.f_tiny, l, h - 20.0, 120.0, if stale { WARN } else { FG }, a * if stale { 1.0 } else { 0.4 }, Align::Left);
                        }
                    }
                    self.hits.extend(hits);
                    if let Some(p) = pop_hit.filter(|_| live) {
                        self.hits.insert(0, p); // 카드가 칸 위에 있으니 먼저 잡는다
                    }
                    // '＋ 계정'을 누르면: 구글 / 네이버 고르기(달력 위에 덮는다)
                    if let (Some(mx0), true) = (menu_at, self.acct_menu) {
                        let (mw, rh) = (170.0, 30.0);
                        g.rrect(mx0 - 1.0, 39.0, mw + 2.0, rh * 2.0 + 10.0, 12.0, FG, a * 0.25);
                        g.rrect(mx0, 40.0, mw, rh * 2.0 + 8.0, 11.0, BG, a);
                        for (k, (label, c, hit)) in [("구글 계정 연결", LIME, Hit::CalLogin), ("네이버 캘린더 연결", NAVER, Hit::CalNaver)].into_iter().enumerate() {
                            let b = [mx0 + 4.0, 44.0 + k as f32 * rh, mx0 + mw - 4.0, 44.0 + k as f32 * rh + rh - 2.0];
                            g.rrect(b[0], b[1], b[2] - b[0], b[3] - b[1], 8.0, c, a * if over(b) { 0.3 } else { 0.0 });
                            g.ellipse(b[0] + 12.0, (b[1] + b[3]) / 2.0, 4.0, 4.0, c, a);
                            g.text(label, &g.f_small, b[0] + 24.0, b[1] + 6.0, mw - 32.0, FG, a, Align::Left);
                            if live {
                                self.hits.insert(k, (b, hit));
                            }
                        }
                    }
                    if !cal.note.is_empty() {
                        g.text(&cal.note, &g.f_tiny, l, h - 18.0, w - 80.0, WARN, a, Align::Center);
                    }
                }
            } else if a > 0.0 && self.mode == 4 {
                // ── 날씨 쪽: 지금 있는 곳의 날씨(큰 판 / 작은 판) ──
                let wx = &self.wxn;
                let big = self.cfg.wbig;
                let (l, r) = (x + 20.0, x + w - 20.0);
                let dim = |k: f32| a * k;
                if wx.seen.is_none() {
                    g.text("날씨를 받는 중…", &g.f_body, l, 50.0, w - 40.0, FG, dim(0.5), Align::Center);
                } else {
                    // 위 줄: 동네 · (큰 판이면) 일출·일몰 · 받은 시각
                    let stale = wx.seen.map_or(true, |t| t.elapsed() > Duration::from_secs(3600));
                    g.text(if wx.place.is_empty() { "지금 있는 곳" } else { &wx.place }, &g.f_tiny, l, 12.0, 220.0, FG, dim(0.6), Align::Left);
                    let mut top = format!("날씨 {}", wx.stamp);
                    if !wx.sunrise.is_empty() {
                        top = format!("일출 {} · 일몰 {}   {top}", wx.sunrise, wx.sunset);
                    }
                    g.text(&top, &g.f_tiny, r - 300.0, 12.0, 300.0, if stale { WARN } else { FG }, dim(if stale { 1.0 } else { 0.5 }), Align::Right);
                    // 지금: 그림 · 기온 · 하늘 · 최고/최저
                    let (iy, ik) = if big { (62.0, 1.9) } else { (44.0, 1.25) };
                    g.zoom(ik, l + 22.0, iy);
                    wx_icon(g, wx.kind, wx.day, l + 22.0, iy, a);
                    g.zoom(1.0, 0.0, 0.0);
                    let tx = l + if big { 58.0 } else { 44.0 };
                    let temp = format!("{:.0}°", wx.temp);
                    let tf = if big { &g.f_clock } else { &g.f_title };
                    g.text(&temp, tf, tx, if big { 36.0 } else { 32.0 }, 90.0, FG, a, Align::Left);
                    let nx = tx + g.measure(&temp, tf) + 10.0;
                    g.text(weather::NAMES[wx.kind], &g.f_body, nx, if big { 40.0 } else { 30.0 }, 120.0, FG, a, Align::Left);
                    g.text(&format!("최고 {:.0}° · 최저 {:.0}°", wx.hi, wx.lo), &g.f_tiny, nx, if big { 60.0 } else { 47.0 }, 140.0, FG, dim(0.6), Align::Left);
                    // 알아 둘 것들(칸): 체감 · 습도 · 바람 · 자외선 · 강수 · 기압 · 미세먼지
                    let mut chips: Vec<(String, String, Rgb)> = vec![
                        ("체감".into(), format!("{:.0}°", wx.feels), FG),
                        ("습도".into(), format!("{:.0}%", wx.humid), FG),
                        ("바람".into(), format!("{} {:.1}m/s", weather::wind_name(wx.wind_dir), wx.wind), FG),
                    ];
                    if big {
                        chips.push(("자외선".into(), format!("{:.0} {}", wx.uv, weather::uv_name(wx.uv)), if wx.uv >= 6.0 { WARN } else { FG }));
                        chips.push(("강수".into(), format!("{:.1}mm", wx.rain), if wx.rain > 0.0 { LIME } else { FG }));
                        chips.push(("기압".into(), format!("{:.0}hPa", wx.pressure), FG));
                    }
                    for (v, fine, name) in [(wx.pm10, false, "미세먼지"), (wx.pm25, true, "초미세먼지")] {
                        if let Some(v) = v.filter(|_| big || !fine) {
                            let gr = weather::dust_grade(v, fine);
                            chips.push((name.into(), format!("{} {:.0}", weather::DUST[gr], v), [LIME, FG, WARN, WARN][gr]));
                        }
                    }
                    if big {
                        let cw = (r - l) / chips.len() as f32;
                        for (i, (k, v, c)) in chips.iter().enumerate() {
                            let cx = l + i as f32 * cw;
                            g.text(k, &g.f_tiny, cx, 94.0, cw - 6.0, FG, dim(0.5), Align::Left);
                            g.text(v, &g.f_body, cx, 108.0, cw - 6.0, *c, a, Align::Left);
                        }
                    } else {
                        // 작은 판: 오른쪽에 한 줄로
                        let mut cx = r - 30.0;
                        for (k, v, c) in chips.iter().rev() {
                            let vw = g.measure(v, &g.f_body);
                            let kw = g.measure(k, &g.f_tiny);
                            cx -= vw;
                            g.text(v, &g.f_body, cx, 30.0, vw + 2.0, *c, a, Align::Left);
                            cx -= kw + 5.0;
                            g.text(k, &g.f_tiny, cx, 33.0, kw + 2.0, FG, dim(0.5), Align::Left);
                            cx -= 16.0;
                            if cx < nx + 130.0 {
                                break;
                            }
                        }
                    }
                    // 시간별 12칸
                    let hy = if big { 144.0 } else { 70.0 };
                    if big {
                        g.rrect(l, hy - 8.0, r - l, 1.0, 0.5, FG, dim(0.12));
                        g.text("시간별", &g.f_tiny, l, hy - 2.0, 60.0, FG, dim(0.5), Align::Left);
                    }
                    let y0 = hy + if big { 14.0 } else { 0.0 };
                    let n = wx.hours.len().max(1);
                    let cw = (r - l - 26.0) / n as f32;
                    for (i, hr) in wx.hours.iter().enumerate() {
                        let cx = l + cw * (i as f32 + 0.5);
                        g.text(&format!("{}시", hr.hour), &g.f_tiny, cx - cw / 2.0, y0, cw, FG, dim(0.55), Align::Center);
                        g.zoom(0.8, cx, y0 + 25.0);
                        wx_icon(g, hr.kind, hr.day, cx, y0 + 25.0, a);
                        g.zoom(1.0, 0.0, 0.0);
                        g.text(&format!("{:.0}°", hr.temp), &g.f_time, cx - cw / 2.0, y0 + 37.0, cw, FG, a, Align::Center);
                    }
                }
            } else if a > 0.0 && self.mode == 3 {
                // 시세 쪽: 설정의 --- 구분선으로 나뉜 구역마다 윤기 나는 버튼 하나. 누르면 값까지 나오는 큰 판으로 자란다.
                g.text("시세", &g.f_title, x + 20.0, 15.0, 120.0, FG, a, Align::Left);
                if self.q.is_empty() {
                    let msg = if self.q_tried.is_empty() { "시세를 받는 중…".to_string() } else { format!("{} 시세를 못 받았습니다 · 15초마다 다시 시도", self.q_tried) };
                    g.text(&msg, &g.f_body, x + 20.0, 56.0, w - 40.0, if self.q_tried.is_empty() { FG } else { WARN }, a * if self.q_tried.is_empty() { 0.5 } else { 1.0 }, Align::Center);
                }
                let k = self.bg.pos.clamp(0.0, 1.0);
                let groups: Vec<usize> = self.q.iter().map(|q| q.group).collect();
                let (panels, cells, (lw, _)) = q_layout(&groups, self.bg.pos, self.cfg.cols());
                let live = p > 0.9;
                let (mx, my) = self.mouse;
                let base = ((p - 0.35) / 0.65).clamp(0.0, 1.0);
                let ox = x + (w - lw) / 2.0;
                let mut first = 0; // 이 구역의 첫 종목
                for (gi, pn) in panels.iter().enumerate() {
                    let n = groups[first..].iter().take_while(|g| **g == groups[first]).count();
                    // 쪽이 열릴 때 위 구역부터 차례로 떠오른다
                    let rise = (self.pgf.pos * 1.6 - gi as f32 * 0.14).clamp(0.0, 1.0);
                    let (a, dy) = (base * rise, (1.0 - rise) * 8.0);
                    let (cx, cy, cw, ch) = (ox + pn[0], pn[1] + dy, pn[2], pn[3]);
                    let on = live && mx >= cx && mx < cx + cw && my >= cy && my < cy + ch;
                    if let Some(sp) = self.qs.get_mut(gi) {
                        sp.target = if on { 1.0 } else { 0.0 };
                    }
                    let e = self.qs.get(gi).map_or(0.0, |s| s.pos).clamp(-0.3, 1.4);
                    // 구역 판은 빨강·파랑을 쓰지 않는다: 오름 빨강·내림 파랑은 종목 글자에만(판까지 칠하면 빨간 판 안의 파란 글자처럼 뒤섞여 보인다)
                    let col = (0.55, 0.62, 0.72);
                    // 도톰해짐: 살짝 커지며 떠오르고 그림자가 깊어진다
                    let (bx, by, bw, bh) = (cx - 2.0 * e, cy - 2.0 * e, cw + 4.0 * e, ch + 2.0 * e);
                    let r = 15.0_f32.min(bh / 2.0);
                    let body = mix((0.07, 0.08, 0.095), col, 0.12 + 0.06 * e);
                    g.rrect(bx + 1.0, by + 3.0 + 2.0 * e, bw - 2.0, bh, r, (0.0, 0.0, 0.0), a * (0.55 + 0.2 * e)); // 아래 그림자
                    g.rrect(bx, by, bw, bh, r, mix(body, FG, 0.22 + 0.1 * e), a); // 밝은 테두리
                    g.rrect(bx + 1.0, by + 1.0, bw - 2.0, bh - 2.0, r - 1.0, body, a);
                    let gl = (bh * 0.42).min(15.0); // 윤기·속빛의 두께(높은 버튼에서도 띠처럼 얇게)
                    g.rrect(bx + 8.0, by + bh - gl - 2.5, bw - 16.0, gl, gl * 0.5, col, a * (0.06 + 0.04 * e)); // 아래에서 비치는 빛
                    g.rrect(bx + 3.0, by + 1.5, bw - 6.0, gl, r - 2.5, FG, a * (0.085 + 0.06 * e)); // 윗면 윤기(두 겹)
                    g.rrect(bx + 7.0, by + 2.0, bw - 14.0, gl * 0.5, gl * 0.25, FG, a * (0.07 + 0.04 * e));
                    for (q, c) in self.q[first..first + n].iter().zip(&cells[first..]) {
                        let (ix, iy, iw) = (ox + c[0], c[1] + dy - e + (c[3] - 18.0) / 2.0, c[2]);
                        let col = if q.pct > 0.0 { UP } else if q.pct < 0.0 { DOWN } else { FG };
                        // 시세 위젯처럼 한 줄: 이름 · (큰 판이면 값) · 등락률
                        let vw = g.measure(&q.price, &g.f_time) + 12.0;
                        let nw = |k: f32| iw - 58.0 - vw * k;
                        // 칸이 좁으면 글자를 줄인다(그래도 넘치면 말줄임). 자라는 중에 글자 크기가 흔들리지 않게 목표 크기로 고른다.
                        let room = nw(self.big as u8 as f32);
                        let (f, drop) = [(&g.f_body, 0.0), (&g.f_bar, 0.7), (&g.f_small, 1.4), (&g.f_tiny, 2.2)].into_iter().find(|(f, _)| g.measure(&q.name, f) <= room).unwrap_or((&g.f_tiny, 2.2));
                        g.text(&q.name, f, ix, iy + drop, nw(k), FG, a * 0.92, Align::Left);
                        let va = ((k - 0.35) / 0.65).clamp(0.0, 1.0);
                        g.text(&q.price, &g.f_time, ix + iw - 56.0 - vw, iy + 0.5, vw - 4.0, col, a * va, Align::Right); // 값도 오르면 빨강·내리면 파랑
                        g.text(&format!("{:+.2}%", q.pct), &g.f_time, ix + iw - 58.0, iy + 0.5, 58.0, col, a * if q.pct == 0.0 { 0.55 } else { 1.0 }, Align::Right);
                    }
                    first += n;
                }
                // 생존 표시: 마지막으로 받은 시각. 10분 넘게 못 받으면 경고색.
                let stale = self.q_seen.map_or(true, |t| t.elapsed() > Duration::from_secs(600));
                g.text(&format!("시세 {}", self.q_stamp), &g.f_tiny, x + 20.0, 20.0, w - 40.0, if stale { WARN } else { FG }, a * if stale { 1.0 } else { 0.4 }, Align::Right);
            }
            g.unclip();
        }
        if self.ent.is_some() && h >= 0.5 {
            self.draw_entry(h);
        }
        let g = &self.g;
        g.present()?;
        if let Ok(dir) = std::env::var("NOTCH_CAT_SHOT") {
            let _ = g.snapshot(&PathBuf::from(dir).join("shot.raw"));
        }
        if self.first_frame {
            self.first_frame = false;
            log("frame ok");
        }
        Ok(())
    }

    /// 창을 모니터 위 가운데(또는 오른쪽으로 비켜서)에 놓는다.
    unsafe fn place(&self) {
        let wpx = (WIN_W * self.scale).round() as i32;
        let dx = if self.cfg.aside { (460.0 * self.scale) as i32 } else { 0 };
        let x = self.mon.left + ((self.mon.right - self.mon.left) - wpx) / 2 + dx;
        let _ = SetWindowPos(self.hwnd, None, x, self.mon.top, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
    }

    /// 다음 곡 목록 칸(음악 화면 오른쪽). 다섯 줄이 보이고 더 있으면 휠로 넘긴다.
    /// 유튜브와 내 음악이 둘 다 있으면 둘 다 보인다(지금 소리 나는 쪽이 위).
    unsafe fn draw_queue(&mut self, x0: f32, qw: f32, h: f32, a: f32, live: bool) {
        let g = &self.g;
        let q = &self.qn;
        let (mx, my) = self.mouse;
        let (px, pw) = (x0 + 6.0, qw - 22.0);
        self.qrect = [x0, 0.0, x0 + qw, h];
        g.rrect(x0 - 2.0, 18.0, 1.0, h - 36.0, 0.5, FG, a * 0.14); // 가운데 칸막이
        g.text("다음 곡", &g.f_sub, px, 10.0, pw, FG, a, Align::Left);
        // 줄 목록: (머리인가, 유튜브인가, 항목)
        enum Row<'a> { Head(String, &'a str), Item(bool, &'a queue::Item) }
        let mut rows: Vec<Row> = vec![];
        let yt_first = queue::is_browser(&self.n.app);
        let yt = q.yt_live() && !q.yt.is_empty();
        let lo = !q.local.is_empty();
        let both = yt && lo;
        let mut order = vec![];
        if yt_first { order.extend([true, false]) } else { order.extend([false, true]) }
        for is_yt in order {
            if is_yt && yt {
                if both { rows.push(Row::Head(format!("유튜브 · {}", q.yt_kind), &q.yt_stamp)); }
                rows.extend(q.yt.iter().map(|it| Row::Item(true, it)));
            } else if !is_yt && lo {
                if both { rows.push(Row::Head(format!("내 음악 · {}", q.local_dir), &q.local_stamp)); }
                rows.extend(q.local.iter().map(|it| Row::Item(false, it)));
            }
        }
        // 머리 오른쪽: 어디서 왔나 + 마지막으로 받은 시각(생존 표시)
        let src = if both { String::new() } else if yt { format!("유튜브 {} · {}", q.yt_kind, q.yt_stamp) } else if lo { format!("{} · {}", q.local_dir, q.local_stamp) } else { String::new() };
        g.text(&src, &g.f_tiny, px + 60.0, 14.0, pw - 60.0, FG, a * 0.4, Align::Right);
        let top = 34.0;
        let rh = ((h - top - 10.0) / 5.0).min(44.0);
        if rows.is_empty() {
            let (m1, m2) = if self.n.app.is_empty() || !queue::is_browser(&self.n.app) {
                ("곡 파일이 있는 폴더를 못 찾았습니다", "음악·다운로드·바탕화면 폴더에서 찾습니다")
            } else {
                ("유튜브 목록이 아직 안 왔습니다", "크롬에 '노치 고양이 유튜브' 확장을 켜면 나옵니다")
            };
            g.text(m1, &g.f_bar, px, top + rh, pw, FG, a * 0.6, Align::Left);
            g.text(m2, &g.f_tiny, px, top + rh + 20.0, pw, FG, a * 0.4, Align::Left);
            self.qrows.clear();
            return;
        }
        let vis = 5usize;
        let max_top = rows.len().saturating_sub(vis);
        if self.qtop > max_top {
            self.qtop = max_top;
        }
        self.qrows.clear();
        g.clip(x0, top - 2.0, qw, h - top - 4.0);
        let mut y = top;
        for row in rows.iter().skip(self.qtop).take(vis + 1) {
            match row {
                Row::Head(t, stamp) => {
                    g.text(t, &g.f_tiny, px, y + rh - 16.0, pw - 40.0, LIME, a * 0.8, Align::Left);
                    g.text(stamp, &g.f_tiny, px, y + rh - 16.0, pw, FG, a * 0.4, Align::Right);
                }
                Row::Item(is_yt, it) => {
                    let over = live && mx >= x0 && mx < x0 + qw && my >= y && my < y + rh && my < h - 6.0;
                    if over {
                        g.rrect(px - 4.0, y + 1.0, pw + 8.0, rh - 2.0, 8.0, FG, a * 0.1);
                    }
                    let thumb = rh >= 28.0;
                    let ts = rh - 8.0;
                    let tx = if thumb { px + ts + 10.0 } else { px };
                    if thumb {
                        match it.thumb.as_ref().and_then(|b| self.qbmp.get(&(Arc::as_ptr(b) as usize))) {
                            Some(bm) => g.art(bm, px, y + 4.0, ts, 6.0, a),
                            None => {
                                g.rrect(px, y + 4.0, ts, ts, 6.0, if *is_yt { WARN } else { LIME }, a * 0.18);
                                g.text(if *is_yt { "▶" } else { "♪" }, &g.f_bar, px, y + 4.0 + ts / 2.0 - 8.0, ts, FG, a * 0.7, Align::Center);
                            }
                        }
                    }
                    let two = rh >= 34.0 && !it.sub.is_empty();
                    let ty = if two { y + rh / 2.0 - 17.0 } else { y + rh / 2.0 - 9.0 };
                    g.text(&it.title, &g.f_bar, tx, ty, px + pw - tx, FG, a, Align::Left);
                    if two {
                        g.text(&it.sub, &g.f_tiny, tx, ty + 18.0, px + pw - tx, FG, a * 0.55, Align::Left);
                    }
                    if live && y < h - 10.0 {
                        let i = self.qrows.len();
                        self.qrows.push((*is_yt, it.key.clone()));
                        self.hits.push(([x0, y.max(top), x0 + qw, (y + rh).min(h - 6.0)], Hit::QItem(i)));
                    }
                }
            }
            y += rh;
        }
        g.unclip();
        // 더 있으면 오른쪽에 얇은 위치 막대
        if rows.len() > vis {
            let track = h - top - 14.0;
            let bh = (track * vis as f32 / rows.len() as f32).max(14.0);
            let by = top + (track - bh) * self.qtop as f32 / max_top.max(1) as f32;
            g.rrect(x0 + qw - 9.0, top, 3.0, track, 1.5, FG, a * 0.08);
            g.rrect(x0 + qw - 9.0, by, 3.0, bh, 1.5, FG, a * 0.45);
        }
    }

    /// 쪽 넘기기. 지금은 두 쪽이라 어느 쪽으로 넘겨도 서로 오간다.
    unsafe fn turn(&mut self, d: i32) {
        const PAGES: i32 = 4;
        if !self.hover || self.paged.elapsed() < Duration::from_millis(250) {
            return;
        }
        self.paged = Instant::now();
        self.page = (self.page as i32 + d).rem_euclid(PAGES) as usize;
        self.pgf.pos = 0.0;
        self.pgf.target = 1.0;
        self.tick();
    }

    /// 일정 넣기 칸을 연다: 노치가 잠깐 자판을 받게 하고, 칸이 물방울처럼 흘러나온다.
    /// 쓸 수 있는 캘린더를 계정별로(계정 번호, 캘린더 번호들 — 기본 캘린더가 맨 앞)
    fn acct_cals(&self) -> Vec<(usize, Vec<usize>)> {
        let mut out: Vec<(usize, Vec<usize>)> = Vec::new();
        for (i, c) in self.caln.cals.iter().enumerate().filter(|(_, c)| c.writable) {
            match out.iter_mut().find(|o| o.0 == c.account) {
                Some(o) => o.1.push(i),
                None => out.push((c.account, vec![i])),
            }
        }
        for o in out.iter_mut() {
            // 구글은 아이디가 메일 주소인 캘린더가 기본
            if let Some(p) = o.1.iter().position(|&i| self.caln.accounts.iter().any(|a| *a == self.caln.cals[i].id)) {
                let d = o.1.remove(p);
                o.1.insert(0, d);
            }
        }
        out
    }

    /// 일정 칸의 계정 고르기: step 0 = 누름(켜고 끄기), ±1 = 휠(그 계정 안 다음/앞 캘린더)
    fn ent_acct(&mut self, ai: usize, step: i32) {
        let Some((_, list)) = self.acct_cals().into_iter().find(|o| o.0 == ai) else { return };
        let Some(e) = self.ent.as_mut() else { return };
        match e.cals.iter().position(|c| list.contains(c)) {
            Some(p) if step == 0 => {
                e.cals.remove(p);
            }
            Some(p) => {
                let k = list.iter().position(|c| *c == e.cals[p]).unwrap_or(0) as i32;
                e.cals[p] = list[(k + step).rem_euclid(list.len() as i32) as usize];
            }
            None => e.cals.push(list[0]),
        }
        self.dirty = true;
    }

    unsafe fn ent_open(&mut self) {
        if self.ent.as_ref().is_some_and(|e| !e.closing) {
            return self.ent_focus();
        }
        let (today, now) = gcal::today();
        let day = self.cal_sel;
        let min = if day == today { ((now / 60 + 1) * 60).min(23 * 60) } else { 540 };
        // 처음 고르는 캘린더: 계정의 기본 캘린더(아이디가 메일 주소), 없으면 쓸 수 있는 첫 캘린더
        let cals = &self.caln.cals;
        let cal = cals.iter().position(|c| c.writable && self.caln.accounts.iter().any(|a| *a == c.id)).or_else(|| cals.iter().position(|c| c.writable)).unwrap_or(0);
        self.ent = Some(Ent { text: String::new(), comp: String::new(), day, min, cals: vec![cal], prev: GetForegroundWindow(), focused: false, closing: false, caret: Instant::now(), seg: None, buf: String::new(), parked: false, cur: 0 });
        self.drop.target = 1.0;
        self.ent_focus();
    }

    /// 일정 칸이 지금 노치 아래에 나와 있다(접어 두거나 닫는 중이 아니다)
    fn ent_up(&self) -> bool {
        self.ent.as_ref().is_some_and(|e| !e.closing && !e.parked)
    }

    /// 다른 창으로 가서 칸을 접어 둔다. 자판은 놓되 적던 글자·날짜는 남는다.
    unsafe fn ent_park(&mut self) {
        let Some(e) = self.ent.as_mut() else { return };
        e.parked = true;
        e.focused = false;
        e.seg = None;
        e.buf.clear();
        TYPING.with(|t| t.set(false));
        let ex = GetWindowLongW(self.hwnd, GWL_EXSTYLE) as u32;
        SetWindowLongW(self.hwnd, GWL_EXSTYLE, (ex | WS_EX_NOACTIVATE.0) as i32);
        self.drop.target = 0.0;
        self.nb.target = 0.0;
        self.dirty = true;
    }

    /// 마우스가 나갔다: 접고 다음에 열 땐 기본 쪽부터
    unsafe fn leave(&mut self) {
        self.tracking = false;
        self.hover = false;
        if !self.naver_guide {
            self.page = 0; // 네이버 안내 중이면 브라우저에 다녀와도 달력 쪽 그대로
        }
        self.cal_month = 0;
        self.cal_big = false;
        self.cal_pop = None;
        self.acct_menu = false;
        self.mouse = (-1.0, -1.0);
        self.qtop = 0;
        self.tick();
    }

    unsafe fn ent_focus(&mut self) {
        TYPING.with(|t| t.set(true));
        let ex = GetWindowLongW(self.hwnd, GWL_EXSTYLE) as u32;
        SetWindowLongW(self.hwnd, GWL_EXSTYLE, (ex & !WS_EX_NOACTIVATE.0) as i32);
        let ok = SetForegroundWindow(self.hwnd).as_bool();
        let _ = SetFocus(self.hwnd);
        if let Some(e) = self.ent.as_mut() {
            e.focused = ok || GetForegroundWindow() == self.hwnd;
            e.caret = Instant::now();
        }
        self.dirty = true;
    }

    unsafe fn ent_close(&mut self) {
        let Some(e) = self.ent.as_mut() else { return };
        if e.closing {
            return;
        }
        e.closing = true;
        let prev = e.prev;
        TYPING.with(|t| t.set(false));
        let ex = GetWindowLongW(self.hwnd, GWL_EXSTYLE) as u32;
        SetWindowLongW(self.hwnd, GWL_EXSTYLE, (ex | WS_EX_NOACTIVATE.0) as i32);
        if GetForegroundWindow() == self.hwnd && !prev.0.is_null() && IsWindow(prev).as_bool() {
            let _ = SetForegroundWindow(prev);
        }
        self.drop.target = 0.0;
        self.dirty = true;
    }

    unsafe fn ent_save(&mut self) {
        let Some(e) = self.ent.as_ref() else { return };
        let title = e.shown().trim().to_string();
        if title.is_empty() {
            self.face.slow_blink(300);
            return;
        }
        if e.cals.is_empty() {
            self.face.slow_blink(300); // 넣을 곳을 하나도 안 골랐다
            return;
        }
        for &cal in &e.cals {
            let _ = self.cal_tx.send(gcal::Cmd::Add { cal, title: title.clone(), day: e.day, start: e.min, end: -1 });
        }
        self.cal_sel = e.day;
        self.face.happy(900);
        self.ent_close();
    }

    unsafe fn ent_char(&mut self, c: u32) {
        let Some(e) = self.ent.as_mut() else { return };
        e.caret = Instant::now();
        if let Some(i) = e.seg {
            // 날짜·시각 칸을 고른 동안: 숫자는 그 칸에, 다 치면 다음 칸으로. 다른 글자를 치면 제목 칸으로 돌아간다.
            match char::from_u32(c) {
                Some(ch) if ch.is_ascii_digit() => {
                    let (d, m, next) = entry::digit(e.day, e.min, i, &mut e.buf, ch);
                    (e.day, e.min) = (d, m);
                    if next {
                        // 일 다음은 오전/오후를 건너뛰고 시로(오전/오후는 숫자로 못 친다). 분까지 치면 제목 칸으로.
                        e.seg = if i == 1 && e.min >= 0 { Some(3) } else { seg_step(i, 1, e.min) };
                    }
                    self.cal_sel = e.day;
                    self.dirty = true;
                    return;
                }
                _ if c == 8 => {
                    e.buf.clear();
                    self.dirty = true;
                    return;
                }
                _ if c >= 32 && c != 127 => {
                    e.seg = None;
                    e.buf.clear();
                }
                _ => return,
            }
        }
        match c {
            8 if e.cur > 0 => {
                e.cur -= 1;
                let i = e.at();
                e.text.remove(i);
            }
            c if c >= 32 && c != 127 => {
                if let Some(ch) = char::from_u32(c) {
                    e.insert(&ch.to_string());
                }
            }
            _ => {} // Enter·Esc·Tab 은 ent_key 에서
        }
        self.dirty = true;
    }

    unsafe fn ent_key(&mut self, k: u32) {
        if let Some(e) = self.ent.as_mut() {
            if let Some(i) = e.seg {
                match k {
                    // 칸을 고른 동안: ↑↓ 그 칸을 한 걸음, ←→ 옆 칸, Esc 제목 칸으로
                    0x26 | 0x28 => {
                        (e.day, e.min) = entry::bump(e.day, e.min, i, if k == 0x26 { 1 } else { -1 });
                        e.buf.clear();
                        self.cal_sel = e.day;
                    }
                    0x25 | 0x27 | 0x09 => {
                        e.buf.clear();
                        e.seg = seg_step(i, if k == 0x25 { -1 } else { 1 }, e.min).or(Some(i));
                    }
                    0x1B => {
                        e.seg = None;
                        e.buf.clear();
                    }
                    0x0D => self.ent_save(),
                    _ => {}
                }
                self.dirty = true;
                return;
            }
        }
        // 글자 칸: Home·End·←→ 커서, Delete 뒤 글자 지우기(한글 조합 중엔 IME 가 먼저 받는다)
        if let Some(e) = self.ent.as_mut().filter(|e| e.comp.is_empty()) {
            let n = e.text.chars().count();
            let moved = match k {
                0x24 => Some(0),
                0x23 => Some(n),
                0x25 => Some(e.cur.saturating_sub(1)),
                0x27 => Some((e.cur + 1).min(n)),
                0x2E => {
                    if e.cur < n {
                        let i = e.at();
                        e.text.remove(i);
                    }
                    Some(e.cur)
                }
                _ => None,
            };
            if let Some(c) = moved {
                e.cur = c;
                e.caret = Instant::now();
                self.dirty = true;
                return;
            }
        }
        match k {
            0x0D => self.ent_save(),
            0x1B => self.ent_close(),
            // 위·아래: 하루씩, 쪽 위·아래: 한 시간씩
            0x26 | 0x28 => {
                if let Some(e) = self.ent.as_mut() {
                    e.day += if k == 0x26 { -1 } else { 1 };
                }
            }
            0x21 | 0x22 => {
                if let Some(e) = self.ent.as_mut() {
                    (e.day, e.min) = entry::pick_time(e.day, e.min, if k == 0x21 { 5 } else { 4 });
                }
            }
            _ => {}
        }
        self.dirty = true;
    }

    /// 노치 아래로 흘러나온 일정 넣기 칸. 몸통과 칸 사이를 오목한 목으로 이어 두 물방울이 붙은 것처럼 보이게 한다.
    unsafe fn draw_entry(&mut self, h: f32) {
        let t = self.drop.pos;
        if self.ent.as_ref().is_some_and(|e| e.closing) && t < 0.02 && self.drop.target == 0.0 {
            self.ent = None;
            return;
        }
        let Some(e) = self.ent.as_ref() else { return };
        let g = &self.g;
        let cx = WIN_W / 2.0;
        let tt = t.clamp(0.0, 1.3);
        // 칸: 몸통 아래에서 작게 맺혔다가 커지며 내려온다(스프링이라 끝에서 살짝 '뾱' 튕긴다)
        let bw = ENT_W * (0.3 + 0.7 * tt.min(1.15));
        let bh = ENT_H * tt;
        let top = h + ENT_GAP * tt.min(1.0);
        if bh < 2.0 {
            return;
        }
        // 목: 몸통 바닥과 칸 위를 잇는 오목한 다리. 붙는 곳은 넓고 가운데는 잘록하다.
        let edge = 96.0 * tt.min(1.0);
        let waist = 22.0 * tt.min(1.0) + 6.0;
        let (y0, y1) = (h - 6.0, top + 6.0);
        let n = 14;
        let mut pts: Vec<(f32, f32)> = Vec::with_capacity(2 * n + 2);
        for k in 0..=n {
            let u = k as f32 / n as f32;
            let half = waist + (edge - waist) * (2.0 * u - 1.0).powi(2);
            pts.push((cx - half, y0 + (y1 - y0) * u));
        }
        for k in (0..=n).rev() {
            let u = k as f32 / n as f32;
            let half = waist + (edge - waist) * (2.0 * u - 1.0).powi(2);
            pts.push((cx + half, y0 + (y1 - y0) * u));
        }
        g.rrect(cx - bw / 2.0 - 1.0, top + 4.0, bw + 2.0, bh, 22.0_f32.min(bh / 2.0), (0.0, 0.0, 0.0), 0.35 * tt.min(1.0)); // 그림자
        g.poly(&pts, BG, 1.0);
        g.rrect(cx - bw / 2.0, top, bw, bh, 22.0_f32.min(bh / 2.0), BG, 1.0);
        // 적는 동안 온 알림: 몸통은 그대로 두고 칸 아래로 같은 목 모양의 작은 물방울이 볼록 나와 알린다
        let nt = self.nb.pos.clamp(0.0, 1.3);
        if nt > 0.02 && !e.closing {
            let nw = NOTE_W * (0.3 + 0.7 * nt.min(1.15));
            let nh = NOTE_H * nt;
            let ntop = top + bh + ENT_GAP * nt.min(1.0);
            if nh >= 2.0 {
                let (y0, y1) = (top + bh - 6.0, ntop + 6.0);
                let (edge, waist) = (70.0 * nt.min(1.0), 16.0 * nt.min(1.0) + 5.0);
                let mut pts: Vec<(f32, f32)> = Vec::with_capacity(2 * n + 2);
                for k in 0..=n {
                    let u = k as f32 / n as f32;
                    pts.push((cx - (waist + (edge - waist) * (2.0 * u - 1.0).powi(2)), y0 + (y1 - y0) * u));
                }
                for k in (0..=n).rev() {
                    let u = k as f32 / n as f32;
                    pts.push((cx + (waist + (edge - waist) * (2.0 * u - 1.0).powi(2)), y0 + (y1 - y0) * u));
                }
                let rr = 20.0_f32.min(nh / 2.0);
                g.rrect(cx - nw / 2.0 - 1.0, ntop + 4.0, nw + 2.0, nh, rr, (0.0, 0.0, 0.0), 0.35 * nt.min(1.0));
                g.poly(&pts, BG, 1.0);
                g.rrect(cx - nw / 2.0, ntop, nw, nh, rr, BG, 1.0);
                let na = ((nt - 0.55) / 0.4).clamp(0.0, 1.0);
                if na > 0.0 {
                    let nx = cx - NOTE_W / 2.0;
                    let nn = &self.note;
                    match &self.note_icon {
                        Some(b) => g.art(b, nx + 16.0, ntop + 12.0, 30.0, 7.0, na),
                        None => g.ellipse(nx + 31.0, ntop + 27.0, 5.0, 5.0, LIME, na),
                    }
                    let tw = NOTE_W - 76.0;
                    g.text(&nn.app, &g.f_tiny, nx + 58.0, ntop + 9.0, tw, FG, na * 0.55, Align::Left);
                    g.text(if nn.title.is_empty() { &nn.body } else { &nn.title }, &g.f_lyric, nx + 58.0, ntop + 22.0, tw, FG, na, Align::Left);
                }
            }
        }
        let a = ((t - 0.55) / 0.4).clamp(0.0, 1.0); // 칸이 거의 다 나온 뒤에 글자가 떠오른다
        if a <= 0.0 || e.closing {
            return;
        }
        let (l, r) = (cx - ENT_W / 2.0 + 18.0, cx + ENT_W / 2.0 - 18.0);
        let (mx, my) = self.mouse;
        let over = |b: [f32; 4]| mx >= b[0] && mx < b[2] && my >= b[1] && my < b[3];
        let mut hits: Vec<([f32; 4], Hit)> = Vec::new();
        // 1줄: 글자 칸 + 닫기
        let fb = [l, top + 12.0, r - 34.0, top + 44.0];
        g.rrect(fb[0], fb[1], fb[2] - fb[0], 32.0, 10.0, FG, a * if e.focused && e.seg.is_none() { 0.1 } else { 0.06 });
        let shown = e.shown();
        let pre = &e.text[..e.at()]; // 커서 앞 글
        if shown.is_empty() {
            let hint = if e.focused { "무슨 일정인가요?   Enter 저장 · Esc 닫기" } else { "여기를 눌러 일정을 적어 주세요" };
            g.text(hint, &g.f_body, fb[0] + 12.0, fb[1] + 6.0, fb[2] - fb[0] - 24.0, FG, a * 0.45, Align::Left);
        } else {
            g.text(&shown, &g.f_body, fb[0] + 12.0, fb[1] + 6.0, fb[2] - fb[0] - 24.0, FG, a, Align::Left);
        }
        if !e.comp.is_empty() {
            let tw = g.measure(pre, &g.f_body);
            let cw = g.measure(&e.comp, &g.f_body);
            g.rrect(fb[0] + 12.0 + tw, fb[1] + 25.0, cw, 1.5, 0.5, LIME, a); // 조합 중인 글자 밑줄
        }
        if e.focused && e.seg.is_none() && (e.caret.elapsed().as_millis() / 530) % 2 == 0 {
            let cxp = fb[0] + 12.0 + if pre.is_empty() && e.comp.is_empty() { 0.0 } else { g.measure(&format!("{pre}{}", e.comp), &g.f_body) + 1.0 };
            g.rrect(cxp.min(fb[2] - 10.0), fb[1] + 8.0, 1.6, 17.0, 0.8, LIME, a);
        }
        hits.push((fb, Hit::EntField));
        let xb = [r - 26.0, top + 16.0, r, top + 40.0];
        g.ellipse(r - 13.0, top + 28.0, 12.0, 12.0, FG, a * if over(xb) { 0.18 } else { 0.07 });
        ribbon(g, &[(r - 17.0, top + 24.0), (r - 9.0, top + 32.0)], |_| 1.5, FG, a * 0.8);
        ribbon(g, &[(r - 9.0, top + 24.0), (r - 17.0, top + 32.0)], |_| 1.5, FG, a * 0.8);
        hits.push((xb, Hit::EntClose));
        // 2줄: 언제 · 어느 캘린더 · 저장
        let y2 = top + 52.0;
        // 날짜·시각: 칸마다 눌러 고르고 숫자를 치거나, 휠·↑↓로 바꾼다
        {
            let (_, m, d) = gcal::ymd(e.day);
            let mut parts: Vec<(String, Option<usize>)> = vec![(m.to_string(), Some(0)), ("월".into(), None), (d.to_string(), Some(1)), (format!("일 ({})", gcal::WEEK[gcal::weekday(e.day)]), None)];
            if e.min < 0 {
                parts.push(("종일".into(), None));
            } else {
                let (hh, mi) = (e.min / 60, e.min % 60);
                parts.push((if hh < 12 { "오전" } else { "오후" }.into(), Some(2)));
                parts.push(((if hh % 12 == 0 { 12 } else { hh % 12 }).to_string(), Some(3)));
                parts.push((":".into(), None));
                parts.push((format!("{mi:02}"), Some(4)));
            }
            let mut x = l + 2.0;
            for (txt, seg) in &parts {
                let tw = g.measure(txt, &g.f_lyric);
                match seg {
                    Some(i) => {
                        let b = [x - 4.0, y2 - 1.0, x + tw + 4.0, y2 + 25.0];
                        let on = e.seg == Some(*i);
                        g.rrect(b[0], b[1], b[2] - b[0], 26.0, 7.0, if on { LIME } else { FG }, a * if on { 0.26 } else if over(b) { 0.14 } else { 0.06 });
                        g.text(txt, &g.f_lyric, x, y2, tw + 2.0, LIME, a, Align::Left);
                        if on && !e.buf.is_empty() {
                            g.rrect(x, y2 + 22.0, tw, 1.5, 0.5, LIME, a); // 숫자를 치는 중
                        }
                        hits.push((b, Hit::EntSeg(*i)));
                        x += tw + 10.0;
                    }
                    None => {
                        g.text(txt, &g.f_lyric, x, y2, tw + 2.0, LIME, a * 0.75, Align::Left);
                        x += tw + if txt == ":" { 3.0 } else { 7.0 };
                    }
                }
            }
        }
        let multi = self.acct_cals().len() >= 2;
        if let Some(c) = self.caln.cals.get(e.cals.first().copied().unwrap_or(usize::MAX)).filter(|_| !multi) {
            let name = if c.name.is_empty() { c.id.clone() } else { c.name.clone() };
            let nw = g.measure(&name, &g.f_tiny).min(150.0);
            let cb = [r - 74.0 - nw - 34.0, y2 + 1.0, r - 74.0, y2 + 23.0];
            g.rrect(cb[0], cb[1], cb[2] - cb[0], 22.0, 11.0, FG, a * if over(cb) { 0.16 } else { 0.07 });
            g.ellipse(cb[0] + 12.0, cb[1] + 11.0, 4.0, 4.0, c.color, a);
            g.text(&name, &g.f_tiny, cb[0] + 22.0, cb[1] + 4.0, nw + 4.0, FG, a * 0.85, Align::Left);
            hits.push((cb, Hit::EntCal));
        }
        if multi {
            // 계정이 둘 이상: 넣을 곳을 고른다(눌러서 켜고 끄기, 둘 다 켜면 둘 다 들어간다, 휠로 그 계정 안 캘린더 바꾸기)
            let mut chips = Vec::new();
            for (ai, list) in self.acct_cals() {
                let sel = e.cals.iter().copied().find(|c| list.contains(c));
                let Some(c) = self.caln.cals.get(sel.unwrap_or(list[0])) else { continue };
                let acct = self.caln.accounts.get(ai).map(|s| if s.starts_with("네이버 ") { "네이버".to_string() } else { "구글".to_string() }).unwrap_or_default();
                // 기본 캘린더면 계정 이름만, 다른 캘린더로 바꿨으면 '네이버 · 배당알림'
                let label = if sel.unwrap_or(list[0]) == list[0] { acct } else { format!("{acct} · {}", c.name.chars().take(6).collect::<String>()) };
                let w = g.measure(&label, &g.f_tiny) + 30.0;
                chips.push((ai, sel.is_some(), c.color, label, w));
            }
            let mut x = r - 74.0 - chips.iter().map(|c| c.4 + 5.0).sum::<f32>();
            for (ai, on, color, label, w) in chips {
                if x < l + 290.0 {
                    x += w + 5.0;
                    continue; // 자리가 모자라면 앞쪽 것을 뺀다
                }
                let b = [x, y2 + 1.0, x + w, y2 + 23.0];
                g.rrect(b[0], b[1], w, 22.0, 11.0, if on { LIME } else { FG }, a * if on { 0.24 } else if over(b) { 0.16 } else { 0.07 });
                g.ellipse(b[0] + 12.0, b[1] + 11.0, 4.0, 4.0, color, a * if on { 1.0 } else { 0.4 });
                g.text(&label, &g.f_tiny, b[0] + 21.0, b[1] + 4.0, w - 24.0, if on { LIME } else { FG }, a * if on { 0.95 } else { 0.55 }, Align::Left);
                hits.push((b, Hit::EntAcct(ai)));
                x += w + 5.0;
            }
        }
        let sb = [r - 64.0, y2, r, y2 + 24.0];
        g.rrect(sb[0], sb[1], 64.0, 24.0, 12.0, LIME, a * if over(sb) { 1.0 } else { 0.88 });
        g.text("저장", &g.f_bar, sb[0], sb[1] + 3.0, 64.0, PUPIL, a, Align::Center);
        hits.push((sb, Hit::EntSave));
        // 3줄: 날 · 시각 고르기
        let y3 = top + 82.0;
        let mut x = l;
        let (today, _) = gcal::today();
        let chip = |hits: &mut Vec<([f32; 4], Hit)>, x: &mut f32, label: &str, on: bool, hit: Hit| {
            let w = g.measure(label, &g.f_tiny) + 18.0;
            let b = [*x, y3, *x + w, y3 + 22.0];
            g.rrect(b[0], b[1], w, 22.0, 11.0, if on { LIME } else { FG }, a * if on { 0.24 } else if over(b) { 0.16 } else { 0.07 });
            g.text(label, &g.f_tiny, b[0], b[1] + 4.0, w, if on { LIME } else { FG }, a * 0.9, Align::Center);
            hits.push((b, hit));
            *x += w + 5.0;
        };
        for (i, (label, d)) in entry::DAYS.iter().enumerate() {
            chip(&mut hits, &mut x, label, e.day == today + d, Hit::EntDay(i));
        }
        x += 10.0;
        chip(&mut hits, &mut x, entry::TIMES[0].0, e.min < 0, Hit::EntTime(0)); // 시각 단추 대신 위 칸을 고친다
        g.text(if multi { "넣을 곳: 눌러서 켜고 끄기 · 둘 다 켜면 둘 다 · 휠로 캘린더 바꾸기" } else { "칸을 눌러 숫자 · 휠 · ↑↓ · 달력 날짜 누르기" }, &g.f_tiny, x + 6.0, y3 + 4.0, r - x - 6.0, FG, a * 0.4, Align::Left);
        let focused = e.focused;
        self.hits.extend(hits);
        if focused {
            self.dirty = true; // 깜빡이는 글자 자리
        }
    }

    unsafe fn click(&mut self, px: f32, py: f32) {
        let (x, y) = (px / self.scale, py / self.scale);
        let Some(&(_, hit)) = self.hits.iter().find(|(r, _)| x >= r[0] && x < r[2] && y >= r[1] && y < r[3]) else { return };
        if let Hit::Page(d) = hit {
            return self.turn(d);
        }
        if !matches!(hit, Hit::CalEv(..)) {
            self.cal_pop = None; // 일정 카드는 다른 곳을 누르면 닫힌다
        }
        let menu = std::mem::take(&mut self.acct_menu);
        match hit {
            Hit::CalAcct => self.acct_menu = !menu,
            Hit::CalEv(z, j) => self.cal_pop = (j != usize::MAX).then_some((z, j)),
            Hit::CalNav(0) => {
                self.cal_month = 0;
                self.cal_sel = gcal::today().0;
            }
            Hit::CalNav(d) => self.cal_month += d,
            Hit::CalDay(z) => {
                self.cal_sel = z;
                // 작은 달력에서 날을 누르면 크게 펴서 칸마다 일정 제목을 보여 준다(그 날이 든 달로)
                let (zy, zm, _) = gcal::ymd(z);
                let (ty, tm, _) = gcal::ymd(gcal::today().0);
                self.cal_month = (zy * 12 + zm as i32) - (ty * 12 + tm as i32);
                self.cal_big = true;
                // 일정 칸이 나와 있으면 누른 날이 그 칸의 날짜가 된다
                if let Some(e) = self.ent.as_mut().filter(|e| !e.closing && !e.parked) {
                    e.day = z;
                }
            }
            Hit::CalAdd => self.ent_open(),
            Hit::EntField => {
                let g = &self.g;
                let l = WIN_W / 2.0 - ENT_W / 2.0 + 18.0 + 12.0; // 글자가 시작하는 곳(draw_entry 와 같다)
                if let Some(e) = self.ent.as_mut() {
                    e.seg = None;
                    e.buf.clear();
                    // 누른 자리에 가장 가까운 글자 사이로 커서를 옮긴다
                    let n = e.text.chars().count();
                    e.cur = (0..=n)
                        .min_by(|&p, &q| {
                            let at = |c: usize| (g.measure(&e.text.chars().take(c).collect::<String>(), &g.f_body) - (x - l)).abs();
                            at(p).total_cmp(&at(q))
                        })
                        .unwrap_or(n);
                }
                self.ent_focus();
            }
            Hit::EntSeg(i) => {
                if let Some(e) = self.ent.as_mut() {
                    if i == 2 && e.seg == Some(2) {
                        (e.day, e.min) = entry::bump(e.day, e.min, 2, 1); // 오전/오후는 한 번 더 누르면 바뀐다
                    }
                    e.seg = Some(i);
                    e.buf.clear();
                }
                self.ent_focus();
            }
            Hit::EntDay(i) => {
                if let Some(e) = self.ent.as_mut() {
                    e.day = gcal::today().0 + entry::DAYS[i].1;
                    self.cal_sel = e.day;
                }
            }
            Hit::EntTime(i) => {
                if let Some(e) = self.ent.as_mut() {
                    // 종일 단추: 누르면 종일, 한 번 더 누르면 오전 9시로
                    (e.day, e.min) = if i == 0 && e.min < 0 { (e.day, 540) } else { entry::pick_time(e.day, e.min, i) };
                    if e.min < 0 && e.seg.is_some_and(|s| s >= 2) {
                        e.seg = None;
                    }
                }
            }
            Hit::EntCal => {
                let w: Vec<usize> = self.caln.cals.iter().enumerate().filter(|(_, c)| c.writable).map(|(i, _)| i).collect();
                if let Some(e) = self.ent.as_mut() {
                    let cur = e.cals.first().copied().unwrap_or(usize::MAX);
                    let at = w.iter().position(|i| *i == cur).map_or(0, |p| (p + 1) % w.len().max(1));
                    e.cals = w.get(at).copied().into_iter().collect();
                }
            }
            Hit::EntAcct(ai) => self.ent_acct(ai, 0),
            Hit::EntSave => self.ent_save(),
            Hit::EntClose => self.ent_close(),
            Hit::CalNaver => {
                self.naver_guide = true;
                self.naver_n = self.caln.accounts.iter().filter(|a| a.starts_with("네이버 ")).count();
                self.cal_big = false;
                self.tick();
            }
            Hit::NaverClose => {
                self.naver_guide = false;
                self.tick();
            }
            Hit::NaverOpen => {
                ShellExecuteW(None, w!("open"), w!("https://nid.naver.com/user2/help/myInfoV2?m=viewSecurity&lang=ko_KR"), None, None, SW_SHOWNORMAL);
            }
            Hit::NaverInput => {
                // 윈도우 기본 '아이디·비밀번호' 창을 따로 띄운다(노치는 그대로 돈다)
                log("naver: 연결 단추");
                let tx = self.cal_tx.clone();
                let owner = self.hwnd.0 as isize; // 노치를 주인으로: 노치는 맨 위라 그 창도 다른 창 뒤로 숨지 않는다
                std::thread::spawn(move || {
                    if let Some((id, pw)) = naver::ask(owner) {
                        let _ = tx.send(gcal::Cmd::Naver { id, pw });
                    }
                });
            }
            Hit::CalLogin => {
                let _ = self.cal_tx.send(gcal::Cmd::Login);
                self.cal_wait = Instant::now() + Duration::from_secs(300);
            }
            _ => {}
        }
        if matches!(hit, Hit::CalNav(_) | Hit::CalDay(_) | Hit::CalEv(..) | Hit::CalAdd | Hit::CalLogin | Hit::CalNaver | Hit::CalAcct | Hit::NaverOpen | Hit::NaverInput | Hit::NaverClose | Hit::EntField | Hit::EntDay(_) | Hit::EntTime(_) | Hit::EntCal | Hit::EntAcct(_) | Hit::EntSeg(_) | Hit::EntSave | Hit::EntClose) {
            self.dirty = true;
            return;
        }
        if let Hit::Layout = hit {
            if self.page == 3 && self.cal_big {
                self.cal_big = false; // 큰 달력 → 원래 모양으로
                return self.tick();
            }
            if self.page == 3 {
                self.cfg.cv = (self.cfg.cv + 1) % CALV.len();
            } else if self.page == 2 {
                self.cfg.wbig = !self.cfg.wbig;
            } else {
                self.cfg.ml = (self.cfg.ml + 1) % MUSIC.len();
            }
            self.cfg.save();
            self.pgf.pos = 0.0;
            self.pgf.target = 1.0;
            return self.tick();
        }
        if let Hit::Queue = hit {
            self.qopen = !self.qopen;
            self.qtop = 0;
            return self.tick();
        }
        if let Hit::QItem(i) = hit {
            if let Some((yt, k)) = self.qrows.get(i).cloned() {
                if yt {
                    let mut q = self.queue.lock().unwrap();
                    let tab = q.yt_tab;
                    q.yt_cmd = Some((tab, k.split('\n').next().unwrap_or("").to_string()));
                } else if self.n.app == "ALSong" {
                    // 알송은 '이 곡 틀어'를 못 받는다. 목록이 알송 순서 그대로라 그 곡까지 '다음'을 보낸다.
                    let at = self.qn.local.iter().position(|x| x.key == k).unwrap_or(0);
                    for _ in 0..=at.min(30) {
                        let _ = self.tx.send(media::Cmd::Next);
                    }
                } else {
                    queue::play_local(&k, &self.qn.exe);
                }
                self.face.happy(500);
            }
            return;
        }
        let cmd = match hit {
            Hit::Prev => Cmd::Prev,
            Hit::Toggle => Cmd::Toggle,
            Hit::Next => Cmd::Next,
            Hit::Bar(b0, bw) => Cmd::Seek((((x - b0) / bw).clamp(0.0, 1.0) * self.n.dur_ms as f32) as i64),
            Hit::Line(t) => Cmd::Seek(t as i64),
            Hit::Shuffle => Cmd::Shuffle,
            Hit::Repeat => Cmd::Repeat,
            Hit::Page(_) | Hit::Layout | Hit::CalNav(_) | Hit::CalDay(_) | Hit::CalEv(..) | Hit::CalAdd | Hit::CalLogin | Hit::CalNaver | Hit::CalAcct | Hit::NaverOpen | Hit::NaverInput | Hit::NaverClose | Hit::Queue | Hit::QItem(_) | Hit::EntField | Hit::EntDay(_) | Hit::EntTime(_) | Hit::EntCal | Hit::EntAcct(_) | Hit::EntSeg(_) | Hit::EntSave | Hit::EntClose => return,
        };
        let _ = self.tx.send(cmd);
    }

    unsafe fn tray_menu(&mut self) -> bool {
        let Ok(m) = CreatePopupMenu() else { return false };
        let check = |on: bool| if on { MF_CHECKED } else { MF_UNCHECKED };
        let title: Vec<u16> = format!("노치 고양이 {VERSION}").encode_utf16().chain([0]).collect();
        let _ = AppendMenuW(m, MF_GRAYED, 0, PCWSTR(title.as_ptr()));
        let _ = AppendMenuW(m, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(m, MF_STRING, 8, w!("시세 종목 설정…"));
        let _ = AppendMenuW(m, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(m, check(self.cfg.hide_fullscreen), 1, w!("전체 화면일 때 숨기기"));
        let _ = AppendMenuW(m, check(self.cfg.autostart), 2, w!("윈도우 시작할 때 자동 실행"));
        let _ = AppendMenuW(m, check(self.cfg.aside), 4, w!("오른쪽으로 비켜 놓기"));
        let _ = AppendMenuW(m, check(self.cfg.tiny), 6, w!("최대화 창에서 마우스가 가까이 오면 작게 줄이기"));
        let _ = AppendMenuW(m, check(self.cfg.bar), 7, w!("닫힘에 시세 한 줄"));
        let _ = AppendMenuW(m, check(self.cfg.two), 9, w!("시세를 세로 두 칸으로 (끄면 위젯처럼 한 칸)"));
        let _ = AppendMenuW(m, MF_SEPARATOR, 0, None);
        for (i, name) in MUSIC_NAMES.iter().enumerate() {
            let t: Vec<u16> = format!("음악 화면: {name}").encode_utf16().chain([0]).collect();
            let _ = AppendMenuW(m, check(self.cfg.ml == i), 20 + i, PCWSTR(t.as_ptr()));
        }
        let _ = AppendMenuW(m, MF_STRING, 5, w!("얼굴 바꾸기"));
        let _ = AppendMenuW(m, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(m, MF_STRING, 3, w!("끝내기"));
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let _ = SetForegroundWindow(self.hwnd);
        let id = TrackPopupMenu(m, TPM_RETURNCMD | TPM_RIGHTBUTTON, pt.x, pt.y, 0, self.hwnd, None).0;
        let _ = DestroyMenu(m);
        match id {
            1 => self.cfg.hide_fullscreen = !self.cfg.hide_fullscreen,
            2 => {
                self.cfg.autostart = !self.cfg.autostart;
                self.cfg.apply_autostart();
            }
            3 => return true,
            4 => {
                self.cfg.aside = !self.cfg.aside;
                self.place();
            }
            5 => {
                self.face.touch();
                self.face.next_face();
                return false;
            }
            7 => self.cfg.bar = !self.cfg.bar,
            9 => self.cfg.two = !self.cfg.two,
            20..=22 => self.cfg.ml = id as usize - 20,
            8 => {
                settings::open(self.quotes.clone(), cat_icon());
                return false;
            }
            6 => {
                self.cfg.tiny = !self.cfg.tiny;
                self.maxed = false;
            }
            _ => return false,
        }
        self.cfg.save();
        false
    }
}

/// 트레이에 띄울 고양이 얼굴. 실행 파일에 넣어 둔 ico 에서 32px 그림을 꺼낸다.
unsafe fn cat_icon() -> HICON {
    const ICO: &[u8] = include_bytes!("../assets/cat.ico");
    let n = u16::from_le_bytes([ICO[4], ICO[5]]) as usize;
    for e in (0..n).map(|i| &ICO[6 + 16 * i..22 + 16 * i]) {
        if e[0] == 32 {
            let (size, off) = (u32::from_le_bytes([e[8], e[9], e[10], e[11]]) as usize, u32::from_le_bytes([e[12], e[13], e[14], e[15]]) as usize);
            if let Ok(h) = CreateIconFromResourceEx(&ICO[off..off + size], true, 0x0003_0000, 32, 32, LR_DEFAULTCOLOR) {
                return h;
            }
        }
    }
    LoadIconW(None, IDI_APPLICATION).unwrap_or_default()
}

unsafe fn tray(hwnd: HWND, add: bool) {
    let mut d = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
        uCallbackMessage: WM_TRAY,
        hIcon: cat_icon(),
        ..Default::default()
    };
    for (i, c) in "노치 고양이".encode_utf16().enumerate() {
        d.szTip[i] = c;
    }
    let _ = Shell_NotifyIconW(if add { NIM_ADD } else { NIM_DELETE }, &d);
}

unsafe fn run() -> Result<()> {
    let _one = CreateMutexW(None, true, w!("notch-cat-single"))?;
    if GetLastError() == ERROR_ALREADY_EXISTS {
        // 이미 켜져 있으면 그걸 닫고 내가 대신 뜬다(새로 받은 판을 실행하면 그대로 바뀌게)
        let Ok(old) = FindWindowW(w!("NotchCat"), None) else { return Ok(()) };
        let _ = PostMessageW(old, WM_CLOSE, WPARAM(0), LPARAM(0));
        for _ in 0..40 {
            std::thread::sleep(Duration::from_millis(100));
            if !IsWindow(old).as_bool() {
                break;
            }
        }
        log("replaced the running copy");
    }
    trim_log();
    log(&format!("start {VERSION}"));
    let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

    let inst = GetModuleHandleW(None)?;
    let class = w!("NotchCat");
    RegisterClassW(&WNDCLASSW {
        lpfnWndProc: Some(wndproc),
        hInstance: inst.into(),
        hCursor: LoadCursorW(None, IDC_ARROW)?,
        lpszClassName: class,
        ..Default::default()
    });

    // ponytail: 주 모니터 하나에만 뜬다. 해상도·배율을 바꾸면 껐다 켜야 한다(바뀜 감지는 필요해지면).
    let hmon = MonitorFromPoint(POINT::default(), MONITOR_DEFAULTTOPRIMARY);
    let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    let _ = GetMonitorInfoW(hmon, &mut mi);
    let mon = mi.rcMonitor;
    let dpi = GetDpiForSystem() as f32;
    let logical_w = (mon.right - mon.left) as f32 * 96.0 / dpi;
    let grow = if logical_w >= 2500.0 { 1.2 } else { 1.0 }; // 넓은 모니터에선 키운다
    let scale = dpi / 96.0 * grow;
    let (wpx, hpx) = ((WIN_W * scale).round() as i32, (WIN_H * scale).round() as i32);

    let hwnd = CreateWindowExW(
        WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        class,
        w!("노치 고양이"),
        WS_POPUP,
        mon.left + ((mon.right - mon.left) - wpx) / 2,
        mon.top,
        wpx,
        hpx,
        None,
        None,
        inst,
        None,
    )?;
    let g = Gfx::new(hwnd, wpx as u32, hpx as u32, 96.0 * scale)?;
    let now = Arc::new(Mutex::new(Now::default()));
    let lyr = Arc::new(Mutex::new(Lyr::default()));
    let tx = media::spawn(now.clone(), lyr.clone());
    let queue = Arc::new(Mutex::new(queue::Queue::default()));
    queue::spawn(now.clone(), queue.clone());
    queue::serve(queue.clone());
    queue::write_ext();
    let mut cfg = Cfg::load();
    cfg.apply_autostart();
    cfg.ensure_shortcut();
    let spot = cfg.place;

    let cal = Arc::new(Mutex::new(gcal::Cal::default()));
    let mut app = App {
        hwnd,
        g,
        scale,
        mon,
        cfg,
        now,
        lyr,
        tx,
        w: Spring::new(IDLE.0, 240.0, 0.78),
        h: Spring::new(0.0, 240.0, 0.82),
        fade: Spring::new(1.0, 200.0, 1.0),
        scroll: Spring::new(0.0, 170.0, 0.9),
        tm: Spring::new(0.0, 220.0, 1.0),
        face: face::Face::new(),
        born: Instant::now(),
        bump_until: Instant::now(),
        was_title: String::new(),
        was_playing: false,
        hover: false,
        showing_lyric: false,
        mouse: (-1.0, -1.0),
        tracking: false,
        hidden: false,
        maxed: false,
        shy: false,
        note_shy: false,
        last_pt: (0, 0),
        quotes: {
            let q = Arc::new(Mutex::new(quotes::Quotes::default()));
            quotes::spawn(q.clone());
            q
        },
        q: Vec::new(),
        q_stamp: String::new(),
        q_tried: String::new(),
        q_seen: None,
        pct: None,
        typing: false,
        page: 0,
        paged: Instant::now(),
        pgf: Spring::new(1.0, 260.0, 1.0),
        big: true, // 시세는 늘 값까지 나오는 큰 판(사용자 요청 2026-11-13, 눌러서 줄이기 없앰)
        bg: Spring::new(1.0, 190.0, 0.68),
        meter: meter::Meter::default(),
        bars: [0.0; 28],
        qs: Vec::new(),
        arr: [Spring::new(0.0, 320.0, 0.42), Spring::new(0.0, 320.0, 0.42)],
        ent: None,
        drop: Spring::new(0.0, 210.0, 0.5),
        nb: Spring::new(0.0, 210.0, 0.5),
        wx: {
            let w = Arc::new(Mutex::new(weather::Weather::default()));
            weather::spawn(w.clone(), spot.0, spot.1);
            w
        },
        wxn: weather::Weather::default(),
        cal: cal.clone(),
        caln: gcal::Cal::default(),
        cal_tx: gcal::spawn(cal),
        cal_month: 0,
        cal_big: false,
        cal_pop: None,
        acct_menu: false,
        naver_guide: false,
        naver_n: 0,
        cal_sel: gcal::today().0,
        cal_wait: Instant::now(),
        notes: {
            let n = Arc::new(Mutex::new(notif::Note::default()));
            notif::spawn(n.clone());
            n
        },
        note: notif::Note::default(),
        note_until: Instant::now(),
        note_icon: None,
        open_h: OPEN.1,
        mode: 0,
        last_input: 0,
        moved: Instant::now(),
        keyed: Instant::now() - Duration::from_secs(60),
        ck: Spring::new(0.0, 200.0, 1.0),
        ticks: 0,
        n: Now::default(),
        lines: Vec::new(),
        lyr_st: St::Idle,
        lyr_stamp: String::new(),
        lyr_ver: 0,
        cur: None,
        instr: None,
        text: String::new(),
        prev_text: String::new(),
        art: None,
        art_ptr: 0,
        hits: Vec::new(),
        rgn: [-1; 4],
        hold: (0.0, 0.0),
        view: String::new(),
        dirty: true,
        first_frame: true,
        queue,
        qn: queue::Queue::default(),
        qopen: std::env::var("NOTCH_CAT_QUEUE").is_ok(),
        qtop: 0,
        qrect: [0.0; 4],
        qrows: vec![],
        qbmp: HashMap::new(),
    };
    app.place();
    app.draw()?; // 빈(투명) 첫 장을 낸 뒤에 창을 보인다
    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    tray(hwnd, true);
    SetTimer(hwnd, 1, 50, None);
    log("window ok");
    if std::env::var("NOTCH_CAT_SETTINGS").is_ok() {
        settings::open(app.quotes.clone(), cat_icon()); // 시험용: 켜자마자 설정창
    }
    if let Some(n) = std::env::var("NOTCH_CAT_PAGE").ok().and_then(|v| v.parse().ok()) {
        app.hover = true; // 시험용: 그 쪽을 펼친 채로 띄운다(NOTCH_CAT_SHOT 과 함께 쓰면 마우스 없이 찍힌다)
        app.page = n;
        app.cal_big = std::env::var("NOTCH_CAT_CALBIG").is_ok();
        app.acct_menu = std::env::var("NOTCH_CAT_ACCT").is_ok(); // 시험용: '＋ 계정' 고르기 판
        app.naver_guide = std::env::var("NOTCH_CAT_NAVER").is_ok(); // 시험용: 네이버 연결 안내
        if std::env::var("NOTCH_CAT_CALPOP").is_ok() {
            app.cal_pop = Some((gcal::today().0, 1)); // 시험용: 오늘 두 번째 일정 카드
        }
    }
    if let Ok(v) = std::env::var("NOTCH_CAT_ENTRY") {
        // 시험용: 달력 쪽에 일정 칸을 연 채로 띄운다. "seg" 면 시 칸을 고른 채로, "note" 면 알림도 하나 띄운다.
        app.hover = true;
        app.page = 3;
        app.ent_open();
        if let Some(e) = app.ent.as_mut() {
            e.insert("치과 예약");
            e.seg = v.contains("seg").then_some(3);
            if v.contains("both") {
                e.cals = vec![0, 2]; // 두 계정 모두에 넣기
            }
            if v.contains("mid") {
                e.cur = 2; // 커서를 글 가운데로("치과|예약")
                e.comp = "ㄱ".into();
            }
        }
        if v.contains("note") {
            let mut n = app.notes.lock().unwrap();
            n.ver += 1;
            n.app = "카카오톡".into();
            n.title = "시험 알림: 노치 몸통은 그대로".into();
        }
    }

    let mut last = Instant::now();
    let mut was_anim = false;
    let mut msg = MSG::default();
    loop {
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            if msg.message == WM_QUIT {
                tray(hwnd, false);
                log("quit");
                return Ok(());
            }
            // 설정창이 열려 있으면 Tab·Esc 같은 자판을 그 창이 먼저 받는다
            if settings::hwnd().is_some_and(|h| IsDialogMessageW(h, &msg).as_bool()) {
                continue;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let evs: Vec<Ev> = EVENTS.with(|v| v.borrow_mut().drain(..).collect());
        for e in evs {
            match e {
                Ev::Move(mx, my) => {
                    app.mouse = (mx / app.scale, my / app.scale);
                    app.dirty = true;
                    if !app.tracking {
                        app.tracking = true;
                        let mut t = TRACKMOUSEEVENT { cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: hwnd, dwHoverTime: 0 };
                        let _ = TrackMouseEvent(&mut t);
                        release_below(hwnd); // 노치로 들어오기 직전에 있던 창(크롬 탭 미리보기 등)을 닫게 한다
                    }
                    // 꼬리·그림자 여백 위에서는 펼치지 않는다
                    let px0 = (WIN_W - app.w.pos) / 2.0;
                    let on_pill = app.mouse.0 >= px0 - EAR && app.mouse.0 <= px0 + app.w.pos + EAR && app.mouse.1 <= app.h.pos + 2.0;
                    if !app.hover && on_pill {
                        app.hover = true;
                        if let Some(e) = app.ent.as_mut().filter(|e| e.parked && !e.closing) {
                            e.parked = false; // 접어 둔 일정 칸을 달력 쪽과 함께 다시 꺼낸다
                            app.page = 3;
                            app.cal_sel = e.day;
                            app.drop.target = 1.0;
                        }
                        app.face.touch();
                        if !app.n.has {
                            app.face.happy(900); // 펼칠 게 없을 땐 반가워한다
                        }
                        app.tick();
                    }
                }
                Ev::Wheel(d) => {
                    let (mx, my) = app.mouse;
                    let r = app.qrect;
                    let under = app.hits.iter().find(|(b, _)| mx >= b[0] && mx < b[2] && my >= b[1] && my < b[3]).map(|(_, h)| *h);
                    let seg = under.and_then(|h| if let Hit::EntSeg(i) = h { Some(i) } else { None });
                    if let Some(Hit::EntAcct(ai)) = under {
                        app.ent_acct(ai, -d); // 넣을 곳 위에서 굴리면 그 계정 안 캘린더를 바꾼다
                    } else if let Some(i) = seg {
                        // 날짜·시각 칸 위에서 굴리면 그 칸만 바꾼다(위로 굴리면 커진다)
                        if let Some(e) = app.ent.as_mut() {
                            (e.day, e.min) = entry::bump(e.day, e.min, i, -d);
                            e.seg = Some(i);
                            e.buf.clear();
                            app.cal_sel = e.day;
                        }
                        app.dirty = true;
                    } else if app.ent_up() && my > app.h.pos {
                        // 일정 칸 위에서 굴린 것은 쪽을 넘기지 않는다
                    } else if app.qopen && mx >= r[0] && mx < r[2] && my >= r[1] && my < r[3] {
                        // 목록 위에서 굴리면 목록만 넘긴다(쪽은 그대로)
                        app.qtop = (app.qtop as i32 + d * 2).max(0) as usize;
                        app.dirty = true;
                        app.tick();
                    } else {
                        app.turn(d)
                    }
                }
                Ev::Leave if std::env::var("NOTCH_CAT_SHOT").is_ok() => {} // 시험 중엔 마우스가 없어도 펼친 채로
                Ev::Leave if app.ent_up() => app.tracking = false, // 적는 동안은 마우스가 나가도 펼친 채로
                Ev::Leave => app.leave(),
                Ev::Click(x, y) => app.click(x, y),
                Ev::Char(c) => app.ent_char(c),
                Ev::Key(k) => app.ent_key(k),
                Ev::Text(t) => {
                    if let Some(e) = app.ent.as_mut() {
                        e.seg = None; // 한글을 치면 제목 칸으로
                        e.insert(&t);
                        e.caret = Instant::now();
                    }
                    app.dirty = true;
                }
                Ev::Comp(t) => {
                    if let Some(e) = app.ent.as_mut() {
                        e.comp = t;
                        e.caret = Instant::now();
                    }
                    app.dirty = true;
                }
                Ev::Blur => {
                    // 다른 창으로 갔다: 적던 게 없으면 닫고, 있으면 칸과 노치를 접어 두되 적던 것은 남긴다.
                    // 다시 노치에 마우스를 올리면 달력 쪽으로 열리며 칸이 그대로 나온다.
                    if app.ent.as_ref().is_some_and(|e| e.text.is_empty() && e.comp.is_empty()) {
                        app.ent_close();
                    } else if app.ent.is_some() {
                        app.ent_park();
                        app.leave();
                    }
                }
                Ev::Tick => app.tick(),
                Ev::Tray => {
                    if app.tray_menu() {
                        let _ = DestroyWindow(hwnd);
                    }
                }
            }
        }
        let anim = app.animating();
        if anim || app.dirty {
            let dt = if was_anim { last.elapsed().as_secs_f32() } else { 1.0 / 60.0 };
            last = Instant::now();
            app.w.step(dt);
            app.h.step(dt);
            app.fade.step(dt);
            app.drop.step(dt);
            app.nb.step(dt);
            app.tm.step(dt);
            app.ck.step(dt);
            app.pgf.step(dt);
            app.arr[0].step(dt);
            app.arr[1].step(dt);
            app.bg.step(dt);
            app.qs.iter_mut().for_each(|s| s.step(dt));
            app.face.step(dt);
            app.scroll.step(dt);
            app.dirty = false;
            if let Err(e) = app.draw() {
                // 그래픽 장치를 잃었다(절전에서 깨어남·드라이버 재시작 등, 0x8899000C). 장치를 통째로 다시 만들어야 하므로
                // 새로 켜서 이어 간다. 켜자마자 또 나면 되풀이하지 않고 끈다.
                log(&format!("ERR draw {e:?}"));
                if app.born.elapsed() > Duration::from_secs(20) {
                    if let Ok(exe) = std::env::current_exe() {
                        if std::process::Command::new(exe).spawn().is_ok() {
                            log("restart after draw error");
                            tray(hwnd, false);
                            return Ok(());
                        }
                    }
                }
                return Err(e);
            }
        } else {
            let _ = WaitMessage();
        }
        was_anim = anim;
    }
}

fn main() {
    std::panic::set_hook(Box::new(|i| log(&format!("ERR panic: {i}"))));
    if let Err(e) = unsafe { run() } {
        log(&format!("ERR {e:?}"));
        std::process::exit(1);
    }
}
