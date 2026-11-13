// 지금 재생 중인 곡(GSMTC). 0.5초마다 읽고, 재생/멈춤/넘기기/탐색 명령을 받는다.
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use windows::core::Result;
use windows::Foundation::TimeSpan;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionMediaProperties as Props,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};
use windows::Storage::Streams::DataReader;

use crate::lyrics::{self, Lyr};

pub enum Cmd {
    Toggle,
    Next,
    Prev,
    Seek(i64),
    Shuffle,
    Repeat,
}

#[derive(Default, Clone)]
pub struct Now {
    pub has: bool,
    pub title: String,
    pub artist: String,
    pub playing: bool,
    pub pos_ms: i64,
    pub pos_at: Option<Instant>,
    pub dur_ms: i64,
    pub shuffle: Option<bool>, // 재생기가 알려 줄 때만(모르면 단추를 안 그린다)
    pub repeat: Option<i32>,   // 0 끔, 1 한 곡, 2 전체
    pub art: Option<Arc<Vec<u8>>>,
    pub seen: Option<Instant>, // 마지막으로 읽기에 성공한 때(생존 표시)
    pub stamp: String,
    pub app: String, // 재생기(SourceAppUserModelId) — 다음 곡 목록을 찾을 때 쓴다
}

impl Now {
    pub fn pos(&self) -> i64 {
        let mut p = self.pos_ms;
        if self.playing {
            if let Some(t) = self.pos_at {
                p += t.elapsed().as_millis() as i64;
            }
        }
        if self.dur_ms > 0 { p.min(self.dur_ms) } else { p }
    }
}

fn playing(s: &Session) -> bool {
    s.GetPlaybackInfo().and_then(|i| i.PlaybackStatus()).map(|x| x == Status::Playing).unwrap_or(false)
}

fn pick(mgr: &Manager) -> Option<Session> {
    if let Ok(list) = mgr.GetSessions() {
        for s in list {
            if playing(&s) {
                return Some(s);
            }
        }
    }
    mgr.GetCurrentSession().ok()
}

fn art_bytes(p: &Props) -> Result<Vec<u8>> {
    let stream = p.Thumbnail()?.OpenReadAsync()?.get()?;
    let n = stream.Size()? as u32;
    let rd = DataReader::CreateDataReader(&stream)?;
    rd.LoadAsync(n)?.get()?;
    let mut buf = vec![0u8; n as usize];
    rd.ReadBytes(&mut buf)?;
    Ok(buf)
}

fn now_ticks() -> i64 {
    // 1601년부터 100ns (윈도우 DateTime 과 같은 눈금)
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    (d.as_nanos() / 100) as i64 + 116_444_736_000_000_000
}

