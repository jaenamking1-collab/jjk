# 노치 고양이 (Rust, 윈도우 노치 앱)

- 빌드: `cargo test` → `cargo build --release`. 집 PC는 rustup gnu 도구모음(`~/.cargo/bin`). 정식 exe는 `main` 푸시 → `notch-cat-exe` 워크플로가 릴리스 `notch` 에 올린다(구글 등록증은 Actions 비밀값으로만 들어간다 — 손으로 빌드한 exe는 달력 로그인이 안 된다).
- 설치·갱신: `설치.bat` 을 **explorer.exe 로** 실행(%LOCALAPPDATA%\NotchCat). 셸에서 바로 켜면 Claude 앱 가상 AppData 안에서 돈다.
- ⚠️ 시험 exe를 켜면 단일 실행 규칙 때문에 **사용자가 쓰던 노치가 꺼진다** → 시험 끝나면 설치본을 다시 켠다.
- ⛔ **사용자 화면에 진짜 마우스 클릭·휠을 보내지 않는다**(2026-11-13: 노치가 접힌 사이 클릭이 사용자 웹페이지에 떨어졌다). 화면 확인은 마우스 없이:
  `NOTCH_CAT_SHOT=<폴더>`(펼친 채 유지 + shot.raw 저장) + `NOTCH_CAT_PAGE=<쪽>` 또는 `NOTCH_CAT_ENTRY=seg,note`(일정 칸·알림). shot.raw = 너비·높이·pitch(u32 LE) + BGRA.
