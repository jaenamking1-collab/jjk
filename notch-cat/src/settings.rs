// 시세 종목 설정창. 시세 위젯(tools/ticker.pyw)의 설정창과 같은 파일을 같은 규칙으로 고친다
// (줄 형식·지수 버튼은 quotes.rs 에 있다). 노치 창은 자판을 못 받으므로 보통 창을 따로 띄운다.
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{DRAWITEMSTRUCT, ODS_FOCUS, ODS_SELECTED};
use windows::Win32::UI::HiDpi::GetDpiForSystem;
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::quotes::{self, Quotes, IDX_BTNS};

const fn rgb(r: u32, g: u32, b: u32) -> COLORREF {
    COLORREF(r | g << 8 | b << 16)
}
const BG: COLORREF = rgb(0x0E, 0x10, 0x14);
const FIELD: COLORREF = rgb(0x1A, 0x1E, 0x26);
const FG: COLORREF = rgb(0xF3, 0xF4, 0xF6);
const DIM: COLORREF = rgb(0xA4, 0xAC, 0xB8);
const LIME: COLORREF = rgb(0xA3, 0xE6, 0x35);
const INK: COLORREF = rgb(0x10, 0x16, 0x0B);
const BAD: COLORREF = rgb(0xFF, 0x5C, 0x66);

const ID_EDIT: i32 = 10;
const ID_IDX: i32 = 100; // 지수 버튼 100…105
const W: f32 = 344.0;

#[derive(Clone)]
struct St {
    win: HWND,
    edit: HWND,
    err: HWND,
    fonts: [HFONT; 2],
    brushes: [HBRUSH; 2], // 바탕, 입력칸
    quotes: Arc<Mutex<Quotes>>,
}

thread_local! {
    static ST: RefCell<Option<St>> = const { RefCell::new(None) };
}

fn st() -> Option<St> {
    ST.with(|s| s.borrow().clone())
}

/// 열려 있는 설정창(자판 처리를 위해 메시지 고리가 묻는다).
pub fn hwnd() -> Option<HWND> {
    st().map(|s| s.win)
}

