# 노치 고양이 (Rust, 윈도우 노치 앱)

- 빌드: `cargo test` → `cargo build --release`. 집 PC는 rustup gnu 도구모음(`~/.cargo/bin`). 정식 exe는 `main` 푸시 → `notch-cat-exe` 워크플로가 릴리스 `notch` 에 올린다(구글 등록증은 Actions 비밀값으로만 들어간다 — 손으로 빌드한 exe는 달력 로그인이 안 된다).
- 설치·갱신: `설치.bat` 을 **explorer.exe 로** 실행(%LOCALAPPDATA%\NotchCat). 셸에서 바로 켜면 Claude 앱 가상 AppData 안에서 돈다.
- ⚠️ 시험 exe를 켜면 단일 실행 규칙 때문에 **사용자가 쓰던 노치가 꺼진다** → 시험 끝나면 설치본을 다시 켠다.
- ⛔ **사용자 화면에 진짜 마우스 클릭·휠을 보내지 않는다**(2026-11-13: 노치가 접힌 사이 클릭이 사용자 웹페이지에 떨어졌다). 화면 확인은 마우스 없이:
  `NOTCH_CAT_SHOT=<폴더>`(펼친 채 유지 + shot.raw 저장) + `NOTCH_CAT_PAGE=<쪽>` 또는 `NOTCH_CAT_ENTRY=seg,note`(일정 칸·알림). shot.raw = 너비·높이·pitch(u32 LE) + BGRA.
- 구글 비밀값을 붙여 넣으면 끝 줄바꿈이 따라 들어갈 수 있다 → `gcal::client()` 가 trim 한다(invalid_client 의 원인이었다).
