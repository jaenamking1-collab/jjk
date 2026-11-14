//! 파일 선반: 파일·폴더를 노치에 끌어다 놓으면 '어디 있는지'만 기억해 두고(원본은 그대로),
//! 끌어 내리면 놓은 곳으로 **복사만** 된다(같은 드라이브여도 이동하지 않게 효과를 COPY 로 묶는다).
//! 목록은 %APPDATA%\notch-cat\shelf.json 에 남아 껐다 켜도 그대로다.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{json, Value};
use windows::core::{implement, Result, HSTRING};
use windows::Win32::Foundation::{HWND, POINTL, SIZE};
use windows::Win32::Graphics::Gdi::HBITMAP;
use windows::Win32::System::Com::{IDataObject, DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL};
use windows::Win32::System::Ole::*;
use windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS;
use windows::Win32::UI::Shell::*;

pub const MAX: usize = 12;
const CF_HDROP: u16 = 15;

/// 선반 쪽이 메뉴에서 켜져 있나(꺼져 있으면 끌어다 놓기를 받지 않는다)
pub static ON: AtomicBool = AtomicBool::new(true);

#[derive(Default)]
pub struct Shelf {
    pub items: Vec<String>,
}

fn file() -> PathBuf {
    crate::data_dir().join("shelf.json")
}

