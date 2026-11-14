//! 화면 녹화: 가운데가 뚫린 틀 창(가장자리를 끌어 크기, 아래 줄을 끌어 옮김, '시작'·'멈춤'·닫기, Esc 취소).
//! 틀 안쪽을 초당 30장 받아 H.264 MP4 로 저장한다(Media Foundation — 설치 없이). 소리는 넣지 않는다.
//! 저장 위치: 동영상\NotchCat\녹화 YYYY-MM-DD HHMMSS.mp4. 틀·노치는 녹화에 찍히지 않게 뺀다.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use windows::core::{w, HSTRING};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::WindowsAndMessaging::*;

pub const WM_RECSAVING: u32 = WM_APP + 7; // 노치에게: 멈췄다, 저장 중
pub const WM_RECDONE: u32 = WM_APP + 8; // 노치에게: 저장 끝(또는 취소)
pub static DONE: Mutex<Option<Result<PathBuf, String>>> = Mutex::new(None);

const B: i32 = 6; // 틀 두께(여기를 끌면 크기가 바뀐다)
const BAR: i32 = 44; // 아래 단추 줄

struct St {
    notch: isize,
    stop: Arc<AtomicBool>,
    frames: Arc<AtomicU32>,
    start: Option<Instant>,
    btns: Vec<(RECT, u8)>, // 1 시작/멈춤 2 닫기
}

static ST: Mutex<Option<St>> = Mutex::new(None);

pub fn videos() -> PathBuf {
    unsafe {
        use windows::Win32::UI::Shell::{SHGetKnownFolderPath, FOLDERID_Videos, KNOWN_FOLDER_FLAG};
        if let Ok(p) = SHGetKnownFolderPath(&FOLDERID_Videos, KNOWN_FOLDER_FLAG(0), None) {
            let s = p.to_string().unwrap_or_default();
            windows::Win32::System::Com::CoTaskMemFree(Some(p.0 as _));
            return PathBuf::from(s).join("NotchCat");
        }
    }
    std::env::temp_dir().join("NotchCat")
}

/// 녹화할 안쪽(화면 좌표). H.264 는 짝수 크기여야 한다.
unsafe fn inner(hw: HWND) -> RECT {
    let mut r = RECT::default();
    let _ = GetWindowRect(hw, &mut r);
    let (l, t) = (r.left + B, r.top + B);
    let w = ((r.right - B) - l) & !1;
    let h = ((r.bottom - BAR) - t) & !1;
    RECT { left: l, top: t, right: l + w.max(2), bottom: t + h.max(2) }
}

/// 틀 창 모양: 바깥에서 안쪽을 뚫는다
unsafe fn shape(hw: HWND) {
    let mut r = RECT::default();
    let _ = GetWindowRect(hw, &mut r);
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    let outer = CreateRectRgn(0, 0, w, h);
    let hole = CreateRectRgn(B, B, w - B, h - BAR);
    let _ = CombineRgn(outer, outer, hole, RGN_DIFF);
    let _ = DeleteObject(hole);
    SetWindowRgn(hw, outer, true);
}

/// 틀을 띄운다(마우스가 있는 모니터 가운데, 화면의 60%)
pub unsafe fn open(notch: HWND) {
    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);
    let mon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
    let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    let _ = GetMonitorInfoW(mon, &mut mi);
    let r = mi.rcWork;
    let (mw, mh) = (r.right - r.left, r.bottom - r.top);
    let (w, h) = (mw * 6 / 10, mh * 6 / 10);
    *ST.lock().unwrap() = Some(St { notch: notch.0 as isize, stop: Arc::new(AtomicBool::new(false)), frames: Arc::new(AtomicU32::new(0)), start: None, btns: vec![] });
    let inst = GetModuleHandleW(None).unwrap_or_default();
    let class = w!("NotchCatRec");
    RegisterClassW(&WNDCLASSW { lpfnWndProc: Some(proc), hInstance: inst.into(), hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(), lpszClassName: class, ..Default::default() });
    if let Ok(hw) = CreateWindowExW(WS_EX_TOPMOST | WS_EX_TOOLWINDOW, class, w!("화면 녹화"), WS_POPUP, r.left + (mw - w) / 2, r.top + (mh - h) / 2, w + 2 * B, h + B + BAR, None, None, inst, None) {
        if std::env::var("NOTCH_CAT_RECSHOW").is_err() {
            let _ = SetWindowDisplayAffinity(hw, WDA_EXCLUDEFROMCAPTURE); // 틀은 녹화에 안 찍힌다(시험할 땐 NOTCH_CAT_RECSHOW 로 보이게)
        }
        shape(hw);
        let _ = ShowWindow(hw, SW_SHOW);
        let _ = SetForegroundWindow(hw);
        let _ = SetFocus(hw);
        SetTimer(hw, 1, 500, None);
    }
}

