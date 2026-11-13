// 구형 Windows Media Player(레거시, wmplayer.exe)는 윈도우 '지금 재생'(GSMTC)에 안 나온다.
// 실행 중인 플레이어에 COM '원격(Remote)' 방식으로 붙어 곡 제목·위치를 읽고 재생 명령을 보낸다.
// COM 개체는 STA 전용이라 자기 스레드에서만 만지고, 결과는 Mutex 로, 명령은 채널로 주고받는다.
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use windows::core::{implement, w, Interface, IUnknownImpl, Result, BSTR, GUID};
use windows::Win32::Foundation::{BOOL, E_NOINTERFACE, E_NOTIMPL};
use windows::Win32::Media::MediaPlayer::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::Ole::*;
use windows::Win32::UI::WindowsAndMessaging::{DispatchMessageW, FindWindowW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE};

#[derive(Clone, Default, Debug)]
pub struct Info {
    pub title: String,
    pub artist: String,
    pub playing: bool,
    pub pos_ms: i64,
    pub dur_ms: i64,
    pub path: String,
}

pub enum Cmd {
    Toggle,
    Next,
    Prev,
    Seek(i64),
}

#[implement(IOleClientSite, IServiceProvider, IWMPRemoteMediaServices)]
struct Site;

impl IOleClientSite_Impl for Site_Impl {
    fn SaveObject(&self) -> Result<()> { Err(E_NOTIMPL.into()) }
    fn GetMoniker(&self, _: &OLEGETMONIKER, _: &OLEWHICHMK) -> Result<IMoniker> { Err(E_NOTIMPL.into()) }
    fn GetContainer(&self) -> Result<IOleContainer> { Err(E_NOINTERFACE.into()) }
    fn ShowObject(&self) -> Result<()> { Ok(()) }
    fn OnShowWindow(&self, _: BOOL) -> Result<()> { Ok(()) }
    fn RequestNewObjectLayout(&self) -> Result<()> { Err(E_NOTIMPL.into()) }
}

impl IServiceProvider_Impl for Site_Impl {
    fn QueryService(&self, _svc: *const GUID, riid: *const GUID, ppv: *mut *mut core::ffi::c_void) -> Result<()> {
        // 플레이어는 서비스 GUID 를 따로(DF333473-…) 주고 riid 로 IWMPRemoteMediaServices 를 찾는다.
        if unsafe { *riid } == IWMPRemoteMediaServices::IID {
            unsafe { self.QueryInterface(riid, ppv).ok() }
        } else {
            Err(E_NOINTERFACE.into())
        }
    }
}

impl IWMPRemoteMediaServices_Impl for Site_Impl {
    fn GetServiceType(&self, t: *mut BSTR) -> Result<()> {
        unsafe { *t = BSTR::from("Remote") };
        Ok(())
    }
    fn GetApplicationName(&self, n: *mut BSTR) -> Result<()> {
        unsafe { *n = BSTR::from("NotchCat") };
        Ok(())
    }
    fn GetScriptableObject(&self, _: *mut BSTR, _: *mut Option<IDispatch>) -> Result<()> { Err(E_NOTIMPL.into()) }
    fn GetCustomUIMode(&self, _: *mut BSTR) -> Result<()> { Err(E_NOTIMPL.into()) }
}

/// 레거시 플레이어 창이 떠 있나. 안 떠 있을 때 붙으면 플레이어를 새로 띄워 버리므로 꼭 먼저 본다.
pub fn running() -> bool {
    unsafe { FindWindowW(w!("WMPlayerApp"), None) }.is_ok()
}

struct Conn {
    ole: IOleObject,
    player: IWMPPlayer4,
}

impl Drop for Conn {
    fn drop(&mut self) {
        unsafe {
            let _ = self.ole.SetClientSite(None);
            let _ = self.ole.Close(OLECLOSE_NOSAVE);
        }
    }
}

fn connect() -> Result<Conn> {
    unsafe {
        let ole: IOleObject = CoCreateInstance(&WindowsMediaPlayer, None, CLSCTX_INPROC_SERVER)?;
        let site: IOleClientSite = Site.into();
        ole.SetClientSite(&site)?;
        let player: IWMPPlayer4 = ole.cast()?;
        Ok(Conn { ole, player })
    }
}

