//! 영상 자르기: 앞뒤를 잘라 새 파일로. 처음부터 자르면 다시 굽지 않고 그대로 옮겨 담는다(빠르고 화질 그대로).
//! 앞부분을 자르면 영상만 다시 구워 고른 자리에서 정확히 시작한다. 소리는 어느 쪽이든 그대로 옮긴다.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use windows::core::{Interface, GUID, HSTRING, PROPVARIANT};
use windows::Foundation::TimeSpan;
use windows::Media::Core::MediaSource;
use windows::Media::Playback::{MediaPlaybackState, MediaPlayer};
use windows::Storage::StorageFile;
use windows::Win32::Media::MediaFoundation::*;

pub const WM_TRIMDONE: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 9;
pub static DONE: Mutex<Option<Result<PathBuf, String>>> = Mutex::new(None);

const TICK: f64 = 10_000_000.0; // 1초 = 100나노초 1천만

unsafe fn start_mf() {
    let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED);
    let _ = MFStartup(MF_VERSION, MFSTARTUP_FULL);
}

unsafe fn reader(path: &Path, decode: bool) -> windows::core::Result<IMFSourceReader> {
    start_mf();
    let mut attrs = None;
    MFCreateAttributes(&mut attrs, 1)?;
    let attrs = attrs.ok_or_else(windows::core::Error::empty)?;
    if decode {
        attrs.SetUINT32(&MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING, 1)?;
    }
    MFCreateSourceReaderFromURL(&HSTRING::from(path.as_os_str()), &attrs)
}

/// 영상 길이(초)
pub fn duration(path: &Path) -> Option<f64> {
    unsafe {
        let r = reader(path, false).ok()?;
        let v = r.GetPresentationAttribute(MF_SOURCE_READER_MEDIASOURCE.0 as u32, &MF_PD_DURATION).ok()?;
        let t = u64::try_from(&v).ok()?;
        Some(t as f64 / TICK)
    }
}

unsafe fn seek(r: &IMFSourceReader, secs: f64) {
    let v = PROPVARIANT::from((secs.max(0.0) * TICK) as i64);
    let _ = r.SetCurrentPosition(&GUID::zeroed(), &v);
}

/// 영상 줄을 차례로 읽는다(미리보기·재생·장면 찾기가 같이 쓴다). 장은 NV12(밝기 판 + 색 판)로 받는다.
struct Video {
    r: IMFSourceReader,
    w: usize,  // 보이는 너비·높이
    h: usize,
    x: usize,  // 보이는 부분의 시작
    y: usize,
    cw: usize, // 장에 실제로 담긴 크기(908×622 영상 → 912×624)
    ch: usize,
    stride: usize,
}

const VS: u32 = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;