impl Shelf {
    pub fn load() -> Shelf {
        let v: Value = std::fs::read_to_string(file()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        Shelf { items: v.as_array().into_iter().flatten().filter_map(|p| p.as_str().map(str::to_string)).take(MAX).collect() }
    }

    pub fn save(&self) {
        let _ = std::fs::write(file(), json!(self.items).to_string());
    }

    /// 새로 놓은 것들을 맨 앞에(이미 있으면 앞으로). 12개를 넘으면 오래된 것부터 빠진다.
    pub fn add(&mut self, paths: &[String]) {
        for p in paths.iter().rev() {
            self.items.retain(|q| !q.eq_ignore_ascii_case(p));
            self.items.insert(0, p.clone());
        }
        self.items.truncate(MAX);
    }
}

/// 보여 줄 이름(파일·폴더 이름)
pub fn name(p: &str) -> String {
    Path::new(p).file_name().map_or_else(|| p.to_string(), |n| n.to_string_lossy().into_owned())
}

pub fn exists(p: &str) -> bool {
    Path::new(p).exists()
}

#[implement(IDropTarget)]
pub struct Target {
    ok: Cell<bool>, // 지금 끌려온 것이 파일인가
}

impl Target {
    pub fn new() -> Target {
        Target { ok: Cell::new(false) }
    }
}

fn hdrop_fmt() -> FORMATETC {
    FORMATETC { cfFormat: CF_HDROP, ptd: std::ptr::null_mut(), dwAspect: DVASPECT_CONTENT.0, lindex: -1, tymed: TYMED_HGLOBAL.0 as u32 }
}

/// 끌려온 것에서 파일 경로들
unsafe fn paths_of(d: &IDataObject) -> Vec<String> {
    let Ok(mut m) = d.GetData(&hdrop_fmt()) else { return vec![] };
    let h = HDROP(m.u.hGlobal.0);
    let n = DragQueryFileW(h, u32::MAX, None);
    let mut out = Vec::new();
    for i in 0..n {
        let len = DragQueryFileW(h, i, None) as usize;
        let mut buf = vec![0u16; len + 1];
        DragQueryFileW(h, i, Some(&mut buf));
        out.push(String::from_utf16_lossy(&buf[..len]));
    }
    ReleaseStgMedium(&mut m);
    out
}

impl IDropTarget_Impl for Target_Impl {
    fn DragEnter(&self, d: Option<&IDataObject>, _: MODIFIERKEYS_FLAGS, _: &POINTL, eff: *mut DROPEFFECT) -> Result<()> {
        let ok = ON.load(Ordering::Relaxed) && d.is_some_and(|d| unsafe { d.QueryGetData(&hdrop_fmt()) }.is_ok());
        self.ok.set(ok);
        unsafe { *eff = if ok { DROPEFFECT_COPY } else { DROPEFFECT_NONE } };
        if ok {
            crate::push(crate::Ev::DragIn);
        }
        Ok(())
    }
    fn DragOver(&self, _: MODIFIERKEYS_FLAGS, _: &POINTL, eff: *mut DROPEFFECT) -> Result<()> {
        unsafe { *eff = if self.ok.get() { DROPEFFECT_COPY } else { DROPEFFECT_NONE } };
        Ok(())
    }
    fn DragLeave(&self) -> Result<()> {
        if self.ok.get() {
            crate::push(crate::Ev::DragOut);
        }
        Ok(())
    }
    fn Drop(&self, d: Option<&IDataObject>, _: MODIFIERKEYS_FLAGS, _: &POINTL, eff: *mut DROPEFFECT) -> Result<()> {
        let paths = if self.ok.get() { d.map(|d| unsafe { paths_of(d) }).unwrap_or_default() } else { vec![] };
        // 원본 쪽이 '옮겨졌다'며 지우지 않게 늘 복사로 답한다
        unsafe { *eff = if paths.is_empty() { DROPEFFECT_NONE } else { DROPEFFECT_COPY } };
        crate::push(if paths.is_empty() { crate::Ev::DragOut } else { crate::Ev::Dropped(paths) });
        Ok(())
    }
}

/// 노치 창이 끌어다 놓기를 받게 한다
pub unsafe fn listen(hwnd: HWND) {
    let _ = OleInitialize(None);
    let t: IDropTarget = Target::new().into();
    let _ = RegisterDragDrop(hwnd, &t);
}

/// 파일 하나를 끌어 내린다(놓는 곳으로 복사만). 끝날 때까지 돌아오지 않는다.
pub unsafe fn drag_out(hwnd: HWND, path: &Path) -> bool {
    let Ok(item) = SHCreateItemFromParsingName::<_, _, IShellItem>(&HSTRING::from(path.as_os_str()), None) else { return false };
    let Ok(obj) = item.BindToHandler::<_, IDataObject>(None, &BHID_DataObject) else { return false };
    SHDoDragDrop(hwnd, &obj, None, DROPEFFECT_COPY).is_ok()
}

/// 탐색기와 같은 미리보기(사진·영상은 작은 그림, 나머지는 아이콘)
pub unsafe fn thumb(path: &str) -> Option<HBITMAP> {
    let f: IShellItemImageFactory = SHCreateItemFromParsingName(&HSTRING::from(path), None).ok()?;
    f.GetImage(SIZE { cx: 96, cy: 96 }, SIIGBF_RESIZETOFIT).ok()
}

/// 기본 프로그램으로 열기(두 번 누름)
pub unsafe fn open(path: &str) {
    ShellExecuteW(None, &HSTRING::from("open"), &HSTRING::from(path), None, None, windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_newest_first_no_dupes() {
        let mut s = Shelf::default();
        s.add(&["C:\\a.hwp".into(), "C:\\b.jpg".into()]);
        assert_eq!(s.items, vec!["C:\\a.hwp", "C:\\b.jpg"]);
        s.add(&["c:\\B.JPG".into()]); // 같은 파일(대소문자만 다름)은 앞으로만
        assert_eq!(s.items, vec!["c:\\B.JPG", "C:\\a.hwp"]);
        let many: Vec<String> = (0..20).map(|i| format!("C:\\f{i}.txt")).collect();
        s.add(&many);
        assert_eq!(s.items.len(), MAX);
        assert_eq!(s.items[0], "C:\\f0.txt");
        assert_eq!(name("C:\\폴더\\보고서.hwpx"), "보고서.hwpx");
    }

    #[test]
    #[ignore] // 탐색기가 넘겨주는 것과 같은 자료(셸 IDataObject)를 받는 쪽에 직접 넣어 본다
    fn drop_reads_explorer_paths() {
        unsafe {
            let _ = OleInitialize(None);
            let me = std::env::current_exe().unwrap();
            let item: IShellItem = SHCreateItemFromParsingName(&HSTRING::from(me.as_os_str()), None).unwrap();
            let obj: IDataObject = item.BindToHandler(None, &BHID_DataObject).unwrap();
            let t: IDropTarget = Target::new().into();
            let mut eff = DROPEFFECT_MOVE | DROPEFFECT_COPY;
            t.DragEnter(&obj, MODIFIERKEYS_FLAGS(0), POINTL::default(), &mut eff).unwrap();
            assert_eq!(eff, DROPEFFECT_COPY); // 이동이 아니라 복사로 답한다
            t.Drop(&obj, MODIFIERKEYS_FLAGS(0), POINTL::default(), &mut eff).unwrap();
            assert_eq!(eff, DROPEFFECT_COPY);
            let evs: Vec<crate::Ev> = crate::EVENTS.with(|v| v.borrow_mut().drain(..).collect());
            let got = evs.iter().find_map(|e| if let crate::Ev::Dropped(p) = e { Some(p.clone()) } else { None }).expect("받은 경로 없음");
            assert_eq!(got, vec![me.display().to_string()]);
        }
    }
}
