//! 클립보드 쪽: 복사한 것(글자·그림)을 최근 10개까지 자동으로, ☆ 로 고정한 것은 10개까지 따로 남긴다.
//! %APPDATA%\notch-cat\clip\ 에 index.json 과 그림(PNG, 미리보기 PNG)으로 저장해 껐다 켜도 남는다.
//! 비밀번호 관리 프로그램이 "기록하지 말라" 표시를 붙인 복사는 받아 적지 않는다.

use serde_json::{json, Value};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use windows::core::{Interface, PCWSTR};
use windows::Win32::Foundation::{HANDLE, HGLOBAL, HWND};
use windows::Win32::Graphics::Imaging::*;
use windows::Win32::System::Com::{CoCreateInstance, IStream, CLSCTX_INPROC_SERVER, STATFLAG_NONAME, STATSTG, STREAM_SEEK_SET};
use windows::Win32::System::DataExchange::*;
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::UI::Shell::SHCreateMemStream;

pub const MAX: usize = 10;
const CF_DIB: u32 = 8;
const CF_UNICODETEXT: u32 = 13;

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub id: u64,              // 받아 적은 때(밀리초) — 그림 파일 이름
    pub text: Option<String>, // 글자면 글자
    pub img: Option<(u32, u32)>, // 그림이면 크기
    pub key: u64,             // 같은 것인지 가리는 값(글자 또는 PNG 바이트)
    pub at: String,           // 받아 적은 시각 "13:25"
}

#[derive(Default)]
pub struct Clip {
    pub recent: Vec<Item>,
    pub pinned: Vec<Item>,
}

pub fn dir() -> PathBuf {
    let d = crate::data_dir().join("clip");
    let _ = std::fs::create_dir_all(&d);
    d
}

pub fn png_path(id: u64) -> PathBuf {
    dir().join(format!("{id}.png"))
}

pub fn thumb_path(id: u64) -> PathBuf {
    dir().join(format!("{id}_t.png"))
}

pub fn key_of(b: &[u8]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    b.hash(&mut h);
    h.finish()
}

fn item_json(i: &Item) -> Value {
    json!({"id": i.id, "text": i.text, "w": i.img.map(|v| v.0), "h": i.img.map(|v| v.1), "key": i.key.to_string(), "at": i.at})
}

fn item_from(v: &Value) -> Option<Item> {
    let img = match (v["w"].as_u64(), v["h"].as_u64()) {
        (Some(w), Some(h)) => Some((w as u32, h as u32)),
        _ => None,
    };
    let text = v["text"].as_str().map(str::to_string);
    if text.is_none() && img.is_none() {
        return None;
    }
    Some(Item { id: v["id"].as_u64()?, text, img, key: v["key"].as_str()?.parse().ok()?, at: v["at"].as_str().unwrap_or("").into() })
}

