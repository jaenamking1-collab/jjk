//! 스크린샷 편집 창: 자르기 · 글자(글꼴·크기·색, 끌어서 크기, 두 번 눌러 다시 편집) · 이모지 · 글자 인식.
//! '복사'·'저장'은 결과 그림을 클립보드에 넣는다(노치가 클립보드 쪽 새 항목으로 받아 적는다).
//! 글자 인식은 그림 전체를 읽어 낱말 상자를 기억해 두고, 그림 위를 끌면 글자를 긁듯이 골라 클립보드에 넣는다(윈도우 기본 글자 인식, 한국어).
//! 크롭은 틀의 손잡이를 끌어 남길 곳을 정한 뒤 적용, 자르기는 끌자마자 자른다. 되돌리기는 20단계.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use windows::core::{w, Interface, HSTRING, PCWSTR};
use windows::Foundation::Numerics::Matrix3x2;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::DirectWrite::*;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{BeginPaint, EndPaint, InvalidateRect, PAINTSTRUCT};
use windows::Win32::Graphics::Imaging::*;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::Ime::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, ReleaseCapture, SetCapture, VK_SHIFT};
use windows::Win32::UI::WindowsAndMessaging::*;

pub const WM_EDTOAST: u32 = WM_APP + 6; // 노치에게: 알림 물방울을 띄워 달라
pub const WM_EDCLOSE: u32 = WM_APP + 10; // 노치에게: 닫아 달라(노치 안으로 빨아들인 뒤 없앤다)
pub static TOAST: Mutex<Option<(String, String)>> = Mutex::new(None);

const EFFECTS: [&str; 3] = ["그림자", "외곽선", "없음"];
const FONTS: [(&str, &str); 6] = [("맑은 고딕", "Malgun Gothic"), ("굴림", "Gulim"), ("바탕", "Batang"), ("궁서", "Gungsuh"), ("Segoe UI", "Segoe UI"), ("Impact", "Impact")];
const COLORS: [u32; 7] = [0xFFFFFF, 0x111111, 0xEF4444, 0xFACC15, 0xA3E635, 0x3B82F6, 0xEC4899];
const EMOJIS: &str = "😀 😂 🥹 😍 😎 🤔 😭 😡 😱 🥳 👍 👎 👏 🙏 💪 👉 👈 👆 ❤️ 💔 ✨ ⭐ 🔥 🎉 ✅ ❌ ⚠️ ❓ ❗ 💡 📌 📎 📷 🎵 ☕ 🍀 🐱 🐶 💯 🚀";
const BAR: f32 = 54.0; // 단추 줄 높이
const STYLE_H: f32 = 44.0; // 글자 모양 줄 높이

