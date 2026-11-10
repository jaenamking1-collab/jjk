// 노치 고양이 — 화면 위 가운데 노치. 1단계: 틀·움직임·음악·가사.
// 설계: docs/superpowers/specs/2026-11-10-notch-cat-design.md
#![windows_subsystem = "windows"]

mod face;
mod gfx;
mod lrc;
mod lyrics;
mod media;
mod spring;

use std::cell::RefCell;
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
use windows::Win32::UI::Input::KeyboardAndMouse::{TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT};
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use gfx::{Align, Gfx, Rgb};
use lyrics::{Lyr, St};
use media::{Cmd, Now};
use spring::Spring;

const VERSION: &str = env!("CARGO_PKG_VERSION");

// 창(=그릴 수 있는 최대 넓이), 논리 단위
const WIN_W: f32 = 560.0;
const WIN_H: f32 = 290.0;
// 노치 크기
const IDLE: (f32, f32) = (150.0, 28.0);
const BAR_H: f32 = 28.0;
const FACE_ZOOM: f32 = 1.15;
const QUIET: (f32, f32) = (250.0, 104.0); // 음악이 없을 때 펼친 크기(시계)
const TINY: (f32, f32) = (48.0, 9.0); // 일할 때 줄어든 크기
const OPEN: (f32, f32) = (460.0, 246.0);
const SHADOW: f32 = 20.0; // 펼쳤을 때 그림자가 번지는 폭

// 색: 디자인.md '대비 강한 조합' — 미드나잇 네이비 · 쿨 화이트 · 일렉트릭 라임
const BG: Rgb = (0.0, 0.0, 0.0);
const FG: Rgb = (0.953, 0.957, 0.965);
const EYE: Rgb = (0.639, 0.902, 0.208); // 고양이 눈(라임)
const PUPIL: Rgb = (0.063, 0.086, 0.043);
const TAIL: Rgb = (0.075, 0.095, 0.075);
const NOSE: Rgb = (0.757, 0.784, 0.741);
const WHISK: Rgb = (0.81, 0.83, 0.81);
const GLOW: [Rgb; 3] = [(0.176, 0.831, 0.749), (0.3, 0.8, 0.45), (0.55, 0.85, 0.2)]; // 아래 빛: 청록 → 라임
const EAR: f32 = 6.0; // 위쪽 모서리가 바깥으로 휘는 크기
const TAIL_ROOM: f32 = 22.0; // 꼬리가 나올 옆 여백
const LIME: Rgb = (0.639, 0.902, 0.208);
const WARN: Rgb = (1.0, 0.416, 0.102);

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
    aside: bool, // 가운데 대신 오른쪽으로 비켜 놓기(다른 노치 앱과 나란히 볼 때)
}