impl Clip {
    pub fn load() -> Clip {
        let v: Value = std::fs::read_to_string(dir().join("index.json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        let list = |k: &str| v[k].as_array().into_iter().flatten().filter_map(item_from).take(MAX).collect();
        Clip { recent: list("recent"), pinned: list("pinned") }
    }

    pub fn save(&self) {
        let v = json!({"recent": self.recent.iter().map(item_json).collect::<Vec<_>>(), "pinned": self.pinned.iter().map(item_json).collect::<Vec<_>>()});
        let _ = std::fs::write(dir().join("index.json"), v.to_string());
    }

    /// 새로 복사한 것을 맨 앞에. 고정에 이미 있으면 넣지 않고(고정한 걸 꺼내 쓴 것), 최근에 있으면 맨 앞으로 옮긴다.
    /// 넣었으면 true. 밀려난 항목은 돌려준다(그림 파일을 지우라고).
    pub fn add(&mut self, it: Item) -> (bool, Vec<Item>) {
        if self.pinned.iter().any(|p| p.key == it.key) {
            return (false, vec![]);
        }
        if let Some(k) = self.recent.iter().position(|p| p.key == it.key) {
            let old = self.recent.remove(k);
            self.recent.insert(0, old);
            return (true, vec![]);
        }
        self.recent.insert(0, it);
        let gone = if self.recent.len() > MAX { self.recent.split_off(MAX) } else { vec![] };
        (true, gone)
    }

    /// 최근 k 번째를 고정으로. 고정이 꽉 찼으면 false.
    pub fn pin(&mut self, k: usize) -> bool {
        if k >= self.recent.len() || self.pinned.len() >= MAX {
            return false;
        }
        let it = self.recent.remove(k);
        self.pinned.insert(0, it);
        true
    }

    /// 고정 k 번째를 뺀다(없어진 항목을 돌려준다 — 그림 파일을 지우라고)
    pub fn unpin(&mut self, k: usize) -> Option<Item> {
        (k < self.pinned.len()).then(|| self.pinned.remove(k))
    }
}

/// 그림 파일 지우기(밀려나거나 뺀 항목)
pub fn forget(items: &[Item]) {
    for i in items.iter().filter(|i| i.img.is_some()) {
        let _ = std::fs::remove_file(png_path(i.id));
        let _ = std::fs::remove_file(thumb_path(i.id));
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

unsafe fn fmt(name: &str) -> u32 {
    let w = wide(name);
    RegisterClipboardFormatW(PCWSTR(w.as_ptr()))
}

unsafe fn open(hwnd: HWND) -> bool {
    for _ in 0..10 {
        if OpenClipboard(hwnd).is_ok() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(20)); // 다른 프로그램이 잠깐 쥐고 있다
    }
    false
}

unsafe fn global_bytes(h: HANDLE) -> Option<Vec<u8>> {
    let g = HGLOBAL(h.0);
    let n = GlobalSize(g);
    let p = GlobalLock(g) as *const u8;
    if p.is_null() || n == 0 {
        return None;
    }
    let v = std::slice::from_raw_parts(p, n).to_vec();
    let _ = GlobalUnlock(g);
    Some(v)
}

/// 지금 클립보드를 읽는다: (항목, PNG, 미리보기 PNG). 비밀번호 표시가 있거나 글자·그림이 아니면 None.
pub unsafe fn capture(hwnd: HWND, id: u64, at: String) -> Option<(Item, Option<(Vec<u8>, Vec<u8>)>)> {
    if !open(hwnd) {
        return None;
    }
    let out = (|| {
        // 비밀번호 관리 프로그램 등이 '기록하지 말라'고 표시한 것
        if IsClipboardFormatAvailable(fmt("ExcludeClipboardContentFromMonitorProcessing")).is_ok() || IsClipboardFormatAvailable(fmt("Clipboard Viewer Ignore")).is_ok() {
            return None;
        }
        let can = fmt("CanIncludeInClipboardHistory");
        if IsClipboardFormatAvailable(can).is_ok() {
            if let Some(b) = GetClipboardData(can).ok().and_then(|h| global_bytes(h)) {
                if b.len() >= 4 && u32::from_le_bytes([b[0], b[1], b[2], b[3]]) == 0 {
                    return None;
                }
            }
        }
        if IsClipboardFormatAvailable(CF_UNICODETEXT).is_ok() {
            let b = GetClipboardData(CF_UNICODETEXT).ok().and_then(|h| global_bytes(h))?;
            let u: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|c| *c != 0).collect();
            let t = String::from_utf16_lossy(&u);
            if t.trim().is_empty() {
                return None;
            }
            let t: String = t.chars().take(20_000).collect(); // 아주 긴 글은 앞부분만
            return Some((Item { id, key: key_of(t.as_bytes()), text: Some(t), img: None, at }, None));
        }
        if IsClipboardFormatAvailable(CF_DIB).is_ok() {
            let dib = GetClipboardData(CF_DIB).ok().and_then(|h| global_bytes(h))?;
            let (png, thumb, w, h) = dib_to_png(&dib).ok()?;
            return Some((Item { id, key: key_of(&png), text: None, img: Some((w, h)), at }, Some((png, thumb))));
        }
        None
    })();
    let _ = CloseClipboard();
    out
}

fn wic() -> windows::core::Result<IWICImagingFactory> {
    unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER) }
}

unsafe fn encode_png(f: &IWICImagingFactory, src: &IWICBitmapSource, w: u32, h: u32) -> windows::core::Result<Vec<u8>> {
    let out: IStream = SHCreateMemStream(None).ok_or_else(windows::core::Error::empty)?;
    let enc = f.CreateEncoder(&GUID_ContainerFormatPng, std::ptr::null())?;
    enc.Initialize(&out, WICBitmapEncoderNoCache)?;
    let mut fr = None;
    let mut props = None;
    enc.CreateNewFrame(&mut fr, &mut props)?;
    let fr = fr.ok_or_else(windows::core::Error::empty)?;
    fr.Initialize(props.as_ref())?;
    fr.SetSize(w, h)?;
    let mut pf = GUID_WICPixelFormat32bppBGRA;
    fr.SetPixelFormat(&mut pf)?;
    fr.WriteSource(src, std::ptr::null())?;
    fr.Commit()?;
    enc.Commit()?;
    let mut st = STATSTG::default();
    out.Stat(&mut st, STATFLAG_NONAME)?;
    out.Seek(0, STREAM_SEEK_SET, None)?;
    let mut buf = vec![0u8; st.cbSize as usize];
    let mut read = 0u32;
    out.Read(buf.as_mut_ptr() as _, buf.len() as u32, Some(&mut read)).ok()?;
    buf.truncate(read as usize);
    Ok(buf)
}