#[derive(Clone, Copy, PartialEq, Debug)]
enum Tool {
    Move,
    Crop,  // 끌어서 바로 자르기
    Frame, // 크롭: 틀을 조절한 뒤 적용
    Text,
    Ocr,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Btn {
    Tool(Tool),
    EmojiPanel,
    Emoji(usize),
    Font,
    FontPick(usize),
    Effect,
    EffColor(u8),
    Smaller,
    Bigger,
    Color(usize),
    Apply,
    Cancel,
    Undo,
    Copy,
    Save,
    Close,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Obj {
    pub text: String,
    pub emoji: bool,
    pub font: usize,
    pub size: f32, // 그림 점 단위 글자 크기
    pub color: usize,
    pub fx: u8, // 0 그림자 · 1 외곽선 · 2 없음
    pub ec: u8, // 그림자·외곽선 색(COLORS 번호, 255 = 자동: 글자색과 반대 밝기)
    pub x: f32,
    pub y: f32,
}

enum Drag {
    None,
    Move(usize, f32, f32),  // 칸, 잡은 곳과 칸 왼쪽 위의 차이
    Resize(usize, f32, f32), // 칸, 처음 크기, 처음 마우스 x(그림 좌표)
    Rect((f32, f32), (f32, f32)), // 자르기 영역(그림 좌표)
    Frame(u8, (f32, f32), [f32; 4]), // 크롭 틀: 잡은 곳(1 왼·2 위·4 오른·8 아래·16 통째), 처음 마우스, 처음 틀
    Words(usize), // 글자 인식: 끌기 시작한 낱말
}

/// 읽어 낸 낱말 하나(그림 좌표 상자 x·y·너비·높이, 몇째 줄)
#[derive(Clone, Debug, PartialEq)]
pub struct Word {
    pub text: String,
    pub r: [f32; 4],
    pub line: usize,
}

/// 되돌리기용 상태 하나(그림은 바뀌지 않으면 같이 쓴다)
#[derive(Clone)]
struct Snap {
    px: Rc<Vec<u8>>,
    w: u32,
    h: u32,
    objs: Vec<Obj>,
}

const UNDO_MAX: usize = 20;

/// 글자 수 → UTF-16 자리
fn u16_at(s: &str, ci: usize) -> u32 {
    s.chars().take(ci).map(|c| c.len_utf16() as u32).sum()
}

/// 글자 수 → 바이트 자리
fn byte_at(s: &str, ci: usize) -> usize {
    s.char_indices().nth(ci).map_or(s.len(), |(b, _)| b)
}

/// 크롭 틀에서 잡힌 곳(없으면 None, 안쪽은 16)
fn frame_hit(r: [f32; 4], p: (f32, f32), tol: f32) -> Option<u8> {
    if p.0 < r[0] - tol || p.0 > r[2] + tol || p.1 < r[1] - tol || p.1 > r[3] + tol {
        return None;
    }
    let mut m = 0;
    if (p.0 - r[0]).abs() <= tol {
        m |= 1;
    } else if (p.0 - r[2]).abs() <= tol {
        m |= 4;
    }
    if (p.1 - r[1]).abs() <= tol {
        m |= 2;
    } else if (p.1 - r[3]).abs() <= tol {
        m |= 8;
    }
    Some(if m == 0 { 16 } else { m })
}

/// 크롭 틀을 끈 결과(그림 밖으로 못 나가고, 8점보다 작아지지 않는다)
fn frame_drag(r0: [f32; 4], m: u8, dx: f32, dy: f32, w: f32, h: f32) -> [f32; 4] {
    let [mut x0, mut y0, mut x1, mut y1] = r0;
    if m == 16 {
        let dx = dx.min(w - x1).max(-x0);
        let dy = dy.min(h - y1).max(-y0);
        return [x0 + dx, y0 + dy, x1 + dx, y1 + dy];
    }
    if m & 1 != 0 {
        x0 = (x0 + dx).min(x1 - 8.0).max(0.0);
    }
    if m & 4 != 0 {
        x1 = (x1 + dx).max(x0 + 8.0).min(w);
    }
    if m & 2 != 0 {
        y0 = (y0 + dy).min(y1 - 8.0).max(0.0);
    }
    if m & 8 != 0 {
        y1 = (y1 + dy).max(y0 + 8.0).min(h);
    }
    [x0, y0, x1, y1]
}

/// 그 자리에서 가장 가까운 낱말과 거리(상자 안이면 0)
fn word_at(words: &[Word], p: (f32, f32)) -> Option<(usize, f32)> {
    words
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let dx = (w.r[0] - p.0).max(p.0 - w.r[0] - w.r[2]).max(0.0);
            let dy = (w.r[1] - p.1).max(p.1 - w.r[1] - w.r[3]).max(0.0);
            (i, dx + dy * 3.0) // 같은 줄의 낱말을 먼저 잡게 세로 거리를 더 크게 친다
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

/// 고른 낱말들(a~b, 읽는 순서)을 글로 — 같은 줄은 띄어쓰기, 줄이 바뀌면 줄바꿈
fn sel_text(words: &[Word], a: usize, b: usize) -> String {
    let (a, b) = (a.min(b), a.max(b).min(words.len().saturating_sub(1)));
    let mut out = String::new();
    for i in a..=b {
        let Some(w) = words.get(i) else { break };
        if i > a {
            out.push(if words[i - 1].line == w.line { ' ' } else { '\n' });
        }
        out.push_str(&w.text);
    }
    out
}

struct Ed {
    hwnd: HWND,
    notch: HWND,
    d2: ID2D1Factory,
    dw: IDWriteFactory,
    rt: Option<ID2D1HwndRenderTarget>,
    bmp: Option<ID2D1Bitmap>,
    px: Rc<Vec<u8>>, // BGRA(불투명)
    w: u32,
    h: u32,
    objs: Vec<Obj>,
    sel: Option<usize>,
    editing: bool,
    comp: String, // 조합 중인 한글
    tool: Tool,
    drag: Drag,
    emoji_open: bool,
    font: usize,
    size: f32,
    color: usize,
    mouse: (f32, f32),
    btns: Vec<([f32; 4], Btn)>,
    view: (f32, f32, f32), // 그림을 놓은 곳(왼쪽, 위, 배율)
    status: (String, Instant),
    s: f32, // 화면 배율
    caret: usize, // 적는 자리(글자 수)
    undo: Vec<Snap>,
    edit_pre: Option<Snap>, // 글자 편집을 시작하기 전
    drag_pre: Option<Snap>, // 끌기 시작하기 전(실제로 움직였을 때만 되돌리기에 쌓는다)
    down_at: (f32, f32),
    click_edit: bool, // 이미 골라 둔 글자를 다시 눌렀다 → 안 끌고 떼면 편집
    crop: [f32; 4],   // 크롭 틀(그림 좌표 x0·y0·x1·y1)
    words: Vec<Word>, // 글자 인식 결과
    osel: Option<(usize, usize)>, // 고른 낱말(시작, 끝)
    font_open: bool, // 글꼴 목록이 펼쳐져 있다
    fx: u8,          // 새 글자의 효과(그림자·외곽선·없음)
    ec: u8,          // 새 글자의 효과 색(255 자동)
}

thread_local! {
    static ED: RefCell<Option<Ed>> = const { RefCell::new(None) };
    static WANT_H: std::cell::Cell<i32> = const { std::cell::Cell::new(0) }; // 바라는 높이(처음 크기 또는 사용자가 끌어 바꾼 크기) — 노치 아래 남은 자리에 맞춰 줄인다
}

fn rgb(c: u32, a: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F { r: ((c >> 16) & 255) as f32 / 255.0, g: ((c >> 8) & 255) as f32 / 255.0, b: (c & 255) as f32 / 255.0, a }
}

fn rc(x: f32, y: f32, w: f32, h: f32) -> D2D_RECT_F {
    D2D_RECT_F { left: x, top: y, right: x + w, bottom: y + h }
}

fn notify(notch: HWND, title: &str, body: &str) {
    *TOAST.lock().unwrap() = Some((title.into(), body.into()));
    unsafe {
        let _ = PostMessageW(notch, WM_EDTOAST, WPARAM(0), LPARAM(0));
    }
}

/// 글자 칸 하나의 모양(배율 k 로 그릴 때)
unsafe fn layout(dw: &IDWriteFactory, o: &Obj, k: f32) -> Option<IDWriteTextLayout> {
    let fam = if o.emoji { "Segoe UI Emoji" } else { FONTS[o.font % FONTS.len()].1 };
    let f = dw.CreateTextFormat(&HSTRING::from(fam), None, DWRITE_FONT_WEIGHT_BOLD, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL, (o.size * k).max(1.0), w!("ko-kr")).ok()?;
    let t: Vec<u16> = if o.text.is_empty() { "글자".encode_utf16().collect() } else { o.text.encode_utf16().collect() };
    dw.CreateTextLayout(&t, &f, 10_000.0, 10_000.0).ok()
}

/// 글자 칸의 크기(그림 좌표)
unsafe fn obj_box(dw: &IDWriteFactory, o: &Obj) -> [f32; 4] {
    let Some(l) = layout(dw, o, 1.0) else { return [o.x, o.y, 10.0, 10.0] };
    let mut m = DWRITE_TEXT_METRICS::default();
    let _ = l.GetMetrics(&mut m);
    [o.x, o.y, m.widthIncludingTrailingWhitespace.max(8.0), m.height.max(8.0)]
}

/// 글자 칸들을 그린다(배율 k, 그림 왼쪽 위 ox·oy)
unsafe fn draw_objs(rt: &ID2D1RenderTarget, dw: &IDWriteFactory, objs: &[Obj], ox: f32, oy: f32, k: f32, comp: Option<(usize, usize, &str)>) {
    for (i, o) in objs.iter().enumerate() {
        let mut o = o.clone();
        if let Some((_, at, c)) = comp.filter(|c| c.0 == i) {
            let b = byte_at(&o.text, at);
            o.text.insert_str(b, c);
        }
        let Some(l) = layout(dw, &o, k) else { continue };
        let empty = o.text.is_empty();
        // 글자가 어느 바탕에서도 보이게: 그림자 또는 외곽선(글자색과 반대 밝기)
        if !o.emoji && !empty && o.fx < 2 {
            // 자동: 외곽선은 글자색과 반대 밝기, 그림자는 늘 어둡게(검정 글자면 회색 — 흰 그림자는 흰 바탕에서 안 보였다)
            let back = if (o.ec as usize) < COLORS.len() {
                COLORS[o.ec as usize]
            } else if o.fx == 0 {
                if o.color == 1 { 0x6B7280 } else { 0x000000 }
            } else if o.color == 1 {
                0xFFFFFF
            } else {
                0x000000
            };
            if o.fx == 0 {
                // 그림자: 글자 크기에 맞춰 오른쪽 아래로 밀고(바탕이 흰색이어도 보이게) 두 겹으로 부드럽게
                let d = (o.size * 0.06 * k).max(2.0);
                for (m, al) in [(1.0f32, 0.35f32), (0.6, 0.55)] {
                    if let Ok(sh) = rt.CreateSolidColorBrush(&rgb(back, al), None) {
                        rt.DrawTextLayout(D2D_POINT_2F { x: ox + o.x * k + d * m, y: oy + o.y * k + d * m }, &l, &sh, D2D1_DRAW_TEXT_OPTIONS_NONE);
                    }
                }
            } else if let Ok(sh) = rt.CreateSolidColorBrush(&rgb(back, 1.0), None) {
                // 같은 글자를 둘레 16방향으로 조금씩 밀어 찍어 테두리를 만든다
                let r = (o.size * 0.07 * k).max(1.0);
                for i in 0..16 {
                    let t = i as f32 * std::f32::consts::PI / 8.0;
                    rt.DrawTextLayout(D2D_POINT_2F { x: ox + o.x * k + r * t.cos(), y: oy + o.y * k + r * t.sin() }, &l, &sh, D2D1_DRAW_TEXT_OPTIONS_NONE);
                }
            }
        }
        if let Ok(b) = rt.CreateSolidColorBrush(&rgb(COLORS[o.color % COLORS.len()], if empty { 0.35 } else { 1.0 }), None) {
            rt.DrawTextLayout(D2D_POINT_2F { x: ox + o.x * k, y: oy + o.y * k }, &l, &b, D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT);
        }
    }
}

impl Ed {
    fn say(&mut self, s: &str) {
        self.status = (s.into(), Instant::now());
    }

    /// 화면 좌표 → 그림 좌표
    fn to_img(&self, p: (f32, f32)) -> (f32, f32) {
        let (ox, oy, k) = self.view;
        ((p.0 - ox) / k, (p.1 - oy) / k)
    }

    unsafe fn target(&mut self) -> Option<ID2D1HwndRenderTarget> {
        if self.rt.is_none() {
            let mut r = RECT::default();
            let _ = GetClientRect(self.hwnd, &mut r);
            let props = D2D1_RENDER_TARGET_PROPERTIES { dpiX: 96.0 * self.s, dpiY: 96.0 * self.s, ..Default::default() };
            let hp = D2D1_HWND_RENDER_TARGET_PROPERTIES { hwnd: self.hwnd, pixelSize: D2D_SIZE_U { width: r.right as u32, height: r.bottom as u32 }, presentOptions: D2D1_PRESENT_OPTIONS_NONE };
            self.rt = self.d2.CreateHwndRenderTarget(&props, &hp).ok();
            self.bmp = None;
        }
        self.rt.clone()
    }

    unsafe fn image(&mut self, rt: &ID2D1RenderTarget) -> Option<ID2D1Bitmap> {
        if self.bmp.is_none() {
            let props = D2D1_BITMAP_PROPERTIES { pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_IGNORE }, dpiX: 96.0, dpiY: 96.0 };
            self.bmp = rt.CreateBitmap(D2D_SIZE_U { width: self.w, height: self.h }, Some(self.px.as_ptr() as _), self.w * 4, &props).ok();
        }
        self.bmp.clone()
    }

    unsafe fn paint(&mut self) {
        let Some(hrt) = self.target() else { return };
        let rt: ID2D1RenderTarget = hrt.cast().unwrap();
        let mut r = RECT::default();
        let _ = GetClientRect(self.hwnd, &mut r);
        let (cw, ch) = (r.right as f32 / self.s, r.bottom as f32 / self.s);
        let text_sel = self.sel.and_then(|i| self.objs.get(i)).filter(|o| !o.emoji).cloned();
        let any_sel = self.sel.and_then(|i| self.objs.get(i)).cloned();
        let style = any_sel.is_some() || self.tool == Tool::Text;
        let frame = self.tool == Tool::Frame;
        let fx_cur = text_sel.as_ref().map_or(self.fx, |o| o.fx);
        let ec_cur = text_sel.as_ref().map_or(self.ec, |o| o.ec);
        let row2 = style && (text_sel.is_some() || self.tool == Tool::Text) && fx_cur < 2; // 그림자·외곽선 색 줄
        let sty_h = if style || frame { STYLE_H } else { 0.0 } + if row2 { STYLE_H - 6.0 } else { 0.0 };
        let per = ((cw - 28.0) / 40.0).floor().max(1.0) as usize;
        let emo_h = (EMOJIS.split(' ').count().div_ceil(per)) as f32 * 44.0 + 8.0;
        let top = BAR + sty_h + if self.emoji_open { emo_h } else { 0.0 };
        let (aw, ah) = (cw - 32.0, ch - top - 40.0);
        let k = (aw / self.w as f32).min(ah / self.h as f32).min(1.0);
        let (iw, ih) = (self.w as f32 * k, self.h as f32 * k);
        let (ox, oy) = (16.0 + (aw - iw) / 2.0, top + 8.0 + (ah - ih) / 2.0);
        self.view = (ox, oy, k);
        let bmp = self.image(&rt);
        rt.BeginDraw();
        rt.Clear(Some(&rgb(0x000000, 1.0))); // 노치와 한 몸으로 보이게 같은 검정
        let br = |c: u32, a: f32| rt.CreateSolidColorBrush(&rgb(c, a), None).unwrap();
        if let Some(b) = &bmp {
            rt.DrawBitmap(b, Some(&rc(ox, oy, iw, ih)), 1.0, D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, None);
        }
        let comp = self.sel.filter(|_| self.editing).map(|i| (i, self.caret, self.comp.as_str()));
        rt.PushAxisAlignedClip(&rc(ox, oy, iw, ih), D2D1_ANTIALIAS_MODE_ALIASED);
        draw_objs(&rt, &self.dw, &self.objs, ox, oy, k, comp);
        rt.PopAxisAlignedClip();
        // 고른 칸: 테두리 + 오른쪽 아래 크기 손잡이 + (적는 중이면) 깜빡이는 글자 자리
        if let Some(i) = self.sel {
            if let Some(o) = self.objs.get(i) {
                let mut o2 = o.clone();
                if self.editing {
                    let at = byte_at(&o2.text, self.caret);
                    o2.text.insert_str(at, &self.comp);
                }
                let b = obj_box(&self.dw, &o2);
                let bx = rc(ox + b[0] * k - 4.0, oy + b[1] * k - 4.0, b[2] * k + 8.0, b[3] * k + 8.0);
                rt.DrawRectangle(&bx, &br(0xA3E635, 1.0), 1.5, None);
                rt.FillRectangle(&rc(bx.right - 5.0, bx.bottom - 5.0, 10.0, 10.0), &br(0xA3E635, 1.0));
                if self.editing && (self.status.1.elapsed().as_millis() / 500) % 2 == 0 || self.editing && o.text.is_empty() {
                    // 적는 자리(조합 중인 글자 뒤)
                    let (mut cx, mut cy, mut chh) = (0.0f32, 0.0f32, b[3]);
                    if !o2.text.is_empty() {
                        if let Some(l) = layout(&self.dw, &o2, 1.0) {
                            let mut m = DWRITE_HIT_TEST_METRICS::default();
                            let at = u16_at(&o2.text, self.caret + self.comp.chars().count());
                            if l.HitTestTextPosition(at, BOOL(0), &mut cx, &mut cy, &mut m).is_ok() {
                                chh = m.height;
                            }
                        }
                    }
                    rt.FillRectangle(&rc(ox + (b[0] + cx) * k, oy + (b[1] + cy) * k + 2.0, 2.0, chh * k - 4.0), &br(0xA3E635, 1.0));
                }
            }
        }
        // 글자 인식: 읽힌 낱말은 밑줄로, 고른 낱말은 줄마다 이어서 파랗게
        if self.tool == Tool::Ocr {
            for w in &self.words {
                rt.FillRectangle(&rc(ox + w.r[0] * k, oy + (w.r[1] + w.r[3]) * k, w.r[2] * k, 1.5), &br(0x3B82F6, 0.55));
            }
            if let Some((a, b)) = self.osel {
                let (a, b) = (a.min(b), a.max(b).min(self.words.len().saturating_sub(1)));
                let mut i = a;
                while i <= b && i < self.words.len() {
                    let ln = self.words[i].line;
                    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
                    while i <= b && i < self.words.len() && self.words[i].line == ln {
                        let r = self.words[i].r;
                        x0 = x0.min(r[0]);
                        y0 = y0.min(r[1]);
                        x1 = x1.max(r[0] + r[2]);
                        y1 = y1.max(r[1] + r[3]);
                        i += 1;
                    }
                    rt.FillRectangle(&rc(ox + x0 * k - 2.0, oy + y0 * k - 1.0, (x1 - x0) * k + 4.0, (y1 - y0) * k + 2.0), &br(0x3B82F6, 0.42));
                }
            }
        }
        // 크롭 틀: 바깥은 어둡게, 테두리·삼등분 선·손잡이 여덟 개
        if frame {
            let [x0, y0, x1, y1] = self.crop;
            let (fx, fy, fw, fh) = (ox + x0 * k, oy + y0 * k, (x1 - x0) * k, (y1 - y0) * k);
            let dim = br(0x000000, 0.62);
            rt.SetAntialiasMode(D2D1_ANTIALIAS_MODE_ALIASED); // 어두운 네 조각 사이에 실금이 안 보이게
            rt.FillRectangle(&rc(ox, oy, iw, fy - oy), &dim);
            rt.FillRectangle(&rc(ox, fy + fh, iw, oy + ih - fy - fh), &dim);
            rt.FillRectangle(&rc(ox, fy, fx - ox, fh), &dim);
            rt.FillRectangle(&rc(fx + fw, fy, ox + iw - fx - fw, fh), &dim);
            rt.SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
            let thin = br(0xFFFFFF, 0.35);
            for t in [1.0f32, 2.0] {
                rt.DrawLine(D2D_POINT_2F { x: fx + fw * t / 3.0, y: fy }, D2D_POINT_2F { x: fx + fw * t / 3.0, y: fy + fh }, &thin, 1.0, None);
                rt.DrawLine(D2D_POINT_2F { x: fx, y: fy + fh * t / 3.0 }, D2D_POINT_2F { x: fx + fw, y: fy + fh * t / 3.0 }, &thin, 1.0, None);
            }
            rt.DrawRectangle(&rc(fx, fy, fw, fh), &br(0xFFFFFF, 0.95), 1.5, None);
            for (hx, hy) in [(0.0f32, 0.0f32), (0.5, 0.0), (1.0, 0.0), (0.0, 0.5), (1.0, 0.5), (0.0, 1.0), (0.5, 1.0), (1.0, 1.0)] {
                let h = rc(fx + fw * hx - 5.0, fy + fh * hy - 5.0, 10.0, 10.0);
                rt.FillRectangle(&h, &br(0xFFFFFF, 1.0));
                rt.DrawRectangle(&h, &br(0x000000, 0.7), 1.0, None);
            }
        }
        // 자르기 영역
        if let Drag::Rect(a, b) = self.drag {
            let (x0, y0, x1, y1) = (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1));
            let c = 0xA3E635;
            rt.FillRectangle(&rc(ox + x0 * k, oy + y0 * k, (x1 - x0) * k, (y1 - y0) * k), &br(c, 0.15));
            rt.DrawRectangle(&rc(ox + x0 * k, oy + y0 * k, (x1 - x0) * k, (y1 - y0) * k), &br(c, 1.0), 1.5, None);
        }
        // 단추 줄
        self.btns.clear();
        rt.FillRectangle(&rc(0.0, 0.0, cw, top), &br(0x0B0B0B, 1.0));
        let tf = self.dw.CreateTextFormat(w!("Malgun Gothic"), None, DWRITE_FONT_WEIGHT_SEMI_BOLD, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL, 13.0, w!("ko-kr")).unwrap();
        let _ = tf.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER);
        let _ = tf.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
        let (mx, my) = self.mouse;
        let btn = |rt: &ID2D1RenderTarget, btns: &mut Vec<([f32; 4], Btn)>, x: f32, y: f32, label: &str, wd: f32, on: bool, b: Btn| {
            let r = [x, y, x + wd, y + 32.0];
            let hov = mx >= r[0] && mx < r[2] && my >= r[1] && my < r[3];
            let rr = D2D1_ROUNDED_RECT { rect: rc(r[0], r[1], wd, 32.0), radiusX: 16.0, radiusY: 16.0 };
            rt.FillRoundedRectangle(&rr, &br(if on { 0xA3E635 } else { 0xFFFFFF }, if on { 0.25 } else if hov { 0.14 } else { 0.06 }));
            let t: Vec<u16> = label.encode_utf16().collect();
            rt.DrawText(&t, &tf, &rc(r[0], r[1], wd, 32.0), &br(if on { 0xA3E635 } else { 0xF3F4F6 }, 1.0), D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT, DWRITE_MEASURING_MODE_NATURAL);
            btns.push((r, b));
            x + wd + 6.0
        };
        let mut x = 14.0;
        let mut btns = std::mem::take(&mut self.btns);
        // 창이 좁으면 왼쪽 도구는 그림만, 더 좁으면 되돌리기도 그림만(오른쪽 단추와 겹치지 않게)
        let compact = cw < 800.0;
        let tools: [(&str, &str, f32, bool, Btn); 5] = [
            ("✂ 자르기", "✂", 82.0, self.tool == Tool::Crop, Btn::Tool(Tool::Crop)),
            ("▣ 크롭", "▣", 72.0, frame, Btn::Tool(Tool::Frame)),
            ("T 글자", "T", 70.0, self.tool == Tool::Text, Btn::Tool(Tool::Text)),
            ("😀 이모지", "😀", 86.0, self.emoji_open, Btn::EmojiPanel),
            ("🔍 글자 인식", "🔍", 100.0, self.tool == Tool::Ocr, Btn::Tool(Tool::Ocr)),
        ];
        for (full, icon, wd, on, b) in tools {
            x = if compact { btn(&rt, &mut btns, x, 11.0, icon, 40.0, on, b) } else { btn(&rt, &mut btns, x, 11.0, full, wd, on, b) };
        }
        let _ = x;
        let mut rx = cw - 14.0 - 64.0;
        btn(&rt, &mut btns, rx, 11.0, "닫기", 64.0, false, Btn::Close);
        rx -= 70.0;
        btn(&rt, &mut btns, rx, 11.0, "저장", 64.0, false, Btn::Save);
        rx -= 78.0;
        btn(&rt, &mut btns, rx, 11.0, "복사", 72.0, true, Btn::Copy);
        let uw = if cw < 600.0 { 44.0 } else { 96.0 };
        rx -= uw + 6.0;
        btn(&rt, &mut btns, rx, 11.0, if uw < 50.0 { "↶" } else { "↶ 되돌리기" }, uw, false, Btn::Undo);
        if self.undo.is_empty() && !self.editing {
            // 되돌릴 것이 없으면 흐리게
            rt.FillRoundedRectangle(&D2D1_ROUNDED_RECT { rect: rc(rx, 11.0, uw, 32.0), radiusX: 16.0, radiusY: 16.0 }, &br(0x0B0B0B, 0.65));
        }
        // 둘째 줄: 크롭 틀 적용·취소
        if frame {
            let y = BAR - 4.0;
            let mut x = 14.0;
            x = btn(&rt, &mut btns, x, y, "✓ 적용", 76.0, true, Btn::Apply);
            x = btn(&rt, &mut btns, x, y, "취소", 60.0, false, Btn::Cancel);
            let t: Vec<u16> = format!("{} × {}", (self.crop[2] - self.crop[0]).round(), (self.crop[3] - self.crop[1]).round()).encode_utf16().collect();
            rt.DrawText(&t, &tf, &rc(x + 4.0, y, 120.0, 32.0), &br(0xF3F4F6, 0.9), D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL);
        }
        // 둘째 줄: 글자 모양(글자 칸·이모지를 골랐거나 글자 도구일 때)
        if style {
            let y = BAR - 4.0;
            let mut x = 14.0;
            if text_sel.is_some() || self.tool == Tool::Text {
                let f = text_sel.as_ref().map_or(self.font, |o| o.font);
                x = btn(&rt, &mut btns, x, y, &format!("글꼴 {} {}", FONTS[f].0, if self.font_open { "▴" } else { "▾" }), 128.0, self.font_open, Btn::Font);
            }
            let sz = any_sel.as_ref().map_or(self.size, |o| o.size);
            x = btn(&rt, &mut btns, x, y, "−", 34.0, false, Btn::Smaller);
            let t: Vec<u16> = format!("크기 {}", sz.round()).encode_utf16().collect();
            rt.DrawText(&t, &tf, &rc(x - 2.0, y, 70.0, 32.0), &br(0xF3F4F6, 0.9), D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL);
            x = btn(&rt, &mut btns, x + 70.0, y, "+", 34.0, false, Btn::Bigger);
            if text_sel.is_some() || self.tool == Tool::Text {
                x += 10.0;
                let cur = text_sel.as_ref().map_or(self.color, |o| o.color);
                for (i, c) in COLORS.iter().enumerate() {
                    let r = [x + 4.0, y + 6.0, x + 24.0, y + 26.0];
                    rt.FillEllipse(&D2D1_ELLIPSE { point: D2D_POINT_2F { x: x + 14.0, y: y + 16.0 }, radiusX: 9.0, radiusY: 9.0 }, &br(*c, 1.0));
                    // 검정 칸도 검정 바탕에서 보이게 모든 칸에 옅은 테두리
                    rt.DrawEllipse(&D2D1_ELLIPSE { point: D2D_POINT_2F { x: x + 14.0, y: y + 16.0 }, radiusX: 9.0, radiusY: 9.0 }, &br(0xFFFFFF, 0.55), 1.0, None);
                    if i == cur {
                        rt.DrawEllipse(&D2D1_ELLIPSE { point: D2D_POINT_2F { x: x + 14.0, y: y + 16.0 }, radiusX: 12.0, radiusY: 12.0 }, &br(0xA3E635, 1.0), 2.0, None);
                    }
                    btns.push((r, Btn::Color(i)));
                    x += 26.0;
                }
                let fx = text_sel.as_ref().map_or(self.fx, |o| o.fx);
                btn(&rt, &mut btns, x + 10.0, y, &format!("{} ▸", EFFECTS[fx as usize % 3]), 86.0, false, Btn::Effect);
                if row2 {
                    // 셋째 줄: 그림자 색 / 외곽선 색 — 자동(글자색 반대) + 7색
                    let y2 = y + STYLE_H - 6.0;
                    let t: Vec<u16> = format!("{} 색", EFFECTS[fx_cur as usize % 3]).encode_utf16().collect();
                    rt.DrawText(&t, &tf, &rc(14.0, y2, 80.0, 32.0), &br(0xF3F4F6, 0.85), D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL);
                    let mut x2 = btn(&rt, &mut btns, 100.0, y2, "자동", 56.0, ec_cur == 255, Btn::EffColor(255)) + 4.0;
                    for (i, c) in COLORS.iter().enumerate() {
                        let (cx, cy) = (x2 + 14.0, y2 + 16.0);
                        rt.FillEllipse(&D2D1_ELLIPSE { point: D2D_POINT_2F { x: cx, y: cy }, radiusX: 9.0, radiusY: 9.0 }, &br(*c, 1.0));
                        rt.DrawEllipse(&D2D1_ELLIPSE { point: D2D_POINT_2F { x: cx, y: cy }, radiusX: 9.0, radiusY: 9.0 }, &br(0xFFFFFF, 0.55), 1.0, None);
                        if ec_cur as usize == i {
                            rt.DrawEllipse(&D2D1_ELLIPSE { point: D2D_POINT_2F { x: cx, y: cy }, radiusX: 12.0, radiusY: 12.0 }, &br(0xA3E635, 1.0), 2.0, None);
                        }
                        btns.push(([x2 + 4.0, y2 + 6.0, x2 + 24.0, y2 + 26.0], Btn::EffColor(i as u8)));
                        x2 += 26.0;
                    }
                }
            }
        }
        // 이모지 판
        if self.emoji_open {
            let ef = self.dw.CreateTextFormat(w!("Segoe UI Emoji"), None, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL, 22.0, w!("ko-kr")).unwrap();
            let _ = ef.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER);
            let _ = ef.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
            for (i, e) in EMOJIS.split(' ').enumerate() {
                let per = ((cw - 28.0) / 40.0).floor().max(1.0) as usize;
                let (c, rr) = (i % per, i / per);
                let r = [14.0 + c as f32 * 40.0, BAR + sty_h + 4.0 + rr as f32 * 44.0, 14.0 + c as f32 * 40.0 + 38.0, BAR + sty_h + 4.0 + rr as f32 * 44.0 + 40.0];
                if mx >= r[0] && mx < r[2] && my >= r[1] && my < r[3] {
                    rt.FillRoundedRectangle(&D2D1_ROUNDED_RECT { rect: rc(r[0], r[1], 38.0, 40.0), radiusX: 8.0, radiusY: 8.0 }, &br(0xFFFFFF, 0.12));
                }
                let t: Vec<u16> = e.encode_utf16().collect();
                rt.DrawText(&t, &ef, &rc(r[0], r[1], 38.0, 40.0), &br(0xFFFFFF, 1.0), D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT, DWRITE_MEASURING_MODE_NATURAL);
                btns.push((r, Btn::Emoji(i)));
            }
        }
        if self.font_open && style {
            let cur = text_sel.as_ref().map_or(self.font, |o| o.font);
            let (lx, ly, lw, ih) = (14.0, BAR - 4.0 + 36.0, 180.0, 36.0);
            let panel = D2D1_ROUNDED_RECT { rect: rc(lx, ly, lw, ih * FONTS.len() as f32 + 8.0), radiusX: 12.0, radiusY: 12.0 };
            rt.FillRoundedRectangle(&panel, &br(0x161616, 0.98));
            rt.DrawRoundedRectangle(&panel, &br(0xFFFFFF, 0.14), 1.0, None);
            for (i, (name, fam)) in FONTS.iter().enumerate() {
                let r = [lx + 4.0, ly + 4.0 + i as f32 * ih, lx + lw - 4.0, ly + 4.0 + (i + 1) as f32 * ih];
                let hov = mx >= r[0] && mx < r[2] && my >= r[1] && my < r[3];
                if hov || i == cur {
                    rt.FillRoundedRectangle(&D2D1_ROUNDED_RECT { rect: rc(r[0], r[1], r[2] - r[0], ih), radiusX: 8.0, radiusY: 8.0 }, &br(if i == cur { 0xA3E635 } else { 0xFFFFFF }, if i == cur { 0.22 } else { 0.10 }));
                }
                // 이름을 그 글꼴로 써서 모양을 미리 본다
                if let Ok(ff) = self.dw.CreateTextFormat(&HSTRING::from(*fam), None, DWRITE_FONT_WEIGHT_BOLD, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL, 16.0, w!("ko-kr")) {
                    let _ = ff.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
                    let t: Vec<u16> = format!("{name}  가나 Ab").encode_utf16().collect();
                    rt.DrawText(&t, &ff, &rc(r[0] + 12.0, r[1], r[2] - r[0] - 16.0, ih), &br(if i == cur { 0xA3E635 } else { 0xF3F4F6 }, 1.0), D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL);
                }
                btns.insert(0, (r, Btn::FontPick(i))); // 아래 그림보다 먼저 잡히게
            }
        }
        self.btns = btns;
        // 아래 안내 줄
        let hint = if self.status.1.elapsed() < Duration::from_secs(4) && !self.status.0.is_empty() {
            self.status.0.clone()
        } else {
            match self.tool {
                Tool::Crop => "자르기: 남길 곳을 끌면 바로 잘립니다 · 잘못 잘랐으면 되돌리기(Ctrl+Z)".into(),
                Tool::Frame => "크롭: 손잡이를 끌어 남길 곳을 정하고, 틀 안쪽을 끌어 옮깁니다 · Enter 적용 · Esc 취소".into(),
                Tool::Text if self.editing => "글자를 적으세요 · Enter 또는 빈 곳을 누르면 끝 · Shift+Enter 줄바꿈".into(),
                Tool::Text => "글자: 그림을 누르면 그 자리에 글자 칸이 하나 생깁니다".into(),
                Tool::Ocr => "글자 인식: 글자 위를 끌어서 고르면 클립보드에 복사됩니다 · Ctrl+A 전부 · Ctrl+C 복사".into(),
                Tool::Move if self.editing => "글자를 고치는 중 · ← → 로 자리 옮기기 · Enter 또는 빈 곳을 누르면 끝".into(),
                Tool::Move => "칸을 끌어 옮기고, 오른쪽 아래 손잡이로 크기 · 고른 글자를 다시 누르면 편집 · Delete 지우기 · Ctrl+Z 되돌리기".into(),
            }
        };
        let hf = self.dw.CreateTextFormat(w!("Malgun Gothic"), None, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL, 12.0, w!("ko-kr")).unwrap();
        let t: Vec<u16> = hint.encode_utf16().collect();
        rt.DrawText(&t, &hf, &rc(16.0, ch - 28.0, cw - 32.0, 20.0), &br(0xF3F4F6, 0.55), D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL);
        if rt.EndDraw(None, None).is_err() {
            self.rt = None; // 장치를 잃었다 → 다음에 새로
        }
        self.dump();
    }

    /// 시험용: NOTCH_CAT_EDDUMP=<파일> 이면 그릴 때마다 지금 상태를 적는다(화면을 누르지 않고 확인)
    fn dump(&self) {
        thread_local!(static OUT: Option<String> = std::env::var("NOTCH_CAT_EDDUMP").ok());
        OUT.with(|o| {
            let Some(f) = o else { return };
            let v = serde_json::json!({
                "w": self.w, "h": self.h, "s": self.s, "view": [self.view.0, self.view.1, self.view.2],
                "tool": format!("{:?}", self.tool), "sel": self.sel, "editing": self.editing, "caret": self.caret,
                "objs": self.objs.iter().map(|o| serde_json::json!({"text": o.text, "x": o.x, "y": o.y, "size": o.size, "emoji": o.emoji, "font": o.font, "fx": o.fx, "ec": o.ec})).collect::<Vec<_>>(), "font_open": self.font_open,
                "undo": self.undo.len(), "crop": self.crop, "words": self.words.len(),
                "word0": self.words.first().map(|w| w.r), "wordN": self.words.last().map(|w| w.r),
                "osel": self.osel.map(|(a, b)| sel_text(&self.words, a, b)), "status": self.status.0,
                "btns": self.btns.iter().map(|(r, b)| serde_json::json!([r, format!("{b:?}")])).collect::<Vec<_>>(),
            });
            let _ = std::fs::write(f, v.to_string());
        });
    }

    fn snap(&self) -> Snap {
        Snap { px: self.px.clone(), w: self.w, h: self.h, objs: self.objs.clone() }
    }

    fn push_undo(&mut self, s: Snap) {
        if self.undo.len() >= UNDO_MAX {
            self.undo.remove(0);
        }
        self.undo.push(s);
    }

    /// 한 단계 되돌린다(적는 중이면 먼저 그 편집을 끝낸다 → 그 편집이 되돌려진다)
    unsafe fn undo(&mut self) {
        self.end_edit();
        self.drag = Drag::None;
        self.drag_pre = None;
        let Some(s) = self.undo.pop() else { return self.say("되돌릴 것이 없습니다") };
        if !Rc::ptr_eq(&s.px, &self.px) {
            self.bmp = None;
            self.words.clear();
            self.osel = None;
            if matches!(self.tool, Tool::Ocr | Tool::Frame) {
                self.tool = Tool::Move;
            }
        }
        self.px = s.px;
        self.w = s.w;
        self.h = s.h;
        self.objs = s.objs;
        self.sel = None;
        self.say(&format!("되돌렸습니다 · 남은 되돌리기 {}번", self.undo.len()));
    }

    /// 그림을 이 영역만 남긴다(되돌리기에 쌓는다)
    fn crop_to(&mut self, r: (i32, i32, i32, i32)) -> bool {
        let r = (r.0.max(0), r.1.max(0), r.2.min(self.w as i32), r.3.min(self.h as i32));
        if r.2 - r.0 <= 4 || r.3 - r.1 <= 4 || (r.0 == 0 && r.1 == 0 && r.2 == self.w as i32 && r.3 == self.h as i32) {
            return false;
        }
        let s = self.snap();
        self.push_undo(s);
        self.px = Rc::new(crate::capture::crop(&self.px, self.w as i32, r));
        self.w = (r.2 - r.0) as u32;
        self.h = (r.3 - r.1) as u32;
        for o in &mut self.objs {
            o.x -= r.0 as f32;
            o.y -= r.1 as f32;
        }
        self.bmp = None;
        self.words.clear();
        self.osel = None;
        self.tool = Tool::Move;
        true
    }

    fn apply_frame(&mut self) {
        let c = self.crop;
        self.rt = None;
        if self.crop_to((c[0].round() as i32, c[1].round() as i32, c[2].round() as i32, c[3].round() as i32)) {
            self.say(&format!("{} × {} 로 잘랐습니다 · Ctrl+Z 로 되돌리기", self.w, self.h));
        } else {
            self.tool = Tool::Move;
            self.say("바뀐 곳이 없습니다");
        }
    }

    /// 그림 전체의 글자를 읽어 낱말 상자를 기억한다
    fn load_words(&mut self) {
        self.osel = None;
        match read_words(&self.px, self.w, self.h) {
            Ok(v) if !v.is_empty() => {
                self.say(&format!("낱말 {}개를 찾았습니다 · 글자 위를 끌어서 고르세요", v.len()));
                self.words = v;
            }
            Ok(_) => {
                self.tool = Tool::Move;
                self.say("이 그림에서 글자를 찾지 못했습니다");
            }
            Err(e) => {
                self.tool = Tool::Move;
                self.say(&format!("글자 인식을 못 했습니다: {e}"));
            }
        }
    }

    /// 고른 낱말을 클립보드에 글자로 넣는다
    unsafe fn copy_words(&mut self) {
        let Some((a, b)) = self.osel else { return self.say("먼저 글자 위를 끌어서 고르세요") };
        let t = sel_text(&self.words, a, b);
        if t.is_empty() {
            return;
        }
        crate::clip::put_text(self.hwnd, &t);
        let short: String = t.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(60).collect();
        self.say(&format!("복사: {short}"));
        notify(self.notch, "글자를 읽어 복사했습니다", &short);
    }

    /// 그 자리(그림 좌표)가 글자 칸의 몇째 글자 앞인가
    unsafe fn caret_at(&self, i: usize, ip: (f32, f32)) -> usize {
        let Some(o) = self.objs.get(i) else { return 0 };
        if o.text.is_empty() {
            return 0;
        }
        let Some(l) = layout(&self.dw, o, 1.0) else { return o.text.chars().count() };
        let (mut trail, mut inside, mut m) = (BOOL(0), BOOL(0), DWRITE_HIT_TEST_METRICS::default());
        if l.HitTestPoint(ip.0 - o.x, ip.1 - o.y, &mut trail, &mut inside, &mut m).is_err() {
            return o.text.chars().count();
        }
        let at = m.textPosition + if trail.as_bool() { m.length } else { 0 };
        let mut n = 0u32;
        for (ci, c) in o.text.chars().enumerate() {
            if n >= at {
                return ci;
            }
            n += c.len_utf16() as u32;
        }
        o.text.chars().count()
    }

    fn start_edit(&mut self, i: usize, caret: usize) {
        if self.edit_pre.is_none() {
            self.edit_pre = Some(self.snap());
        }
        self.sel = Some(i);
        self.editing = true;
        self.caret = caret.min(self.objs.get(i).map_or(0, |o| o.text.chars().count()));
        self.status.1 = Instant::now();
    }

    /// 조합 중인 한글을 글자 칸에 넣고 조합을 끝낸다(빈 곳을 눌러도 마지막 글자가 사라지지 않게)
    unsafe fn commit_comp(&mut self) {
        if self.comp.is_empty() {
            return;
        }
        let c = std::mem::take(&mut self.comp);
        let at = self.caret;
        if let Some(o) = self.sel.and_then(|i| self.objs.get_mut(i)) {
            let b = byte_at(&o.text, at);
            o.text.insert_str(b, &c);
            self.caret += c.chars().count();
        }
        let ctx = ImmGetContext(self.hwnd);
        let _ = ImmNotifyIME(ctx, NI_COMPOSITIONSTR, CPS_CANCEL, 0);
        let _ = ImmReleaseContext(self.hwnd, ctx);
    }

    /// 결과 그림(글자·이모지를 그림에 새긴 것) BGRA
    unsafe fn render(&self) -> Option<Vec<u8>> {
        let wic: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).ok()?;
        let pb: Vec<u8> = (*self.px).clone();
        let wb = wic.CreateBitmapFromMemory(self.w, self.h, &GUID_WICPixelFormat32bppPBGRA, self.w * 4, &pb).ok()?;
        let props = D2D1_RENDER_TARGET_PROPERTIES { pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED }, dpiX: 96.0, dpiY: 96.0, ..Default::default() };
        let rt = self.d2.CreateWicBitmapRenderTarget(&wb, &props).ok()?;
        rt.BeginDraw();
        draw_objs(&rt, &self.dw, &self.objs, 0.0, 0.0, 1.0, None);
        rt.EndDraw(None, None).ok()?;
        let mut out = vec![0u8; (self.w * self.h * 4) as usize];
        wb.CopyPixels(std::ptr::null(), self.w * 4, &mut out).ok()?;
        for p in out.chunks_exact_mut(4) {
            p[3] = 255;
        }
        Some(out)
    }

    unsafe fn copy(&mut self, save: bool) {
        let Some(px) = self.render() else { return self.say("그림을 만들지 못했습니다") };
        let ok = crate::clip::put_image(self.hwnd, &px, self.w, self.h);
        if save {
            let dir = pictures().join("NotchCat");
            let _ = std::fs::create_dir_all(&dir);
            let f = dir.join(format!("스크린샷 {}.png", stamp()));
            let saved = crate::clip::bgra_to_png(&px, self.w, self.h).is_some_and(|png| std::fs::write(&f, png).is_ok());
            if saved {
                self.say("저장하고 클립보드에도 넣었습니다");
                notify(self.notch, "스크린샷을 저장했습니다", &format!("사진 › NotchCat › {}", f.file_name().unwrap_or_default().to_string_lossy()));
            } else {
                self.say("저장하지 못했습니다");
            }
        } else if ok {
            self.say("클립보드에 넣었습니다 · Ctrl+V 로 붙여 넣으세요");
            notify(self.notch, "편집한 그림을 복사했습니다", "클립보드 쪽에 새로 들어갔습니다");
        }
    }

    fn hit_obj(&self, p: (f32, f32)) -> Option<(usize, bool)> {
        let k = self.view.2;
        for (i, o) in self.objs.iter().enumerate().rev() {
            let b = unsafe { obj_box(&self.dw, o) };
            let (hx, hy) = (b[0] + b[2] + 4.0 / k, b[1] + b[3] + 4.0 / k);
            if (p.0 - hx).abs() < 9.0 / k && (p.1 - hy).abs() < 9.0 / k {
                return Some((i, true)); // 크기 손잡이
            }
            if p.0 >= b[0] - 4.0 / k && p.0 < b[0] + b[2] + 4.0 / k && p.1 >= b[1] - 4.0 / k && p.1 < b[1] + b[3] + 4.0 / k {
                return Some((i, false));
            }
        }
        None
    }

    fn end_edit(&mut self) {
        if self.editing {
            unsafe { self.commit_comp() };
            self.editing = false;
            if let Some(i) = self.sel.filter(|i| self.objs.get(*i).is_some_and(|o| o.text.trim().is_empty())) {
                self.objs.remove(i);
                self.sel = None;
            }
            if let Some(pre) = self.edit_pre.take() {
                if pre.objs != self.objs {
                    self.push_undo(pre);
                }
            }
            // 글자를 하나 넣었으면 '옮기기'로 돌아간다 — 다음 클릭이 새 글자 칸을 만들지 않게
            if self.tool == Tool::Text {
                self.tool = Tool::Move;
            }
        }
    }

    /// 고른 칸의 모양(글꼴·크기·색)을 바꾸기 전에 되돌리기에 쌓는다(적는 중이면 편집 한 덩어리로 묶인다)
    fn style_undo(&mut self) {
        if !self.editing && self.sel.is_some_and(|i| i < self.objs.len()) {
            let s = self.snap();
            self.push_undo(s);
        }
    }

    unsafe fn press(&mut self, b: Btn) {
        if !matches!(b, Btn::Font | Btn::FontPick(_)) {
            self.font_open = false;
        }
        match b {
            Btn::Tool(t) => {
                self.end_edit();
                self.tool = if self.tool == t { Tool::Move } else { t };
                self.emoji_open = false;
                self.osel = None;
                match self.tool {
                    Tool::Frame => {
                        self.sel = None;
                        self.crop = [0.0, 0.0, self.w as f32, self.h as f32];
                    }
                    Tool::Ocr => {
                        self.sel = None;
                        self.load_words();
                    }
                    _ => {}
                }
                self.rt = None; // 둘째 줄이 생기거나 없어져 그림 자리가 바뀐다
            }
            Btn::Apply => self.apply_frame(),
            Btn::Cancel => {
                self.tool = Tool::Move;
                self.rt = None;
            }
            Btn::Undo => self.undo(),
            Btn::EmojiPanel => {
                self.end_edit();
                self.emoji_open = !self.emoji_open;
                self.rt = None; // 그림 자리가 바뀐다
            }
            Btn::Emoji(i) => {
                let e = EMOJIS.split(' ').nth(i).unwrap_or("😀");
                let size = (self.w.min(self.h) as f32 * 0.18).clamp(32.0, 160.0);
                let s = self.snap();
                self.push_undo(s);
                self.objs.push(Obj { text: e.into(), emoji: true, font: 0, size, color: 0, fx: 0, ec: 255, x: self.w as f32 / 2.0 - size / 2.0, y: self.h as f32 / 2.0 - size / 2.0 });
                self.sel = Some(self.objs.len() - 1);
                self.tool = Tool::Move;
                self.emoji_open = false;
                self.rt = None;
            }
            Btn::Font => self.font_open = !self.font_open, // 아래로 목록을 펼친다
            Btn::FontPick(i) => {
                self.font_open = false;
                self.style_undo();
                self.font = i;
                if let Some(o) = self.sel.and_then(|i| self.objs.get_mut(i)).filter(|o| !o.emoji) {
                    o.font = i;
                }
            }
            Btn::EffColor(c) => {
                self.style_undo();
                self.ec = c;
                if let Some(o) = self.sel.and_then(|i| self.objs.get_mut(i)).filter(|o| !o.emoji) {
                    o.ec = c;
                }
            }
            Btn::Effect => {
                self.style_undo();
                self.fx = (self.sel.and_then(|i| self.objs.get(i)).filter(|o| !o.emoji).map_or(self.fx, |o| o.fx) + 1) % 3;
                if let Some(o) = self.sel.and_then(|i| self.objs.get_mut(i)).filter(|o| !o.emoji) {
                    o.fx = self.fx;
                }
            }
            Btn::Smaller | Btn::Bigger => {
                self.style_undo();
                let f = if b == Btn::Bigger { 1.2 } else { 1.0 / 1.2 };
                match self.sel.and_then(|i| self.objs.get_mut(i)) {
                    Some(o) => o.size = (o.size * f).clamp(8.0, 600.0),
                    None => self.size = (self.size * f).clamp(8.0, 600.0),
                }
            }
            Btn::Color(c) => {
                self.style_undo();
                self.color = c;
                if let Some(o) = self.sel.and_then(|i| self.objs.get_mut(i)).filter(|o| !o.emoji) {
                    o.color = c;
                }
            }
            Btn::Copy => {
                self.end_edit();
                self.copy(false);
            }
            Btn::Save => {
                self.end_edit();
                self.copy(true);
            }
            Btn::Close => {
                let _ = PostMessageW(self.notch, WM_EDCLOSE, WPARAM(0), LPARAM(0));
            }
        }
    }

    unsafe fn down(&mut self, p: (f32, f32)) {
        self.mouse = p;
        if let Some((_, b)) = self.btns.iter().find(|(r, _)| p.0 >= r[0] && p.0 < r[2] && p.1 >= r[1] && p.1 < r[3]).copied() {
            return self.press(b);
        }
        if self.font_open {
            self.font_open = false; // 목록 밖을 누르면 닫기만
            return;
        }
        let ip = self.to_img(p);
        let inside = ip.0 >= 0.0 && ip.1 >= 0.0 && ip.0 < self.w as f32 && ip.1 < self.h as f32;
        let k = self.view.2;
        self.down_at = p;
        self.click_edit = false;
        if self.editing {
            // 적는 중: 그 글자 칸 안을 누르면 적는 자리만 옮기고, 다른 곳이면 편집을 끝낸다(글자 도구였으면 옮기기로)
            if let Some((i, false)) = self.hit_obj(ip).filter(|(i, _)| Some(*i) == self.sel) {
                self.commit_comp();
                self.caret = self.caret_at(i, ip);
                self.status.1 = Instant::now();
                return;
            }
            self.end_edit();
        }
        match self.tool {
            Tool::Crop if inside => self.drag = Drag::Rect(ip, ip),
            Tool::Crop => {}
            Tool::Frame => {
                if let Some(m) = frame_hit(self.crop, ip, 10.0 / k) {
                    self.drag = Drag::Frame(m, ip, self.crop);
                }
            }
            Tool::Ocr => {
                self.osel = None;
                if let Some((i, d)) = word_at(&self.words, ip).filter(|_| inside) {
                    self.drag = Drag::Words(i);
                    if d == 0.0 {
                        self.osel = Some((i, i));
                    }
                }
            }
            Tool::Text if inside => {
                if let Some((i, _)) = self.hit_obj(ip).filter(|(i, _)| !self.objs[*i].emoji) {
                    let c = self.caret_at(i, ip);
                    self.start_edit(i, c);
                } else {
                    self.edit_pre = Some(self.snap());
                    self.objs.push(Obj { text: String::new(), emoji: false, font: self.font, size: self.size, color: self.color, fx: self.fx, ec: self.ec, x: ip.0, y: ip.1 - self.size * 0.6 });
                    let i = self.objs.len() - 1;
                    self.start_edit(i, 0);
                }
            }
            _ => match self.hit_obj(ip) {
                Some((i, true)) => {
                    self.sel = Some(i);
                    self.drag_pre = Some(self.snap());
                    self.drag = Drag::Resize(i, self.objs[i].size, ip.0);
                }
                Some((i, false)) => {
                    self.click_edit = self.sel == Some(i) && !self.objs[i].emoji;
                    self.sel = Some(i);
                    self.drag_pre = Some(self.snap());
                    self.drag = Drag::Move(i, ip.0 - self.objs[i].x, ip.1 - self.objs[i].y);
                }
                None => self.sel = None,
            },
        }
    }

    unsafe fn moved(&mut self, p: (f32, f32)) {
        self.mouse = p;
        let ip = self.to_img(p);
        // 살짝 흔들린 것은 끌기로 치지 않는다(그냥 누른 것 → 편집으로 들어갈 수 있게)
        let far = (p.0 - self.down_at.0).abs() + (p.1 - self.down_at.1).abs() > 3.0;
        if matches!(self.drag, Drag::Move(..) | Drag::Resize(..)) {
            if self.drag_pre.is_some() && !far {
                return;
            }
            if let Some(s) = self.drag_pre.take() {
                self.push_undo(s);
            }
        }
        match self.drag {
            Drag::Move(i, dx, dy) => {
                if let Some(o) = self.objs.get_mut(i) {
                    o.x = ip.0 - dx;
                    o.y = ip.1 - dy;
                }
            }
            Drag::Resize(i, s0, x0) => {
                if let Some(o) = self.objs.get_mut(i) {
                    let b = obj_box(&self.dw, &Obj { size: s0, ..o.clone() });
                    o.size = (s0 * (b[2] + ip.0 - x0) / b[2].max(1.0)).clamp(8.0, 600.0);
                }
            }
            Drag::Rect(a, _) => self.drag = Drag::Rect(a, ip),
            Drag::Frame(m, p0, r0) => self.crop = frame_drag(r0, m, ip.0 - p0.0, ip.1 - p0.1, self.w as f32, self.h as f32),
            Drag::Words(a) => {
                if far || self.osel.is_some() {
                    if let Some((c, _)) = word_at(&self.words, ip) {
                        self.osel = Some((a, c));
                    }
                }
            }
            Drag::None => {}
        }
    }

    unsafe fn up(&mut self) {
        let still = self.drag_pre.take().is_some(); // 누르기만 하고 안 끌었다
        match std::mem::replace(&mut self.drag, Drag::None) {
            Drag::Rect(a, b) if self.tool == Tool::Crop => {
                let r = (a.0.min(b.0) as i32, a.1.min(b.1) as i32, a.0.max(b.0) as i32, a.1.max(b.1) as i32);
                if self.crop_to(r) {
                    self.say("잘랐습니다 · Ctrl+Z 로 되돌리기");
                }
            }
            Drag::Words(_) if self.osel.is_some() => self.copy_words(),
            Drag::Move(i, ..) if still && self.click_edit => {
                // 이미 골라 둔 글자를 다시 눌렀다 → 그 자리에서 편집
                let c = self.caret_at(i, self.to_img(self.mouse));
                self.start_edit(i, c);
            }
            _ => {}
        }
        self.click_edit = false;
    }

    unsafe fn dbl(&mut self, p: (f32, f32)) {
        let ip = self.to_img(p);
        match self.tool {
            Tool::Frame => {
                if frame_hit(self.crop, ip, 0.0) == Some(16) {
                    self.apply_frame(); // 틀 안을 두 번 누르면 적용
                }
            }
            Tool::Ocr => {
                if let Some((i, _)) = word_at(&self.words, ip).filter(|w| w.1 == 0.0) {
                    self.osel = Some((i, i));
                    self.copy_words();
                }
            }
            _ => {
                if let Some((i, _)) = self.hit_obj(ip).filter(|(i, _)| !self.objs[*i].emoji) {
                    // 두 번 누르면 다시 글자 편집
                    if !(self.editing && self.sel == Some(i)) {
                        self.end_edit();
                    }
                    let c = self.caret_at(i, ip);
                    self.start_edit(i, c);
                }
            }
        }
    }

    unsafe fn key(&mut self, vk: u32) {
        match vk {
            0x1B if self.font_open => self.font_open = false,
            0x1B => {
                if self.editing {
                    self.end_edit();
                } else {
                    self.tool = Tool::Move;
                    self.emoji_open = false;
                    self.sel = None;
                    self.osel = None;
                    self.rt = None;
                }
            }
            0x0D if self.tool == Tool::Frame => self.apply_frame(),
            0x2E if !self.editing => {
                if let Some(i) = self.sel.take().filter(|i| *i < self.objs.len()) {
                    let s = self.snap();
                    self.push_undo(s);
                    self.objs.remove(i);
                }
            }
            0x25 | 0x27 | 0x24 | 0x23 | 0x2E => {
                // 적는 중: ← → Home End 로 자리 옮기기, Delete 는 뒤 글자 지우기
                self.commit_comp();
                let at = self.caret;
                let Some(o) = self.sel.and_then(|i| self.objs.get_mut(i)) else { return };
                let n = o.text.chars().count();
                match vk {
                    0x25 => self.caret = at.saturating_sub(1),
                    0x27 => self.caret = (at + 1).min(n),
                    0x24 => self.caret = 0,
                    0x23 => self.caret = n,
                    _ if at < n => {
                        let b = byte_at(&o.text, at);
                        o.text.remove(b);
                    }
                    _ => {}
                }
                self.status.1 = Instant::now();
            }
            _ => {}
        }
    }

    unsafe fn char(&mut self, c: u32) {
        // Ctrl+Z · Ctrl+C · Ctrl+A 는 글자 신호(26·3·1)로 온다
        match c {
            26 => return self.undo(),
            3 if !self.editing => {
                return if self.tool == Tool::Ocr { self.copy_words() } else { self.copy(false) };
            }
            1 if !self.editing && self.tool == Tool::Ocr && !self.words.is_empty() => {
                self.osel = Some((0, self.words.len() - 1));
                return;
            }
            _ => {}
        }
        if !self.editing {
            return;
        }
        let at = self.caret;
        let Some(o) = self.sel.and_then(|i| self.objs.get_mut(i)) else { return };
        match c {
            8 if at > 0 => {
                let b = byte_at(&o.text, at - 1);
                o.text.remove(b);
                self.caret -= 1;
            }
            13 if GetKeyState(VK_SHIFT.0 as i32) < 0 => {
                let b = byte_at(&o.text, at);
                o.text.insert(b, '\n');
                self.caret += 1;
            }
            13 => self.end_edit(),
            c if c >= 32 => {
                if let Some(ch) = char::from_u32(c) {
                    let b = byte_at(&o.text, at);
                    o.text.insert(b, ch);
                    self.caret += 1;
                }
            }
            _ => {}
        }
        self.status.1 = Instant::now();
    }
}