impl Cfg {
    fn load() -> Cfg {
        let v: serde_json::Value = std::fs::read_to_string(data_dir().join("settings.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Cfg { hide_fullscreen: v["hide_fullscreen"].as_bool().unwrap_or(true), autostart: v["autostart"].as_bool().unwrap_or(true), aside: v["aside"].as_bool().unwrap_or(false), tiny: v["tiny"].as_bool().unwrap_or(true) }
    }

    fn save(&self) {
        let v = serde_json::json!({ "hide_fullscreen": self.hide_fullscreen, "autostart": self.autostart, "aside": self.aside, "tiny": self.tiny });
        let _ = std::fs::write(data_dir().join("settings.json"), v.to_string());
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
}

thread_local! {
    static EVENTS: RefCell<Vec<Ev>> = const { RefCell::new(Vec::new()) };
}

fn push(e: Ev) {
    EVENTS.with(|v| v.borrow_mut().push(e));
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_MOUSEMOVE => push(Ev::Move((lp.0 & 0xFFFF) as i16 as f32, ((lp.0 >> 16) & 0xFFFF) as i16 as f32)),
        WM_MOUSELEAVE => push(Ev::Leave),
        WM_LBUTTONUP => push(Ev::Click((lp.0 & 0xFFFF) as i16 as f32, ((lp.0 >> 16) & 0xFFFF) as i16 as f32)),
        WM_TIMER => push(Ev::Tick),
        WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as isize),
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

/// 앞에 있는 창이 최대화돼 있나(일하는 중).
unsafe fn maximized_in_front(own: HWND) -> bool {
    let fg = GetForegroundWindow();
    !fg.0.is_null() && fg != own && IsZoomed(fg).as_bool()
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
    last_pt: (i32, i32),
    last_input: u32,
    moved: Instant, // 마지막으로 클릭하거나 자판을 친 때
    ticks: u32,
    n: Now,
    lines: Vec<(u64, String)>,
    lyr_st: St,
    lyr_stamp: String,
    lyr_ver: u64,
    cur: Option<usize>,
    text: String,
    prev_text: String,
    art: Option<ID2D1Bitmap>,
    art_ptr: usize,
    hits: Vec<([f32; 4], Hit)>,
    rgn: [i32; 4],
    view: String,
    dirty: bool,
    first_frame: bool,
}

impl App {
    fn animating(&self) -> bool {
        !(self.w.settled() && self.h.settled() && self.fade.settled() && self.scroll.settled() && self.tm.settled() && self.face.settled())
    }

    /// 0.05초마다: 음악·가사 상태를 읽어 목표 크기와 보일 글자를 정한다.
    unsafe fn tick(&mut self) {
        self.ticks += 1;
        self.n = self.now.lock().unwrap().clone();
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
        let cur = if self.n.has { lrc::current(&self.lines, pos.max(0) as u64) } else { None };
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
        let text = match (self.n.has, self.n.playing, line) {
            (false, _, _) => String::new(),
            (true, true, Some(l)) => l.to_string(),
            (true, true, None) => self.n.title.clone(),
            _ => String::new(), // 멈춰 있으면 얼굴만
        };
        self.showing_lyric = self.n.has && self.n.playing && line.is_some();
        if text != self.text {
            self.prev_text = std::mem::replace(&mut self.text, text);
            self.fade.pos = 0.0;
            self.fade.target = 1.0;
        }
        if self.ticks % 10 == 0 {
            self.hidden = self.cfg.hide_fullscreen && fullscreen_in_front(self.hwnd, &self.mon);
            let maxed = self.cfg.tiny && std::env::var("NOTCH_CAT_TINY").map_or(true, |v| v != "0") && maximized_in_front(self.hwnd);
            if maxed && !self.maxed {
                self.moved = Instant::now() - Duration::from_secs(9); // 창을 꽉 채워 여는 순간 바로 줄어든다(마우스를 움직이면 돌아온다)
            }
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
        let close = mx > px0 - 50.0 && mx < px0 + pw + 50.0 && my < ph + 45.0;
        // 클릭하거나 자판을 치면 고양이가 나오고(마우스만 움직여서는 안 나온다), 8초 조용하면 다시 줄어든다
        let mut li = windows::Win32::UI::Input::KeyboardAndMouse::LASTINPUTINFO { cbSize: std::mem::size_of::<windows::Win32::UI::Input::KeyboardAndMouse::LASTINPUTINFO>() as u32, dwTime: 0 };
        let _ = windows::Win32::UI::Input::KeyboardAndMouse::GetLastInputInfo(&mut li);
        if li.dwTime != self.last_input && (pt.x, pt.y) == self.last_pt {
            self.moved = Instant::now(); // 입력은 있었는데 마우스는 제자리 = 클릭·자판
        }
        self.last_input = li.dwTime;
        self.last_pt = (pt.x, pt.y);
        let tiny = self.maxed && !close && !self.hover && self.moved.elapsed() > Duration::from_secs(8);
        if tiny {
            self.face.tail.target = 0.0;
        }
        self.tm.target = if self.text.is_empty() || tiny { 0.0 } else { 100.0 };

        let bump = if Instant::now() < self.bump_until { (6.0, 2.0) } else { (0.0, 0.0) };
        let (tw, th) = if self.hidden {
            (IDLE.0 * 0.6, 0.0)
        } else if tiny {
            TINY
        } else if self.hover {
            if self.n.has { OPEN } else { QUIET }
        } else if self.text.is_empty() {
            (IDLE.0 + bump.0, IDLE.1 + bump.1)
        } else {
            // 닫힘은 작게: 곡명은 짧게 자르고, 가사 줄일 때만 조금 더 넓힌다
            let max = if self.showing_lyric { 300.0 } else { 230.0 };
            ((self.g.measure(&self.text, &self.g.f_bar) + 70.0).clamp(IDLE.0, max) + bump.0, BAR_H + bump.1)
        };
        // 펼칠 때는 끝에서 살짝 출렁이며 모이고(가로·세로 박자가 달라 물결처럼), 접힐 때는 차분하게
        if th > BAR_H + 10.0 {
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
            "{}|{}|{}|{}|{}|{:?}|{}|{}|{}|{}",
            local().wMinute, self.text, self.n.title, self.n.playing, self.art_ptr, self.cur, self.lyr_st as u8,
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
        let p = ((h - BAR_H) / (if self.n.has { OPEN.1 } else { QUIET.1 } - BAR_H)).clamp(0.0, 1.0); // 0 닫힘 … 1 펼침
                let m = (SHADOW * p).max(EAR + 1.0).max(if self.face.tail.pos > 0.3 { TAIL_ROOM } else { 0.0 }); // 그림자·휜 모서리·꼬리 자리만큼 영역을 넓힌다
        let rgn = if h < 0.5 { [0; 4] } else { [((x - m) * s).floor() as i32, 0, ((x + w + m) * s).ceil() as i32, ((h + m) * s).ceil() as i32] };
        if rgn != self.rgn {
            self.rgn = rgn;
            SetWindowRgn(self.hwnd, CreateRectRgn(rgn[0], rgn[1], rgn[2], rgn[3]), false);
        }
        self.hits.clear();
        g.begin();
        if h >= 0.5 {
            let r = (14.0 + 16.0 * p).min(h);
            for i in 0..8 {
                let gr = (8 - i) as f32 * 2.2; // 겹겹이 옅게 → 부드러운 그림자
                g.rrect(x - gr, -r, w + gr * 2.0, h + r + gr + 3.0, r + gr, (0.0, 0.0, 0.0), 0.045 * p);
            }
            let t = self.born.elapsed().as_secs_f32();
            let ca = (1.0 - p * 3.0).clamp(0.0, 1.0) * (h / BAR_H).min(1.0); // 닫힘 내용의 진하기
            let fa = ((h - 12.0) / 12.0).clamp(0.0, 1.0); // 아주 작게 줄면 얼굴은 사라진다
            // 꼬리: 잠들면 옆에서 슬그머니 나오는 짧고 도톰한 물음표. 몸보다 먼저 그려서 뿌리가 몸 뒤에 숨는다.
            let reveal = (self.face.tail.pos / 100.0).clamp(0.0, 1.0);
            if reveal > 0.003 && ca * fa > 0.0 {
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
                ribbon(g, &pts, |u| 2.0 * (3.1 - 0.9 * u * u * (3.0 - 2.0 * u)), TAIL, ca * fa);
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
            if ca * fa > 0.0 {
                use std::f32::consts::PI;
                let ca = ca * fa;
                let f = &self.face;
                let st = &face::STYLES[f.style];
                let fcx = x + w / 2.0 + (28.0 - w / 2.0) * tm;
                // 아래 빛: 넓고 옅은 것부터 세 겹. 숨 쉬듯 아주 조금만 변한다.
                let gk = f.glow.pos / 100.0 * (1.0 + 0.12 * (t * 1.31).sin()) * (1.0 - 0.45 * tm) * 2.4;
                for (i, (gw, gh, ga)) in [(72.0, 14.0, 0.035), (52.0, 8.0, 0.06), (28.0, 4.0, 0.1)].into_iter().enumerate() {
                    g.ellipse(fcx, h - 2.0 + gh * 0.1, gw / 2.0, gh / 2.0, GLOW[i], ca * ga * gk);
                }
                g.zoom(FACE_ZOOM, fcx, 14.0); // 눈·코·입·수염을 한꺼번에 키운다
                let breath = if f.asleep { (t * 1.208).sin() * 0.35 } else { 0.0 };
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
                            // 감은 눈 ∪
                            let y = cy + 1.0;
                            ribbon(g, &quad((ex - st.rx + 0.5, y), (ex, y + 3.2), (ex + st.rx - 0.5, y), 10), |_| 1.6, EYE, ea);
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

            // ── 닫힘: 한 줄(얼굴 오른쪽) ──
            let a = ca * fa * tm;
            if a > 0.0 && n.has {
                let dim = if n.playing { 1.0 } else { 0.55 };
                let f = self.fade.pos.clamp(0.0, 1.0);
                let tw = (w - 68.0).max(1.0);
                let lw = (self.w.target - 68.0).max(1.0); // 목표 너비 기준으로 말줄임(움직이는 동안 글자가 흔들리지 않게)
                g.clip(x + 56.0, 0.0, tw, BAR_H + 4.0);
                g.text(&self.prev_text, &g.f_bar, x + 56.0, 5.5 - f * 6.0, lw, FG, a * dim * (1.0 - f), Align::Left);
                g.text(&self.text, &g.f_bar, x + 56.0, 5.5 + (1.0 - f) * 6.0, lw, FG, a * dim * f, Align::Left);
                g.unclip();
            }

            // ── 펼침: 음악 쪽 ──
            let a = ((p - 0.35) / 0.65).clamp(0.0, 1.0);
            if a > 0.0 && n.has {
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
                g.text(&mmss(pos), &g.f_time, x + 18.0, 87.0, 44.0, FG, a, Align::Left);
                g.text(&if n.dur_ms > 0 { mmss(n.dur_ms) } else { "–:––".into() }, &g.f_time, x + w - 62.0, 87.0, 44.0, FG, a, Align::Right);
                g.rrect(b0, 94.0, bw, 5.0, 2.5, FG, a * 0.22);
                if n.dur_ms > 0 {
                    g.rrect(b0, 94.0, (bw * pos as f32 / n.dur_ms as f32).clamp(5.0, bw), 5.0, 2.5, LIME, a);
                    if live {
                        self.hits.push(([b0, 84.0, b0 + bw, 108.0], Hit::Bar(b0, bw)));
                    }
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
                if self.lyr_st == St::Found {
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
                g.text(&alive, &g.f_tiny, x + 18.0, 227.0, w - 36.0, if stale { WARN } else { FG }, a * if stale { 1.0 } else { 0.4 }, Align::Right);
            } else if a > 0.0 {
                // 음악이 없으면 시계와 날짜
                let t = local();
                let h12 = if t.wHour % 12 == 0 { 12 } else { t.wHour % 12 };
                let time = format!("{} {}:{:02}", if t.wHour < 12 { "오전" } else { "오후" }, h12, t.wMinute);
                let day = format!("{}월 {}일 {}요일", t.wMonth, t.wDay, ["일", "월", "화", "수", "목", "금", "토"][t.wDayOfWeek as usize % 7]);
                g.text(&time, &g.f_clock, x, 26.0, w, FG, a, Align::Center);
                g.text(&day, &g.f_sub, x, 66.0, w, FG, a * 0.6, Align::Center);
            }
            g.unclip();
        }
        g.present()?;
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

    unsafe fn click(&mut self, px: f32, py: f32) {
        let (x, y) = (px / self.scale, py / self.scale);
        let Some(&(_, hit)) = self.hits.iter().find(|(r, _)| x >= r[0] && x < r[2] && y >= r[1] && y < r[3]) else { return };
        let cmd = match hit {
            Hit::Prev => Cmd::Prev,
            Hit::Toggle => Cmd::Toggle,
            Hit::Next => Cmd::Next,
            Hit::Bar(b0, bw) => Cmd::Seek((((x - b0) / bw).clamp(0.0, 1.0) * self.n.dur_ms as f32) as i64),
            Hit::Line(t) => Cmd::Seek(t as i64),
        };
        let _ = self.tx.send(cmd);
    }

    unsafe fn tray_menu(&mut self) -> bool {
        let Ok(m) = CreatePopupMenu() else { return false };
        let check = |on: bool| if on { MF_CHECKED } else { MF_UNCHECKED };
        let title: Vec<u16> = format!("노치 고양이 {VERSION}").encode_utf16().chain([0]).collect();
        let _ = AppendMenuW(m, MF_GRAYED, 0, PCWSTR(title.as_ptr()));
        let _ = AppendMenuW(m, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(m, check(self.cfg.hide_fullscreen), 1, w!("전체 화면일 때 숨기기"));
        let _ = AppendMenuW(m, check(self.cfg.autostart), 2, w!("윈도우 시작할 때 자동 실행"));
        let _ = AppendMenuW(m, check(self.cfg.aside), 4, w!("오른쪽으로 비켜 놓기"));
        let _ = AppendMenuW(m, check(self.cfg.tiny), 6, w!("최대화 창에서는 작게 줄이기"));
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

unsafe fn tray(hwnd: HWND, add: bool) {
    let mut d = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
        uCallbackMessage: WM_TRAY,
        hIcon: LoadIconW(None, IDI_APPLICATION).unwrap_or_default(),
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
        return Ok(()); // 이미 켜져 있다
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
    let cfg = Cfg::load();
    cfg.apply_autostart();

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
        last_pt: (0, 0),
        last_input: 0,
        moved: Instant::now(),
        ticks: 0,
        n: Now::default(),
        lines: Vec::new(),
        lyr_st: St::Idle,
        lyr_stamp: String::new(),
        lyr_ver: 0,
        cur: None,
        text: String::new(),
        prev_text: String::new(),
        art: None,
        art_ptr: 0,
        hits: Vec::new(),
        rgn: [-1; 4],
        view: String::new(),
        dirty: true,
        first_frame: true,
    };
    app.place();
    app.draw()?; // 빈(투명) 첫 장을 낸 뒤에 창을 보인다
    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    tray(hwnd, true);
    SetTimer(hwnd, 1, 50, None);
    log("window ok");

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
                    }
                    // 꼬리·그림자 여백 위에서는 펼치지 않는다
                    let px0 = (WIN_W - app.w.pos) / 2.0;
                    let on_pill = app.mouse.0 >= px0 - EAR && app.mouse.0 <= px0 + app.w.pos + EAR && app.mouse.1 <= app.h.pos + 2.0;
                    if !app.hover && on_pill {
                        app.hover = true;
                        app.face.touch();
                        if !app.n.has {
                            app.face.happy(900); // 펼칠 게 없을 땐 반가워한다
                        }
                        app.tick();
                    }
                }
                Ev::Leave => {
                    app.tracking = false;
                    app.hover = false;
                    app.mouse = (-1.0, -1.0);
                    app.tick();
                }
                Ev::Click(x, y) => app.click(x, y),
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
            app.tm.step(dt);
            app.face.step(dt);
            app.scroll.step(dt);
            app.dirty = false;
            app.draw()?;
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