impl Video {
    unsafe fn open(path: &Path) -> Option<Video> {
        let r = reader(path, true).ok()?;
        let t = MFCreateMediaType().ok()?;
        t.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).ok()?;
        t.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_NV12).ok()?;
        r.SetCurrentMediaType(VS, None, &t).ok()?;
        let mut v = Video { r, w: 0, h: 0, x: 0, y: 0, cw: 0, ch: 0, stride: 0 };
        v.format()?;
        Some(v)
    }

    unsafe fn format(&mut self) -> Option<()> {
        let cur = self.r.GetCurrentMediaType(VS).ok()?;
        let fs = cur.GetUINT64(&MF_MT_FRAME_SIZE).ok()?;
        (self.cw, self.ch) = ((fs >> 32) as usize, (fs & 0xFFFF_FFFF) as usize);
        (self.x, self.y, self.w, self.h) = (0, 0, self.cw, self.ch);
        // 보이는 부분(MFVideoArea: x 소수·정수, y 소수·정수, 너비, 높이) — 없으면 전부
        let mut a = [0u8; 16];
        if cur.GetBlob(&MF_MT_MINIMUM_DISPLAY_APERTURE, &mut a, None).is_ok() {
            let short = |i: usize| i16::from_le_bytes([a[i], a[i + 1]]).max(0) as usize;
            let long = |i: usize| i32::from_le_bytes([a[i], a[i + 1], a[i + 2], a[i + 3]]).max(0) as usize;
            let (x, y, w, h) = (short(2), short(6), long(8), long(12));
            if w > 0 && h > 0 && x + w <= self.cw && y + h <= self.ch {
                (self.x, self.y, self.w, self.h) = (x, y, w, h);
            }
        }
        self.stride = cur.GetUINT32(&MF_MT_DEFAULT_STRIDE).map(|s| (s as i32).unsigned_abs() as usize).unwrap_or(0);
        Some(())
    }

    /// 다음 장(때(초), 장). 끝이거나 못 읽으면 None.
    unsafe fn next(&mut self) -> Option<(f64, IMFSample)> {
        for _ in 0..1000 {
            let (mut flags, mut ts) = (0u32, 0i64);
            let mut s: Option<IMFSample> = None;
            self.r.ReadSample(VS, 0, None, Some(&mut flags), Some(&mut ts), Some(&mut s)).ok()?;
            if flags & MF_SOURCE_READERF_CURRENTMEDIATYPECHANGED.0 as u32 != 0 {
                self.format()?;
            }
            if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                return None;
            }
            if let Some(s) = s {
                return Some((ts as f64 / TICK, s));
            }
        }
        None
    }

    /// 장의 줄을 하나씩 집어 쓰게 해 준다. 한 줄 간격(바이트)은 너비보다 클 수 있다(너비 908 → 912칸 이상).
    /// 이걸 너비로 짐작하면 줄이 조금씩 밀려 빗금처럼 깨진다(2026-11-26 사용자 화면).
    unsafe fn rows<T>(&self, s: &IMFSample, f: impl FnOnce(&dyn Fn(usize) -> *const u8) -> T) -> Option<T> {
        let buf = s.GetBufferByIndex(0).ok()?;
        if let Ok(b2) = buf.cast::<IMF2DBuffer>() {
            let (mut p0, mut pitch) = (std::ptr::null_mut(), 0i32);
            b2.Lock2D(&mut p0, &mut pitch).ok()?;
            let out = f(&|y| p0.offset(y as isize * pitch as isize) as *const u8);
            let _ = b2.Unlock2D();
            return Some(out);
        }
        let (mut p, mut len) = (std::ptr::null_mut(), 0u32);
        buf.Lock(&mut p, None, Some(&mut len)).ok()?;
        let pitch = self.stride.max(self.cw);
        let out = (len as usize >= pitch * self.ch * 3 / 2).then(|| f(&|y| p.add(y * pitch) as *const u8));
        let _ = buf.Unlock();
        out
    }

    /// 장 → BGRA. 밝기를 k×k 점 평균 내 1/k 크기로(색은 가운데 점).
    unsafe fn bgra(&self, s: &IMFSample, k: usize) -> Option<(Vec<u8>, u32, u32)> {
        let k = k.max(1);
        let (w, h) = (self.w / k, self.h / k);
        if w == 0 || h == 0 {
            return None;
        }
        // 영상 신호 → 화면 색(1024 배 정수). 큰 영상은 BT.709, 작은 영상은 BT.601.
        let (rv, gu, gv, bu) = if self.h >= 720 { (1836, 218, 546, 2163) } else { (1634, 401, 832, 2066) };
        let mut px = vec![255u8; w * h * 4];
        self.rows(s, |row| {
            for oy in 0..h {
                let sy = self.y + oy * k;
                let uv = row(self.ch + (sy + k / 2) / 2);
                for ox in 0..w {
                    let sx = self.x + ox * k;
                    let mut sum = 0u32;
                    for j in 0..k {
                        let p = row(sy + j).add(sx);
                        for i in 0..k {
                            sum += *p.add(i) as u32;
                        }
                    }
                    let c = ((sum / (k * k) as u32) as i32 - 16) * 1192;
                    let at = ((sx + k / 2) / 2) * 2;
                    let (u, v) = (*uv.add(at) as i32 - 128, *uv.add(at + 1) as i32 - 128);
                    let o = (oy * w + ox) * 4;
                    px[o] = ((c + bu * u) >> 10).clamp(0, 255) as u8;
                    px[o + 1] = ((c - gu * u - gv * v) >> 10).clamp(0, 255) as u8;
                    px[o + 2] = ((c + rv * v) >> 10).clamp(0, 255) as u8;
                }
            }
        })?;
        Some((px, w as u32, h as u32))
    }

    /// 장 → 화면을 32×18 칸으로 나눈 칸별 밝기
    unsafe fn signature(&self, s: &IMFSample) -> Option<Vec<u8>> {
        self.rows(s, |row| {
            let mut out = Vec::with_capacity(32 * 18);
            for gy in 0..18 {
                for gx in 0..32 {
                    let mut sum = 0u32;
                    for j in 0..4 {
                        let p = row(self.y + (gy * 4 + j) * self.h / 72).add(self.x);
                        for i in 0..4 {
                            sum += *p.add((gx * 4 + i) * self.w / 128) as u32;
                        }
                    }
                    out.push((sum / 16) as u8);
                }
            }
            out
        })
    }
}

