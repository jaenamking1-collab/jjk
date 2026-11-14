//! 스크린샷 편집 창: 자르기 · 글자(글꼴·크기·색, 끌어서 크기, 두 번 눌러 다시 편집) · 이모지 · 글자 인식.
//! '복사'·'저장'은 결과 그림을 클립보드에 넣는다(노치가 클립보드 쪽 새 항목으로 받아 적는다).
//! 글자 인식은 끌어서 고른 곳의 글자를 읽어 클립보드에 글자로 넣는다(윈도우 기본 글자 인식, 한국어).

use std::cell::RefCell;
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
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, ReleaseCapture, SetCapture, VK_CONTROL, VK_SHIFT};
use windows::Win32::UI::WindowsAndMessaging::*;

pub const WM_EDTOAST: u32 = WM_APP + 6; // 노치에게: 알림 물방울을 띄워 달라
pub static TOAST: Mutex<Option<(String, String)>> = Mutex::new(None);

const FONTS: [(&str, &str); 6] = [("맑은 고딕", "Malgun Gothic"), ("굴림", "Gulim"), ("바탕", "Batang"), ("궁서", "Gungsuh"), ("Segoe UI", "Segoe UI"), ("Impact", "Impact")];
const COLORS: [u32; 7] = [0xFFFFFF, 0x111111, 0xEF4444, 0xFACC15, 0xA3E635, 0x3B82F6, 0xEC4899];
const EMOJIS: &str = "😀 😂 🥹 😍 😎 🤔 😭 😡 😱 🥳 👍 👎 👏 🙏 💪 👉 👈 👆 ❤️ 💔 ✨ ⭐ 🔥 🎉 ✅ ❌ ⚠️ ❓ ❗ 💡 📌 📎 📷 🎵 ☕ 🍀 🐱 🐶 💯 🚀";
const BAR: f32 = 54.0; // 단추 줄 높이
const STYLE_H: f32 = 44.0; // 글자 모양 줄 높이

#[derive(Clone, Copy, PartialEq, Debug)]
enum Tool {
    Move,
    Crop,
    Text,
    Ocr,
}

#[derive(Clone, Copy, PartialEq)]
enum Btn {
    Tool(Tool),
    EmojiPanel,
    Emoji(usize),
    Font,
    Smaller,
    Bigger,
    Color(usize),
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
    pub x: f32,
    pub y: f32,
}

enum Drag {
    None,
    Move(usize, f32, f32),  // 칸, 잡은 곳과 칸 왼쪽 위의 차이
    Resize(usize, f32, f32), // 칸, 처음 크기, 처음 마우스 x(그림 좌표)
    Rect((f32, f32), (f32, f32)), // 자르기·글자 인식 영역(그림 좌표)
}

struct Ed {
    hwnd: HWND,
    notch: HWND,
    d2: ID2D1Factory,
    dw: IDWriteFactory,
    rt: Option<ID2D1HwndRenderTarget>,
    bmp: Option<ID2D1Bitmap>,
    px: Vec<u8>, // BGRA(불투명)
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
}

