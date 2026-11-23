# 노치 고양이 (Rust, 윈도우 노치 앱)

- 빌드: `cargo test` → `cargo build --release`. 집 PC는 rustup gnu 도구모음(`~/.cargo/bin`). 정식 exe는 `main` 푸시 → `notch-cat-exe` 워크플로가 릴리스 `notch` 에 올린다(구글 등록증은 Actions 비밀값으로만 들어간다 — 손으로 빌드한 exe는 달력 로그인이 안 된다).
- 설치·갱신: `설치.bat` 을 **explorer.exe 로** 실행(%LOCALAPPDATA%\NotchCat). 셸에서 바로 켜면 Claude 앱 가상 AppData 안에서 돈다.
- ⚠️ 시험 exe를 켜면 단일 실행 규칙 때문에 **사용자가 쓰던 노치가 꺼진다** → 시험 끝나면 설치본을 다시 켠다.
- ⛔ **사용자 화면에 진짜 마우스 클릭·휠을 보내지 않는다**(2026-11-13: 노치가 접힌 사이 클릭이 사용자 웹페이지에 떨어졌다). 화면 확인은 마우스 없이:
  `NOTCH_CAT_SHOT=<폴더>`(펼친 채 유지 + shot.raw 저장) + `NOTCH_CAT_PAGE=<쪽>` 또는 `NOTCH_CAT_ENTRY=seg,note`(일정 칸·알림). shot.raw = 너비·높이·pitch(u32 LE) + BGRA.
- ⛔ 시험 exe 는 사용자의 `%APPDATA%\notch-cat`(클립보드 기록·설정)를 그대로 읽고 쓴다. 보관 수를 줄인 시험판을 켜면 기록이 잘린다(2026-11-15: 20→10 시험판 때문에 오래된 10개를 잃었다). 클립보드·설정을 건드리는 시험은 그 폴더를 시험 폴더로 복사하고 `APPDATA=<시험 폴더>` 로 켠다(`ai_alerts.json` 도 같이 복사해야 사용량 알림이 안 끼어든다).
- 편집기 상태 확인: `NOTCH_CAT_EDDUMP=<파일>` 이면 편집 창이 그릴 때마다 상태(도구·글자 칸·단추 위치·되돌리기 수·글자 인식 선택)를 json 으로 적는다. `NOTCH_CAT_CLIPOPEN=<숫자>` 는 그 칸을 펼친다.
- 구글 비밀값을 붙여 넣으면 끝 줄바꿈이 따라 들어갈 수 있다 → `gcal::client()` 가 trim 한다(invalid_client 의 원인이었다).
- 새 판 스스로 받기(update.rs, 2026-11-17): 설치본은 켜고 1분 뒤·6시간마다 릴리스 지문과 비교해 스스로 바꾼다. 시험판은 이름을 NotchTest.exe 로 하거나 NOTCH_CAT_* 변수를 주면 건너뛴다 — NotchCat.exe 이름으로 target 밖에서 켜면 릴리스 판으로 바뀐다.
- 파일 전송 쪽(xfer.rs, 쪽 번호 8, 2026-11-23): 쪽을 펼치면 와이파이 창구를 열고 QR 을 띄운다. 휴대폰 웹 화면에서 올린 파일은 `%USERPROFILE%\Downloads`, 이 쪽에 끌어다 놓은 파일은 휴대폰에서 받기. 10분 쉬면 닫힘. 시험은 `NOTCH_CAT_PAGE=8 NOTCH_CAT_XFERLOCAL=1`(이 PC 안에서만 열어 방화벽을 안 건드림).