/// secs 쯤의 한 장면(BGRA, 너비, 높이) — 미리보기용
pub fn frame_at(path: &Path, secs: f64) -> Option<(Vec<u8>, u32, u32)> {
    unsafe {
        let mut v = Video::open(path)?;
        seek(&v.r, secs);
        let mut last: Option<IMFSample> = None;
        for _ in 0..3000 {
            let Some((t, s)) = v.next() else { break };
            last = Some(s);
            if t + 0.002 >= secs {
                break; // 원하는 때에 닿았다
            }
        }
        v.bgra(&last?, 1)
    }
}

/// 장면이 바뀌는 때. sig = (때, 칸별 밝기)를 0.1초쯤 간격으로.
/// 앞 장과의 차이가 크고 둘레의 평소 움직임보다 확 튀는 곳만 친다(스크롤·흔들림처럼 계속 바뀌는 곳은 아니다).
pub fn cuts(sig: &[(f64, Vec<u8>)]) -> Vec<f64> {
    const MIN: f32 = 8.0; // 칸 밝기 평균 차(0…255)
    let d: Vec<f32> = sig
        .windows(2)
        .map(|p| p[0].1.iter().zip(&p[1].1).map(|(a, b)| a.abs_diff(*b) as f32).sum::<f32>() / p[0].1.len().max(1) as f32)
        .collect();
    let mut found: Vec<(f64, f32)> = Vec::new();
    for i in 0..d.len() {
        if d[i] < MIN {
            continue;
        }
        let mut around: Vec<f32> = (i.saturating_sub(6)..(i + 7).min(d.len())).filter(|j| j.abs_diff(i) > 1).map(|j| d[j]).collect();
        around.sort_by(f32::total_cmp);
        if d[i] < around.get(around.len() / 2).copied().unwrap_or(0.0) * 3.0 {
            continue;
        }
        let t = sig[i + 1].0;
        match found.last_mut() {
            Some(last) if t - last.0 < 1.0 => {
                if d[i] > last.1 {
                    *last = (t, d[i]);
                }
            }
            _ => found.push((t, d[i])),
        }
    }
    found.into_iter().map(|f| f.0).filter(|t| *t > 0.3).collect()
}

/// 표본에서 찾은 때를 장 단위로 맞춘다: 그 앞 0.2초 안에서 크게(가장 큰 변화의 절반 이상) 바뀐 첫 장.
fn exact(found: &[f64], steps: &[(f64, u32)]) -> Vec<f64> {
    found
        .iter()
        .map(|&t| {
            let near = &steps[steps.partition_point(|s| s.0 < t - 0.2)..steps.partition_point(|s| s.0 <= t + 0.001)];
            let top = near.iter().map(|s| s.1).max().unwrap_or(0);
            near.iter().find(|s| s.1 > 0 && s.1 * 2 >= top).map_or(t, |s| s.0)
        })
        .collect()
}

/// 뒤에서 장면 바뀌는 곳 찾기. 버리면(drop) 멈춘다.
pub struct Scenes {
    stop: Arc<AtomicBool>,
    pub out: Arc<Mutex<Found>>,
}

#[derive(Default)]
pub struct Found {
    pub at: Vec<f64>,
    pub upto: f64, // 어디까지 훑었나(초)
    pub done: bool,
}