thread_local! {
    static ED: RefCell<Option<Ed>> = const { RefCell::new(None) };
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
unsafe fn draw_objs(rt: &ID2D1RenderTarget, dw: &IDWriteFactory, objs: &[Obj], ox: f32, oy: f32, k: f32, comp: Option<(usize, &str)>) {
    for (i, o) in objs.iter().enumerate() {
        let mut o = o.clone();
        if let Some((_, c)) = comp.filter(|c| c.0 == i) {
            o.text.push_str(c);
        }
        let Some(l) = layout(dw, &o, k) else { continue };
        let empty = o.text.is_empty();
        // 글자가 어느 바탕에서도 보이게 살짝 그림자
        if !o.emoji {
            if let Ok(sh) = rt.CreateSolidColorBrush(&rgb(if o.color == 1 { 0xFFFFFF } else { 0x000000 }, if empty { 0.0 } else { 0.45 }), None) {
                rt.DrawTextLayout(D2D_POINT_2F { x: ox + o.x * k + 1.5 * k.max(0.5), y: oy + o.y * k + 1.5 * k.max(0.5) }, &l, &sh, D2D1_DRAW_TEXT_OPTIONS_NONE);
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
        let sty_h = if style { STYLE_H } else { 0.0 };
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
        rt.Clear(Some(&rgb(0x161616, 1.0)));
        let br = |c: u32, a: f32| rt.CreateSolidColorBrush(&rgb(c, a), None).unwrap();
        if let Some(b) = &bmp {
            rt.DrawBitmap(b, Some(&rc(ox, oy, iw, ih)), 1.0, D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, None);
        }
        let comp = self.sel.filter(|_| self.editing).map(|i| (i, self.comp.as_str()));
        rt.PushAxisAlignedClip(&rc(ox, oy, iw, ih), D2D1_ANTIALIAS_MODE_ALIASED);
        draw_objs(&rt, &self.dw, &self.objs, ox, oy, k, comp);
        rt.PopAxisAlignedClip();
        // 고른 칸: 테두리 + 오른쪽 아래 크기 손잡이 + (적는 중이면) 깜빡이는 글자 자리
        if let Some(i) = self.sel {
            if let Some(o) = self.objs.get(i) {
                let mut o2 = o.clone();
                if self.editing {
                    o2.text.push_str(&self.comp);
                }
                let b = obj_box(&self.dw, &o2);
                let bx = rc(ox + b[0] * k - 4.0, oy + b[1] * k - 4.0, b[2] * k + 8.0, b[3] * k + 8.0);
                rt.DrawRectangle(&bx, &br(0xA3E635, 1.0), 1.5, None);
                rt.FillRectangle(&rc(bx.right - 5.0, bx.bottom - 5.0, 10.0, 10.0), &br(0xA3E635, 1.0));
                if self.editing && (self.status.1.elapsed().as_millis() / 500) % 2 == 0 || self.editing && o.text.is_empty() {
                    rt.FillRectangle(&rc(ox + (b[0] + b[2]) * k + 1.0, oy + b[1] * k + 2.0, 2.0, b[3] * k - 4.0), &br(0xA3E635, 1.0));
                }
            }
        }
        // 자르기·글자 인식 영역
        if let Drag::Rect(a, b) = self.drag {
            let (x0, y0, x1, y1) = (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1));
            let c = if self.tool == Tool::Ocr { 0x3B82F6 } else { 0xA3E635 };
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
        x = btn(&rt, &mut btns, x, 11.0, "✂ 자르기", 82.0, self.tool == Tool::Crop, Btn::Tool(Tool::Crop));
        x = btn(&rt, &mut btns, x, 11.0, "T 글자", 70.0, self.tool == Tool::Text, Btn::Tool(Tool::Text));
        x = btn(&rt, &mut btns, x, 11.0, "😀 이모지", 86.0, self.emoji_open, Btn::EmojiPanel);
        x = btn(&rt, &mut btns, x, 11.0, "🔍 글자 인식", 100.0, self.tool == Tool::Ocr, Btn::Tool(Tool::Ocr));
        let _ = x;
        let mut rx = cw - 14.0 - 64.0;
        btn(&rt, &mut btns, rx, 11.0, "닫기", 64.0, false, Btn::Close);
        rx -= 70.0;
        btn(&rt, &mut btns, rx, 11.0, "저장", 64.0, false, Btn::Save);
        rx -= 78.0;
        btn(&rt, &mut btns, rx, 11.0, "복사", 72.0, true, Btn::Copy);
        // 둘째 줄: 글자 모양(글자 칸·이모지를 골랐거나 글자 도구일 때)
        if style {
            let y = BAR - 4.0;
            let mut x = 14.0;
            if text_sel.is_some() || self.tool == Tool::Text {
                let f = text_sel.as_ref().map_or(self.font, |o| o.font);
                x = btn(&rt, &mut btns, x, y, &format!("글꼴 {} ▾", FONTS[f].0), 128.0, false, Btn::Font);
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
                    if i == cur {
                        rt.DrawEllipse(&D2D1_ELLIPSE { point: D2D_POINT_2F { x: x + 14.0, y: y + 16.0 }, radiusX: 12.0, radiusY: 12.0 }, &br(0xA3E635, 1.0), 2.0, None);
                    }
                    btns.push((r, Btn::Color(i)));
                    x += 26.0;
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
        self.btns = btns;
        // 아래 안내 줄
        let hint = if self.status.1.elapsed() < Duration::from_secs(4) && !self.status.0.is_empty() {
            self.status.0.clone()
        } else {
            match self.tool {
                Tool::Crop => "자르기: 남길 곳을 끌어서 고르세요".into(),
                Tool::Text => "글자: 그림을 누르면 그 자리에 글자 칸이 생깁니다 · Enter 끝".into(),
                Tool::Ocr => "글자 인식: 읽을 곳을 끌어서 고르면 그 글자가 클립보드에 들어갑니다".into(),
                Tool::Move => "칸을 끌어 옮기고, 오른쪽 아래 손잡이로 크기 · 글자는 두 번 눌러 다시 편집 · Delete 지우기".into(),
            }
        };
        let hf = self.dw.CreateTextFormat(w!("Malgun Gothic"), None, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL, 12.0, w!("ko-kr")).unwrap();
        let t: Vec<u16> = hint.encode_utf16().collect();
        rt.DrawText(&t, &hf, &rc(16.0, ch - 28.0, cw - 32.0, 20.0), &br(0xF3F4F6, 0.55), D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL);
        if rt.EndDraw(None, None).is_err() {
            self.rt = None; // 장치를 잃었다 → 다음에 새로
        }
    }

    /// 결과 그림(글자·이모지를 그림에 새긴 것) BGRA
    unsafe fn render(&self) -> Option<Vec<u8>> {
        let wic: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).ok()?;
        let pb: Vec<u8> = self.px.clone();
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

    unsafe fn ocr(&mut self, a: (f32, f32), b: (f32, f32)) {
        let r = (a.0.min(b.0).max(0.0) as i32, a.1.min(b.1).max(0.0) as i32, (a.0.max(b.0) as i32).min(self.w as i32), (a.1.max(b.1) as i32).min(self.h as i32));
        if r.2 - r.0 < 4 || r.3 - r.1 < 4 {
            return;
        }
        let px = crate::capture::crop(&self.px, self.w as i32, r);
        match read_text(&px, (r.2 - r.0) as u32, (r.3 - r.1) as u32) {
            Ok(t) if !t.trim().is_empty() => {
                crate::clip::put_text(self.hwnd, &t);
                let short: String = t.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(40).collect();
                self.say(&format!("읽은 글자를 클립보드에 넣었습니다: {short}"));
                notify(self.notch, "글자를 읽어 복사했습니다", &short);
            }
            Ok(_) => self.say("글자를 찾지 못했습니다 · 조금 넓게 골라 보세요"),
            Err(e) => self.say(&format!("글자 인식을 못 했습니다: {e}")),
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
            self.editing = false;
            self.comp.clear();
            if let Some(i) = self.sel.filter(|i| self.objs.get(*i).is_some_and(|o| o.text.trim().is_empty())) {
                self.objs.remove(i);
                self.sel = None;
            }
        }
    }

    unsafe fn press(&mut self, b: Btn) {
        match b {
            Btn::Tool(t) => {
                self.end_edit();
                self.tool = if self.tool == t { Tool::Move } else { t };
                self.emoji_open = false;
            }
            Btn::EmojiPanel => {
                self.end_edit();
                self.emoji_open = !self.emoji_open;
                self.rt = None; // 그림 자리가 바뀐다
            }
            Btn::Emoji(i) => {
                let e = EMOJIS.split(' ').nth(i).unwrap_or("😀");
                let size = (self.w.min(self.h) as f32 * 0.18).clamp(32.0, 160.0);
                self.objs.push(Obj { text: e.into(), emoji: true, font: 0, size, color: 0, x: self.w as f32 / 2.0 - size / 2.0, y: self.h as f32 / 2.0 - size / 2.0 });
                self.sel = Some(self.objs.len() - 1);
                self.tool = Tool::Move;
                self.emoji_open = false;
                self.rt = None;
            }
            Btn::Font => {
                self.font = (self.sel.and_then(|i| self.objs.get(i)).map_or(self.font, |o| o.font) + 1) % FONTS.len();
                if let Some(o) = self.sel.and_then(|i| self.objs.get_mut(i)).filter(|o| !o.emoji) {
                    o.font = self.font;
                }
            }
            Btn::Smaller | Btn::Bigger => {
                let f = if b == Btn::Bigger { 1.2 } else { 1.0 / 1.2 };
                match self.sel.and_then(|i| self.objs.get_mut(i)) {
                    Some(o) => o.size = (o.size * f).clamp(8.0, 600.0),
                    None => self.size = (self.size * f).clamp(8.0, 600.0),
                }
            }
            Btn::Color(c) => {
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
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }

    unsafe fn down(&mut self, p: (f32, f32)) {
        self.mouse = p;
        if let Some((_, b)) = self.btns.iter().find(|(r, _)| p.0 >= r[0] && p.0 < r[2] && p.1 >= r[1] && p.1 < r[3]).copied() {
            return self.press(b);
        }
        let ip = self.to_img(p);
        let inside = ip.0 >= 0.0 && ip.1 >= 0.0 && ip.0 < self.w as f32 && ip.1 < self.h as f32;
        match self.tool {
            Tool::Crop | Tool::Ocr if inside => self.drag = Drag::Rect(ip, ip),
            Tool::Text if inside => {
                self.end_edit();
                if let Some((i, _)) = self.hit_obj(ip).filter(|(i, _)| !self.objs[*i].emoji) {
                    self.sel = Some(i);
                    self.editing = true;
                } else {
                    self.objs.push(Obj { text: String::new(), emoji: false, font: self.font, size: self.size, color: self.color, x: ip.0, y: ip.1 - self.size * 0.6 });
                    self.sel = Some(self.objs.len() - 1);
                    self.editing = true;
                }
                self.status.1 = Instant::now();
            }
            _ => {
                if !matches!(self.hit_obj(ip), Some((i, _)) if Some(i) == self.sel) {
                    self.end_edit();
                }
                match self.hit_obj(ip) {
                    Some((i, true)) => {
                        self.sel = Some(i);
                        self.drag = Drag::Resize(i, self.objs[i].size, ip.0);
                    }
                    Some((i, false)) => {
                        self.sel = Some(i);
                        self.drag = Drag::Move(i, ip.0 - self.objs[i].x, ip.1 - self.objs[i].y);
                    }
                    None => self.sel = None,
                }
            }
        }
    }

    unsafe fn moved(&mut self, p: (f32, f32)) {
        self.mouse = p;
        let ip = self.to_img(p);
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
            Drag::None => {}
        }
    }

    unsafe fn up(&mut self) {
        match std::mem::replace(&mut self.drag, Drag::None) {
            Drag::Rect(a, b) if self.tool == Tool::Crop => {
                let r = (a.0.min(b.0).max(0.0) as i32, a.1.min(b.1).max(0.0) as i32, (a.0.max(b.0) as i32).min(self.w as i32), (a.1.max(b.1) as i32).min(self.h as i32));
                if r.2 - r.0 > 4 && r.3 - r.1 > 4 {
                    self.px = crate::capture::crop(&self.px, self.w as i32, r);
                    self.w = (r.2 - r.0) as u32;
                    self.h = (r.3 - r.1) as u32;
                    for o in &mut self.objs {
                        o.x -= r.0 as f32;
                        o.y -= r.1 as f32;
                    }
                    self.bmp = None;
                    self.tool = Tool::Move;
                    self.say("잘랐습니다");
                }
            }
            Drag::Rect(a, b) if self.tool == Tool::Ocr => self.ocr(a, b),
            _ => {}
        }
    }

    unsafe fn dbl(&mut self, p: (f32, f32)) {
        let ip = self.to_img(p);
        if let Some((i, _)) = self.hit_obj(ip).filter(|(i, _)| !self.objs[*i].emoji) {
            self.sel = Some(i);
            self.editing = true; // 두 번 누르면 다시 글자 편집
            self.status.1 = Instant::now();
        }
    }

    unsafe fn key(&mut self, vk: u32) {
        let ctrl = GetKeyState(VK_CONTROL.0 as i32) < 0;
        if ctrl && vk == 0x43 && !self.editing {
            return self.copy(false);
        }
        match vk {
            0x1B => {
                if self.editing {
                    self.end_edit();
                } else {
                    self.tool = Tool::Move;
                    self.emoji_open = false;
                    self.sel = None;
                }
            }
            0x2E if !self.editing => {
                if let Some(i) = self.sel.take() {
                    self.objs.remove(i);
                }
            }
            _ => {}
        }
    }

    fn char(&mut self, c: u32) {
        if !self.editing {
            return;
        }
        let Some(o) = self.sel.and_then(|i| self.objs.get_mut(i)) else { return };
        match c {
            8 => {
                o.text.pop();
            }
            13 if unsafe { GetKeyState(VK_SHIFT.0 as i32) } < 0 => o.text.push('\n'),
            13 => self.end_edit(),
            c if c >= 32 => {
                if let Some(ch) = char::from_u32(c) {
                    o.text.push(ch);
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

/// 그림(BGRA) 속 글자를 읽는다(한국어 우선, 없으면 사용자 언어)
pub fn read_text(px: &[u8], w: u32, h: u32) -> windows::core::Result<String> {
    use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    // 너무 작으면 잘 못 읽으니 2배로 키운다
    let (px, w, h) = if w < 600 { (upscale2(px, w, h), w * 2, h * 2) } else { (px.to_vec(), w, h) };
    let buf = windows::Security::Cryptography::CryptographicBuffer::CreateFromByteArray(&px)?;
    let sb = SoftwareBitmap::CreateCopyFromBuffer(&buf, BitmapPixelFormat::Bgra8, w as i32, h as i32)?;
    let eng = windows::Globalization::Language::CreateLanguage(&HSTRING::from("ko")).and_then(|l| OcrEngine::TryCreateFromLanguage(&l)).or_else(|_| OcrEngine::TryCreateFromUserProfileLanguages())?;
    let res = eng.RecognizeAsync(&sb)?.get()?;
    let mut out = Vec::new();
    for line in res.Lines()? {
        out.push(line.Text()?.to_string());
    }
    Ok(out.join("\n"))
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
    let ww = ((w as i32 + 60).max(900)).min(sw * 9 / 10);
    let wh = ((h as i32 + 160).max(420)).min(sh * 9 / 10);
    let hw = CreateWindowExW(WINDOW_EX_STYLE(0), class, w!("노치 고양이 · 스크린샷 편집"), WS_OVERLAPPEDWINDOW | WS_VISIBLE, work.left + (sw - ww) / 2, work.top + (sh - wh) / 2, ww, wh, None, None, inst, None)?;
    let s = GetDpiForWindow(hw) as f32 / 96.0;
    let d2: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
    let dw: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
    let size = (h as f32 * 0.09).clamp(18.0, 96.0);
    ED.with(|e| {
        *e.borrow_mut() = Some(Ed { hwnd: hw, notch, d2, dw, rt: None, bmp: None, px, w, h, objs: vec![], sel: None, editing: false, comp: String::new(), tool: Tool::Move, drag: Drag::None, emoji_open: false, font: 0, size, color: 0, mouse: (-1.0, -1.0), btns: vec![], view: (0.0, 0.0, 1.0), status: (String::new(), Instant::now()), s });
    });
    SetTimer(hw, 1, 500, None); // 글자 자리 깜빡임
    let _ = SetForegroundWindow(hw);
    let _ = InvalidateRect(hw, None, false);
    Ok(hw)
}

/// 시험용: 열린 편집 창에 글자·이모지를 넣고 결과 그림을 PNG 로 저장한다
pub unsafe fn demo(out: &std::path::Path) {
    ED.with(|e| {
        if let Some(ed) = e.borrow_mut().as_mut() {
            let s = ed.size;
            ed.objs.push(Obj { text: "노치 고양이 편집".into(), emoji: false, font: 0, size: s, color: 3, x: 20.0, y: 20.0 });
            ed.objs.push(Obj { text: "👍".into(), emoji: true, font: 0, size: s * 1.6, color: 0, x: ed.w as f32 * 0.6, y: ed.h as f32 * 0.4 });
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
            draw_objs(&rt, &dw, &[Obj { text: "안녕하세요 노치".into(), emoji: false, font: 0, size: 48.0, color: 1, x: 10.0, y: 20.0 }], 0.0, 0.0, 1.0, None);
            rt.EndDraw(None, None).unwrap();
            let mut px = vec![0u8; (w * h * 4) as usize];
            wb.CopyPixels(std::ptr::null(), w * 4, &mut px).unwrap();
            let t = read_text(&px, w, h).unwrap();
            println!("읽은 글자: {t}");
            assert!(t.replace(' ', "").contains("안녕하세요"), "{t}");
        }
    }
}