fn bstr(f: impl FnOnce(*mut BSTR) -> Result<()>) -> String {
    let mut b = BSTR::new();
    if f(&mut b).is_ok() { b.to_string().trim().to_string() } else { String::new() }
}

fn read(c: &Conn) -> Result<Option<Info>> {
    unsafe {
        let mut st = WMPPlayState(0);
        c.player.playState(&mut st)?;
        if st != wmppsPlaying && st != wmppsPaused {
            return Ok(None);
        }
        let m = c.player.currentMedia()?;
        let mut title = bstr(|p| m.getItemInfo(&BSTR::from("Title"), p));
        if title.is_empty() {
            title = bstr(|p| m.name(p));
        }
        let mut artist = bstr(|p| m.getItemInfo(&BSTR::from("Author"), p));
        if artist.is_empty() {
            artist = bstr(|p| m.getItemInfo(&BSTR::from("WM/AlbumArtist"), p));
        }
        let path = bstr(|p| m.sourceURL(p));
        let mut dur = 0f64;
        let _ = m.duration(&mut dur);
        let mut pos = 0f64;
        let _ = c.player.controls().and_then(|k| k.currentPosition(&mut pos));
        Ok(Some(Info { title, artist, playing: st == wmppsPlaying, pos_ms: (pos * 1000.0) as i64, dur_ms: (dur * 1000.0) as i64, path }))
    }
}

fn run(c: &Conn, cmd: Cmd) -> Result<()> {
    unsafe {
        let k = c.player.controls()?;
        match cmd {
            Cmd::Toggle => {
                let mut st = WMPPlayState(0);
                c.player.playState(&mut st)?;
                if st == wmppsPlaying { k.pause() } else { k.play() }
            }
            Cmd::Next => k.next(),
            Cmd::Prev => k.previous(),
            Cmd::Seek(ms) => k.SetcurrentPosition(ms as f64 / 1000.0),
        }
    }
}

/// 읽은 값(없으면 None)과 명령 채널. log 는 붙음·끊김·오류를 한 번씩 남긴다.
pub fn spawn(log: fn(&str)) -> (Arc<Mutex<Option<Info>>>, mpsc::Sender<Cmd>) {
    let out = Arc::new(Mutex::new(None));
    let (tx, rx) = mpsc::channel::<Cmd>();
    let o = out.clone();
    std::thread::spawn(move || {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }
        let mut conn: Option<Conn> = None;
        let mut warned = false;
        loop {
            // STA 는 메시지를 돌려야 다른 프로세스(플레이어)와의 COM 호출이 막히지 않는다.
            unsafe {
                let mut msg = MSG::default();
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            let cmd = rx.recv_timeout(Duration::from_millis(250));
            if let Err(mpsc::RecvTimeoutError::Disconnected) = cmd {
                return;
            }
            if !running() {
                if conn.take().is_some() {
                    log("WMP: 레거시 플레이어 꺼짐");
                }
                *o.lock().unwrap() = None;
                warned = false;
                continue;
            }
            if conn.is_none() {
                match connect() {
                    Ok(c) => {
                        log("WMP: 레거시 플레이어에 붙음");
                        conn = Some(c);
                    }
                    Err(e) => {
                        if !warned {
                            log(&format!("WARN WMP: 못 붙음 {e}"));
                            warned = true;
                        }
                        std::thread::sleep(Duration::from_secs(3));
                        continue;
                    }
                }
            }
            let c = conn.as_ref().unwrap();
            if let Ok(cmd) = cmd {
                if let Err(e) = run(c, cmd) {
                    log(&format!("WARN WMP 명령 실패: {e}"));
                }
                std::thread::sleep(Duration::from_millis(120));
            }
            match read(c) {
                Ok(i) => *o.lock().unwrap() = i,
                Err(e) => {
                    log(&format!("WARN WMP: 읽기 실패, 다시 붙음 {e}"));
                    conn = None;
                    *o.lock().unwrap() = None;
                }
            }
        }
    });
    (out, tx)
}