impl Drop for Scenes {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

pub fn find_scenes(path: &Path) -> Scenes {
    let (stop, out) = (Arc::new(AtomicBool::new(false)), Arc::new(Mutex::new(Found::default())));
    let (st, o, path) = (stop.clone(), out.clone(), path.to_path_buf());
    std::thread::spawn(move || unsafe {
        let mut sig: Vec<(f64, Vec<u8>)> = Vec::new();
        let mut steps: Vec<(f64, u32)> = Vec::new();
        if let Some(mut v) = Video::open(&path) {
            let mut prev: Vec<u8> = Vec::new();
            while !st.load(Ordering::Relaxed) {
                let Some((t, s)) = v.next() else { break };
                let Some(g) = v.signature(&s) else { continue };
                // 바뀐 정도는 장마다 적어 둔다: 점은 '바뀌기 시작한 그 장'에 찍는다
                steps.push((t, prev.iter().zip(&g).map(|(a, b)| a.abs_diff(*b) as u32).sum()));
                prev.clone_from(&g);
                if sig.last().is_some_and(|l| t - l.0 < 0.095) {
                    continue;
                }
                sig.push((t, g));
                if sig.len() % 50 == 0 {
                    let at = exact(&cuts(&sig), &steps);
                    let mut f = o.lock().unwrap();
                    (f.at, f.upto) = (at, t);
                }
            }
        }
        let at = exact(&cuts(&sig), &steps);
        let mut f = o.lock().unwrap();
        (f.at, f.upto, f.done) = (at, sig.last().map_or(0.0, |l| l.0), true);
    });
    Scenes { stop, out }
}

/// 미리보기 재생. 버리면(drop) 멈춘다.
pub struct Player {
    stop: Arc<AtomicBool>,
    pub out: Arc<Mutex<Playing>>,
}

#[derive(Default)]
pub struct Playing {
    pub pos: f64,
    pub frame: Option<(Vec<u8>, u32, u32)>, // 새 장(가져가면 비운다)
    pub done: bool,
    pub silent: bool, // 이 영상엔 소리가 없다(또는 소리를 못 틀었다)
}

impl Drop for Player {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

unsafe fn has_audio(path: &Path) -> bool {
    reader(path, false).ok().is_some_and(|r| r.GetNativeMediaType(MF_SOURCE_READER_FIRST_AUDIO_STREAM.0 as u32, 0).is_ok())
}

/// 윈도우 재생기로 소리만 낸다(화면은 안 붙인다). 버리면 꺼진다.
struct Sound(MediaPlayer);

impl Drop for Sound {
    fn drop(&mut self) {
        let _ = self.0.Pause();
        let _ = self.0.Close();
    }
}

impl Sound {
    fn open(path: &Path) -> windows::core::Result<Sound> {
        let file = StorageFile::GetFileFromPathAsync(&HSTRING::from(path.as_os_str()))?.get()?;
        let p = Sound(MediaPlayer::new()?);
        p.0.CommandManager()?.SetIsEnabled(false)?; // 노치 음악 쪽·미디어 키에 안 잡히게
        p.0.SetSource(&MediaSource::CreateFromStorageFile(&file)?)?;
        Ok(p)
    }

    /// a 초부터 틀고, 실제로 흐르기 시작한 때를 돌려준다(a 초에 해당하는 시각).
    fn start(&self, a: f64, stop: &AtomicBool) -> windows::core::Result<Option<Instant>> {
        let s = self.0.PlaybackSession()?;
        let wait = |ok: &dyn Fn() -> bool| {
            let t0 = Instant::now();
            while !ok() && t0.elapsed() < Duration::from_secs(3) && !stop.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(5));
            }
            ok()
        };
        if !wait(&|| s.NaturalDuration().is_ok_and(|d| d.Duration > 0)) {
            return Ok(None);
        }
        s.SetPosition(TimeSpan { Duration: (a * TICK) as i64 })?;
        self.0.Play()?;
        let at = || s.Position().map_or(0.0, |p| p.Duration as f64 / TICK);
        if !wait(&|| s.PlaybackState().is_ok_and(|x| x == MediaPlaybackState::Playing) && at() > a + 0.005) {
            return Ok(None);
        }
        Ok(Instant::now().checked_sub(Duration::from_secs_f64((at() - a).max(0.0))))
    }
}

/// a~b 초를 실제 빠르기로. 늦으면 장을 건너뛴다.
pub fn play(path: &Path, a: f64, b: f64) -> Player {
    let (stop, out) = (Arc::new(AtomicBool::new(false)), Arc::new(Mutex::new(Playing { pos: a, ..Default::default() })));
    let (st, o, path) = (stop.clone(), out.clone(), path.to_path_buf());
    std::thread::spawn(move || unsafe {
        if let Some(mut v) = Video::open(&path) {
            seek(&v.r, a);
            // 소리가 있으면 소리가 흐르기 시작한 때에 화면을 맞춘다
            let sound = has_audio(&path).then(|| Sound::open(&path).ok()).flatten();
            let mut began: Option<Instant> = sound.as_ref().and_then(|s| s.start(a, &st).ok().flatten());
            o.lock().unwrap().silent = began.is_none();
            let mut shown = -1.0f64;
            while !st.load(Ordering::Relaxed) {
                let Some((t, s)) = v.next() else { break };
                if t > b {
                    break;
                }
                if t + 0.02 < a {
                    continue; // 앞 키프레임부터 읽힌다
                }
                let began = *began.get_or_insert_with(Instant::now);
                let wait = (t - a) - began.elapsed().as_secs_f64();
                if wait < -0.08 || t - shown < 0.025 {
                    continue;
                }
                if wait > 0.0 {
                    std::thread::sleep(Duration::from_secs_f64(wait.min(1.0)));
                }
                // 미리보기 칸(256×144)의 두 배쯤이면 충분하다
                let k = (v.w / 512).max(v.h / 288).max(1);
                if let Some(f) = v.bgra(&s, k) {
                    shown = t;
                    let mut p = o.lock().unwrap();
                    (p.pos, p.frame) = (t, Some(f));
                }
            }
        }
        o.lock().unwrap().done = true;
    });
    Player { stop, out }
}

