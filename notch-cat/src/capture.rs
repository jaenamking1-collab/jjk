//! 스크린샷: 화면을 그대로 얼려 보여 주고(어둡게), 끌어서 고른 곳만 밝게. 놓으면 그 그림을 넘긴다.
//! 노치는 찍는 동안 숨는다. Esc·오른쪽 단추는 취소.

use std::sync::Mutex;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture, SetFocus};
use windows::Win32::UI::WindowsAndMessaging::*;

/// 찍은 결과: BGRA(위에서 아래로), 너비, 높이
pub type Shot = (Vec<u8>, u32, u32);

pub static DONE: Mutex<Option<Shot>> = Mutex::new(None);
pub const WM_SNAPPED: u32 = WM_APP + 5; // 노치에게: 다 찍었다(또는 취소)

struct St {
    notch: isize, // 노치 창(HWND 는 스레드 사이로 못 넘겨 숫자로)
    px: Vec<u8>,  // 얼린 화면(BGRA)
    dim: Vec<u8>, // 어둡게 한 것
    w: i32,
    h: i32,
    a: Option<(i32, i32)>,
    b: (i32, i32),
}

static ST: Mutex<Option<St>> = Mutex::new(None);

/// 화면의 한 부분을 BGRA 로 읽는다(가상 화면 좌표)
pub unsafe fn grab(x: i32, y: i32, w: i32, h: i32) -> Vec<u8> {
    let scr = GetDC(None);
    let mem = CreateCompatibleDC(scr);
    let bmp = CreateCompatibleBitmap(scr, w, h);
    let old = SelectObject(mem, bmp);
    let _ = BitBlt(mem, 0, 0, w, h, scr, x, y, SRCCOPY | CAPTUREBLT);
    let mut bi = BITMAPINFO { bmiHeader: BITMAPINFOHEADER { biSize: 40, biWidth: w, biHeight: -h, biPlanes: 1, biBitCount: 32, ..Default::default() }, ..Default::default() };
    let mut px = vec![0u8; (w * h * 4) as usize];
    GetDIBits(mem, bmp, 0, h as u32, Some(px.as_mut_ptr() as _), &mut bi, DIB_RGB_COLORS);
    SelectObject(mem, old);
    let _ = DeleteObject(bmp);
    let _ = DeleteDC(mem);
    ReleaseDC(None, scr);
    for p in px.chunks_exact_mut(4) {
        p[3] = 255;
    }
    px
}

/// 고르기 화면을 띄운다(마우스가 있는 모니터). 노치는 미리 숨겨 둔다.
pub unsafe fn start(notch: HWND) {
    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);
    let mon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
    let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    let _ = GetMonitorInfoW(mon, &mut mi);
    let r = mi.rcMonitor;
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    let px = grab(r.left, r.top, w, h);
    let dim = px.iter().enumerate().map(|(i, v)| if i % 4 == 3 { 255 } else { (*v as u32 * 45 / 100) as u8 }).collect();
    *ST.lock().unwrap() = Some(St { notch: notch.0 as isize, px, dim, w, h, a: None, b: (0, 0) });
    let inst = GetModuleHandleW(None).unwrap_or_default();
    let class = w!("NotchCatSnap");
    RegisterClassW(&WNDCLASSW { lpfnWndProc: Some(proc), hInstance: inst.into(), hCursor: LoadCursorW(None, IDC_CROSS).unwrap_or_default(), lpszClassName: class, ..Default::default() });
    if let Ok(hw) = CreateWindowExW(WS_EX_TOPMOST | WS_EX_TOOLWINDOW, class, w!("스크린샷"), WS_POPUP | WS_VISIBLE, r.left, r.top, w, h, None, None, inst, None) {
        let _ = SetForegroundWindow(hw);
        let _ = SetFocus(hw);
    }
}

/// 고른 사각형(창 좌표, 왼쪽 위·오른쪽 아래)
fn rect(a: (i32, i32), b: (i32, i32)) -> (i32, i32, i32, i32) {
    (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1))
}

/// BGRA 에서 사각형만 잘라 낸다
pub fn crop(px: &[u8], w: i32, r: (i32, i32, i32, i32)) -> Vec<u8> {
    let mut out = Vec::with_capacity(((r.2 - r.0) * (r.3 - r.1) * 4) as usize);
    for y in r.1..r.3 {
        let s = ((y * w + r.0) * 4) as usize;
        out.extend_from_slice(&px[s..s + ((r.2 - r.0) * 4) as usize]);
    }
    out
}

unsafe fn finish(hw: HWND, take: bool) {
    let st = ST.lock().unwrap().take();
    let _ = ReleaseCapture();
    let _ = DestroyWindow(hw);
    let Some(st) = st else { return };
    let shot = st.a.filter(|_| take).and_then(|a| {
        let r = rect(a, st.b);
        (r.2 - r.0 > 3 && r.3 - r.1 > 3).then(|| (crop(&st.px, st.w, r), (r.2 - r.0) as u32, (r.3 - r.1) as u32))
    });
    *DONE.lock().unwrap() = shot;
    let _ = PostMessageW(HWND(st.notch as _), WM_SNAPPED, WPARAM(0), LPARAM(0));
}