fn pictures() -> std::path::PathBuf {
    unsafe {
        use windows::Win32::UI::Shell::{SHGetKnownFolderPath, FOLDERID_Pictures, KNOWN_FOLDER_FLAG};
        if let Ok(p) = SHGetKnownFolderPath(&FOLDERID_Pictures, KNOWN_FOLDER_FLAG(0), None) {
            let s = p.to_string().unwrap_or_default();
            windows::Win32::System::Com::CoTaskMemFree(Some(p.0 as _));
            return s.into();
        }
    }
    std::env::temp_dir()
}

/// 파일 이름용 날짜·시각 "2026-11-14 1402 33"
pub fn stamp() -> String {
    let t = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    format!("{:04}-{:02}-{:02} {:02}{:02}{:02}", t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond)
}

/// 그림(BGRA) 속 글자를 낱말 상자와 함께 읽는다(한국어 우선, 없으면 사용자 언어). 상자는 원래 그림 좌표
pub fn read_words(px: &[u8], w: u32, h: u32) -> windows::core::Result<Vec<Word>> {
    use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    // 너무 작으면 잘 못 읽으니 2배로 키운다
    let (px, w, h, k) = if w < 600 { (upscale2(px, w, h), w * 2, h * 2, 2.0) } else { (px.to_vec(), w, h, 1.0) };
    let buf = windows::Security::Cryptography::CryptographicBuffer::CreateFromByteArray(&px)?;
    let sb = SoftwareBitmap::CreateCopyFromBuffer(&buf, BitmapPixelFormat::Bgra8, w as i32, h as i32)?;
    let eng = windows::Globalization::Language::CreateLanguage(&HSTRING::from("ko")).and_then(|l| OcrEngine::TryCreateFromLanguage(&l)).or_else(|_| OcrEngine::TryCreateFromUserProfileLanguages())?;
    let res = eng.RecognizeAsync(&sb)?.get()?;
    let mut out = Vec::new();
    for (li, line) in res.Lines()?.into_iter().enumerate() {
        for word in line.Words()? {
            let r = word.BoundingRect()?;
            out.push(Word { text: word.Text()?.to_string(), r: [r.X / k, r.Y / k, r.Width / k, r.Height / k], line: li });
        }
    }
    Ok(out)
}