/// 결과 파일 이름: "이름 (자름).mp4", 있으면 "(자름 2)"…
pub fn out_path(src: &Path) -> PathBuf {
    let stem = src.file_stem().map_or_else(|| "영상".into(), |s| s.to_string_lossy().into_owned());
    let dir = src.parent().map(Path::to_path_buf).unwrap_or_default();
    for k in 1..100 {
        let n = if k == 1 { format!("{stem} (자름).mp4") } else { format!("{stem} (자름 {k}).mp4") };
        let p = dir.join(n);
        if !p.exists() {
            return p;
        }
    }
    dir.join(format!("{stem} (자름).mp4"))
}

/// a~b 초를 잘라 dst 에. progress 는 0…1000.
pub fn cut(src: &Path, dst: &Path, a: f64, b: f64, progress: &AtomicU32) -> windows::core::Result<()> {
    unsafe {
        let r = if a < 0.05 { copy(src, dst, a, b, progress) } else { encode(src, dst, a, b, progress) };
        if r.is_err() {
            let _ = std::fs::remove_file(dst); // 만들다 만 파일은 남기지 않는다
        }
        r.map(|_| progress.store(1000, Ordering::Relaxed))
    }
}

fn failed(what: &str) -> windows::core::Error {
    windows::core::Error::new(windows::core::HRESULT(0x80004005u32 as i32), what)
}