/// 클립보드 그림(DIB) → (PNG, 미리보기 PNG 160px, 너비, 높이). DIB 앞에 BMP 파일 머리를 붙여 WIC 로 읽는다.
pub unsafe fn dib_to_png(dib: &[u8]) -> windows::core::Result<(Vec<u8>, Vec<u8>, u32, u32)> {
    let rd = |o: usize| u32::from_le_bytes([dib[o], dib[o + 1], dib[o + 2], dib[o + 3]]);
    if dib.len() < 40 {
        return Err(windows::core::Error::empty());
    }
    let (hsz, bits, comp, used) = (rd(0) as usize, u16::from_le_bytes([dib[14], dib[15]]), rd(16), rd(32) as usize);
    let masks = if comp == 3 && hsz == 40 { 12 } else { 0 };
    let pal = if used > 0 { used } else if bits <= 8 { 1 << bits } else { 0 } * 4;
    let off = 14 + hsz + masks + pal;
    let mut bmp = Vec::with_capacity(14 + dib.len());
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&((14 + dib.len()) as u32).to_le_bytes());
    bmp.extend_from_slice(&[0, 0, 0, 0]);
    bmp.extend_from_slice(&(off as u32).to_le_bytes());
    bmp.extend_from_slice(dib);
    let f = wic()?;
    let st = f.CreateStream()?;
    st.InitializeFromMemory(&bmp)?;
    let frame = f.CreateDecoderFromStream(&st, std::ptr::null(), WICDecodeMetadataCacheOnLoad)?.GetFrame(0)?;
    let (mut w, mut h) = (0u32, 0u32);
    frame.GetSize(&mut w, &mut h)?;
    // 32비트 화면 캡처는 투명도 칸이 비어(0) 있는 일이 많다 → 불투명으로 읽는다
    let conv = f.CreateFormatConverter()?;
    conv.Initialize(&frame, &GUID_WICPixelFormat32bppBGR, WICBitmapDitherTypeNone, None, 0.0, WICBitmapPaletteTypeMedianCut)?;
    let src: IWICBitmapSource = conv.cast()?;
    let png = encode_png(&f, &src, w, h)?;
    let k = (160.0 / w.max(h).max(1) as f32).min(1.0);
    let (tw, th) = (((w as f32 * k) as u32).max(1), ((h as f32 * k) as u32).max(1));
    let sc = f.CreateBitmapScaler()?;
    sc.Initialize(&src, tw, th, WICBitmapInterpolationModeFant)?;
    let thumb = encode_png(&f, &sc.cast()?, tw, th)?;
    Ok((png, thumb, w, h))
}

/// PNG → 클립보드용 DIB(32비트, 아래에서 위로)
unsafe fn png_to_dib(png: &[u8]) -> windows::core::Result<Vec<u8>> {
    let f = wic()?;
    let st = f.CreateStream()?;
    st.InitializeFromMemory(png)?;
    let frame = f.CreateDecoderFromStream(&st, std::ptr::null(), WICDecodeMetadataCacheOnLoad)?.GetFrame(0)?;
    let (mut w, mut h) = (0u32, 0u32);
    frame.GetSize(&mut w, &mut h)?;
    let conv = f.CreateFormatConverter()?;
    conv.Initialize(&frame, &GUID_WICPixelFormat32bppBGRA, WICBitmapDitherTypeNone, None, 0.0, WICBitmapPaletteTypeMedianCut)?;
    let stride = w as usize * 4;
    let mut px = vec![0u8; stride * h as usize];
    conv.CopyPixels(std::ptr::null(), stride as u32, &mut px)?;
    let mut dib = Vec::with_capacity(40 + px.len());
    for v in [40u32, w, h] {
        dib.extend_from_slice(&v.to_le_bytes());
    }
    dib.extend_from_slice(&1u16.to_le_bytes());
    dib.extend_from_slice(&32u16.to_le_bytes());
    for v in [0u32, px.len() as u32, 0, 0, 0, 0] {
        dib.extend_from_slice(&v.to_le_bytes());
    }
    for row in px.chunks_exact(stride).rev() {
        dib.extend_from_slice(row);
    }
    Ok(dib)
}