/// 그림(BGRA) 속 글자를 글로만
#[cfg(test)]
pub fn read_text(px: &[u8], w: u32, h: u32) -> windows::core::Result<String> {
    let ws = read_words(px, w, h)?;
    Ok(if ws.is_empty() { String::new() } else { sel_text(&ws, 0, ws.len() - 1) })
}

fn upscale2(px: &[u8], w: u32, h: u32) -> Vec<u8> {
    let mut o = vec![0u8; (w * h * 16) as usize];
    for y in 0..h * 2 {
        for x in 0..w * 2 {
            let s = (((y / 2) * w + x / 2) * 4) as usize;
            let d = ((y * w * 2 + x) * 4) as usize;
            o[d..d + 4].copy_from_slice(&px[s..s + 4]);
        }
    }
    o
}

unsafe fn ime(hwnd: HWND, kind: IME_COMPOSITION_STRING) -> String {
    let c = ImmGetContext(hwnd);
    let n = ImmGetCompositionStringW(c, kind, None, 0);
    let mut buf = vec![0u16; (n.max(0) as usize) / 2];
    if n > 0 {
        ImmGetCompositionStringW(c, kind, Some(buf.as_mut_ptr() as _), n as u32);
    }
    let _ = ImmReleaseContext(hwnd, c);
    String::from_utf16_lossy(&buf)
}

