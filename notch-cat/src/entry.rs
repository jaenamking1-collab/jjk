// 일정 넣기 창. 노치 창은 자판을 못 받으므로 작은 보통 창을 노치 아래에 띄운다.
// 제목을 적고, 날(오늘·내일·모레·다음 주)과 시각(종일·09:00·12:00·18:00·±1시간)을 단추로 고른 뒤 저장하면 구글 캘린더에 들어간다.
use std::cell::RefCell;
use std::sync::mpsc::Sender;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::DRAWITEMSTRUCT;
use windows::Win32::UI::HiDpi::GetDpiForSystem;
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::gcal::{self, CalInfo, Cmd};
use crate::settings::{paint_button, text_of, wz, BAD, BG, DIM, FG, FIELD};

const W: f32 = 380.0;
const H: f32 = 232.0;
const ID_TITLE: i32 = 10;
const ID_CAL: i32 = 20;
const ID_DAY: i32 = 200; // 오늘·내일·모레·다음 주
const ID_TIME: i32 = 210; // 종일·09:00·12:00·18:00·+1시간·−1시간
const DAYS: [(&str, i64); 4] = [("오늘", 0), ("내일", 1), ("모레", 2), ("다음 주", 7)];
const TIMES: [(&str, i32); 6] = [("종일", -1), ("09:00", 540), ("12:00", 720), ("18:00", 1080), ("+1시간", 60), ("−1시간", -60)];

#[derive(Clone)]
struct St {
    win: HWND,
    title: HWND,
    when: HWND,
    combo: HWND,
    err: HWND,
    fonts: [HFONT; 2],
    brushes: [HBRUSH; 2],
    tx: Sender<Cmd>,
    cals: Vec<usize>, // 콤보 줄 → 캘린더 번호
    day: i64,
    min: i32, // -1 = 종일
}

thread_local! {
    static ST: RefCell<Option<St>> = const { RefCell::new(None) };
}

fn st() -> Option<St> {
    ST.with(|s| s.borrow().clone())
}

pub fn hwnd() -> Option<HWND> {
    st().map(|s| s.win)
}

/// "11월 13일 (금) 오전 9:00" / "11월 13일 (금) 종일"
pub fn when_text(day: i64, min: i32) -> String {
    let (_, m, d) = gcal::ymd(day);
    let head = format!("{m}월 {d}일 ({})", gcal::WEEK[gcal::weekday(day)]);
    if min < 0 {
        return format!("{head} 종일");
    }
    let (h, mi) = (min / 60, min % 60);
    format!("{head} {} {}:{mi:02}", if h < 12 { "오전" } else { "오후" }, if h % 12 == 0 { 12 } else { h % 12 })
}

/// 시각 단추를 눌렀을 때의 새 (날, 분). ±1시간은 자정을 넘으면 날도 넘어간다.
pub fn pick_time(day: i64, min: i32, i: usize) -> (i64, i32) {
    let (_, v) = TIMES[i];
    if i < 4 {
        return (day, v);
    }
    let base = if min < 0 { 540 } else { min } + v;
    (day + base.div_euclid(1440) as i64, base.rem_euclid(1440))
}