pub fn spawn(now: Arc<Mutex<Now>>, lyr: Arc<Mutex<Lyr>>) -> mpsc::Sender<Cmd> {
    let (tx, rx) = mpsc::channel::<Cmd>();
    std::thread::spawn(move || {
        let mut mgr: Option<Manager> = None;
        let mut key = String::new(); // 지금 곡
        let mut lyr_key = String::new(); // 가사를 요청한 곡
        let mut since = 0u32; // 곡이 바뀐 뒤 읽은 횟수
        let mut warned = false;
        loop {
            let cmd = rx.recv_timeout(Duration::from_millis(500));
            if let Err(mpsc::RecvTimeoutError::Disconnected) = cmd {
                return;
            }
            if mgr.is_none() {
                match Manager::RequestAsync().and_then(|a| a.get()) {
                    Ok(m) => mgr = Some(m),
                    Err(e) => {
                        if !warned {
                            crate::log(&format!("WARN 음악 정보를 못 연다: {e}"));
                            warned = true;
                        }
                        std::thread::sleep(Duration::from_secs(5));
                        continue;
                    }
                }
            }
            let sess = pick(mgr.as_ref().unwrap());
            if let (Ok(c), true) = (&cmd, key.starts_with("ALSong|")) {
                let code = match c { Cmd::Toggle => 14, Cmd::Next => 11, Cmd::Prev => 12, _ => 0 };
                if code > 0 && !crate::alsong::cmd(code) {
                    crate::log("WARN 알송 명령 실패");
                }
                std::thread::sleep(Duration::from_millis(150));
                if code == 12 {
                    // '이전'은 곡 중간이면 같은 곡을 처음부터 다시 튼다(제목이 그대로) → 위치를 0 으로 안다.
                    std::thread::sleep(Duration::from_millis(450));
                    if let Some(raw) = crate::alsong::raw().filter(|r| format!("ALSong|{r}") == key) {
                        let dur = crate::alsong::list(&raw).map_or(0, |(r, i)| r[i].secs * 1000);
                        let mut n = now.lock().unwrap();
                        n.pos_ms = 0;
                        n.pos_at = Some(Instant::now());
                        n.dur_ms = dur;
                        crate::log(&format!("알송: 같은 곡 처음부터 (길이 {}초)", dur / 1000));
                    }
                }
            } else if let (Ok(c), Some(s)) = (&cmd, &sess) {
                let r = match c {
                    Cmd::Toggle => s.TryTogglePlayPauseAsync().and_then(|a| a.get()),
                    Cmd::Next => s.TrySkipNextAsync().and_then(|a| a.get()),
                    Cmd::Prev => s.TrySkipPreviousAsync().and_then(|a| a.get()),
                    Cmd::Seek(ms) => s.TryChangePlaybackPositionAsync(ms * 10_000).and_then(|a| a.get()),
                    Cmd::Shuffle => {
                        let on = s.GetPlaybackInfo().and_then(|i| i.IsShuffleActive()).and_then(|v| v.Value()).unwrap_or(false);
                        s.TryChangeShuffleActiveAsync(!on).and_then(|a| a.get())
                    }
                    Cmd::Repeat => {
                        use windows::Media::MediaPlaybackAutoRepeatMode as M;
                        let now = s.GetPlaybackInfo().and_then(|i| i.AutoRepeatMode()).and_then(|v| v.Value()).unwrap_or(M::None);
                        // 끔 → 전체 → 한 곡 → 끔 (휴대폰 음악 앱과 같은 순서)
                        let next = if now == M::None { M::List } else if now == M::List { M::Track } else { M::None };
                        s.TryChangeAutoRepeatModeAsync(next).and_then(|a| a.get())
                    }
                };
                if let Err(e) = r {
                    crate::log(&format!("WARN 재생 명령 실패: {e}"));
                }
                std::thread::sleep(Duration::from_millis(120)); // 재생기가 상태를 바꿀 틈
            }
            // 알송은 '지금 재생'에 안 나온다. 다른 재생기가 안 틀 때만 창 제목으로 대신 보인다.
            if !sess.as_ref().map_or(false, playing) {
                if let Some(raw) = crate::alsong::raw() {
                    let k = format!("ALSong|{raw}");
                    if k != key {
                        let (title, artist) = crate::alsong::now().unwrap_or_default();
                        // 알송 곡이 바뀌는 걸 직접 봤을 때만 위치를 안다(처음 본 곡은 길이를 비워 막대를 안 그린다).
                        let seen_start = key.starts_with("ALSong|");
                        let dur = if seen_start { crate::alsong::list(&raw).map_or(0, |(r, i)| r[i].secs * 1000) } else { 0 };
                        key = k.clone();
                        since = 0;
                        let mut n = now.lock().unwrap();
                        *n = Now { has: true, title: title.clone(), artist: artist.clone(), playing: true, pos_ms: 0, pos_at: Some(Instant::now()), dur_ms: dur, app: "ALSong".into(), ..Default::default() };
                        crate::log(&format!("알송: '{title}' / '{artist}' 길이 {}초", dur / 1000));
                    }
                    since += 1;
                    {
                        let mut n = now.lock().unwrap();
                        n.seen = Some(Instant::now());
                        n.stamp = crate::hhmm();
                    }
                    if lyr_key != key && since >= 2 {
                        lyr_key = key.clone();
                        let (t, ar, d) = { let n = now.lock().unwrap(); (n.title.clone(), n.artist.clone(), n.dur_ms) };
                        lyrics::fetch(lyr.clone(), key.clone(), t, ar, d);
                    }
                    continue;
                }
                if crate::alsong::stopped() && sess.is_none() {
                    key = "ALSong|".into(); // 멈춤 → 다음 재생은 처음부터
                    let mut n = now.lock().unwrap();
                    n.has = false;
                    n.seen = Some(Instant::now());
                    n.stamp = crate::hhmm();
                    continue;
                }
            }
            let Some(s) = sess else {
                let mut n = now.lock().unwrap();
                n.has = false;
                n.seen = Some(Instant::now());
                n.stamp = crate::hhmm();
                key.clear();
                continue;
            };
            let read = || -> Result<(Props, bool, i64, i64)> {
                let p = s.TryGetMediaPropertiesAsync()?.get()?;
                let pl = playing(&s);
                let tl = s.GetTimelineProperties()?;
                let TimeSpan { Duration: pos } = tl.Position()?;
                let TimeSpan { Duration: end } = tl.EndTime()?;
                let upd = tl.LastUpdatedTime()?.UniversalTime;
                let mut pos = pos / 10_000;
                if pl && upd > 0 {
                    pos += ((now_ticks() - upd) / 10_000).max(0);
                }
                Ok((p, pl, pos, end / 10_000))
            };
            let Ok((p, pl, pos, dur)) = read() else { continue };
            let title = p.Title().map(|h| h.to_string()).unwrap_or_default();
            let artist = p.Artist().map(|h| h.to_string()).unwrap_or_default();
            let k = format!("{artist}|{title}");
            let changed = k != key;
            if changed {
                key = k.clone();
                since = 0;
            }
            since += 1;
            let need_art = changed || (since <= 12 && now.lock().unwrap().art.is_none());
            let art = if need_art { art_bytes(&p).ok().filter(|b| !b.is_empty()).map(Arc::new) } else { None };
            {
                let mut n = now.lock().unwrap();
                n.has = !title.is_empty();
                n.title = title.clone();
                n.artist = artist.clone();
                n.playing = pl;
                n.pos_ms = pos;
                n.pos_at = Some(Instant::now());
                n.dur_ms = dur;
                let info = s.GetPlaybackInfo().ok();
                n.shuffle = info.as_ref().and_then(|i| i.IsShuffleActive().ok()).and_then(|v| v.Value().ok());
                n.repeat = info.as_ref().and_then(|i| i.AutoRepeatMode().ok()).and_then(|v| v.Value().ok()).map(|m| m.0);
                if changed || art.is_some() {
                    n.art = art;
                }
                n.seen = Some(Instant::now());
                n.stamp = crate::hhmm();
                n.app = s.SourceAppUserModelId().map(|h| h.to_string()).unwrap_or_default();
            }
            // 길이를 알아야 맞는 가사를 고르기 쉽다. 곡이 바뀐 직후엔 길이가 늦게 오므로 조금 기다린다.
            if !title.is_empty() && lyr_key != key && (dur > 0 || since >= 4) {
                lyr_key = key.clone();
                lyrics::fetch(lyr.clone(), key.clone(), title, artist, dur);
            }
        }
    });
    tx
}