unsafe extern "system" fn proc(hw: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    // 편집 창 일을 하는 중에 윈도우가 같은 창에 메시지를 곧바로 다시 보내면(자르기·커서 잡기 등) 이미 빌려 쓰는 중이다.
    // 그때 빌리려 하면 노치째 꺼진다 → 기본 처리로 넘긴다
    let busy = ED.with(|e| e.try_borrow_mut().is_err());
    if busy {
        return DefWindowProcW(hw, msg, wp, lp);
    }
    let s = ED.with(|e| e.borrow().as_ref().map_or(1.0, |e| e.s));
    let pt = (((lp.0 & 0xFFFF) as i16) as f32 / s, (((lp.0 >> 16) & 0xFFFF) as i16) as f32 / s);
    let mut redraw = true;
    let r = ED.with(|e| {
        let mut b = e.borrow_mut();
        let Some(ed) = b.as_mut().filter(|e| e.hwnd == hw) else { return None };
        match msg {
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                BeginPaint(hw, &mut ps);
                ed.paint();
                let _ = EndPaint(hw, &ps);
                redraw = false;
                return Some(LRESULT(0));
            }
            WM_SIZE => {
                ed.rt = None;
            }
            WM_LBUTTONDOWN => {
                SetCapture(hw);
                ed.down(pt);
            }
            WM_LBUTTONDBLCLK => ed.dbl(pt),
            WM_MOUSEMOVE => ed.moved(pt),
            WM_LBUTTONUP => {
                let _ = ReleaseCapture();
                ed.up();
            }
            WM_KEYDOWN => ed.key(wp.0 as u32),
            WM_CHAR => {
                ed.char(wp.0 as u32);
                return Some(LRESULT(0));
            }
            0x010D /* WM_IME_STARTCOMPOSITION */ => return Some(LRESULT(0)),
            0x010F /* WM_IME_COMPOSITION */ => {
                if lp.0 as u32 & GCS_RESULTSTR.0 != 0 {
                    for c in ime(hw, GCS_RESULTSTR).chars() {
                        ed.char(c as u32);
                    }
                }
                ed.comp = if lp.0 as u32 & GCS_COMPSTR.0 != 0 { ime(hw, GCS_COMPSTR) } else { String::new() };
                let _ = InvalidateRect(hw, None, false);
                return Some(LRESULT(0));
            }
            0x010E /* WM_IME_ENDCOMPOSITION */ => ed.comp.clear(),
            WM_TIMER => {
                if !ed.editing && ed.status.1.elapsed() > Duration::from_secs(5) {
                    redraw = false;
                }
            }
            WM_NCHITTEST => {
                // 테두리 없는 창: 왼쪽·오른쪽·아래 가장자리를 끌어 크기를 바꾼다(위는 노치에 붙어 있다)
                let mut r = RECT::default();
                let _ = GetWindowRect(hw, &mut r);
                let (x, y) = (((lp.0 & 0xFFFF) as i16) as i32, (((lp.0 >> 16) & 0xFFFF) as i16) as i32);
                let e = (8.0 * ed.s) as i32;
                let (l, rr, b) = (x < r.left + e, x >= r.right - e, y >= r.bottom - e);
                redraw = false;
                return Some(LRESULT(match (l, rr, b) {
                    (true, _, true) => HTBOTTOMLEFT,
                    (_, true, true) => HTBOTTOMRIGHT,
                    (true, _, _) => HTLEFT,
                    (_, true, _) => HTRIGHT,
                    (_, _, true) => HTBOTTOM,
                    _ => HTCLIENT,
                } as isize));
            }
            0x0214 /* WM_SIZING */ => {
                // 노치 가운데 아래에 붙은 채로 양옆이 같이 늘고 준다
                let mut old = RECT::default();
                let _ = GetWindowRect(hw, &mut old);
                let r = &mut *(lp.0 as *mut RECT);
                let cx = (old.left + old.right) / 2;
                let half = match wp.0 as u32 {
                    WMSZ_LEFT | WMSZ_BOTTOMLEFT => cx - r.left,
                    WMSZ_RIGHT | WMSZ_BOTTOMRIGHT => r.right - cx,
                    _ => (old.right - old.left) / 2,
                }
                .max((270.0 * ed.s) as i32); // 반폭 270 = 너비 540: 단추 줄이 겹치지 않는 가장 좁은 너비
                r.left = cx - half;
                r.right = cx + half;
                r.top = old.top;
                r.bottom = r.bottom.max(old.top + (320.0 * ed.s) as i32);
                WANT_H.with(|w| w.set(r.bottom - r.top));
                redraw = false;
                return Some(LRESULT(1));
            }
            WM_CLOSE => {
                let _ = PostMessageW(ed.notch, WM_EDCLOSE, WPARAM(0), LPARAM(0));
                redraw = false;
                return Some(LRESULT(0));
            }
            WM_DESTROY => {
                *b = None;
                return Some(LRESULT(0));
            }
            _ => redraw = false,
        }
        None
    });
    if redraw {
        let _ = InvalidateRect(hw, None, false);
    }
    r.unwrap_or_else(|| DefWindowProcW(hw, msg, wp, lp))
}