fn file_name() -> String {
    format!("녹화 {}.mp4", crate::editor::stamp())
}

unsafe fn paint(hw: HWND) {
    let mut ps = PAINTSTRUCT::default();
    let dc = BeginPaint(hw, &mut ps);
    let mut r = RECT::default();
    let _ = GetClientRect(hw, &mut r);
    let mut g = ST.lock().unwrap();
    if let Some(st) = g.as_mut() {
        let rec = st.start.is_some();
        let edge = CreateSolidBrush(COLORREF(if rec { 0x4444EF } else { 0x35E6A3 })); // 녹화 중 빨강, 아니면 라임(BGR)
        let _ = FillRect(dc, &r, edge);
        let _ = DeleteObject(edge);
        let bar = RECT { left: B, top: r.bottom - BAR + 4, right: r.right - B, bottom: r.bottom - B };
        let dark = CreateSolidBrush(COLORREF(0x161616));
        let _ = FillRect(dc, &bar, dark);
        let _ = DeleteObject(dark);
        SetBkMode(dc, TRANSPARENT);
        let font = CreateFontW(-16, 0, 0, 0, 600, 0, 0, 0, 1, 0, 0, 5, 0, w!("Malgun Gothic"));
        let old = SelectObject(dc, font);
        let ir = inner(hw);
        st.btns.clear();
        let mut x = bar.left + 8;
        let mut btn = |dc: HDC, label: &str, wd: i32, col: u32, id: u8, btns: &mut Vec<(RECT, u8)>| {
            let b = RECT { left: x, top: bar.top + 6, right: x + wd, bottom: bar.bottom - 6 };
            let br = CreateSolidBrush(COLORREF(col));
            let _ = FillRect(dc, &b, br);
            let _ = DeleteObject(br);
            SetTextColor(dc, COLORREF(0xFFFFFF));
            let mut t: Vec<u16> = label.encode_utf16().collect();
            let mut bb = b;
            DrawTextW(dc, &mut t, &mut bb, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
            btns.push((b, id));
            x += wd + 8;
        };
        let mut btns = std::mem::take(&mut st.btns);
        if rec {
            btn(dc, "■ 멈추고 저장", 130, 0x2222C8, 1, &mut btns);
        } else {
            btn(dc, "● 녹화 시작", 120, 0x2222C8, 1, &mut btns);
        }
        btn(dc, "✕ 닫기", 80, 0x3A3A3A, 2, &mut btns);
        st.btns = btns;
        let info = match st.start {
            Some(t) => {
                let s = t.elapsed().as_secs();
                format!("녹화 중 {}:{:02} · {}×{}", s / 60, s % 60, ir.right - ir.left, ir.bottom - ir.top)
            }
            None => format!("{}×{} · 가장자리를 끌어 크기, 이 줄을 끌어 옮기기 · Esc 취소", ir.right - ir.left, ir.bottom - ir.top),
        };
        SetTextColor(dc, COLORREF(0xF3F4F6));
        let mut t: Vec<u16> = info.encode_utf16().collect();
        let mut tr = RECT { left: x + 8, top: bar.top, right: bar.right - 8, bottom: bar.bottom };
        DrawTextW(dc, &mut t, &mut tr, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS);
        SelectObject(dc, old);
        let _ = DeleteObject(font);
    }
    drop(g);
    let _ = EndPaint(hw, &ps);
}

/// 녹화 시작: 일하는 줄을 따로 띄운다
unsafe fn start(hw: HWND) {
    let ir = inner(hw);
    let mut g = ST.lock().unwrap();
    let Some(st) = g.as_mut() else { return };
    if st.start.is_some() {
        return;
    }
    st.start = Some(Instant::now());
    let (stop, frames, notch) = (st.stop.clone(), st.frames.clone(), st.notch);
    drop(g);
    let dir = videos();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(file_name());
    std::thread::spawn(move || {
        let r = unsafe { record(ir, &path, stop, &frames) };
        *DONE.lock().unwrap() = Some(r.map(|_| path).map_err(|e| e.message().to_string()));
        unsafe {
            let _ = PostMessageW(HWND(notch as _), WM_RECDONE, WPARAM(0), LPARAM(0));
        }
    });
    let _ = InvalidateRect(hw, None, true);
}

/// 멈추기(녹화 중이면 저장, 아니면 그냥 닫기)
unsafe fn close(hw: HWND) {
    let st = ST.lock().unwrap().take();
    let _ = DestroyWindow(hw);
    let Some(st) = st else { return };
    st.stop.store(true, Ordering::Relaxed);
    let msg = if st.start.is_some() { WM_RECSAVING } else { WM_RECDONE };
    let _ = PostMessageW(HWND(st.notch as _), msg, WPARAM(0), LPARAM(0));
}

unsafe extern "system" fn proc(hw: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            paint(hw);
            return LRESULT(0);
        }
        WM_NCHITTEST => {
            let (x, y) = ((lp.0 & 0xFFFF) as i16 as i32, ((lp.0 >> 16) & 0xFFFF) as i16 as i32);
            let mut r = RECT::default();
            let _ = GetWindowRect(hw, &mut r);
            let rec = ST.lock().unwrap().as_ref().is_some_and(|s| s.start.is_some());
            let (cx, cy) = (x - r.left, y - r.top);
            let on_btn = ST.lock().unwrap().as_ref().is_some_and(|s| s.btns.iter().any(|(b, _)| cx >= b.left && cx < b.right && cy >= b.top && cy < b.bottom));
            if rec || on_btn {
                return LRESULT(HTCLIENT as isize); // 녹화 중엔 크기·자리를 못 바꾼다
            }
            let k = B + 4;
            let (l, t, rr, bb) = (x < r.left + k, y < r.top + k, x >= r.right - k, y >= r.bottom - k);
            let in_bar = y >= r.bottom - BAR && !bb;
            let ht = match (l, t, rr, bb) {
                (true, true, _, _) => HTTOPLEFT,
                (_, true, true, _) => HTTOPRIGHT,
                (true, _, _, true) => HTBOTTOMLEFT,
                (_, _, true, true) => HTBOTTOMRIGHT,
                (true, ..) => HTLEFT,
                (_, true, ..) => HTTOP,
                (_, _, true, _) => HTRIGHT,
                (.., true) => HTBOTTOM,
                _ if in_bar => HTCAPTION,
                _ => HTCLIENT,
            };
            return LRESULT(ht as isize);
        }
        WM_SIZE => {
            shape(hw);
            let _ = InvalidateRect(hw, None, true);
        }
        WM_LBUTTONUP => {
            let (x, y) = ((lp.0 & 0xFFFF) as i16 as i32, ((lp.0 >> 16) & 0xFFFF) as i16 as i32);
            let id = ST.lock().unwrap().as_ref().and_then(|s| s.btns.iter().find(|(b, _)| x >= b.left && x < b.right && y >= b.top && y < b.bottom).map(|(_, i)| *i));
            let rec = ST.lock().unwrap().as_ref().is_some_and(|s| s.start.is_some());
            match id {
                Some(1) if !rec => start(hw),
                Some(1) | Some(2) => close(hw),
                _ => {}
            }
        }
        WM_KEYDOWN if wp.0 == 0x1B => close(hw),
        WM_TIMER => {
            let _ = InvalidateRect(hw, None, false);
        }
        _ => {}
    }
    DefWindowProcW(hw, msg, wp, lp)
}