/// 앞부분을 자를 때: 영상은 다시 구워 a 초 장부터 정확히 시작한다. 소리는 그대로 옮긴다.
unsafe fn encode(src: &Path, dst: &Path, a: f64, b: f64, progress: &AtomicU32) -> windows::core::Result<()> {
    let mut v = Video::open(src).ok_or_else(|| failed("영상을 읽지 못했습니다"))?;
    let (mut vi, mut ai) = (None, None);
    for i in 0..8u32 {
        let Ok(t) = v.r.GetNativeMediaType(i, 0) else { break };
        let major = t.GetGUID(&MF_MT_MAJOR_TYPE)?;
        if major == MFMediaType_Video && vi.is_none() {
            vi = Some((i, t));
        } else if major == MFMediaType_Audio && ai.is_none() {
            ai = Some((i, t));
        } else {
            let _ = v.r.SetStreamSelection(i, false);
        }
    }
    let (vi, native) = vi.ok_or_else(|| failed("영상 줄이 없습니다"))?;
    v.r.SetStreamSelection(vi, true)?;
    let (w, h) = (v.w & !1, v.h & !1);
    let rate = native.GetUINT64(&MF_MT_FRAME_RATE).ok().filter(|r| (r >> 32) > 0 && (r & 0xFFFF_FFFF) > 0).unwrap_or((30u64 << 32) | 1);
    let fps = (rate >> 32) as f64 / (rate & 0xFFFF_FFFF) as f64;
    // 화질: 원본 크기/길이의 1.3배와 화면 크기 어림값 중 큰 쪽
    let original = std::fs::metadata(src).map_or(0.0, |m| m.len() as f64 * 8.0 / duration(src).unwrap_or(1.0).max(1.0));
    let bitrate = (original * 1.3).max((w * h) as f64 * fps.min(60.0) * 0.12).clamp(1_000_000.0, 40_000_000.0) as u32;
    let ty = |sub: &GUID| -> windows::core::Result<IMFMediaType> {
        let t = MFCreateMediaType()?;
        t.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
        t.SetGUID(&MF_MT_SUBTYPE, sub)?;
        t.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
        t.SetUINT64(&MF_MT_FRAME_SIZE, ((w as u64) << 32) | h as u64)?;
        t.SetUINT64(&MF_MT_FRAME_RATE, rate)?;
        t.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, (1u64 << 32) | 1)?;
        if let Ok(turn) = native.GetUINT32(&MF_MT_VIDEO_ROTATION) {
            t.SetUINT32(&MF_MT_VIDEO_ROTATION, turn)?; // 휴대폰 세로 영상
        }
        Ok(t)
    };
    let mut attrs = None;
    MFCreateAttributes(&mut attrs, 1)?;
    let attrs = attrs.ok_or_else(windows::core::Error::empty)?;
    attrs.SetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, 1)?;
    let sw = MFCreateSinkWriterFromURL(&HSTRING::from(dst.as_os_str()), None, &attrs)?;
    let out = ty(&MFVideoFormat_H264)?;
    out.SetUINT32(&MF_MT_AVG_BITRATE, bitrate)?;
    let vo = sw.AddStream(&out)?;
    let input = ty(&MFVideoFormat_NV12)?;
    input.SetUINT32(&MF_MT_DEFAULT_STRIDE, w as u32)?;
    sw.SetInputMediaType(vo, &input, None)?;
    let mut audio: Option<(u32, u32)> = None;
    if let Some((i, t)) = ai {
        v.r.SetStreamSelection(i, true)?;
        v.r.SetCurrentMediaType(i, None, &t)?;
        let o = sw.AddStream(&t)?;
        sw.SetInputMediaType(o, &t, None)?;
        audio = Some((i, o));
    }
    seek(&v.r, a);
    sw.BeginWriting()?;
    let (mut video_done, mut audio_done, mut wrote) = (false, audio.is_none(), 0u32);
    while !(video_done && audio_done) {
        let (mut si, mut flags, mut ts) = (0u32, 0u32, 0i64);
        let mut s: Option<IMFSample> = None;
        v.r.ReadSample(MF_SOURCE_READER_ANY_STREAM.0 as u32, 0, Some(&mut si), Some(&mut flags), Some(&mut ts), Some(&mut s))?;
        let is_video = si == vi;
        if !is_video && audio.map_or(true, |x| x.0 != si) {
            continue;
        }
        if is_video && flags & MF_SOURCE_READERF_CURRENTMEDIATYPECHANGED.0 as u32 != 0 {
            v.format().ok_or_else(|| failed("영상 모양이 바뀌었습니다"))?;
        }
        if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
            if is_video { video_done = true } else { audio_done = true }
            continue;
        }
        let Some(s) = s else { continue };
        let t = ts as f64 / TICK;
        if t >= b {
            if is_video { video_done = true } else { audio_done = true }
            continue;
        }
        if t + 0.002 < a || (is_video && video_done) || (!is_video && audio_done) {
            continue; // 앞 키프레임부터 읽힌 부분
        }
        let at = (ts - (a * TICK) as i64).max(0);
        if !is_video {
            s.SetSampleTime(at)?;
            sw.WriteSample(audio.unwrap().1, &s)?;
            continue;
        }
        if v.w < w || v.h < h {
            return Err(failed("영상 크기가 중간에 바뀌었습니다"));
        }
        // 보이는 부분만 빈틈없이 옮겨 담는다
        let size = w * h * 3 / 2;
        let buf = MFCreateMemoryBuffer(size as u32)?;
        let mut p = std::ptr::null_mut();
        buf.Lock(&mut p, None, None)?;
        let copied = v.rows(&s, |row| {
            for y in 0..h {
                std::ptr::copy_nonoverlapping(row(v.y + y).add(v.x), p.add(y * w), w);
            }
            for y in 0..h / 2 {
                std::ptr::copy_nonoverlapping(row(v.ch + v.y / 2 + y).add(v.x & !1), p.add((h + y) * w), w);
            }
        });
        buf.Unlock()?;
        copied.ok_or_else(|| failed("장면을 읽지 못했습니다"))?;
        buf.SetCurrentLength(size as u32)?;
        let frame = MFCreateSample()?;
        frame.AddBuffer(&buf)?;
        frame.SetSampleTime(at)?;
        frame.SetSampleDuration(s.GetSampleDuration().unwrap_or((TICK / fps) as i64))?;
        sw.WriteSample(vo, &frame)?;
        wrote += 1;
        progress.store(((t - a) / (b - a).max(0.001) * 1000.0).clamp(0.0, 999.0) as u32, Ordering::Relaxed);
    }
    if wrote == 0 {
        return Err(failed("고른 구간에 장면이 없습니다"));
    }
    sw.Finalize()
}