- ⛔ 시험 exe 는 사용자의 `%APPDATA%\notch-cat`(클립보드 기록·설정)를 그대로 읽고 쓴다. 보관 수를 줄인 시험판을 켜면 기록이 잘린다(2026-11-15: 20→10 시험판 때문에 오래된 10개를 잃었다). 클립보드·설정을 건드리는 시험은 그 폴더를 시험 폴더로 복사하고 `APPDATA=<시험 폴더>` 로 켠다(`ai_alerts.json` 도 같이 복사해야 사용량 알림이 안 끼어든다).
- 클립보드 폴더(`clip\`)의 파일은 DPAPI 암호화(앞머리 `NCE1`, 2026-11-26) — 읽고 쓸 때 `clip::read`/`clip::write` 를 거친다. `std::fs::read` 로 바로 읽으면 암호문이다. 옛 평문은 켤 때 자동 변환.
- 편집기 상태 확인: `NOTCH_CAT_EDDUMP=<파일>` 이면 편집 창이 그릴 때마다 상태(도구·글자 칸·단추 위치·되돌리기 수·글자 인식 선택)를 json 으로 적는다. `NOTCH_CAT_CLIPOPEN=<숫자>` 는 그 칸을 펼친다.
- 구글 비밀값을 붙여 넣으면 끝 줄바꿈이 따라 들어갈 수 있다 → `gcal::client()` 가 trim 한다(invalid_client 의 원인이었다).
- 새 판 스스로 받기(update.rs, 2026-11-17): 설치본은 켜고 1분 뒤·6시간마다 릴리스 지문과 비교해 스스로 바꾼다. 시험판은 이름을 NotchTest.exe 로 하거나 NOTCH_CAT_* 변수를 주면 건너뛴다 — NotchCat.exe 이름으로 target 밖에서 켜면 릴리스 판으로 바뀐다.
  - ⚠️ 학교 망은 release-assets.githubusercontent.com 연결을 끊는다(2026-11-26) → update.rs·설치.bat 이 실패하면 `https://jjk-app.distributionjn.workers.dev/NotchCat.exe`(workers/jjk-app 이 릴리스를 그대로 넘김, worker.js 바꾸면 `npx wrangler deploy`)로 받는다. 지문은 어느 길이든 GitHub 것과 대조. 실패는 log.txt "update: 받기 …" 줄. 지문 확인은 실제 설치본 `%LOCALAPPDATA%\NotchCat\NotchCat.exe`(PowerShell Get-FileHash)로. 내 셸에서 보이는 `%APPDATA%\notch-cat\log.txt` 는 Claude 앱 가상 사본일 수 있다 — 진짜 기록은 explorer.exe 로 띄운 .bat 에서 복사해 본다.
- 영상 자르기(trim.rs, 2026-11-26): 장은 NV12 로 받아 `Video::rows`(2D 버퍼의 실제 줄 간격)와 보이는 영역(aperture)으로 읽는다 — 너비로 줄 간격을 짐작하면 16 배수가 아닌 녹화(908폭)가 빗금처럼 깨진다. 시험 `NOTCH_CAT_TESTVID=<영상> cargo test --release scenes_and_play -- --ignored --nocapture`, 화면 `NOTCH_CAT_PAGE=7 NOTCH_CAT_TRIM=<영상> NOTCH_CAT_TRIMPLAY=1`(알림이 가리면 15초 뒤 화면을 본다). 재생 소리는 윈도우 MediaPlayer(미디어 제어에 안 잡히게 CommandManager 끔), 노치 녹화(rec.rs)는 원래 소리를 안 담는다. 자르기는 처음부터면 그대로 옮기고, 앞을 자르면 영상만 다시 구워 정확히 시작(시험 `exact_cut`, `NOTCH_CAT_TESTOUT=<저장 경로>`). 장면 점은 '바뀌기 시작한 장'(`exact`). 단추 누름 시험은 실제 마우스 대신 `NOTCH_CAT_SHOT` 폴더의 mouse.txt(화면 좌표) + 시험 창에 WM_MOUSEMOVE·WM_LBUTTONUP 보내기(FindWindow "NotchCat", 파워셸은 [NullString]::Value).
- src/*.rs 는 줄끝(LF·CRLF)이 섞여 있다 → `sed -i`·파이썬 텍스트 모드로 고치면 파일 전체가 바뀐 것처럼 된다. Edit 도구나 바이트 단위로 고친다.
- 파일 전송 쪽(xfer.rs, 쪽 번호 8, 2026-11-23): 쪽을 펼치면 와이파이 창구를 열고 QR 을 띄운다. 휴대폰 웹 화면에서 올린 파일은 `%USERPROFILE%\Downloads`, 이 쪽에 끌어다 놓은 파일은 휴대폰에서 받기. 10분 쉬면 닫힘. 시험은 `NOTCH_CAT_PAGE=8 NOTCH_CAT_XFERLOCAL=1`(이 PC 안에서만 열어 방화벽을 안 건드림).
  - 인터넷 중계(2026-11-23): `workers/notch-drop`(Cloudflare Worker + Durable Object, 배포는 그 폴더에서 `wrangler deploy`). 노치가 방을 열고(`/open`) 3초마다 `/inbox` 를 받아 간다. 파일 하나 95MB, 방 합 300MB, 1시간 쉬면 방째 삭제. 주소는 `NOTCH_CAT_RELAY` 로 바꿀 수 있다(집 서버로 옮길 때). 시험: `NOTCH_CAT_RELAY=<주소> NOTCH_CAT_XFEROUT=<파일>`. 휴대폰 화면 HTML 은 xfer.rs 와 worker.js 두 곳에 있다 — 고치면 둘 다.
- 실행도구모음 쪽(launch.rs, 쪽 번호 9, 2026-11-24): '＋ 등록'(파일 창, 여러 개, 바로가기는 .lnk 그대로) → 칸을 누르면 실행, ✕ 는 목록에서만 빼기. 목록 `%APPDATA%\notch-cat\launch.json`, 21개까지. 끌어다 놓기는 선반 몫이라 안 받는다. 시험 `NOTCH_CAT_PAGE=9 NOTCH_CAT_LAUNCH=<경로;경로>`.
