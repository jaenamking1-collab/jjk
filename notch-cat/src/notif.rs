// 윈도우 알림(토스트)을 읽어 노치에 넘긴다. 내용은 화면에만 쓰고 어디에도 적지 않는다.
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use windows::core::Result;
use windows::Foundation::Size;
use windows::Storage::Streams::DataReader;
use windows::UI::Notifications::Management::{UserNotificationListener, UserNotificationListenerAccessStatus as Access};
use windows::UI::Notifications::{KnownNotificationBindings, NotificationKinds, UserNotification};

#[derive(Default, Clone)]
pub struct Note {
    pub ver: u64, // 새 알림마다 1씩 는다
    pub app: String,
    pub title: String,
    pub body: String,
    pub icon: Option<Arc<Vec<u8>>>,
}

fn read(n: &UserNotification) -> Result<Note> {
    let info = n.AppInfo()?.DisplayInfo()?;
    let mut lines = Vec::new();
    for e in n.Notification()?.Visual()?.GetBinding(&KnownNotificationBindings::ToastGeneric()?)?.GetTextElements()? {
        let t = e.Text()?.to_string();
        if !t.trim().is_empty() {
            lines.push(t.replace(['\r', '\n'], " "));
        }
    }
    let icon = (|| -> Result<Vec<u8>> {
        let s = info.GetLogo(Size { Width: 48.0, Height: 48.0 })?.OpenReadAsync()?.get()?;
        let len = s.Size()? as u32;
        let rd = DataReader::CreateDataReader(&s)?;
        rd.LoadAsync(len)?.get()?;
        let mut buf = vec![0u8; len as usize];
        rd.ReadBytes(&mut buf)?;
        Ok(buf)
    })();
    Ok(Note {
        ver: 0,
        app: info.DisplayName()?.to_string(),
        title: lines.first().cloned().unwrap_or_default(),
        body: lines.get(1..).map(|r| r.join(" ")).unwrap_or_default(),
        icon: icon.ok().map(Arc::new),
    })
}

fn watch(out: &Mutex<Note>) -> Result<()> {
    let l = UserNotificationListener::Current()?;
    if l.GetAccessStatus()? != Access::Allowed {
        l.RequestAccessAsync()?.get()?; // 처음 한 번 윈도우가 허용할지 묻는다
    }
    if l.GetAccessStatus()? != Access::Allowed {
        crate::log("notif off: 알림 읽기가 허용되지 않음(설정 > 개인 정보 > 알림)");
        return Ok(());
    }
    crate::log("notif ok");
    let mut seen = HashSet::new();
    let mut first = true;
    loop {
        for n in l.GetNotificationsAsync(NotificationKinds::Toast)?.get()? {
            // 켜기 전부터 쌓여 있던 알림은 건너뛴다
            if seen.insert(n.Id()?) && !first {
                if let Ok(mut note) = read(&n) {
                    let mut o = out.lock().unwrap();
                    note.ver = o.ver + 1;
                    *o = note;
                }
            }
        }
        first = false;
        std::thread::sleep(Duration::from_millis(700));
    }
}

pub fn spawn(out: Arc<Mutex<Note>>) {
    std::thread::spawn(move || {
        if let Err(e) = watch(&out) {
            crate::log(&format!("notif off: {e:?}")); // 빌드 서버 등 알림이 없는 곳
        }
    });
}