/// 처음부터 자를 때: 다시 굽지 않고 그대로 옮긴다.
unsafe fn copy(src: &Path, dst: &Path, a: f64, b: f64, progress: &AtomicU32) -> windows::core::Result<()> {
        let r = reader(src, false)?;
        let sw = MFCreateSinkWriterFromURL(&HSTRING::from(dst.as_os_str()), None, None)?;
        // 영상·소리 줄을 찾아 그대로 옮길 줄로 붙인다
        let mut map: Vec<(u32, u32, bool)> = Vec::new(); // (읽는 줄, 쓰는 줄, 영상인가)
        for i in 0..8u32 {
            let Ok(t) = r.GetNativeMediaType(i, 0) else { break };
            let major = t.GetGUID(&MF_MT_MAJOR_TYPE)?;
            let video = major == MFMediaType_Video;
            if !video && major != MFMediaType_Audio {
                let _ = r.SetStreamSelection(i, false);
                continue;
            }
            r.SetStreamSelection(i, true)?;
            r.SetCurrentMediaType(i, None, &t)?;
            let o = sw.AddStream(&t)?;
            sw.SetInputMediaType(o, &t, None)?;
            map.push((i, o, video));
        }
        if !map.iter().any(|m| m.2) {
            return Err(windows::core::Error::new(windows::core::HRESULT(0x80004005u32 as i32), "영상 줄이 없습니다"));
        }
        seek(&r, a);
        sw.BeginWriting()?;
        let (ta, tb) = ((a * TICK) as i64, (b * TICK) as i64);
        let mut base: Option<i64> = None; // 처음 쓴 영상 키프레임 때 — 여기를 0 으로
        let mut ended = vec![false; map.len()];
        while !ended.iter().all(|e| *e) {
            let (mut si, mut flags, mut ts) = (0u32, 0u32, 0i64);
            let mut s: Option<IMFSample> = None;
            r.ReadSample(MF_SOURCE_READER_ANY_STREAM.0 as u32, 0, Some(&mut si), Some(&mut flags), Some(&mut ts), Some(&mut s))?;
            let Some(k) = map.iter().position(|m| m.0 == si) else { continue };
            if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                ended[k] = true;
                continue;
            }
            let Some(s) = s else { continue };
            if ts > tb {
                ended[k] = true;
                continue;
            }
            let (_, o, video) = map[k];
            if base.is_none() {
                // 영상 키프레임부터 시작(그 전 것은 버린다)
                let key = s.GetUINT32(&MFSampleExtension_CleanPoint).unwrap_or(0) != 0;
                if !video || !key {
                    continue;
                }
                base = Some(ts.min(ta));
            }
            let b0 = base.unwrap_or(0);
            if ts < b0 {
                continue;
            }
            s.SetSampleTime(ts - b0)?;
            sw.WriteSample(o, &s)?;
            if video && tb > ta {
                progress.store((((ts - ta).max(0) as f64 / (tb - ta) as f64) * 1000.0).min(1000.0) as u32, Ordering::Relaxed);
            }
        }
        sw.Finalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // 녹화 시험 파일(rec::records_two_seconds 가 만든 것)을 잘라 본다
    fn cut_and_preview() {
        let src = std::env::temp_dir().join("노치 녹화 시험.mp4");
        let d = duration(&src).expect("길이");
        println!("길이 {d:.2}초");
        let (px, w, h) = frame_at(&src, d / 2.0).expect("장면");
        assert_eq!(px.len(), (w * h * 4) as usize);
        let dst = out_path(&src);
        let p = AtomicU32::new(0);
        cut(&src, &dst, 1.2, d - 0.3, &p).unwrap(); // 1초 키프레임에 맞춰 1.0초부터
        let d2 = duration(&dst).expect("자른 길이");
        println!("자른 길이 {d2:.2}초 → {}", dst.display());
        assert!(d2 > 0.5 && d2 < d, "{d2} / {d}");
        let _ = std::fs::remove_file(dst);
    }
}

#[cfg(test)]
mod scene_tests {
    use super::cuts;

    fn run(levels: &[u8]) -> Vec<f64> {
        cuts(&levels.iter().enumerate().map(|(i, v)| (i as f64 * 0.1, vec![*v; 576])).collect::<Vec<_>>())
    }

    #[test]
    fn one_jump_is_one_scene() {
        let mut v = vec![40u8; 20];
        v.extend([200u8; 20]);
        let c = run(&v);
        assert_eq!(c.len(), 1, "{c:?}");
        assert!((c[0] - 2.0).abs() < 1e-9);
    }

    #[test]
    fn steady_motion_and_noise_are_not_scenes() {
        let moving: Vec<u8> = (0..40).map(|i| if i % 2 == 0 { 60 } else { 90 }).collect();
        assert!(run(&moving).is_empty());
        let noise: Vec<u8> = (0..40).map(|i| 100 + (i % 3) as u8).collect();
        assert!(run(&noise).is_empty());
    }

    #[test]
    fn mark_lands_on_the_changing_frame() {
        let steps = [(1.9, 10), (1.933, 12), (1.967, 900), (2.0, 15), (2.033, 11), (2.067, 9)];
        assert_eq!(super::exact(&[2.067], &steps), vec![1.967]);
        assert_eq!(super::exact(&[5.0], &steps), vec![5.0]);
    }

    #[test]
    fn jumps_within_a_second_are_joined() {
        let mut v = vec![20u8; 20];
        v.extend([120u8; 4]);
        v.extend([250u8; 20]);
        assert_eq!(run(&v).len(), 1);
    }
}

#[cfg(test)]
mod preview {
    use super::*;

