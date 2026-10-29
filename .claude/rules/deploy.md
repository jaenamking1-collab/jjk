---
paths:
  - "Code.gs"
  - "appsscript.json"
  - ".clasp.json"
  - ".claspignore"
  - ".github/workflows/**"
---
# 백엔드(Code.gs) 배포·실행 규칙

> CLAUDE.md 에서 옮김(2026-10-14). `Code.gs`·워크플로를 만질 때 자동으로 읽힌다. 배포는 `gas-deploy` 스킬을 먼저 부른다.

- `Code.gs` is the source of the deployed Apps Script. **Editing it here does NOT deploy it.**
  - ⛔ **배포는 에이전트가 clasp로 한다. 사용자에게 "편집기에 붙여넣고 재배포하세요"라고 시키지 마라.** 두 PC(직장/집) 모두 clasp로 배포한다. `clasp`가 없으면 **묻지 말고 설치해라**: `npm i -g @google/clasp` (2026-08-19 집 PC에서 이것 때문에 헛되이 붙여넣기를 요청했다가 지적받음 — 환경 차이는 내가 메꾼다). 사용자 몫은 `clasp login` 브라우저 '허용' 1회뿐(구글 인증은 에이전트가 못 한다).
  - **`clasp` is set up** (`.clasp.json` → project `포트폴리오관리`, id `1yYeK3W1aHUY…`). `clasp push` from the repo root uploads **only `Code.gs`** — `.claspignore` blocks everything else, which matters because `okx_nft_alert.gs` and `public_dist_proxy.gs` are **separate** Apps Script projects and `portfolio.html`/`dist_notice.html` are frontend files. Verify the file set any time with `clasp status` (should list `Code.gs` alone).
  - **`clasp push` updates the editor content, not the deployment.** Only manual `▶` runs in the editor use the saved code. **The `/exec` web app and the time-driven triggers (installed via `maint` → `resetAllTriggers`) run the deployed version** — so any `Code.gs` change they run needs a redeploy.
  - **배포 절차 (그대로 따라라)**:
    1. 임시 폴더에 `.clasp.json`만 복사해 `clasp pull` → 원격이 내 작업 전 로컬과 같은지 확인. (**저장소 안에서 `clasp pull` 금지** — 로컬 `Code.gs`가 옛 원격본으로 덮인다.)
    2. `clasp push --force` — **`--force` 없으면 매니페스트 확인 프롬프트에서 `Skipping push.`로 끝난다.** 비대화형이라 `echo y |` 파이프도 안 먹는다.
    3. `clasp deploy -i AKfycbwJS1Fd-sDCVKPLJEpEWZmPQEKAOR9pG7y-nPKZOYty65j3ArOmlDzNX2WFqiGNF_s -d "<note>"` — 라이브 배포에 새 버전을 물린다. **`-i`(기존 배포 ID) 없이 `clasp deploy` 만 치면 URL이 새로 생긴다 — 금지.** (clasp 2.4.2 엔 `redeploy` 명령이 없다.)
    4. 다시 `clasp pull`로 원격에 변경분이 들어갔는지 확인.
    5. ⛔ **배포 직후 `resetAllTriggers` 를 반드시 돌린다 — 배포의 일부다, 빼먹지 마라.**
       `clasp push`로 스코프가 바뀌면 **기존 트리거가 전부 `Authorization is required to perform that
       action.`으로 죽는다.** 트리거 목록에는 그대로 보이고 실패 메일도 한 번 오고 마니, **아무도
       모르는 채로 몇 주가 흐른다.** 재인증만으로는 안 살아나고 트리거를 다시 만들어야 한다.
       이건 이미 세 번 일어났다: 7/24~8/26 전면 정지(WORKLOG 128 부근), `snapshotPrices` **8/19→9/11
       3주 정지**, 그리고 9/8 배포 → **9/10 15:16 전면 정지**(WORKLOG 149). 매번 사용자가 "왜 안
       보이냐"고 물어서 발견됐다. 이 한 줄을 빼먹으면 그게 또 반복된다.
       - ✅ **내가 돌린다. 사용자에게 ▶ 를 부탁하지 마라** (2026-09-12 확인). `maint` 워크플로로
         `fn=resetAllTriggers` 를 돌리면 된다 — 웹앱은 소유자 권한으로 돌기 때문에 트리거 설치가
         **된다**(13개 함수 전부 ✅, 트리거 15개 재설치를 로그로 확인). `clasp run-function` 이
         안 되는 것과는 별개 통로다.
       - 화면에는 감지 장치도 있다(`showDistDead`/`renderFreshness`): 데이터가 공지창엔 6시간,
         그 밖의 날엔 30시간 넘게 안 바뀌면 분배금공지 탭에 🚨 배너가 뜬다. 배너는 안전망이지
         예방책이 아니다 — 배포 직후에 `maint` 를 돌려라.
    공개 분배금공지 페이지가 같은 `/exec` URL을 쓴다. 기존 액션의 응답 계약을 바꾸는 배포라면 먼저 알린다(액션 추가처럼 덧붙이기만 하는 변경은 그냥 배포한다).
  - ⛔ **버전 200개가 상한이고, 넘으면 `clasp deploy` 가 통째로 막힌다** (2026-09-21에 걸렸다).
    - ✅ **개수는 앱 머리글 `GAS 버전 N/200` 에 뜬다**(2026-10-01). `script-versions.yml` 이 매일 09:17 KST·배포 직후 세어 `script_versions.json` 을 커밋하고, 190개↑면 카톡+메일로 알린다. 이틀 넘게 안 갱신되면 배지가 회색 ⚠ 로 바뀐다.
    `Cannot create more versions: Script has reached the limit of 200 versions.` **버전을 지우는
    API는 없다** — `DELETE .../versions/N` 은 404 고, 안 쓰는 배포를 `clasp undeploy` 로 지워도
    버전 수는 안 줄었다(6개 지워 봤다). 사람이 편집기에서 지워야 한다.
    - **트리거도 배포본을 돈다**(2026-09-29 실측, WORKLOG 200). `resetAllTriggers` 로 깐 시간 트리거는
      push 한 저장본이 아니라 **배포본**을 실행한다(편집기 ▶ 만 저장본). 그래서 트리거가 쓰는 코드를 바꿔도
      새 버전이 필요하고, **버전 상한에 걸리면 트리거용 변경도 막힌다** — 사용자에게 버전 삭제를 바로 부탁해라.
    - **웹앱(`/exec`)만은 새 버전이 필요하다.** 프론트가 쓰는 액션을 고쳤으면 배포가 뚫릴 때까지
      화면엔 반영되지 않는다. 그땐 사용자에게 삭제를 부탁하는 수밖에 없다(구글 계정 안에서만
      되는 일이다 — 근거를 같이 줘라). **부탁할 땐 아래를 그대로 준다:**
      - 프로젝트 주소: `https://script.google.com/d/1yYeK3W1aHUYd6ok9N-dvt0YRmbB4pxX7AL0kMmZBS4-qFpRxHNASUJvG/edit`
        ⚠️ **"자산"(시트에 붙은 별개 스크립트)과 헷갈리기 쉽다** — 2026-09-21에 그쪽을 여셨다.
        구분법: "자산" 쪽 `Code.gs` 첫 줄이 `// ── 분배금 입력칸 자동 하이라이트`다.
      - 왼쪽 **🕐 프로젝트 기록** → **`버전 일괄 삭제`**(하나씩은 버전 옆 ⋮ → `이 버전 삭제`).
      - **활성 배포가 쓰는 버전은 못 지운다.** 먼저 안 쓰는 배포를 `clasp undeploy` 로 치워
        두면(저장소에서 `grep -o 'AKfycb[A-Za-z0-9_-]*'` 로 실제 쓰이는 것만 남긴다) 라이브
        하나만 묶이고 나머지는 다 지워진다.
    - 교훈: **배포를 습관처럼 하지 마라.** 프론트 파일만 바뀐 변경은 배포 대상이 아니다. `Code.gs` 가 바뀌었으면 위 규칙대로 배포한다.
  - ✅ **원격(클라우드) 세션은 `deploy.yml` 로 배포한다**(Actions, `CLASPRC_JSON` Secret — `actions_run_trigger` 로 실행). 컨테이너에서 clasp OAuth 로그인은 구글이 '확인되지 않은 앱'으로 막는다(2026-09-11 확인, `deploy.yml` 머리말). "원격이라 배포 못 한다"고 말하지 마라. **토큰은 저장소에 절대 남기지 않는다.**
  - **원격 컨테이너에서 운용사 사이트가 막혀도(`connect_rejected`) 포기하지 마라** — `fetch.yml` 워크플로에 URL 을 주면 Actions 러너가 받아 `probe-out` 브랜치에 원본(HTML·이미지·JSON·인증서 체인 `tls.txt`)을 올린다. 읽기는 `curl -H 'Accept: application/vnd.github.raw' https://api.github.com/repos/jaenamking1-collab/jjk/contents/<파일>?ref=probe-out` (raw.githubusercontent 는 캐시돼 늦다). 2026-10-01 HANARO·KIWOOM 을 이걸로 뚫었다.
