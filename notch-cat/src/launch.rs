//! 실행도구모음 쪽: 자주 쓰는 프로그램·바로가기를 '＋ 등록'으로 올려 두고 눌러 실행한다.
//! 끌어다 놓기는 파일 선반이 쓰므로 여기선 파일 창으로만 등록한다. 원본 경로만 기억한다.
//! 목록은 %APPDATA%\notch-cat\launch.json 에 남는다.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

pub const MAX: usize = 21; // 7칸 × 3줄

#[derive(Default)]
pub struct Launch {
    pub items: Vec<String>,
}

fn file() -> PathBuf {
    crate::data_dir().join("launch.json")
}

impl Launch {
    pub fn load() -> Launch {
        let v: Value = std::fs::read_to_string(file()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        Launch { items: v.as_array().into_iter().flatten().filter_map(|p| p.as_str().map(str::to_string)).take(MAX).collect() }
    }

    pub fn save(&self) {
        let _ = std::fs::write(file(), json!(self.items).to_string());
    }

    /// 새로 고른 것은 뒤에 붙인다(이미 있으면 그대로). 넘치면 더 받지 않는다.
    pub fn add(&mut self, paths: &[String]) {
        for p in paths {
            if self.items.len() < MAX && !self.items.iter().any(|q| q.eq_ignore_ascii_case(p)) {
                self.items.push(p.clone());
            }
        }
    }
}

/// 칸에 쓸 이름: 바로가기·실행 파일은 확장자를 뗀다
pub fn name(p: &str) -> String {
    let path = Path::new(p);
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let n = if ["lnk", "exe", "bat", "cmd", "url", "pyw", "py", "appref-ms"].contains(&ext.as_str()) { path.file_stem() } else { path.file_name() };
    n.map_or_else(|| p.to_string(), |n| n.to_string_lossy().into_owned())
}

/// 바탕화면(OneDrive 바탕 화면이 있으면 그쪽)
pub fn desktop() -> PathBuf {
    let home = PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default());
    ["OneDrive\\바탕 화면", "OneDrive\\Desktop", "Desktop"].iter().map(|d| home.join(d)).find(|d| d.is_dir()).unwrap_or(home)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_add() {
        assert_eq!(name("C:\\Users\\a\\Desktop\\자막 스튜디오.lnk"), "자막 스튜디오");
        assert_eq!(name("D:\\도구\\run.BAT"), "run");
        assert_eq!(name("D:\\보고서.hwpx"), "보고서.hwpx");
        let mut l = Launch::default();
        l.add(&["C:\\a.lnk".into(), "C:\\b.exe".into()]);
        l.add(&["c:\\A.LNK".into()]); // 같은 것은 다시 안 붙는다
        assert_eq!(l.items, vec!["C:\\a.lnk", "C:\\b.exe"]);
        l.add(&(0..30).map(|i| format!("C:\\t{i}.lnk")).collect::<Vec<_>>());
        assert_eq!(l.items.len(), MAX);
    }
}