pub unsafe fn open(tx: Sender<Cmd>, all: &[CalInfo], accounts: &[String], day: i64, icon: HICON) {
    if let Some(h) = hwnd() {
        let _ = SetForegroundWindow(h);
        return;
    }
    let inst = GetModuleHandleW(None).unwrap_or_default();
    let class = w!("NotchCatEntry");
    RegisterClassW(&WNDCLASSW { lpfnWndProc: Some(proc), hInstance: inst.into(), hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(), hIcon: icon, lpszClassName: class, ..Default::default() });
    let k = GetDpiForSystem() as f32 / 96.0;
    let px = |v: f32| (v * k).round() as i32;
    let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU;
    let mut rc = RECT { left: 0, top: 0, right: px(W), bottom: px(H) };
    let _ = AdjustWindowRectEx(&mut rc, style, false, WS_EX_TOPMOST);
    let (ww, wh) = (rc.right - rc.left, rc.bottom - rc.top);
    // 노치 바로 아래 가운데
    let sw = GetSystemMetrics(SM_CXSCREEN);
    let Ok(win) = CreateWindowExW(WS_EX_TOPMOST, class, w!("노치 고양이 · 일정 넣기"), style, (sw - ww) / 2, px(70.0), ww, wh, None, None, inst, None) else { return };
    let dark = 1i32;
    let _ = DwmSetWindowAttribute(win, DWMWINDOWATTRIBUTE(20), &dark as *const _ as _, 4);

    let font = |size: f32| CreateFontW(-px(size), 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, w!("Malgun Gothic"));
    let fonts = [font(12.0), font(16.0)];
    let mk = |cls: PCWSTR, text: &str, sty: u32, x: f32, y: f32, cw: f32, ch: f32, id: i32, f: usize| {
        let t = wz(text);
        let c = CreateWindowExW(Default::default(), cls, PCWSTR(t.as_ptr()), WS_CHILD | WS_VISIBLE | WINDOW_STYLE(sty), px(x), px(y), px(cw), px(ch), win, HMENU(id as usize as _), inst, None).unwrap_or_default();
        SendMessageW(c, WM_SETFONT, WPARAM(fonts[f].0 as usize), LPARAM(1));
        c
    };
    let (tab, btn) = (WS_TABSTOP.0, WS_TABSTOP.0 | BS_OWNERDRAW as u32);
    // 처음 시각: 오늘이면 다음 정각, 다른 날이면 오전 9시
    let (today, now) = gcal::today();
    let min = if day == today { ((now / 60 + 1) * 60).min(23 * 60) } else { 540 };
    mk(w!("STATIC"), "", 0, 14.0, 14.0, W - 28.0, 38.0, -2, 0);
    let title = mk(w!("EDIT"), "", tab | ES_AUTOHSCROLL as u32, 24.0, 22.0, W - 48.0, 24.0, ID_TITLE, 1);
    SendMessageW(title, 0x1501, WPARAM(1), LPARAM(w!("무슨 일정인가요?").as_ptr() as isize));
    let when = mk(w!("STATIC"), &when_text(day, min), 0, 16.0, 62.0, W - 32.0, 20.0, -4, 1);
    let mut x = 14.0;
    for (i, (label, _)) in DAYS.iter().enumerate() {
        let bw = 22.0 + label.chars().count() as f32 * 12.5;
        mk(w!("BUTTON"), label, btn, x, 90.0, bw, 26.0, ID_DAY + i as i32, 0);
        x += bw + 6.0;
    }
    let mut x = 14.0;
    for (i, (label, _)) in TIMES.iter().enumerate() {
        let bw = 20.0 + label.chars().map(|c| if c.is_ascii() || c == '−' { 7.5 } else { 12.5 }).sum::<f32>();
        mk(w!("BUTTON"), label, btn, x, 122.0, bw, 26.0, ID_TIME + i as i32, 0);
        x += bw + 6.0;
    }
    // 넣을 캘린더(쓸 수 있는 것만)
    let combo = mk(w!("COMBOBOX"), "", tab | WS_VSCROLL.0 | CBS_DROPDOWNLIST as u32, 14.0, 158.0, W - 28.0, 200.0, ID_CAL, 0);
    let mut cals = Vec::new();
    for (i, c) in all.iter().enumerate().filter(|(_, c)| c.writable) {
        let label = wz(&if accounts.len() > 1 { format!("{} · {}", c.name, accounts.get(c.account).map_or("", |s| s.as_str())) } else { c.name.clone() });
        SendMessageW(combo, CB_ADDSTRING, WPARAM(0), LPARAM(label.as_ptr() as isize));
        cals.push(i);
    }
    // 기본은 그 계정의 기본 캘린더(아이디가 메일 주소와 같다)
    let first = cals.iter().position(|i| accounts.iter().any(|a| *a == all[*i].id)).unwrap_or(0);
    SendMessageW(combo, CB_SETCURSEL, WPARAM(first), LPARAM(0));
    let err = mk(w!("STATIC"), "", 0, 16.0, 198.0, 190.0, 20.0, -3, 0);
    mk(w!("BUTTON"), "취소", btn, W - 14.0 - 84.0 - 6.0 - 58.0, 192.0, 58.0, 28.0, IDCANCEL.0, 0);
    mk(w!("BUTTON"), "넣기", btn, W - 14.0 - 84.0, 192.0, 84.0, 28.0, IDOK.0, 0);

    ST.with(|s| *s.borrow_mut() = Some(St { win, title, when, combo, err, fonts, brushes: [CreateSolidBrush(BG), CreateSolidBrush(FIELD)], tx, cals, day, min }));
    let _ = ShowWindow(win, SW_SHOW);
    let _ = SetForegroundWindow(win);
    let _ = SetFocus(title);
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
            SetTextColor(dc, if ctl == s.err { BAD } else if field || ctl == s.when { FG } else { DIM });
            SetBkColor(dc, if field { FIELD } else { BG });
            return LRESULT(s.brushes[field as usize].0 as isize);
        }
        WM_DRAWITEM => {
            let d = &*(lp.0 as *const DRAWITEMSTRUCT);
            let id = d.CtlID as i32;
            let (today, _) = gcal::today();
            // 지금 고른 날·시각과 '넣기'는 라임
            let on = id == IDOK.0 || DAYS.get((id - ID_DAY) as usize).is_some_and(|x| id >= ID_DAY && s.day == today + x.1) || (ID_TIME..ID_TIME + 4).contains(&id) && s.min == TIMES[(id - ID_TIME) as usize].1;
            paint_button(d, on, s.brushes[0]);
            return LRESULT(1);
        }
        WM_COMMAND => {
            let id = (wp.0 & 0xFFFF) as i32;
            let (today, _) = gcal::today();
            let mut next = None;
            if (ID_DAY..ID_DAY + DAYS.len() as i32).contains(&id) {
                next = Some((today + DAYS[(id - ID_DAY) as usize].1, s.min));
            } else if (ID_TIME..ID_TIME + TIMES.len() as i32).contains(&id) {
                next = Some(pick_time(s.day, s.min, (id - ID_TIME) as usize));
            } else if id == IDOK.0 {
                let title = text_of(s.title).trim().to_string();
                let sel = SendMessageW(s.combo, CB_GETCURSEL, WPARAM(0), LPARAM(0)).0;
                match (title.is_empty(), s.cals.get(sel.max(0) as usize)) {
                    (true, _) => {
                        let _ = SetWindowTextW(s.err, w!("제목을 적어 주세요"));
                    }
                    (_, None) => {
                        let _ = SetWindowTextW(s.err, w!("넣을 수 있는 캘린더가 없습니다"));
                    }
                    (false, Some(cal)) => {
                        let _ = s.tx.send(Cmd::Add { cal: *cal, title, day: s.day, start: s.min, end: -1 });
                        let _ = DestroyWindow(hwnd);
                    }
                }
            } else if id == IDCANCEL.0 {
                let _ = DestroyWindow(hwnd);
            }
            if let Some((day, min)) = next {
                ST.with(|v| {
                    if let Some(x) = v.borrow_mut().as_mut() {
                        (x.day, x.min) = (day, min);
                    }
                });
                let _ = SetWindowTextW(s.when, PCWSTR(wz(&when_text(day, min)).as_ptr()));
                let _ = InvalidateRect(hwnd, None, true);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_time_buttons() {
        let d = gcal::days(2026, 11, 13);
        assert_eq!(when_text(d, 540), "11월 13일 (금) 오전 9:00");
        assert_eq!(when_text(d, 780), "11월 13일 (금) 오후 1:00");
        assert_eq!(when_text(d, 0), "11월 13일 (금) 오전 12:00");
        assert_eq!(when_text(d, -1), "11월 13일 (금) 종일");
        assert_eq!(pick_time(d, 540, 3), (d, 1080)); // 18:00
        assert_eq!(pick_time(d, 23 * 60 + 30, 4), (d + 1, 30)); // +1시간이 자정을 넘으면 다음 날
        assert_eq!(pick_time(d, 30, 5), (d - 1, 23 * 60 + 30));
        assert_eq!(pick_time(d, -1, 4), (d, 600)); // 종일에서 +1시간 → 10:00
        assert_eq!(pick_time(d, 540, 0), (d, -1)); // 종일
    }
}
