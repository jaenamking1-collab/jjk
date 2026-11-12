// 소리 크기: 지금 스피커로 나가는 소리의 순간 크기(0…1). 와이드 음악 화면의 소리 막대가 쓴다.
use std::time::{Duration, Instant};

use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
use windows::Win32::Media::Audio::{eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

#[derive(Default)]
pub struct Meter {
    dev: Option<IAudioMeterInformation>,
    failed: Option<Instant>, // 못 열었으면 잠시 쉬었다 다시(스피커를 바꿨을 때도)
}

impl Meter {
    pub fn peak(&mut self) -> Option<f32> {
        if self.dev.is_none() && self.failed.map_or(true, |t| t.elapsed() > Duration::from_secs(5)) {
            self.dev = unsafe {
                CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL)
                    .and_then(|e| e.GetDefaultAudioEndpoint(eRender, eMultimedia))
                    .and_then(|d| d.Activate::<IAudioMeterInformation>(CLSCTX_ALL, None))
                    .ok()
            };
            if self.dev.is_none() {
                self.failed = Some(Instant::now());
            }
        }
        let v = unsafe { self.dev.as_ref()?.GetPeakValue() };
        if v.is_err() {
            self.dev = None; // 스피커가 바뀌었으면 다음에 다시 연다
            self.failed = Some(Instant::now());
        }
        v.ok()
    }
}