fn wz(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

unsafe fn text_of(h: HWND) -> String {
    let mut buf = vec![0u16; GetWindowTextLengthW(h) as usize + 1];
    let n = GetWindowTextW(h, &mut buf) as usize;
    String::from_utf16_lossy(&buf[..n])
}

fn idx_rows() -> Vec<(&'static str, &'static str)> {
    IDX_BTNS.iter().flat_map(|g| g.iter().copied()).collect()
}

pub unsafe fn open(quotes: Arc<Mutex<Quotes>>, icon: HICON) {
    if let Some(h) = hwnd() {
        let _ = SetForegroundWindow(h);
        return;
    }
    let inst = GetModuleHandleW(None).unwrap_or_default();
    let class = w!("NotchCatSettings");
    RegisterClassW(&WNDCLASSW { lpfnWndProc: Some(proc), hInstance: inst.into(), hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(), hIcon: icon, lpszClassName: class, ..Default::default() });

    let k = GetDpiForSystem() as f32 / 96.0;
    let px = |v: f32| (v * k).round() as i32;
    let path = quotes::config_path();
    let lines = quotes::read_lines(&path);
    // ponytail: 줄이 24줄을 넘으면 입력칸 안에서 휠·화살표로 넘겨 본다(스크롤 막대 없음)
    let edit_h = (lines.len() + 2).clamp(10, 24) as f32 * 18.0 + 12.0;
    let h = 100.0 + edit_h + 150.0;

    let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU;
    let mut rc = RECT { left: 0, top: 0, right: px(W), bottom: px(h) };
    let _ = AdjustWindowRectEx(&mut rc, style, false, WS_EX_TOPMOST);
    let (ww, wh) = (rc.right - rc.left, rc.bottom - rc.top);
    // 트레이 가까이: 작업 영역 오른쪽 아래
    let mut work = RECT::default();
    let _ = SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut work as *mut _ as _), Default::default());
    let Ok(win) = CreateWindowExW(WS_EX_TOPMOST, class, w!("노치 고양이 · 시세 종목"), style, work.right - ww - px(16.0), (work.bottom - wh - px(16.0)).max(work.top), ww, wh, None, None, inst, None) else { return };
    let dark = 1i32;
    let _ = DwmSetWindowAttribute(win, DWMWINDOWATTRIBUTE(20), &dark as *const _ as _, 4); // 제목 줄도 어둡게

    let font = |size: f32, bold: bool, face: PCWSTR| CreateFontW(-px(size), 0, 0, 0, if bold { 700 } else { 400 }, 0, 0, 0, 1, 0, 0, 5, 0, face);
    let fonts = [font(12.0, false, w!("Malgun Gothic")), font(14.0, false, w!("Malgun Gothic"))];
    let mk = |cls: PCWSTR, text: &str, st: u32, x: f32, y: f32, cw: f32, ch: f32, id: i32, f: usize| {
        let t = wz(text);
        let c = CreateWindowExW(Default::default(), cls, PCWSTR(t.as_ptr()), WS_CHILD | WS_VISIBLE | WINDOW_STYLE(st), px(x), px(y), px(cw), px(ch), win, HMENU(id as usize as _), inst, None).unwrap_or_default();
        SendMessageW(c, WM_SETFONT, WPARAM(fonts[f].0 as usize), LPARAM(1));
        c
    };
    let (tab, btn) = (WS_TABSTOP.0, WS_TABSTOP.0 | BS_OWNERDRAW as u32);
    mk(w!("STATIC"), "목록 (한 줄에 하나 · 심볼=이름 · --- 는 구분선)", 0, 14.0, 12.0, W - 28.0, 18.0, -1, 0);
    let mut id = ID_IDX;
    for (r, group) in IDX_BTNS.iter().enumerate() {
        let mut x = 14.0;
        for (label, _) in group.iter() {
            let bw = 26.0 + label.chars().map(|c| if c.is_ascii() { 7.5 } else { 12.5 }).sum::<f32>();
            mk(w!("BUTTON"), label, btn, x, 38.0 + r as f32 * 30.0, bw, 24.0, id, 0);
            x += bw + 6.0;
            id += 1;
        }
    }
    // 입력칸: 바탕 칸(STATIC) 위에 여백을 두고 얹는다
    mk(w!("STATIC"), "", 0, 14.0, 100.0, W - 28.0, edit_h, -2, 0);
    let edit = mk(w!("EDIT"), &lines.join("\r\n"), tab | (ES_MULTILINE | ES_AUTOVSCROLL | ES_AUTOHSCROLL | ES_WANTRETURN) as u32, 22.0, 106.0, W - 44.0, edit_h - 12.0, ID_EDIT, 1);
    let y = 100.0 + edit_h + 8.0;
    let tip = "국내 6자리 코드(069500) · 지수 KOSPI KOSDAQ\n환율 USDKRW=X · 코인 BTC-KRW · 미국 AAPL\n이름이 길면 069500=코덱스200 처럼 직접 지정\n비율 줄(A/B)·@목표는 위젯용이라 그대로 둡니다";
    mk(w!("STATIC"), tip, 0, 14.0, y, W - 28.0, 72.0, -1, 0);
    // 어느 파일을 고치는지 보여 준다(시세 위젯과 같은 파일)
    let tail: Vec<String> = path.components().rev().take(2).map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    mk(w!("STATIC"), &format!("시세 위젯과 같은 목록 · {}", tail.into_iter().rev().collect::<Vec<_>>().join("\\")), 0x4000, 14.0, y + 74.0, W - 28.0, 18.0, -1, 0);
    let err = mk(w!("STATIC"), "", 0, 14.0, y + 102.0, 170.0, 20.0, -3, 0);
    mk(w!("BUTTON"), "취소", btn, W - 14.0 - 76.0 - 6.0 - 58.0, y + 98.0, 58.0, 28.0, IDCANCEL.0, 0);
    mk(w!("BUTTON"), "저장", btn, W - 14.0 - 76.0, y + 98.0, 76.0, 28.0, IDOK.0, 0);

    ST.with(|s| *s.borrow_mut() = Some(St { win, edit, err, fonts, brushes: [CreateSolidBrush(BG), CreateSolidBrush(FIELD)], quotes }));
    let _ = ShowWindow(win, SW_SHOW);
    let _ = SetForegroundWindow(win);
    let _ = SetFocus(edit);
}

unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let Some(s) = st() else { return DefWindowProcW(hwnd, msg, wp, lp) };
    match msg {
        WM_ERASEBKGND => {
            let mut rc = RECT::default();
            let _ = GetClientRect(hwnd, &mut rc);
            FillRect(HDC(wp.0 as _), &rc, s.brushes[0]);
            return LRESULT(1);
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT => {
            let (dc, ctl) = (HDC(wp.0 as _), HWND(lp.0 as _));
            let field = msg == WM_CTLCOLOREDIT || GetDlgCtrlID(ctl) == -2;
            SetTextColor(dc, if ctl == s.err { BAD } else if field { FG } else { DIM });
            SetBkColor(dc, if field { FIELD } else { BG });
            return LRESULT(s.brushes[field as usize].0 as isize);
        }
        WM_DRAWITEM => {
            let d = &*(lp.0 as *const DRAWITEMSTRUCT);
            let id = d.CtlID as i32;
            // 켜진 지수 버튼과 저장은 라임, 나머지는 입력칸 색
            let on = id == IDOK.0 || idx_rows().get((id - ID_IDX) as usize).is_some_and(|r| quotes::has_idx(&quotes::text_lines(&text_of(s.edit)), r.1));
            let down = d.itemState.0 & ODS_SELECTED.0 != 0;
            let fill = match (on, down) {
                (true, false) => LIME,
                (true, true) => rgb(0x86, 0xC4, 0x24),
                (false, false) => FIELD,
                (false, true) => rgb(0x26, 0x2C, 0x38),
            };
            let (rc, dc) = (d.rcItem, d.hDC);
            FillRect(dc, &rc, s.brushes[0]);
            let brush = CreateSolidBrush(fill);
            let focus = d.itemState.0 & ODS_FOCUS.0 != 0;
            let pen = CreatePen(PS_SOLID, 1, if focus && !on { LIME } else { fill });
            let (ob, op) = (SelectObject(dc, brush), SelectObject(dc, pen));
            let r = (rc.bottom - rc.top) * 2 / 5;
            let _ = RoundRect(dc, rc.left, rc.top, rc.right, rc.bottom, r, r);
            SelectObject(dc, ob);
            SelectObject(dc, op);
            let _ = DeleteObject(brush);
            let _ = DeleteObject(pen);
            SetBkMode(dc, TRANSPARENT);
            SetTextColor(dc, if on { INK } else { FG });
            let mut t: Vec<u16> = text_of(d.hwndItem).encode_utf16().collect();
            let mut tr = RECT { top: rc.top + down as i32, bottom: rc.bottom + down as i32, ..rc };
            DrawTextW(dc, &mut t, &mut tr, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
            return LRESULT(1);
        }
        WM_COMMAND => {
            let (id, code) = ((wp.0 & 0xFFFF) as i32, (wp.0 >> 16) as u32);
            let repaint = || {
                for i in 0..idx_rows().len() as i32 {
                    if let Ok(b) = GetDlgItem(hwnd, ID_IDX + i) {
                        let _ = InvalidateRect(b, None, false);
                    }
                }
            };
            if let Some(row) = idx_rows().get((id - ID_IDX) as usize).filter(|_| id >= ID_IDX) {
                // 지수 버튼: 목록 맨 위에 넣고, 다시 누르면 뺀다. 저장을 눌러야 반영.
                let mut lines = quotes::text_lines(&text_of(s.edit));
                quotes::toggle_idx(&mut lines, row.1);
                let _ = SetWindowTextW(s.edit, PCWSTR(wz(&lines.join("\r\n")).as_ptr()));
                repaint();
            } else if id == ID_EDIT && code == EN_CHANGE {
                repaint();
            } else if id == IDOK.0 {
                match quotes::save_lines(&quotes::config_path(), &quotes::text_lines(&text_of(s.edit))) {
                    Ok(()) => {
                        s.quotes.lock().unwrap().reload = true;
                        crate::log("settings saved");
                        let _ = DestroyWindow(hwnd);
                    }
                    Err(e) => {
                        let _ = SetWindowTextW(s.err, PCWSTR(wz(&e).as_ptr()));
                    }
                }
            } else if id == IDCANCEL.0 {
                let _ = DestroyWindow(hwnd);
            }
            return LRESULT(0);
        }
        WM_DESTROY => {
            ST.with(|v| *v.borrow_mut() = None);
            for f in s.fonts {
                let _ = DeleteObject(f);
            }
            for b in s.brushes {
                let _ = DeleteObject(b);
            }
        }
        _ => {}
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}