unsafe fn put(format: u32, bytes: &[u8]) -> bool {
    let Ok(g) = GlobalAlloc(GMEM_MOVEABLE, bytes.len()) else { return false };
    let p = GlobalLock(g) as *mut u8;
    if p.is_null() {
        return false;
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
    let _ = GlobalUnlock(g);
    SetClipboardData(format, HANDLE(g.0)).is_ok()
}

/// 항목을 클립보드에 다시 넣는다(붙여 넣기는 Ctrl+V)
pub unsafe fn restore(hwnd: HWND, it: &Item) -> bool {
    let data: Vec<(u32, Vec<u8>)> = if let Some(t) = &it.text {
        let w: Vec<u8> = t.encode_utf16().chain([0]).flat_map(|c| c.to_le_bytes()).collect();
        vec![(CF_UNICODETEXT, w)]
    } else {
        let Ok(png) = std::fs::read(png_path(it.id)) else { return false };
        let Ok(dib) = png_to_dib(&png) else { return false };
        vec![(CF_DIB, dib), (fmt("PNG"), png)]
    };
    if !open(hwnd) {
        return false;
    }
    let _ = EmptyClipboard();
    let ok = data.iter().all(|(f, b)| put(*f, b));
    let _ = CloseClipboard();
    ok
}

/// 복사가 바뀔 때마다 창에 WM_CLIPBOARDUPDATE 가 오게 한다
pub unsafe fn listen(hwnd: HWND) {
    let _ = AddClipboardFormatListener(hwnd);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(id: u64, s: &str) -> Item {
        Item { id, text: Some(s.into()), img: None, key: key_of(s.as_bytes()), at: String::new() }
    }

    #[test]
    fn recent_keeps_ten_newest_first() {
        let mut c = Clip::default();
        for i in 0..12 {
            c.add(t(i, &format!("글 {i}")));
        }
        assert_eq!(c.recent.len(), MAX);
        assert_eq!(c.recent[0].text.as_deref(), Some("글 11"));
        assert_eq!(c.recent[9].text.as_deref(), Some("글 2"));
        // 같은 것을 다시 복사하면 늘지 않고 맨 앞으로
        let (added, gone) = c.add(t(99, "글 5"));
        assert!(added && gone.is_empty());
        assert_eq!(c.recent.len(), MAX);
        assert_eq!(c.recent[0].id, 5);
    }

    #[test]
    fn pinned_is_separate() {
        let mut c = Clip::default();
        c.add(t(1, "주소"));
        c.add(t(2, "계좌 안내"));
        assert!(c.pin(1)); // '주소' 고정
        assert_eq!(c.pinned[0].text.as_deref(), Some("주소"));
        assert_eq!(c.recent.len(), 1);
        // 고정한 것을 꺼내 쓰면(다시 복사되면) 최근에 또 들어가지 않는다
        assert!(!c.add(t(3, "주소")).0);
        for i in 0..20 {
            c.add(t(10 + i, &format!("새 글 {i}")));
            c.pin(0);
        }
        assert_eq!(c.pinned.len(), MAX); // 고정도 10개까지
        assert!(c.unpin(0).is_some());
        assert_eq!(c.pinned.len(), MAX - 1);
    }

    #[test]
    fn index_round_trip() {
        let mut c = Clip::default();
        c.add(t(1, "가나다"));
        c.add(Item { id: 2, text: None, img: Some((1920, 1080)), key: 77, at: "13:25".into() });
        c.pin(1);
        let v = json!({"recent": c.recent.iter().map(item_json).collect::<Vec<_>>(), "pinned": c.pinned.iter().map(item_json).collect::<Vec<_>>()});
        let back: Vec<Item> = v["recent"].as_array().unwrap().iter().filter_map(item_from).collect();
        assert_eq!(back, c.recent);
        let back: Vec<Item> = v["pinned"].as_array().unwrap().iter().filter_map(item_from).collect();
        assert_eq!(back, c.pinned);
    }

    #[test]
    #[ignore] // WIC 로 그림이 PNG 로 바뀌고 다시 DIB 로 돌아오는지(그림 그리는 장치가 있는 PC 에서)
    fn dib_png_round_trip() {
        unsafe {
            let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_APARTMENTTHREADED);
            // 2×2 32비트 DIB: 빨강·초록·파랑·흰색, 투명도 0(화면 캡처처럼)
            let mut dib = Vec::new();
            for v in [40u32, 2, 2] {
                dib.extend_from_slice(&v.to_le_bytes());
            }
            dib.extend_from_slice(&1u16.to_le_bytes());
            dib.extend_from_slice(&32u16.to_le_bytes());
            for v in [0u32, 16, 0, 0, 0, 0] {
                dib.extend_from_slice(&v.to_le_bytes());
            }
            dib.extend_from_slice(&[0, 0, 255, 0, 0, 255, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0]);
            let (png, thumb, w, h) = dib_to_png(&dib).unwrap();
            assert_eq!((w, h), (2, 2));
            assert!(png.starts_with(&[0x89, b'P', b'N', b'G']) && thumb.starts_with(&[0x89, b'P', b'N', b'G']));
            let back = png_to_dib(&png).unwrap();
            assert_eq!(&back[40..44], &[0, 0, 255, 255]); // 맨 아랫줄 첫 칸 = 빨강, 불투명
        }
    }
}
