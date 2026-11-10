// 노치 고양이 — 화면 위 가운데 노치. 1단계: 틀·움직임·음악·가사.
// 설계: docs/superpowers/specs/2026-11-10-notch-cat-design.md
#![windows_subsystem = "windows"]

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
const IDLE: (f32, f32) = (96.0, 24.0);
const BAR_H: f32 = 28.0;
const OPEN: (f32, f32) = (460.0, 246.0);
const SHADOW: f32 = 20.0; // 펼쳤을 때 그림자가 번지는 폭

// 색: 디자인.md '대비 강한 조합' — 미드나잇 네이비 · 쿨 화이트 · 일렉트릭 라임
const BG: Rgb = (0.0, 0.0, 0.0);
const FG: Rgb = (0.953, 0.957, 0.965);
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
    aside: bool, // 가운데 대신 오른쪽으로 비켜 놓기(다른 노치 앱과 나란히 볼 때)
}

impl Cfg {
    fn load() -> Cfg {
        let v: serde_json::Value = std::fs::read_to_string(data_dir().join("settings.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Cfg { hide_fullscreen: v["hide_fullscreen"].as_bool().unwrap_or(true), autostart: v["autostart"].as_bool().unwrap_or(true), aside: v["aside"].as_bool().unwrap_or(false) }
    }

    fn save(&self) {
        let v = serde_json::json!({ "hide_fullscreen": self.hide_fullscreen, "autostart": self.autostart, "aside": self.aside });
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
    // 상태
    hover: bool,
    showing_lyric: bool,
    mouse: (f32, f32), // 논리 좌표. 밖이면 음수
    tracking: bool,
    hidden: bool,
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
        !(self.w.settled() && self.h.settled() && self.fade.settled() && self.scroll.settled())
    }

    /// 0.1초마다: 음악·가사 상태를 읽어 목표 크기와 보일 글자를 정한다.
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
            _ => self.n.title.clone(),
        };
        self.showing_lyric = self.n.has && self.n.playing && line.is_some();
        if text != self.text {
            self.prev_text = std::mem::replace(&mut self.text, text);
            self.fade.pos = 0.0;
            self.fade.target = 1.0;
        }
        if self.ticks % 5 == 0 {
            self.hidden = self.cfg.hide_fullscreen && fullscreen_in_front(self.hwnd, &self.mon);
        }
        let (tw, th) = if self.hidden {
            (IDLE.0 * 0.6, 0.0)
        } else if !self.n.has {
            IDLE
        } else if self.hover {
            OPEN
        } else {
            {
            // 닫힘은 작게: 곡명은 짧게 자르고, 가사 줄일 때만 조금 더 넓힌다
            let max = if self.showing_lyric { 300.0 } else { 190.0 };
            ((self.g.measure(&self.text, &self.g.f_bar) + 50.0).clamp(110.0, max), BAR_H)
        }
        };
        self.w.target = tw;
        self.h.target = th;
        // 보이는 것이 달라졌을 때만 다시 그린다(쉴 때 CPU 를 안 쓴다)
        let open = self.h.target > BAR_H;
        let view = format!(
            "{}|{}|{}|{}|{:?}|{}|{}|{}|{}",
            self.text, self.n.title, self.n.playing, self.art_ptr, self.cur, self.lyr_st as u8,
            if open { pos / 500 } else { 0 },
            if open { self.n.stamp.as_str() } else { "" },
            self.lyr_stamp
        );
        if view != self.view {
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
        let p = ((h - BAR_H) / (OPEN.1 - BAR_H)).clamp(0.0, 1.0); // 0 닫힘 … 1 펼침
        let m = SHADOW * p; // 그림자 자리만큼 영역을 넓힌다
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
            g.rrect(x, -r, w, h + r, r, BG, 1.0); // 위쪽은 화면 밖으로 넘겨 위 모서리를 각지게
            g.clip(x, 0.0, w, h);
            let n = &self.n;

            // ── 닫힘: 그림 + 한 줄 ──
            let a = (1.0 - p * 3.0).clamp(0.0, 1.0) * (h / BAR_H).min(1.0);
            if a > 0.0 && n.has {
                match &self.art {
                    Some(b) => g.art(b, x + 8.0, 5.0, 18.0, 5.0, a),
                    None => g.rrect(x + 13.0, 10.0, 8.0, 8.0, 4.0, LIME, a),
                }
                let dim = if n.playing { 1.0 } else { 0.55 };
                let f = self.fade.pos.clamp(0.0, 1.0);
                let tw = (w - 44.0).max(1.0);
                let lw = (self.w.target - 44.0).max(1.0); // 목표 너비 기준으로 말줄임(움직이는 동안 글자가 흔들리지 않게)
                g.clip(x + 33.0, 0.0, tw, BAR_H);
                g.text(&self.prev_text, &g.f_bar, x + 33.0, 5.5 - f * 6.0, lw, FG, a * dim * (1.0 - f), Align::Left);
                g.text(&self.text, &g.f_bar, x + 33.0, 5.5 + (1.0 - f) * 6.0, lw, FG, a * dim * f, Align::Left);
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
        hover: false,
        showing_lyric: false,
        mouse: (-1.0, -1.0),
        tracking: false,
        hidden: false,
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
    SetTimer(hwnd, 1, 100, None);
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
                    if !app.hover {
                        app.hover = true;
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