/// BGRA → NV12(밝기 한 장 + 색 반 크기, BT.601 좁은 범위)
pub fn bgra_to_nv12(px: &[u8], w: usize, h: usize) -> Vec<u8> {
    let mut out = vec![0u8; w * h * 3 / 2];
    let (y_plane, uv) = out.split_at_mut(w * h);
    for (i, p) in px.chunks_exact(4).enumerate() {
        let (b, g, r) = (p[0] as i32, p[1] as i32, p[2] as i32);
        y_plane[i] = (((66 * r + 129 * g + 25 * b + 128) >> 8) + 16) as u8;
    }
    for y in (0..h).step_by(2) {
        for x in (0..w).step_by(2) {
            let (mut r, mut g, mut b) = (0, 0, 0);
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let o = ((y + dy) * w + x + dx) * 4;
                b += px[o] as i32;
                g += px[o + 1] as i32;
                r += px[o + 2] as i32;
            }
            let (r, g, b) = (r / 4, g / 4, b / 4);
            let o = (y / 2) * w + x;
            uv[o] = (((-38 * r - 74 * g + 112 * b + 128) >> 8) + 128).clamp(0, 255) as u8;
            uv[o + 1] = (((112 * r - 94 * g - 18 * b + 128) >> 8) + 128).clamp(0, 255) as u8;
        }
    }
    out
}