/// 편집 창이 떠 있나(노치가 접히지 않게)
pub fn is_open() -> bool {
    ED.with(|e| e.try_borrow().map_or(true, |b| b.is_some())) // 빌려 쓰는 중이면 열려 있는 것
}

/// 편집 창을 연다(BGRA 그림). 이미 열려 있으면 그 창을 닫고 새로.
pub unsafe fn open(notch: HWND, px: Vec<u8>, w: u32, h: u32) -> windows::core::Result<HWND> {
    let old = ED.with(|e| e.borrow().as_ref().map(|e| e.hwnd));
    if let Some(o) = old {
        let _ = DestroyWindow(o);
    }
    let inst = GetModuleHandleW(None)?;
    let class = w!("NotchCatEditor");
    RegisterClassW(&WNDCLASSW { style: CS_DBLCLKS, lpfnWndProc: Some(proc), hInstance: inst.into(), hCursor: LoadCursorW(None, IDC_ARROW)?, lpszClassName: class, ..Default::default() });
    // 화면의 80% 안에서 그림이 들어가게
    let mut work = RECT::default();
    let _ = SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut work as *mut _ as _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));
    let (sw, sh) = (work.right - work.left, work.bottom - work.top);
    let ws = windows::Win32::UI::HiDpi::GetDpiForSystem() as f32 / 96.0;
    let ww = ((w as i32 + 60).max((820.0 * ws) as i32)).min(sw * 9 / 10); // 820: 단추 줄이 글자째 다 들어가는 너비
    // 노치가 펼쳐져 있으면(맨 위에 떠 있어 창을 가린다) 그 아래에서 연다
    let mut top = work.top;
    if IsWindowVisible(notch).as_bool() {
        let (mut wr, mut rb) = (RECT::default(), RECT::default());
        use windows::Win32::Graphics::Gdi::{CreateRectRgn, DeleteObject, GetRgnBox, GetWindowRgn, GDI_REGION_TYPE};
        let rgn = CreateRectRgn(0, 0, 0, 0);
        if GetWindowRect(notch, &mut wr).is_ok() && GetWindowRgn(notch, rgn) != GDI_REGION_TYPE(0) && GetRgnBox(rgn, &mut rb) != GDI_REGION_TYPE(0) && rb.bottom > rb.top {
            top = top.max(wr.top + rb.bottom + 10);
        }
        let _ = DeleteObject(rgn);
    }
    let room = work.bottom - top;
    let natural = ((h as i32 + 160).max(420)).min(sh * 9 / 10);
    WANT_H.with(|c| c.set(natural));
    let wh = natural.min(room - 10).max(240);
    // 테두리 없는 창: 노치가 dock() 으로 바로 아래에 붙이고, 물방울처럼 내려오게 투명도도 노치가 맡는다
    let hw = CreateWindowExW(WS_EX_LAYERED, class, w!("노치 고양이 · 스크린샷 편집"), WS_POPUP, work.left + (sw - ww) / 2, top, ww, wh, None, None, inst, None)?;
    let _ = SetLayeredWindowAttributes(hw, COLORREF(0), 0, LWA_ALPHA);
    let round: u32 = 2; // DWMWCP_ROUND
    let _ = windows::Win32::Graphics::Dwm::DwmSetWindowAttribute(hw, windows::Win32::Graphics::Dwm::DWMWA_WINDOW_CORNER_PREFERENCE, &round as *const u32 as _, 4);
    let _ = ShowWindow(hw, SW_SHOW);
    let s = GetDpiForWindow(hw) as f32 / 96.0;
    let d2: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
    let dw: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
    let size = (h as f32 * 0.09).clamp(18.0, 96.0);
    ED.with(|e| {
        *e.borrow_mut() = Some(Ed { hwnd: hw, notch, d2, dw, rt: None, bmp: None, px: Rc::new(px), w, h, objs: vec![], sel: None, editing: false, comp: String::new(), tool: Tool::Move, drag: Drag::None, emoji_open: false, font: 0, size, color: 0, mouse: (-1.0, -1.0), btns: vec![], view: (0.0, 0.0, 1.0), status: (String::new(), Instant::now()), s, caret: 0, undo: vec![], edit_pre: None, drag_pre: None, down_at: (0.0, 0.0), click_edit: false, crop: [0.0; 4], words: vec![], osel: None, font_open: false, fx: 0, ec: 255 });
    });
    SetTimer(hw, 1, 500, None); // 글자 자리 깜빡임
    let _ = SetForegroundWindow(hw);
    let _ = InvalidateRect(hw, None, false);
    Ok(hw)
}

