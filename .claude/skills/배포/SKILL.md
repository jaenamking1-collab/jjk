---
name: jjk-deploy
description: jjk 의 Code.gs(Apps Script 백엔드)를 clasp 로 올리고 라이브 배포에 물린 뒤 트리거를 재설치한다. Code.gs 를 고쳤을 때 쓴다.
---

# 백엔드 배포

## 언제 쓰나

- `Code.gs` 를 고쳤을 때. **프론트(`portfolio.html`·`m.html`·`dist_notice.html`)는 배포 대상이 아니다** — `git push` 로 끝나고 사용자는 새로고침만 한다.
- 배포는 에이전트가 한다. 사용자에게 "편집기에 붙여넣고 재배포하세요"라고 시키지 않는다.

## 순서

**PC 세션 (clasp 로그인 있음)**
1. 임시 폴더에 `.clasp.json` 만 복사해 `clasp pull` → 원격이 내 작업 전 로컬과 같은지 `diff`. **저장소 안에서 `clasp pull` 금지** — 로컬 `Code.gs` 가 옛 원격본으로 덮인다.
2. `clasp status` 로 올라갈 파일이 `Code.gs` 하나인지 확인(`.claspignore` 가 나머지를 막는다).
3. `clasp push --force`.
4. `doGet`/`doPost` 경로가 바뀌었으면: `clasp deploy -i AKfycbwJS1Fd-sDCVKPLJEpEWZmPQEKAOR9pG7y-nPKZOYty65j3ArOmlDzNX2WFqiGNF_s -d "<설명>"`.
5. 다시 임시 폴더에서 `clasp pull` 로 변경분이 들어갔는지 확인.
6. `maint.yml` 을 `fn=resetAllTriggers` 로 돌린다 — **배포의 일부다.**

**원격(클라우드) 세션** — `~/.clasprc.json` 이 없으므로 Actions 로 한다.
0. 올리기 전 원격 확인: `deploy.yml` 을 `check_only=true` 로 돌리면 임시 폴더 `clasp pull` 로 편집기 코드와 커밋의 `Code.gs` 를 비교만 한다(아무것도 안 올린다). 배포할 때도 이 비교가 맨 먼저 돈다.
1. 변경을 `main` 에 푸시한다.
2. `deploy.yml` 을 돌린다(`note=<설명>`, 트리거만 쓰는 변경이면 `push_only=true`). 워크플로가 파일 확인 → `clasp push --force` → `clasp deploy -i <라이브 ID>` 를 한다.
3. `maint.yml` 을 `fn=resetAllTriggers` 로 돌린다.
- 예비: 컨테이너에서 직접 `clasp login` 을 조립하는 절차(아래 '알아둘 사실'). 9/11 엔 구글이 막았다고 적혀 있고(`deploy.yml` 주석) 9/21 엔 됐다(WORKLOG 172). `deploy.yml` 이 실패할 때만 쓴다. 로그인 확인은 `/root/.clasprc.json` 으로 한다.

## 알아둘 사실 (CLAUDE.md 에서 옮김, 2026-09-28)