- ⛔ **`/exec` 가 구글 'Access Denied — You need access'(403)면 소유자 승인 만료다**(9/28·10/6·10/13 반복). 원인: OAuth 동의 화면이 '테스트 중'이라 7일마다 만료 → 2026-10-13 **프로덕션으로 게시**(개인정보처리방침 `privacy.html`). 재배포로는 안 풀린다 — 사용자 편집기 `resetAllTriggers` ▶ + 허용만 된다(WORKLOG 258). 부탁할 땐 "왼쪽 `누르는곳.gs` → `트리거_다시깔기` ▶" 로 말한다 — 그 파일엔 누를 함수 셋만 있다(2026-10-29, 함수 고르는 칸이 열린 파일의 함수만 보여줌). 사람이 누를 함수가 새로 생기면 거기에 한글 이름으로 한 줄 더한다.
- **`clasp run-function` does not work** here — it needs the script deployed as an API executable. 하지만 **그게 "사람이 눌러야 한다"는 뜻은 아니다**: 웹앱(`/exec`)은 소유자 권한으로 도니 아래 `runMaint` 로 실행한다. `MAINT_ALLOW` 에 이름만 추가하면 트리거 설치 함수(`setupKeepWarm` 등)도 마찬가지다.
  - ✅ **진단·복구 함수는 `runMaint` 로 내가 직접 돌린다 — 소유자에게 ▶ 를 부탁하지 마라** (2026-09-12). `Code.gs` 의 `runMaint(fn, arg)` 가 `MAINT_ALLOW` 화이트리스트 함수를 실행하고 `console.log` 를 가로채 **로그까지 응답에 실어 준다.** `APP_TOKEN` 이 필요하다(`PUBLIC_ACTIONS` 아님). 실행 통로는 `.github/workflows/maint.yml`(`APP_TOKEN` Secret) — `actions_run_trigger` 로 `maint.yml` 을 `fn`/`arg` 와 함께 돌리고 로그를 읽는다.
    - 함수를 새로 만들면 **`MAINT_ALLOW` 에 이름을 추가**해야 부를 수 있다.
    - ✅ **`resetAllTriggers` 도 여기서 돈다**(2026-09-12 확인). 배포 직후 `maint` 로 돌려라 — 위 '배포 절차' 5단계.
  - One-time per machine: `npm i -g @google/clasp`(에이전트가 함) + `clasp login`과 `script.google.com/home/usersettings`의 **Apps Script API** 토글(구글 계정 행위 — 소유자가 함). 자격증명은 `~/.clasprc.json`.
  - **`portfolio.html`·`m.html`·`dist_notice.html`은 clasp 배포 대상이 아니다**(`.claspignore`가 막는다). 웹으로 서빙되니 `main` 에 푸시하면 끝.
- **Functions meant to be run by hand from the Apps Script editor go at the very bottom of `Code.gs`, under the `===== 수동 실행 =====` banner** (`setupDistTriggers`, `setupPortfolioTriggers`, `_testNoticeWindow`). Don't scatter them mid-file — they're hard to find in the editor's function dropdown otherwise. Leave a one-line pointer where the code logically belongs.