unsafe fn paint(hw: HWND) {
    let mut ps = PAINTSTRUCT::default();
    let dc = BeginPaint(hw, &mut ps);
    if let Some(st) = ST.lock().unwrap().as_ref() {
        // 깜빡이지 않게 메모리에 그린 뒤 한 번에
        let mem = CreateCompatibleDC(dc);
        let bmp = CreateCompatibleBitmap(dc, st.w, st.h);
        let old = SelectObject(mem, bmp);
        let bi = BITMAPINFO { bmiHeader: BITMAPINFOHEADER { biSize: 40, biWidth: st.w, biHeight: -st.h, biPlanes: 1, biBitCount: 32, ..Default::default() }, ..Default::default() };
        SetDIBitsToDevice(mem, 0, 0, st.w as u32, st.h as u32, 0, 0, 0, st.h as u32, st.dim.as_ptr() as _, &bi, DIB_RGB_COLORS);
        if let Some(a) = st.a {
            let r = rect(a, st.b);
            let (cw, ch) = (r.2 - r.0, r.3 - r.1);
            if cw > 0 && ch > 0 {
                // 고른 곳만 원래 밝기로
                SetDIBitsToDevice(mem, r.0, r.1, cw as u32, ch as u32, r.0, st.h - r.3, 0, st.h as u32, st.px.as_ptr() as _, &bi, DIB_RGB_COLORS);
                let br = CreateSolidBrush(COLORREF(0x35E6A3)); // 라임(BGR)
                for k in 0..2 {
                    let _ = FrameRect(mem, &RECT { left: r.0 - k, top: r.1 - k, right: r.2 + k, bottom: r.3 + k }, br);
                }
                let _ = DeleteObject(br);
                let txt: Vec<u16> = format!(" {cw} × {ch} ").encode_utf16().collect();
                SetBkColor(mem, COLORREF(0x000000));
                SetTextColor(mem, COLORREF(0xFFFFFF));
                let _ = TextOutW(mem, r.0, (r.1 - 22).max(0), &txt);
            }
        } else {
            let tip: Vec<u16> = " 끌어서 찍을 곳을 고르세요 · Esc 취소 ".encode_utf16().collect();
            SetBkColor(mem, COLORREF(0x000000));
            SetTextColor(mem, COLORREF(0xFFFFFF));
            let _ = TextOutW(mem, st.w / 2 - 130, 40, &tip);
        }
        let _ = BitBlt(dc, 0, 0, st.w, st.h, mem, 0, 0, SRCCOPY);
        SelectObject(mem, old);
        let _ = DeleteObject(bmp);
        let _ = DeleteDC(mem);
    }
    let _ = EndPaint(hw, &ps);
}

unsafe extern "system" fn proc(hw: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let pt = || ((lp.0 & 0xFFFF) as i16 as i32, ((lp.0 >> 16) & 0xFFFF) as i16 as i32);
    match msg {
        WM_PAINT => {
            paint(hw);
            return LRESULT(0);
        }
        WM_ERASEBKGND => return LRESULT(1),
        WM_LBUTTONDOWN => {
            if let Some(st) = ST.lock().unwrap().as_mut() {
                st.a = Some(pt());
                st.b = pt();
            }
            SetCapture(hw);
            let _ = InvalidateRect(hw, None, false);
        }
        WM_MOUSEMOVE => {
            if let Some(st) = ST.lock().unwrap().as_mut().filter(|s| s.a.is_some()) {
                st.b = pt();
                let _ = InvalidateRect(hw, None, false);
            }
        }
        WM_LBUTTONUP => finish(hw, true),
        WM_RBUTTONUP => finish(hw, false),
        WM_KEYDOWN if wp.0 == 0x1B => finish(hw, false),
        WM_KILLFOCUS => {} // 다른 창으로 가도 고르기는 계속(Esc 로 취소)
        _ => {}
    }
    DefWindowProcW(hw, msg, wp, lp)
}

/// 시험용: 고르기 화면 없이 바로 이 사각형을 찍은 것처럼(가상 화면 좌표)
pub unsafe fn fake(notch: HWND, x: i32, y: i32, w: i32, h: i32) {
    *DONE.lock().unwrap() = Some((grab(x, y, w, h), w as u32, h as u32));
    let _ = PostMessageW(notch, WM_SNAPPED, WPARAM(0), LPARAM(0));
}

#[allow(dead_code)]
const _: PCWSTR = PCWSTR::null();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_takes_rect_rows() {
        // 4×3 그림, 칸마다 값 = 번호
        let px: Vec<u8> = (0..12u8).flat_map(|i| [i, i, i, 255]).collect();
        let c = crop(&px, 4, rect((3, 2), (1, 0))); // 거꾸로 끌어도 같다
        assert_eq!(c.chunks(4).map(|p| p[0]).collect::<Vec<_>>(), vec![1, 2, 5, 6]);
    }
}