- `.clasp.json` → 프로젝트 `포트폴리오관리`(id `1yYeK3W1aHUY…`). `clasp push` 는 **`Code.gs` 만** 올린다 — `.claspignore` 가 나머지를 막는다. `okx_nft_alert.gs`·`public_dist_proxy.gs`·`stock_sheet.gs`·`ticker_ratio_alert.gs` 는 **별개** Apps Script 프로젝트다.
- `clasp` 가 없으면 **묻지 말고 설치한다**: `npm i -g @google/clasp`(원격 수동 로그인은 `@2.4.2`). PC 마다 한 번: `clasp login`(사용자의 '허용' 클릭) + `script.google.com/home/usersettings` 의 **Apps Script API** 토글(소유자). 자격증명은 `~/.clasprc.json` — **토큰은 저장소에 절대 남기지 않는다.**
- **`clasp push` 는 편집기 코드만 바꾼다.** 시간 트리거와 편집기 ▶ 는 저장된 코드를 쓰니 바로 반영되고, **`/exec` 웹앱은 재배포 전까지 옛 버전**을 돈다. `clasp 2.4.2` 엔 `redeploy` 가 없다 — `deploy -i` 가 그것이다.
- **`clasp run-function` 은 안 된다**(API executable 배포가 필요). 대신 `runMaint(fn, arg)` — `/exec` 가 소유자 권한으로 `MAINT_ALLOW` 화이트리스트 함수를 실행하고 `console.log` 를 응답에 실어 준다. 통로는 `maint.yml`(`APP_TOKEN` Secret). 새 함수는 **`MAINT_ALLOW` 에 이름을 추가**해야 부를 수 있다. 트리거 설치 함수도 이걸로 돈다.
- 트리거가 죽으면 **트리거 목록엔 그대로 보이고 실패 메일도 한 번 오고 만다.** 재인증만으로는 안 살아나고 다시 만들어야 한다. 화면 안전망: `showDistDead`/`renderFreshness` — 공지창 6시간, 평소 30시간 넘게 데이터가 안 바뀌면 분배금공지 탭에 🚨 배너. 배너는 예방책이 아니다.
- **원격 수동 로그인(예비)**: `clasp login` 의 콜백 서버는 컨테이너 안에 떠서 사용자 브라우저가 못 닿는다. 인증 URL 을 직접 만든다 — clasp 공개 client(`1072944905499-vm2v2i5dvn0a0d2o4ca36i1vge8cvbn0.apps.googleusercontent.com`, secret 은 `build/src/auth.js` 안), `redirect_uri=http://localhost:33353`, scope 는 `clasp login` 이 찍는 것 그대로. 사용자가 허용하면 **연결 실패 페이지**가 뜨는데 정상 — 주소창의 `code=` 를 받아 `oauth2.googleapis.com/token` 에 교환하고 `~/.clasprc.json` 을 `{token, oauth2ClientSettings, isLocalCreds:false}` 로 쓴다. `clasp login --status` 로 확인. 콜백 포트를 `curl` 로 찔러 보지 마라(`state mismatch`), `pkill -f clasp` 는 내 셸을 죽인다(WORKLOG 172).
- **버전 200개 상한**(`Cannot create more versions`): 버전을 지우는 API 는 없다(`DELETE .../versions/N` 404, `undeploy` 해도 수는 안 준다). 그동안에도 `doGet`/`doPost` 를 안 타는 일은 `push` 만으로 돌릴 수 있다 — 5분마다 도는 `keepWarm` 의 하루 한 번 따라잡기(`assetDay` 속성)가 `pushTrendData`·`markInputCells` 를 돌린다. 웹앱을 고쳐야 하면 사용자에게 삭제를 부탁하고 **아래를 그대로 준다**:
  - 프로젝트 주소: `https://script.google.com/d/1yYeK3W1aHUYd6ok9N-dvt0YRmbB4pxX7AL0kMmZBS4-qFpRxHNASUJvG/edit`
  - ⚠️ "자산"(시트에 붙은 별개 스크립트)과 헷갈리기 쉽다(2026-09-21 그쪽을 여셨다). 구분법: "자산" 쪽 `Code.gs` 첫 줄이 `// ── 분배금 입력칸 자동 하이라이트`.
  - 왼쪽 **🕐 프로젝트 기록** → **`버전 일괄 삭제`**(하나씩은 ⋮ → `이 버전 삭제`).
  - 활성 배포가 쓰는 버전은 못 지운다. 먼저 안 쓰는 배포를 `clasp undeploy` 로 치운다(`grep -o 'AKfycb[A-Za-z0-9_-]*'` 로 저장소가 실제 쓰는 것만 남긴다).

## 결정 규칙

- **`doGet`/`doPost` 가 타는 코드를 바꿨으면 재배포**(`deploy -i`). 트리거·편집기 ▶ 만 쓰는 코드는 `push` 만으로 반영된다 — **배포를 습관처럼 하지 마라**(버전 200개 상한).
- 공개 분배금공지 페이지가 같은 `/exec` 를 쓴다. **기존 액션의 응답 형식을 바꾸면 먼저 알린다.** 액션 추가처럼 덧붙이기만 하면 그냥 배포한다.
- `clasp deploy` 가 `Cannot create more versions` 로 막히면 버전 200개 상한이다 — 위 '알아둘 사실'의 부탁 문구를 그대로 쓴다.
- `/exec` 가 드라이브식 **'You need access' 403** 이면 코드·배포 설정 문제가 아니다. 소유자 실행 승인이 풀린 것 — 편집기에서 `resetAllTriggers` ▶ 로 재승인을 부탁한다. 이때는 `maint` 도 `/exec` 를 타므로 못 쓴다.

## 완료 확인

- `.claude/CHECKS.md` 의 **배포 직후** 네 항목을 모두 통과해야 "배포했다"고 쓴다.
- `maint` 로그에 `resetAllTriggers` 의 `✅` 줄(트리거 15개)이 찍혔다.
- `probe.yml` 로 고친 액션을 쳐서 `HTTP 200` 과 새 응답을 봤다.

## 과거에 틀렸던 것

- `--force` 를 빼서 매니페스트 확인 프롬프트에서 `Skipping push.` 로 끝났다(비대화형이라 `echo y |` 도 안 먹는다).
- `-i` 없이 `clasp deploy` → **URL 이 새로 생긴다.** 앱·공개 페이지가 옛 URL 을 보고 있어 반영이 안 된다.
- 트리거 재설치를 빠뜨려 조용히 멈췄다: 7/24~8/26 전면 정지, `snapshotPrices` 8/19→9/11 **3주**, 9/8 배포 → 9/10 전면 정지(WORKLOG 149). 매번 사용자가 "왜 안 보이냐"고 물어서 발견됐다.
- `getDistribution` 파서 수정을 push 만 하고 "재배포 불필요"라고 했다 — 웹앱은 옛 버전을 계속 돌렸다(WORKLOG 93).
- "원격이라 배포 못 한다"고 했다 — 막힌 건 `script.google.com` 하나뿐이었고 Actions 로 된다.
- 9/18 배포가 상해 `/exec` 가 익명 403 으로 사흘 죽었다. 같은 ID 로 다시 `deploy` 하자 즉시 복구(WORKLOG 165). 9/28 의 403 은 모양이 같았지만 원인이 승인 풀림이었다(WORKLOG 180) — **응답 페이지가 드라이브식 'You need access' 인지 먼저 본다.**