/// 녹화하는 동안 계속 쓰는 화면 받기 준비물(장마다 새로 만들면 느리다)
struct Grab {
    scr: HDC,
    mem: HDC,
    bmp: HBITMAP,
    old: HGDIOBJ,
    bits: *mut u8,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl Grab {
    unsafe fn new(x: i32, y: i32, w: i32, h: i32) -> Grab {
        let scr = GetDC(None);
        let mem = CreateCompatibleDC(scr);
        let bi = BITMAPINFO { bmiHeader: BITMAPINFOHEADER { biSize: 40, biWidth: w, biHeight: -h, biPlanes: 1, biBitCount: 32, ..Default::default() }, ..Default::default() };
        let mut bits = std::ptr::null_mut();
        let bmp = CreateDIBSection(mem, &bi, DIB_RGB_COLORS, &mut bits, None, 0).unwrap_or_default();
        let old = SelectObject(mem, bmp);
        Grab { scr, mem, bmp, old, bits: bits as *mut u8, x, y, w, h }
    }

    unsafe fn take(&self) -> Vec<u8> {
        let _ = BitBlt(self.mem, 0, 0, self.w, self.h, self.scr, self.x, self.y, SRCCOPY);
        let _ = GdiFlush();
        std::slice::from_raw_parts(self.bits, (self.w * self.h * 4) as usize).to_vec()
    }
}

impl Drop for Grab {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.mem, self.old);
            let _ = DeleteObject(self.bmp);
            let _ = DeleteDC(self.mem);
            ReleaseDC(None, self.scr);
        }
    }
}