/// 노치 바로 아래에 붙인다(가운데 x, 위 y — 화면 픽셀) + 투명도 0~1
pub unsafe fn dock(cx: i32, top: i32, alpha: f32) {
    let Some(hw) = ED.with(|e| e.try_borrow().ok().and_then(|b| b.as_ref().map(|e| e.hwnd))) else { return };
    let mut r = RECT::default();
    let _ = GetWindowRect(hw, &mut r);
    let x = cx - (r.right - r.left) / 2;
    let mut work = RECT::default();
    let _ = SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut work as *mut _ as _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));
    let hh = WANT_H.with(|w| w.get()).min(work.bottom - top - 12).max(240);
    if (r.left, r.top, r.bottom - r.top) != (x, top, hh) {
        let _ = SetWindowPos(hw, None, x, top, r.right - r.left, hh, SWP_NOZORDER | SWP_NOACTIVATE);
    }
    let _ = SetLayeredWindowAttributes(hw, COLORREF(0), (alpha.clamp(0.0, 1.0) * 255.0) as u8, LWA_ALPHA);
}

/// 편집 창의 화면 사각형(픽셀). 떠 있지 않으면 None
pub fn rect() -> Option<RECT> {
    let hw = ED.with(|e| e.try_borrow().ok().and_then(|b| b.as_ref().map(|e| e.hwnd)))?;
    let mut r = RECT::default();
    unsafe { GetWindowRect(hw, &mut r).ok()? };
    Some(r)
}