    /// 장면 한 장을 파일로: NOTCH_CAT_TESTVID(영상)·NOTCH_CAT_TESTOUT(너비·높이 u32 + BGRA). 다른 도구가 뽑은 장면과 대조한다.
    #[test]
    #[ignore]
    fn dump_frame() {
        let src = PathBuf::from(std::env::var("NOTCH_CAT_TESTVID").unwrap());
        let (px, w, h) = frame_at(&src, 4.0).expect("장면");
        println!("{w}x{h}");
        let mut out = [w.to_le_bytes(), h.to_le_bytes()].concat();
        out.extend(px);
        std::fs::write(std::env::var("NOTCH_CAT_TESTOUT").unwrap(), out).unwrap();
    }

    /// 장면 바뀌는 곳과 재생 빠르기를 실제 영상으로
    #[test]
    #[ignore]
    fn scenes_and_play() {
        let src = PathBuf::from(std::env::var("NOTCH_CAT_TESTVID").unwrap());
        let t0 = Instant::now();
        let s = find_scenes(&src);
        while !s.out.lock().unwrap().done {
            std::thread::sleep(Duration::from_millis(50));
        }
        {
            let f = s.out.lock().unwrap();
            println!("장면 {:?} · {:.1}초까지 · {}ms", f.at.iter().map(|t| (t * 1000.0).round() / 1000.0).collect::<Vec<_>>(), f.upto, t0.elapsed().as_millis());
        }
        let p = play(&src, 2.0, 5.0);
        let (t0, mut n, mut last) = (Instant::now(), 0, 0.0);
        loop {
            let (new, pos, done) = {
                let mut o = p.out.lock().unwrap();
                (o.frame.take().is_some(), o.pos, o.done)
            };
            if new {
                n += 1;
                last = pos;
            }
            if done {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let took = t0.elapsed().as_secs_f64();
        println!("재생 3초 구간: {took:.2}초 걸림 · {n}장 · 마지막 {last:.2}");
        println!("소리 {}", if p.out.lock().unwrap().silent { "없음" } else { "있음" });
        assert!((2.8..4.5).contains(&took) && n > 45 && last > 4.8);
    }

    /// 앞부분을 자르면 고른 장부터 정확히 시작하는지: 길이와 첫 장면을 원본과 대조
    #[test]
    #[ignore]
    fn exact_cut() {
        let src = PathBuf::from(std::env::var("NOTCH_CAT_TESTVID").unwrap());
        let (a, b): (f64, f64) = (std::env::var("NOTCH_CAT_TESTAT").map_or(5.1, |v| v.parse().unwrap()), duration(&src).unwrap() - 2.0);
        let dst = PathBuf::from(std::env::var("NOTCH_CAT_TESTOUT").unwrap());
        let _ = std::fs::remove_file(&dst);
        let t0 = Instant::now();
        cut(&src, &dst, a, b, &AtomicU32::new(0)).expect("자르기");
        let d = duration(&dst).unwrap();
        let diff = |x: &(Vec<u8>, u32, u32), y: &(Vec<u8>, u32, u32)| {
            assert_eq!((x.1, x.2), (y.1, y.2));
            x.0.iter().zip(&y.0).map(|(p, q)| p.abs_diff(*q) as f64).sum::<f64>() / x.0.len() as f64
        };
        let first = frame_at(&dst, 0.0).unwrap();
        let want = diff(&first, &frame_at(&src, a).unwrap());
        let early = diff(&first, &frame_at(&src, a - 0.5).unwrap());
        println!("길이 {d:.3} (원함 {:.3}) · {}x{} · 첫 장 차이 {want:.2} · 0.5초 앞과는 {early:.2} · {}ms", b - a, first.1, first.2, t0.elapsed().as_millis());
        assert!((d - (b - a)).abs() < 0.15 && want < 4.0);
    }
}

#[cfg(test)]
mod keys {
    use super::*;

    #[test]
    #[ignore]
    fn list_keyframes() {
        unsafe {
            let src = std::env::temp_dir().join("노치 녹화 시험.mp4");
            let r = reader(&src, false).unwrap();
            let vs = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
            let mut keys = vec![];
            let mut n = 0;
            loop {
                let (mut flags, mut ts) = (0u32, 0i64);
                let mut s: Option<IMFSample> = None;
                r.ReadSample(vs, 0, None, Some(&mut flags), Some(&mut ts), Some(&mut s)).unwrap();
                if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 { break; }
                if let Some(s) = s {
                    n += 1;
                    if s.GetUINT32(&MFSampleExtension_CleanPoint).unwrap_or(0) != 0 { keys.push(ts as f64 / TICK); }
                }
            }
            println!("장 {n} · 키프레임 {:?}", keys);
        }
    }
}