/// 이 사각형(화면 좌표)을 stop 이 켜질 때까지 MP4 로 녹화한다
pub unsafe fn record(r: RECT, path: &std::path::Path, stop: Arc<AtomicBool>, frames: &AtomicU32) -> windows::core::Result<()> {
    let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED);
    MFStartup(MF_VERSION, MFSTARTUP_FULL)?;
    let (w, h) = ((r.right - r.left) as u32, (r.bottom - r.top) as u32);
    let fps = 30u32;
    let mut attrs = None;
    MFCreateAttributes(&mut attrs, 3)?;
    let attrs = attrs.ok_or_else(windows::core::Error::empty)?;
    attrs.SetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, 1)?;
    attrs.SetUINT32(&MF_SINK_WRITER_DISABLE_THROTTLING, 1)?; // 실시간으로 들어오니 속도 맞추기 끔
    // (MF_LOW_LATENCY 는 쓰지 않는다)
    let sw = MFCreateSinkWriterFromURL(&HSTRING::from(path.as_os_str()), None, &attrs)?;
    let ty = |sub: &windows::core::GUID| -> windows::core::Result<IMFMediaType> {
        let t = MFCreateMediaType()?;
        t.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
        t.SetGUID(&MF_MT_SUBTYPE, sub)?;
        t.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
        t.SetUINT64(&MF_MT_FRAME_SIZE, ((w as u64) << 32) | h as u64)?;
        t.SetUINT64(&MF_MT_FRAME_RATE, ((fps as u64) << 32) | 1)?;
        t.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, (1u64 << 32) | 1)?;
        Ok(t)
    };
    let out = ty(&MFVideoFormat_H264)?;
    out.SetUINT32(&MF_MT_AVG_BITRATE, ((w * h) as f32 * fps as f32 * 0.12).clamp(1_000_000.0, 24_000_000.0) as u32)?;
    out.SetUINT32(&MF_MT_MAX_KEYFRAME_SPACING, fps)?; // 1초마다 키프레임(자르기가 1초 단위로 맞게)
    let si = sw.AddStream(&out)?;
    let inp = ty(&MFVideoFormat_NV12)?; // 인코더가 바로 먹는 형식(색 바꾸기는 여기서 직접 — 훨씬 빠르다)
    inp.SetUINT32(&MF_MT_DEFAULT_STRIDE, w)?;
    sw.SetInputMediaType(si, &inp, None)?;
    let mut codec: Option<ICodecAPI> = None;
    // 1초마다 키프레임: 인코더에 직접(자르기가 1초 단위로 맞게). 안 되면 그냥 넘어간다.
    {
        use windows::core::Interface;
        let mut p: *mut core::ffi::c_void = std::ptr::null_mut();
        if sw.GetServiceForStream(si, &windows::core::GUID::zeroed(), &ICodecAPI::IID, &mut p).is_ok() && !p.is_null() {
            let api = ICodecAPI::from_raw(p);
            let _ = api.SetValue(&CODECAPI_AVEncMPVGOPSize, &windows::core::VARIANT::from(fps));
            codec = Some(api);
        }
    }
    sw.BeginWriting()?;
    let t0 = Instant::now();
    let step = Duration::from_micros(1_000_000 / fps as u64);
    // 화면 받기는 따로 일하는 줄에서(굽는 동안 기다리지 않게), 여기서는 받은 장을 굽기만 한다
    let (tx, rx) = std::sync::mpsc::sync_channel::<(Vec<u8>, Duration)>(4);
    let stop_flag = stop.clone();
    let grabber = std::thread::spawn(move || {
        let mut n: u32 = 0;
        let g = unsafe { Grab::new(r.left, r.top, w as i32, h as i32) };
        unsafe { windows::Win32::Media::timeBeginPeriod(1) }; // 쉬기 단위를 1ms 로(기본 15.6ms 면 초당 20장밖에 안 나온다)
        while !stop_flag.load(Ordering::Relaxed) {
            // 화면 받기는 모니터 새로 고침에 맞춰 기다린다 → 쉬지 말고 받다가 1/30초가 된 장만 보낸다
            let due = step * n;
            if let Some(d) = due.checked_sub(t0.elapsed()).filter(|d| *d > Duration::from_millis(20)) {
                std::thread::sleep(d - Duration::from_millis(18));
            }
            let px = unsafe { g.take() };
            let at = t0.elapsed();
            if at + Duration::from_millis(4) < due {
                continue; // 아직 이르다
            }
            if tx.send((px, at)).is_err() {
                break;
            }
            n = (n + 1).max((at.as_micros() / step.as_micros()) as u32 + 1); // 다음 순번(늦었으면 건너뛴 만큼 따라잡는다)
        }
    });
    let mut n: u32 = 0;
    for (px, at) in rx {
        let px = bgra_to_nv12(&px, w as usize, h as usize);
        let buf = MFCreateMemoryBuffer(px.len() as u32)?;
        let mut p = std::ptr::null_mut();
        buf.Lock(&mut p, None, None)?;
        std::ptr::copy_nonoverlapping(px.as_ptr(), p, px.len());
        buf.Unlock()?;
        buf.SetCurrentLength(px.len() as u32)?;
        let s = MFCreateSample()?;
        s.AddBuffer(&buf)?;
        s.SetSampleTime((at.as_nanos() / 100) as i64)?;
        s.SetSampleDuration(10_000_000 / fps as i64)?;
        if n % fps == 0 {
            if let Some(api) = &codec {
                let _ = api.SetValue(&CODECAPI_AVEncVideoForceKeyFrame, &windows::core::VARIANT::from(1u32)); // 이번 장은 키프레임
            }
        }
        sw.WriteSample(si, &s)?;
        n += 1;
        frames.store(n, Ordering::Relaxed);
    }
    let _ = grabber.join();
    windows::Win32::Media::timeEndPeriod(1);
    sw.Finalize()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // 화면 한 귀퉁이를 2초 녹화해 MP4 가 생기는지(그림 장치·인코더가 있는 PC 에서)
    fn records_two_seconds() {
        let path = std::env::temp_dir().join("노치 녹화 시험.mp4");
        let stop = Arc::new(AtomicBool::new(false));
        let frames = Arc::new(AtomicU32::new(0));
        let (s2, f2, p2) = (stop.clone(), frames.clone(), path.clone());
        let t = std::thread::spawn(move || unsafe { record(RECT { left: 0, top: 0, right: 640, bottom: 360 }, &p2, s2, &f2) });
        std::thread::sleep(Duration::from_secs(3)); // 준비(인코더 켜기)에 0.6초쯤 걸린다
        stop.store(true, Ordering::Relaxed);
        t.join().unwrap().unwrap();
        let b = std::fs::read(&path).unwrap();
        println!("장 수 {} · 파일 {} 바이트", frames.load(Ordering::Relaxed), b.len());
        assert!(frames.load(Ordering::Relaxed) > 30);
        assert_eq!(&b[4..8], b"ftyp");
        println!("영상 길이 {:?}초", crate::trim::duration(&path));
    }
}

#[cfg(test)]
mod speed {
    #[test]
    #[ignore]
    fn grab_speed() {
        unsafe {
            let g = super::Grab::new(0, 0, 640, 360);
            let t = std::time::Instant::now();
            for _ in 0..30 {
                let _ = g.take();
            }
            println!("한 장 {:?}", t.elapsed() / 30);
        }
    }
}