/// 편집 창을 없앤다(빌린 것을 놓은 뒤에 — WM_DESTROY 가 다시 ED 를 쓴다)
pub unsafe fn destroy() {
    if let Some(hw) = ED.with(|e| e.try_borrow().ok().and_then(|b| b.as_ref().map(|e| e.hwnd))) {
        let _ = DestroyWindow(hw);
    }
}

/// 시험용: 열린 편집 창에 글자·이모지를 넣고 결과 그림을 PNG 로 저장한다
pub unsafe fn demo(out: &std::path::Path) {
    ED.with(|e| {
        if let Some(ed) = e.borrow_mut().as_mut() {
            let s = ed.size;
            ed.objs.push(Obj { text: "노치 고양이 편집".into(), emoji: false, font: 0, size: s, color: 3, fx: 1, ec: 255, x: 20.0, y: 20.0 });
            ed.objs.push(Obj { text: "👍".into(), emoji: true, font: 0, size: s * 1.6, color: 0, fx: 0, ec: 255, x: ed.w as f32 * 0.6, y: ed.h as f32 * 0.4 });
            ed.sel = Some(0);
            ed.tool = Tool::Text;
            ed.emoji_open = true;
            if let Some(px) = ed.render() {
                if let Some(png) = crate::clip::bgra_to_png(&px, ed.w, ed.h) {
                    let _ = std::fs::write(out, png);
                }
            }
            let _ = InvalidateRect(ed.hwnd, None, false);
        }
    });
}

#[allow(dead_code)]
const _: Option<Matrix3x2> = None;
#[allow(dead_code)]
const _P: PCWSTR = PCWSTR::null();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_drag_keeps_inside() {
        let r = [0.0, 0.0, 100.0, 80.0];
        assert_eq!(frame_drag(r, 1 | 2, 20.0, 10.0, 100.0, 80.0), [20.0, 10.0, 100.0, 80.0]); // 왼쪽 위 모서리
        assert_eq!(frame_drag(r, 4, -500.0, 0.0, 100.0, 80.0), [0.0, 0.0, 8.0, 80.0]); // 뒤집히지 않고 최소 크기
        assert_eq!(frame_drag([10.0, 10.0, 50.0, 50.0], 16, 500.0, -500.0, 100.0, 80.0), [60.0, 0.0, 100.0, 40.0]); // 통째로 옮겨도 그림 안
        assert_eq!(frame_hit(r, (1.0, 40.0), 5.0), Some(1));
        assert_eq!(frame_hit(r, (99.0, 79.0), 5.0), Some(4 | 8));
        assert_eq!(frame_hit(r, (50.0, 40.0), 5.0), Some(16));
        assert_eq!(frame_hit(r, (150.0, 40.0), 5.0), None);
    }

    #[test]
    fn words_select_in_reading_order() {
        let w = |t: &str, x: f32, y: f32, line| Word { text: t.into(), r: [x, y, 30.0, 10.0], line };
        let ws = vec![w("가", 0.0, 0.0, 0), w("나", 40.0, 0.0, 0), w("다", 0.0, 20.0, 1), w("라", 40.0, 20.0, 1)];
        assert_eq!(sel_text(&ws, 1, 2), "나\n다");
        assert_eq!(sel_text(&ws, 3, 0), "가 나\n다 라"); // 거꾸로 끌어도 읽는 순서
        assert_eq!(word_at(&ws, (45.0, 5.0)), Some((1, 0.0)));
        assert_eq!(word_at(&ws, (100.0, 24.0)).map(|x| x.0), Some(3)); // 줄 끝 바깥은 그 줄의 마지막 낱말
        assert_eq!(byte_at("가나다", 2), 6);
        assert_eq!(u16_at("a😀b", 2), 3);
    }

    #[test]
    #[ignore] // 윈도우 글자 인식이 한국어를 읽는지(그림에 글자를 그려 읽어 본다)
    fn ocr_reads_korean() {
        unsafe {
            let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED);
            let (w, h) = (520u32, 120u32);
            let wic: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
            let wb = wic.CreateBitmap(w, h, &GUID_WICPixelFormat32bppPBGRA, WICBitmapCacheOnLoad).unwrap();
            let d2: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
            let dw: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED).unwrap();
            let props = D2D1_RENDER_TARGET_PROPERTIES { pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED }, dpiX: 96.0, dpiY: 96.0, ..Default::default() };
            let rt = d2.CreateWicBitmapRenderTarget(&wb, &props).unwrap();
            rt.BeginDraw();
            rt.Clear(Some(&rgb(0xFFFFFF, 1.0)));
            draw_objs(&rt, &dw, &[Obj { text: "안녕하세요 노치".into(), emoji: false, font: 0, size: 48.0, color: 1, fx: 0, ec: 255, x: 10.0, y: 20.0 }], 0.0, 0.0, 1.0, None);
            rt.EndDraw(None, None).unwrap();
            let mut px = vec![0u8; (w * h * 4) as usize];
            wb.CopyPixels(std::ptr::null(), w * 4, &mut px).unwrap();
            let t = read_text(&px, w, h).unwrap();
            println!("읽은 글자: {t}");
            assert!(t.replace(' ', "").contains("안녕하세요"), "{t}");
        }
    }
}
