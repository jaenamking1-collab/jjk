//! 영상 자르기: 앞뒤를 잘라 새 파일로. 다시 굽지 않고 그대로 옮겨 담는다(빠르고 화질 그대로).
//! 시작은 그 앞의 가장 가까운 키프레임에 맞춘다. 소리 줄이 있으면 같이 옮긴다.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use windows::core::{GUID, HSTRING, PROPVARIANT};
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

/// secs 쯤의 한 장면(BGRA, 너비, 높이) — 미리보기용
pub fn frame_at(path: &Path, secs: f64) -> Option<(Vec<u8>, u32, u32)> {
    unsafe {
        let r = reader(path, true).ok()?;
        let vs = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
        let t = MFCreateMediaType().ok()?;
        t.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).ok()?;
        t.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32).ok()?;
        r.SetCurrentMediaType(vs, None, &t).ok()?;
        let cur = r.GetCurrentMediaType(vs).ok()?;
        let fs = cur.GetUINT64(&MF_MT_FRAME_SIZE).ok()?;
        let (w, h) = ((fs >> 32) as u32, (fs & 0xFFFF_FFFF) as u32);
        let stride = cur.GetUINT32(&MF_MT_DEFAULT_STRIDE).map(|s| s as i32).unwrap_or(w as i32 * 4);
        seek(&r, secs);
        let target = (secs * TICK) as i64;
        let mut last: Option<IMFSample> = None;
        for _ in 0..400 {
            let (mut flags, mut ts) = (0u32, 0i64);
            let mut s: Option<IMFSample> = None;
            r.ReadSample(vs, 0, None, Some(&mut flags), Some(&mut ts), Some(&mut s)).ok()?;
            if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                break;
            }
            if let Some(s) = s {
                last = Some(s);
                if ts + 400_000 >= target {
                    break; // 원하는 때에 닿았다
                }
            }
        }
        let buf = last?.ConvertToContiguousBuffer().ok()?;
        let (mut p, mut len) = (std::ptr::null_mut(), 0u32);
        buf.Lock(&mut p, None, Some(&mut len)).ok()?;
        let row = w as usize * 4;
        let src = std::slice::from_raw_parts(p, len as usize);
        let mut px = vec![0u8; row * h as usize];
        for y in 0..h as usize {
            // 위에서 아래로(stride 가 음수면 아래에서 위로 들어 있다)
            let sy = if stride < 0 { h as usize - 1 - y } else { y };
            let s0 = sy * row;
            if s0 + row <= src.len() {
                px[y * row..y * row + row].copy_from_slice(&src[s0..s0 + row]);
            }
        }
        let _ = buf.Unlock();
        for q in px.chunks_exact_mut(4) {
            q[3] = 255;
        }
        Some((px, w, h))
    }
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
        sw.Finalize()?;
        progress.store(1000, Ordering::Relaxed);
        Ok(())
    }
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
