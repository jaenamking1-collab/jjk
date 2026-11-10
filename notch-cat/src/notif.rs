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

/// 결과를 최대 1초만 기다린다. 앱 그림을 안 내주고 영영 붙잡는 앱이 있어서, 그냥 기다리면 알림 전체가 멈춘다.
fn wait<T: windows::core::RuntimeType + 'static>(op: windows::Foundation::IAsyncOperation<T>) -> Result<T> {
    for _ in 0..50 {
        if op.Status()? != windows::Foundation::AsyncStatus::Started {
            return op.GetResults();
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let _ = op.Cancel();
    Err(windows::core::Error::empty())
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
        let s = wait(info.GetLogo(Size { Width: 48.0, Height: 48.0 })?.OpenReadAsync()?)?;
        let len = s.Size()? as u32;
        let rd = DataReader::CreateDataReader(&s)?;
        wait(windows::core::Interface::cast::<windows::Foundation::IAsyncOperation<u32>>(&rd.LoadAsync(len)?)?)?;
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
    // 같은 번호를 다시 쓰는 앱(캡처 도구 등)이 있어 번호와 만든 시각을 함께 본다
    let mut seen: HashSet<(u32, i64)> = HashSet::new();
    let mut first = true;
    let mut count = u32::MAX;
    loop {
        let mut cur = HashSet::new();
        let list = l.GetNotificationsAsync(NotificationKinds::Toast)?.get()?;
        if list.Size()? != count {
            count = list.Size()?;
            crate::log(&format!("notif count: {count}"));
        }
        for n in list {
            let key = (n.Id()?, n.CreationTime().map(|t| t.UniversalTime).unwrap_or(0));
            cur.insert(key);
            if seen.contains(&key) {
                continue;
            }
            // 켜기 전부터 쌓여 있던 알림은 띄우지 않는다. 기록에는 앱 이름만 남긴다(내용은 안 남긴다).
            if first {
                continue;
            }
            match read(&n) {
                Ok(mut note) => {
                    crate::log(&format!("notif new: {}", note.app));
                    let mut o = out.lock().unwrap();
                    note.ver = o.ver + 1;
                    *o = note;
                }
                Err(e) => crate::log(&format!("notif skip: {:?}", e.code())),
            }
        }
        seen = cur;
        first = false;
        std::thread::sleep(Duration::from_millis(500));
    }
}

pub fn spawn(out: Arc<Mutex<Note>>) {
    std::thread::spawn(move || {
        if let Err(e) = watch(&out) {
            crate::log(&format!("notif off: {e:?}")); // 빌드 서버 등 알림이 없는 곳
        }
    });
}
