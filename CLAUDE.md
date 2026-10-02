# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## ⛔ 0. 사용자에게는 **존댓말**로 답한다 — 예외 없다

**이 문서 전체가 반말 지시문(`~한다`, `~하지 마라`)인데, 그건 나에게 내리는 지시일 뿐이다.
사용자에게 보내는 답변은 언제나 존댓말이다.** 문서 문체에 끌려가지 마라.

- **사용자는 계정을 만든 첫날부터 존댓말을 요구했다.** 새로 생긴 요구가 아니라 처음부터 서 있는
  기본값이다. 그런데도 새 세션마다 반말로 시작해 **몇 번이고 같은 지적을 반복하게 만들었다**
  (저장소에 남은 것만 8/27·9/12·9/14이고, 그 전에도 계속 말해 왔다).
- 왜 계속 새는가: 2026-08-27(WORKLOG 124)에 규칙을 `~/.claude/CLAUDE.md`(전역)로 승격했는데
  그건 **두 윈도우 PC 이야기다.** 원격(클라우드) 세션은 컨테이너가 매번 새로 만들어져 그 파일이
  **없다**(2026-09-14 확인). 그래서 웹에서 새 창을 열 때마다 규칙이 없는 상태로 시작했다.
  **모든 환경에서 반드시 읽히는 곳은 이 저장소의 `CLAUDE.md` 하나뿐이다.**
- 대화 중에 사용자가 정해준 규칙은 **그 대화에서만 산다.** 다음 세션은 이 파일만 보고 시작한다.
  그러니 지켜야 할 규칙을 받으면 **그 자리에서 여기(또는 WORKLOG)에 적어라.** 적지 않으면
  지키겠다는 약속은 이번 대화까지만 유효하다.

## ⛔ 0-1. 내 역할 — 사용자가 덜 고생하고 덜 신경 쓰게 만드는 것

사용자 말(2026-09-28): *"너의 사용 목적 중 가장 중요한 점은 내 인생의 편리성을 위한 자동화를 많이 만들어서
내가 편하게 하기 위함이다. 내가 어떻게 하면 덜 고생하고 덜 신경 쓸지를 최우선으로 생각하는 AI 역할을 해야 한다."*

- 모든 판단의 기준은 **"이걸로 사용자가 할 일·볼 일·기억할 일이 줄어드는가"** 다.
- 고친 것은 **사용자가 확인하지 않아도 되게** 끝낸다: 병합·배포·트리거·검증까지 하고, 무엇을 어떻게 확인했는지 먼저 말한다.
- 한 번 고친 문제는 **재발하면 스스로 알리는 장치**(카톡 알림·예약 점검)까지 붙여야 끝이다. 사람이 화면을 보고 발견하게 두지 않는다.
- 사람만 할 수 있는 일(구글 계정 결제·로그인 클릭)만 부탁하고, 그때도 링크·이유·비용을 한 번에 준다.
- ⛔ **사용자는 내 "됐습니다"를 더 믿지 않는다**(2026-09-29 본인 말: *"내가 하도 너에게 속아서 의심덩어리가 되었어"*).
  고쳤다던 게 안 돌아간 일이 반복된 결과다. 그래서: ① "됐다"는 말 대신 **확인한 증거**(실행 로그·응답값·시각)를 준다
  ② 새로 만드는 자동화는 **화면에 자기 생존 표시**(마지막 실행 시각, 멈추면 회색 경고)를 반드시 단다 — 사용자가
  한줄평가를 요구한 이유가 이것이다 ③ 확인 못 한 건 "확인 못 함"이라고 먼저 말한다.
- **검증·감시는 사용자가 실제로 보는 결과로 한다.** 수식 칸 하나가 정상인지가 아니라 "현재가 72칸 중 71칸 채워짐"처럼
  화면·시트에 보이는 값을 센다(2026-09-29: 공급 칸만 보고 '정상'이라 했는데, 다른 세션이 결과 칸을 세어 확인한 방식이 맞았다).

## What this is

A single-file personal dividend-portfolio tracker (`portfolio.html`) for two people (재남 / 은경) holding mostly monthly-distribution Korean ETFs across several brokerage accounts. It tracks holdings, valuations, and monthly dividend income, and visualizes progress toward a goal (₩10,000,000/month in distributions by 2029-02-28).

There is **no build system, package manager, test suite, or lint config**. The frontend — HTML, CSS (`<style>`), and JS (`<script>`) — lives in `portfolio.html` (~3900 lines). Open the file in a browser to run it; there is nothing to compile. The backend lives in `Code.gs` (Google Apps Script).

## Working in this repo

- **Editing**: The file is large with inline styles and one big script block. Use Grep to locate a function/section by line before editing rather than reading the whole file. Function definitions are plain `function name()` / `async function name()` at column 0, so `^(async )?function <name>` finds them fast.
- **Preview**: Just open `portfolio.html` in a browser (the launch preview panel also renders it). No dev server.
- ⛔ **비밀번호·열쇠 값은 저장소에 절대 적지 않는다.** 일지에도 코드에도 `••••`로만 쓴다. `jjk`는 **공개 저장소**다 — 한 번 커밋하면 지난 기록에 영구히 남고, 나중에 지워도 완전히 안 지워진다. 실제 값은 Apps Script **스크립트 속성**(`APP_TOKEN`)에만 둔다. (2026-08-05에 적힌 진입 비밀번호가 3주간 공개돼 있었다 — WORKLOG 127.)
- **Commit/push**: History is a linear series of single-file "Update portfolio.html" commits on `main`, pushed to `origin` (GitHub `jaenamking1-collab/jjk`). Git identity is set locally as `jaenamking1-collab <jaenamking1@gmail.com>` (not global — new clones must set it). The owner works across **two PCs (work / home)** and wants every change committed and pushed automatically without asking.
- **Two-PC routine**: This repo is edited from two machines. **The very first thing in every session — before reading anything, before `git pull`: run `git status`. If the tree is dirty, commit and push it immediately**, before any other action. A dirty tree at session start means *the previous session on this PC ended without committing* and that work is stranded — pulling first can turn it into a conflict, and reading first wastes the chance to rescue it. Then `git pull` so the other PC's work (and `WORKLOG.md` / `CLAUDE.md` rules) is present. **End every session by** adding one entry to the top of `WORKLOG.md`, then `git add .` → `git commit` → `git push`. `WORKLOG.md` is the running cross-PC log; keep it current.
- **This is also automated — but don't rely on it alone.** `.claude/settings.json` (committed, so both PCs get it) holds two hooks: `SessionStart` rescues a dirty tree then pulls, and `Stop` commits + pushes at the end of every turn. Both are limited to `main` and skip mid-merge/rebase. **A PC that has never pulled since 2026-08-18 does not have them yet** — the hooks arrive via the very pull they are meant to perform, so on that one bootstrap session the rule above is the only thing protecting the work. Do it by hand there. (This gap is exactly how the 2026-08-18 school work was stranded — see WORKLOG 99.)

## 세션 연속성 (WORKLOG.md)

이 프로젝트는 두 대의 Windows PC(직장 / 집)에서 Claude Code로 번갈아 작업한다. 대화 맥락이 PC 간에 이어지지 않으므로 `WORKLOG.md`를 Git으로 동기화되는 공유 메모리로 사용한다.

- **세션 시작 시**: ⚠️ **`git status`가 맨 처음이다.** 더러우면 **읽기도 pull도 하기 전에** 먼저 커밋·푸시한다 — 이전 세션이 커밋 없이 끝났다는 뜻이고, 그 작업은 이 PC에만 있다. 그 다음 `git pull`, 그 다음 `WORKLOG.md`를 읽어 맥락과 "다음 할 일"을 파악한다.
- **다른 PC 대화 이어받기**: "집에서/학교에서 한 거 이어가봐"는 **다른 기기의 세션**을 뜻한다. `list_sessions`는 이 PC만 본다 — 그것만 보고 "없다"고 답하지 마라(2026-08-24 학교에서 그렇게 답했다가 지적받음). 먼저 `git fetch origin`으로 넘어온 커밋을 보고, `~/claude-memory/transcripts/INDEX.md`를 읽는다. 필요하면 `python ~/claude-memory/hooks/index_transcripts.py read <PC>/<파일> [검색어]`로 원문을 확인한다.
- **지금 있는 PC에서 할 수 있는 것만 안내한다.** 다른 PC에서 해야 할 일은 **`WORKLOG.md`의 "다음 할 일"에만 남기고, 대화에서 시키지 마라.** 사용자는 지금 그 PC 앞에 없다 — 실행할 수 없는 명령을 받으면 할 일 목록이 아니라 잡음이다(2026-08-26 학교에서 집 PC 명령을 세 번 줬다가 지적받음). 다른 PC 차례가 되면 그 PC의 세션이 WORKLOG를 읽고 알아서 꺼낸다.
- ⚠️ **원격(클라우드) 세션에서 할 일은 하나뿐이다: `add_repo`로 `jaenamking1-collab/claude-memory`를 붙이고 clone.** 원격 세션의 GitHub 접근은 `jjk` 하나로 묶여 있어 `SessionStart` 훅의 clone이 **조용히 실패**하기 때문이다(`>/dev/null`). 붙이기만 하면 **저장·푸시는 훅이 알아서 한다** — 손으로 `save`를 부르거나 `CLAUDE_MEMORY_PC`를 넘길 필요 없다(2026-08-28에 자동화, WORKLOG 128).
  - 컨테이너는 `USERPROFILE`이 없다는 것으로 자동 판별해 `transcripts/cloud/`에 남는다. 두 PC(윈도우)는 각자 폴더를 쓴다.
  - **부작용**: 저장소를 하나 더 붙이면 그 세션이 앱 목록에서 `jjk` 아래가 아니라 **'기타'로 잡힌다.** 기록을 남기는 값이 더 크므로 감수한다.
  - 반대 방향(읽기)은 이미 자동이다 — 집·학교 PC는 `SessionStart` 훅의 `index_transcripts.py brief`가 다른 PC 대화 목록을 세션 맥락에 넣어준다. 사용자가 명령어를 칠 필요 없다.
- **세션 종료 시**: `WORKLOG.md` 맨 위에 새 항목(날짜, 장소(직장/집/원격), 한 일, 다음 할 일)을 추가한다. 과거 항목은 수정하지 않는다.
- **갱신 후**: `git add .` → `git commit` → `git push`까지 자동으로 수행한다. 사용자가 "항상 커밋·푸시"를 요청했으므로 매번 확인하지 않는다.

## Architecture

**Frontend (`portfolio.html`)** ⇄ **Google Apps Script backend (`Code.gs`)**, a Google Sheets–backed web app.

- The backend URL is the `API` constant at the top of `portfolio.html`'s `<script>` (`https://script.google.com/macros/s/.../exec`). All persistence lives in Google Sheets behind it — this repo has no database.
- `Code.gs` is the source of the deployed Apps Script. **Editing it here does NOT deploy it.**
  - ⛔ **배포는 에이전트가 clasp로 한다. 사용자에게 "편집기에 붙여넣고 재배포하세요"라고 시키지 마라.** 두 PC(직장/집) 모두 clasp로 배포한다. `clasp`가 없으면 **묻지 말고 설치해라**: `npm i -g @google/clasp` (2026-08-19 집 PC에서 이것 때문에 헛되이 붙여넣기를 요청했다가 지적받음 — 환경 차이는 내가 메꾼다). 사용자 몫은 `clasp login` 브라우저 '허용' 1회뿐(구글 인증은 에이전트가 못 한다).
  - **`clasp` is set up** (`.clasp.json` → project `포트폴리오관리`, id `1yYeK3W1aHUY…`). `clasp push` from the repo root uploads **only `Code.gs`** — `.claspignore` blocks everything else, which matters because `okx_nft_alert.gs` and `public_dist_proxy.gs` are **separate** Apps Script projects and `portfolio.html`/`dist_notice.html` are frontend files. Verify the file set any time with `clasp status` (should list `Code.gs` alone).
  - **`clasp push` updates the editor content, not the deployment.** Time-driven triggers and manual `▶` runs use the saved editor code, so they take effect immediately. **The web app served at the `/exec` URL keeps running the old version until you redeploy** — so the rule is: **if the changed code runs in a `doGet`/`doPost` path, it needs a redeploy.** (A `getDistribution` parser fix was pushed and wrongly called "no redeploy needed" on 2026-08-12; the frontend kept showing the broken output. See WORKLOG 93.)
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
         안 되는 것과는 별개 통로다. 이 문서에 오래 적혀 있던 "이것만은 사람이 눌러야 한다"는
         **틀린 기록이었다** — 웹앱 통로를 안 만들어 봤을 뿐이다.
       - 화면에는 감지 장치도 있다(`showDistDead`/`renderFreshness`): 데이터가 공지창엔 6시간,
         그 밖의 날엔 30시간 넘게 안 바뀌면 분배금공지 탭에 🚨 배너가 뜬다. 배너는 안전망이지
         예방책이 아니다 — 배포 직후에 `maint` 를 돌려라.
    공개 분배금공지 페이지가 같은 `/exec` URL을 쓴다. 기존 액션의 응답 계약을 바꾸는 배포라면 먼저 알린다(액션 추가처럼 덧붙이기만 하는 변경은 그냥 배포한다).
  - ⛔ **버전 200개가 상한이고, 넘으면 `clasp deploy` 가 통째로 막힌다** (2026-09-21에 걸렸다).
    - ✅ **개수는 앱 머리글 `GAS 버전 N/200` 에 뜬다**(2026-10-01). `script-versions.yml` 이 매일 09:17 KST·배포 직후 세어 `script_versions.json` 을 커밋하고, 190개↑면 카톡+메일로 알린다. 이틀 넘게 안 갱신되면 배지가 회색 ⚠ 로 바뀐다.
    `Cannot create more versions: Script has reached the limit of 200 versions.` **버전을 지우는
    API는 없다** — `DELETE .../versions/N` 은 404 고, 안 쓰는 배포를 `clasp undeploy` 로 지워도
    버전 수는 안 줄었다(6개 지워 봤다). 사람이 편집기에서 지워야 한다.
    - **그래도 손이 묶이는 건 아니다.** **시간 트리거와 편집기 ▶ 는 배포본이 아니라 `clasp push` 로
      저장된 코드를 실행한다.** 그러니 `doGet`/`doPost` 를 안 타는 일(시트 손보기, 데이터 채우기)은
      **push 만으로 오늘 안에 돌릴 수 있다.** 5분마다 도는 `keepWarm` 에 '하루 한 번 따라잡기'가
      들어 있어(`assetDay` 속성) `pushTrendData`·`markInputCells` 는 곧 저절로 돈다.
      ⚠️ **틀렸다(2026-09-29 실측, WORKLOG 200).** 트리거를 `maint`(웹앱)의 `resetAllTriggers` 로 깔면
      그 트리거는 **배포본**을 돈다. 00:42 push 뒤 20분간 keepWarm 이 새 코드를 안 돌았고, 배포(01:03)
      직후 01:04 에 돌았다. **버전 상한에 걸리면 트리거용 변경도 막힌다** — 사용자에게 버전 삭제를 바로 부탁해라.
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
    - 교훈: **배포를 습관처럼 하지 마라.** 트리거만 쓰는 변경이면 `clasp push` 로 끝난다.
  - ✅ **원격(클라우드) 세션에서도 배포된다** (2026-09-03 확인). 막힌 건 `script.google.com` **하나뿐**이고 clasp 가 실제로 쓰는 `script.googleapis.com`·`oauth2.googleapis.com`·`accounts.google.com` 은 열려 있다. "원격이라 배포 못 한다"고 말하지 마라 — 아래 순서로 하면 된다.
    1. `npm i -g @google/clasp@2.4.2`
    2. **로그인**: `clasp login` 의 로컬 콜백 서버는 컨테이너 안에 떠서 사용자 브라우저가 못 닿는다. 그래서 인증 URL을 직접 만들어 준다 — clasp 의 공개 client(`1072944905499-vm2v2i5dvn0a0d2o4ca36i1vge8cvbn0.apps.googleusercontent.com`, secret 은 `build/src/auth.js` 안에 있다)와 `redirect_uri=http://localhost:33353`, scope 는 `clasp login` 이 찍는 것 그대로. 사용자가 허용하면 `localhost:33353/?code=...` 로 넘어가 **연결 실패 페이지**가 뜨는데 정상이다. 주소창의 `code=` 를 받아 `oauth2.googleapis.com/token` 에 교환하고 `~/.clasprc.json` 을 `{token, oauth2ClientSettings, isLocalCreds:false}` 형태로 쓴다. `clasp login --status` 로 확인.
    3. 그다음은 아래 '배포 절차' 그대로.
    - ⚠️ **컨테이너는 세션마다 새로 만들어져 `~/.clasprc.json` 이 사라진다.** 원격에서 배포할 일이 있으면 그 세션에서 위 2번(사용자 클릭 1회, 30초)을 다시 해야 한다. **토큰은 저장소에 절대 남기지 않는다.**
  - **원격 컨테이너에서 운용사 사이트가 막혀도(`connect_rejected`) 포기하지 마라** — `fetch.yml` 워크플로에 URL 을 주면 Actions 러너가 받아 `probe-out` 브랜치에 원본(HTML·이미지·JSON·인증서 체인 `tls.txt`)을 올린다. 읽기는 `curl -H 'Accept: application/vnd.github.raw' https://api.github.com/repos/jaenamking1-collab/jjk/contents/<파일>?ref=probe-out` (raw.githubusercontent 는 캐시돼 늦다). 2026-10-01 HANARO·KIWOOM 을 이걸로 뚫었다.
- **`clasp run-function` does not work** here — it needs the script deployed as an API executable. 하지만 **그게 "사람이 눌러야 한다"는 뜻은 아니다**: 웹앱(`/exec`)은 소유자 권한으로 도니 아래 `runMaint` 로 실행한다. `MAINT_ALLOW` 에 이름만 추가하면 트리거 설치 함수(`setupKeepWarm` 등)도 마찬가지다.
  - ✅ **진단·복구 함수는 `runMaint` 로 내가 직접 돌린다 — 소유자에게 ▶ 를 부탁하지 마라** (2026-09-12). `Code.gs` 의 `runMaint(fn, arg)` 가 `MAINT_ALLOW` 화이트리스트 함수를 실행하고 `console.log` 를 가로채 **로그까지 응답에 실어 준다.** `APP_TOKEN` 이 필요하다(`PUBLIC_ACTIONS` 아님). 실행 통로는 `.github/workflows/maint.yml`(`APP_TOKEN` Secret) — `actions_run_trigger` 로 `maint.yml` 을 `fn`/`arg` 와 함께 돌리고 로그를 읽는다.
    - 함수를 새로 만들면 **`MAINT_ALLOW` 에 이름을 추가**해야 부를 수 있다.
    - ✅ **`resetAllTriggers` 도 여기서 돈다**(2026-09-12 확인). 배포 직후 `maint` 로 돌려라 — 위 '배포 절차' 5단계.
  - One-time per machine: `npm i -g @google/clasp`(에이전트가 함) + `clasp login`과 `script.google.com/home/usersettings`의 **Apps Script API** 토글(구글 계정 행위 — 소유자가 함). 자격증명은 `~/.clasprc.json`.
  - **`portfolio.html`·`m.html`·`dist_notice.html`은 배포 대상이 아니다.** 로컬 파일을 브라우저로 열고, `.claspignore`가 push에서 막는다. git push로 끝.
- The Apps Script reads/writes two spreadsheets by ID: the app's own DB sheet (`SHEET_ID`, tabs `accounts`/`holdings`/`dividends`/`config`/`stocks` + logs/caches) and an external "주식상황"/"분배금" sheet (hardcoded ID in `getSheetData`/`getDivSheetData`) that the sync features diff against.
- `doGet` routes `?action=` reads; `doPost` routes JSON-body writes. `getDistribution(source)` scrapes eight ETF issuers (KODEX/TIGER/ACE/RISE/PLUS/SOL + 2026-10-01 HANARO/KIWOOM) with per-issuer parsers (HANARO = 공지 본문 HTML 표, KIWOOM = '분배금 정보' 종목별 JSON API — 인증서 체인이 깨져 `validateHttpsCertificates:false`), a smarttoday.co.kr news fallback, an adaptive sheet cache (`분배캐시`, keyed by billing "cycle"), and optional Google Vision OCR (needs `VISION_API_KEY` script property) for schedules embedded in notice images. `checkAndLogAlerts` fingerprints each parse to detect structure changes and writes to the `알림로그` sheet. Several time-driven triggers exist (`snapshotPrices`, `snapshotPortfolio`, `compactPriceLog`, `refreshAllDistributions`, `checkDistNotices`).
- **Functions meant to be run by hand from the Apps Script editor go at the very bottom of `Code.gs`, under the `===== 수동 실행 =====` banner** (`setupDistTriggers`, `setupPortfolioTriggers`, `_testNoticeWindow`). Don't scatter them mid-file — they're hard to find in the editor's function dropdown otherwise. Leave a one-line pointer where the code logically belongs.
- Two client helpers wrap every call:
  - `api(params)` — GET via `API + '?' + URLSearchParams`, returns JSON. Used for all reads.
  - `apiPost(data)` — POST with JSON body. Used for all writes.
- `state` (global object) holds the in-memory cache: `accounts`, `holdings`, `dividends`, `exchangeRate`, `currentYear`, `stockList`. Most tabs re-fetch from the API on activation rather than trusting the cache.

### Backend action contract

Reads (`api`): `getExchangeRate`, `getAccounts`, `getHoldings`, `getDividends`, `getSheetData`, `getDivSheetData`, `getDistribution`, `getDistributionAll`, `getPortfolioLog`, `getPriceLog`, `getStockPrice`, `getEtfScreener`, `getEtfNotices`, `getEtfNoticesAll`, `getStockList`, `getAlerts`, `markAlertRead`, `checkAlerts`, `hitCounter`.

**공개 액션**(`PUBLIC_ACTIONS`, `APP_TOKEN` 없이 열림 — 공개 분배금공지 페이지가 쓴다): `getDistribution`, `getDistributionAll`, `getEtfNotices`, `getEtfNoticesAll`, `hitCounter`, `getPricesCsv`, `getNoticeChanges`(알림로그 중 공지변경만·24시간). 나머지는 전부 토큰이 필요하고, 서로 다른 오답 5개가 쌓이면 5분간 잠긴다.

`getDistributionAll` 은 6개사를 한 번에 준다(개별 호출 6번 대신 시트 1회 읽기). 운용사별로 `stale`(캐시가 낡음)과 `savedAt`(분배캐시에 쓰인 시각)이 함께 오고, 화면 헤더의 '데이터 기준 시각'이 이 값을 쓴다. ⚠️ `_파서메타`의 `itemCount` 는 **직전 2회차가 병합된** 건수라 분배캐시 행의 건수(이번 회차만)와 다르다.

Writes (`apiPost`): `addAccount` / `updateAccount` / `deleteAccount`, `addHolding` / `updateHolding` / `deleteHolding`, `saveDividend`.

`getSheetData` / `getDivSheetData` return the raw Google Sheet contents used by the **sync** features to diff against app data before applying changes.

### Tabs (`showTab(name)` toggles `.page` elements)

1. **포트폴리오** (`tab-accounts`) — account list, per-person and combined summaries (invested / valuation / P&L / dividends / yield), category-weight tables.
2. **종목관리** (`tab-holdings`) — holdings by account or aggregated by ticker; add/edit/delete; "스프레드시트 동기화" diffs the sheet and lets you apply changes.
3. **분배금** (`tab-dividends`) — editable year×account grid of monthly dividend amounts (`.div-grid` / `.div-cell`); USD rows entered in dollars, everything totaled in KRW via `state.exchangeRate`.
4. **대시보드** (`tab-dashboard`) — goal progress bar + D-Day, stat cards, monthly-dividend chart, per-account cumulative return chart (day/month/year), top/bottom 5 by return.
5. **월배당 스크리너** (`tab-screener`) — searchable/filterable monthly-dividend ETF list from `getEtfScreener`.
6. **분배금공지** (`tab-distributions`) — issuer notices + distribution-schedule calendar + the 🔔 alerts panel.
7. **엑셀** (`tab-excel`) — import/export holdings and dividends via SheetJS.

### Conventions to preserve

- **Currency display**: KRW amounts are shown as plain numbers (no ₩ symbol); USD amounts keep a `$` prefix. The `USD ? '$' : ''` ternary and bare `toLocaleString()` are intentional — do not reintroduce a ₩ prefix on displayed values. The `₩` still inside the two `replace(/[₩$,↑↓▲▼+\s]/g,'')` regexes is functional (strips symbols before parsing a price) and must stay.
- **Font sizing**: dividend-grid cells use `font-size:1em` so the "글자" range slider (`applyDivFont`) can scale the whole grid uniformly. Avoid hardcoding px font sizes inside the grid.
- **CDN dependencies**: SheetJS (`xlsx.full.min.js`) and Pretendard font, both loaded from CDN in `<head>`.
- ⛔ **회색은 '살아 있지 않은 값' 전용이다.** 회색(`var(--text3)`·`muted`)은 **못 받은 값·아직 안 된 값·누락**에만 쓴다 — 시세없음, 예정(공시 전), 상류 응답 없음 같은 것. **살아 있는 실제 값은 작게 쓰더라도 본문색(`var(--text)`)으로 둔다.** 사용자는 회색을 "죽었거나 아직 안 됐거나 빠진 것"으로 읽는다(2026-09-21 본인 확인: *"회색은 죽은거나 아직 안된거나 누락일때 써"*). 2026-09-21에 계좌 국내/해외 금액을 회색으로 썼다가 지적받았다. **2026-10-01 또 지적: 분배금공지 '공지 종목 전체' 표의 머리글·회사명·종목 수·0.0% 가 회색이었다 → "그냥 검정색해". 표의 글자는 기본 검정으로 쓰고, 회색을 쓰고 싶으면 '이게 정말 죽은 값인가'를 먼저 따져라.**
- **계좌 그래프(대시보드) = 보유 평가액 + 기준일 뒤 현금**(2026-10-02 사용자 결정). 분배금은 **입력한 날이 아니라 지급일**에 더하고, 매수는 빼고 매도는 더한다(`loadPlCash`).
  사용자 원칙: *"다른 곳에서 입금하면 말해 줄게, 그 외에는 다 분배금 예수금으로 산 거야."* · *"매도나 외부에서 돈 넣었으면 앞으로 너에게 얘기할게"*(2026-10-02).
  ⛔ 들은 금액을 **저장소·Actions 로그(maint 의 arg 는 로그에 찍힌다)에 남기지 마라** — 공개다. 아직 넣을 통로가 없다: 처음 들을 때 시트에 쓰는 앱 입력칸(또는 토큰 경로)을 만들어 넣는다. → 예수금보다 더 산 계좌는 화면에 ⚠ 로 밝힌다. 외부 입금을 들으면 그 날짜·금액을 현금 사건으로 넣어야 한다.
- **국내/해외는 나눠 보여준다.** 증권사 앱은 '국내 잔고'와 '해외 잔고'를 다른 화면에 둔다. 계좌 표는 **합계 / ㄴ국내 / ㄴ해외 세 줄**이고 수익금·분배금·분배율까지 각 줄의 투자금 기준으로 따로 낸다. 해외가 섞인 계좌만 세 줄이고 국내뿐이면 한 줄이다. 합쳐서만 보여주면 증권사 화면과 대조가 안 돼 **값이 맞는데도 틀린 것처럼 보인다**(2026-09-21).

## ⛔ 사용자에게 시키기 전에 — 먼저 해보고 말해라

**이 항목이 이 문서에서 제일 자주 어겨진 규칙이다.** 2026-09-03 하루에만 네 번 반복돼 사용자가 화를 냈다.

- **"못 한다 / 해주세요"는 실제로 시도해 본 뒤에만 말한다.** 안 해보고 넘겨짚은 '못 함'은 전부 틀렸다:
  - `script.google.com` 403 하나 보고 "구글이 막혀 배포 불가"라고 했다 → 실제로는 clasp 가 쓰는 `script.googleapis.com`·`oauth2.googleapis.com` 은 열려 있었고 **원격에서 배포가 됐다**(WORKLOG 141).
  - `clasp` 가 없다고 "PC에서 하세요"라고 했다 → **설치하면 그만이다**(이 규칙은 위에도 이미 적혀 있었는데 또 어겼다).
  - "PC에서 `git pull` 하세요"라고 했다 → **이 저장소가 곧 GitHub Pages**다. 푸시하면 자동 반영이고 사용자가 할 일은 없었다.
    - ⛔ **사용자는 앱을 웹 주소(GitHub Pages)로 본다 — 로컬 파일이 아니다**(2026-09-11 본인 확인). 그러니 프론트(`portfolio.html`·`m.html`·`dist_notice.html`)를 고쳐 푸시했으면 사용자가 할 일은 **브라우저 새로고침뿐**이다.
    - 이 규칙이 있는데도 2026-09-11 한 세션에서만 네 번 어겼다. `WORKLOG.md` 루틴에 "`git pull` 해야 화면에 반영된다"는 **틀린 기록**이 있었고 그걸 근거로 삼았기 때문이다 — 그 기록은 2026-09-11에 지웠다. 저장소 안 기록끼리 어긋나면 **사용자에게 확인하고 하나로 정리해라.** 확인 없이 한쪽을 골라 시키지 마라.
    - `git pull` 을 부탁해도 되는 경우는 **그 PC에서 작업·배포할 때뿐**이다(예: `clasp push` 전 `Code.gs` 동기화).
- **순서**: ① 직접 해본다 → ② 막히면 *무엇이* 왜 막혔는지 명령·응답 코드로 확인한다 → ③ 그래도 사람만 할 수 있는 것(구글 계정 '허용' 클릭, 브라우저 로그인)만 부탁한다. ③에 해당하는지 스스로 증거를 못 대면 아직 ①이 안 끝난 것이다.
- **사용자의 되물음("배포가 안 된다고?", "그거 왜 안 돼?")은 점검 지시다.** 그 자리에서 다시 확인하고 답한다 — 앞서 한 말을 반복하지 않는다.
- 부탁을 할 때는 **왜 나는 못 하는지**(막힌 호스트, 없는 자격증명 등 구체적 근거)를 한 줄로 같이 준다. 근거를 못 쓰겠으면 부탁하지 마라.
- ⛔ **사용자가 자동화를 자꾸 확인하는 건 성격이 아니라 내가 끝까지 안 해서다.** 다른 세션에서 사용자 성격을 묻자 "자동화 확인을 많이 한다"고 답했는데(2026-09-28 지적), 그 확인은 내가 만든 일이다 — 같은 날 고친 코드가 `main`에 없었고, 파서가 조용히 0건을 냈고, OCR 결제가 막힌 걸 사용자가 화면을 보고서야 알았다. 사용자가 묻기 전에 **검증 결과(무엇을 어떻게 확인했는지)를 먼저 내놓고**, 지켜봐야 할 것은 내가 점검을 예약한다.
- ⛔ **고쳤다고 말하려면 `main` 병합 + 배포까지 끝나 있어야 한다.** 작업 브랜치에만 커밋하고 세션을 끝내면 사용자 화면은 그대로다. 2026-09-28 TIGER '월중 지급일' 수정이 `project-thread-sz1358` 에만 남아 있어, 사용자가 "아까 고치라고 했잖아"를 다시 말해야 했다(WORKLOG 196). 세션을 끝내기 전에 `git log origin/main..HEAD` 가 비었는지, `Code.gs` 가 바뀌었으면 `deploy.yml` → `maint resetAllTriggers` 까지 돌았는지 확인한다.
- ⛔ **내가 연 것은 내가 닫는다.** 브라우저(Claude in Chrome 등)로 연 탭·탭 그룹, 띄운 로컬 서버는 **작업이 끝나면 그 세션에서 직접 닫는다.** 사용자에게 "그룹 닫기 하세요"라고 정리를 넘기지 마라(2026-09-28 지적: 크롬에 'Claude' 탭 그룹 8개와 죽은 localhost 탭이 쌓여 있었다). 브라우저 도구가 없는 세션(원격 컨테이너)이라 못 닫을 때만 그 근거를 대고 부탁한다.

## Working principles

Adapted from the [Karpathy coding guidelines](https://x.com/karpathy/status/2015883857489522876) — they matter more than usual here because there is no test suite or type checker to catch mistakes, and the backend must be redeployed by hand. They bias toward caution over speed; for trivial edits, use judgment.

1. **Think before coding.** State assumptions out loud instead of hiding uncertainty. When a request has multiple valid readings (e.g. "remove the ₩" — every page, or just totals?), lay out the options and recommend the simpler one before editing. If something is genuinely ambiguous, stop and ask rather than guess — a wrong guess here ships to a live personal-finance app.
2. **Simplicity first.** This is a personal two-user tool, not a framework. Write the minimal change that solves the actual request — no speculative features, config toggles, abstractions, or defensive handling for cases that can't occur. Match the existing plain-`function`, inline-style, `api()`/`apiPost()` idiom rather than introducing new patterns. *Self-check: "Would a senior engineer call this overcomplicated?" If 200 lines could be 50, rewrite it.*
3. **Surgical changes.** Edit only what the task needs. Don't reformat, rename, or "improve" untouched code in the same file — diffs are reviewed by eye against a ~3900-line file, so noise hides real changes. Remove imports/variables/functions that *your* change orphaned, but flag pre-existing dead code (like the stray top-level debug lines that were in `Code.gs`) instead of silently deleting it unless asked. Preserve the documented **Conventions** above. *The test: every changed line should trace directly to the request.*
4. **Goal-driven execution.** Define how you'll verify before you start, then loop until it holds. Reframe vague tasks as checkable goals: "fix the bug" → reproduce it first — there's no test runner, so reproduce in the browser console or a scratch fetch (e.g. dumping `_distData` to pin down a mis-render), then confirm the reproduction is gone. For multi-step work, state a brief plan with a verify step per line. Remember `Code.gs` only takes effect after a **manual redeploy** — say so explicitly — and state the concrete success criterion ("월합계 셀에 ₩가 사라지고 숫자 크기가 나머지와 같다") and confirm it's met.

## Files

- `portfolio.html` — the entire frontend (desktop). Redirects to `m.html` when the viewport is ≤768px unless `?pc=1` is present; the redirect sits in `<head>` so it runs before the password gate.
- `m.html` — **phone-only frontend** (bottom tab bar; 홈 / 종목 / 분배금 / 더보기). Same backend, same actions, same origin — so it reuses the `jjk_pw_v1` password fingerprint and needs no separate login. Design: `docs/superpowers/specs/2026-08-13-mobile-view-design.md`.
  - ⚠️ **The calculation formulas are duplicated here.** The server does *not* return 평가금액/손익 — the browser computes them (`renderAccountStats`, `portfolio.html:1365`). `m.html` has its own copy in a single block marked `⚠️ 계산 블록`. **Change one, change both** — a layout drift is visible, a number drift is not. Verify by feeding both files the same holdings/dividends and comparing the six summary figures.
- `dist_notice.html` — public standalone copy of the 분배금공지 tab (calendar + 운용사별 일정 + notices). `renderMasterCalendar`·`renderDistGrid`·`renderNotices`·`distIssueNotices` 가 `portfolio.html` 에도 같은 이름으로 있다.
  - ⚠️ **"전부 미러해라"는 틀린 지시다**(2026-09-15 실측으로 정정). 두 파일은 **일부러** 다르다 — `renderMasterCalendar` 만 159줄이 다르고 그게 전부 의도된 것이다: **앱만 보유종목을 안다**(`isHeld`·`보유X`·보유 종목 우선 마킹 — 공개 페이지엔 그 개념 자체가 없다), **공개 페이지만 좁은 칸용 `shortLabel` 을 쓴다.** 이걸 "동기화"한다고 합치면 공개 페이지에 남의 보유내역 개념이 들어간다.
  - ✅ **반드시 같아야 하는 건 '데이터 해석 규칙'뿐이다** — 회차 판정(기준일의 '일', 20일 이하=월중), 예정 합의 문턱(`roundBest.n >= 3`), `uncertain`→`?` 배지, `fillRakil`·`parseDay`. 2026-09-15 확인 시점엔 이 넷이 줄 단위로 같았다. **여기가 갈리면 두 화면이 같은 날짜를 다르게 말한다.**
  - 🔎 두 화면이 달라 보이면 **먼저 데이터를 의심해라.** 2026-09-15 에 `PLUS?`(앱) vs `PLUS`(공개)로 갈린 건 렌더 차이가 아니라 그 순간 받은 분배 데이터가 달랐던 것이고, 뿌리는 PLUS 가 6월 기사를 물고 있던 것이었다(WORKLOG 162).
  - ⛔ **우선순위: 앱(`portfolio.html`)이 먼저다. 공개 페이지는 실수 없게**(2026-10-01 사용자: *"원래 앱이 우선 작업이고 공지페이지는 실수없게 해야 한다"*).
    분배금공지의 **보기 방식**(공지 표·칸 높이·선·글자색·전달 분배금 칸 등)을 공개 페이지에 바꾸면 **같은 턴에 앱의 분배금공지 탭에도** 옮긴다.
    2026-10-01에 공개 페이지만 고치고 앱을 빼먹어 "내 포트폴리오 앱은 왜 안 바꿨어?"를 들었다. 일부러 다른 건 보유종목 관련(`isHeld` 등)뿐이다.
    ⛔ **순서: ① 앱(`portfolio.html`)을 먼저 고친다 → ② 앱을 실데이터로 그려 확인하고 `main` 에 올린다 → ③ 앱에 이상 없음을 본 뒤에야 공개 페이지(`dist_notice.html`)에 적용한다.**
    공개 페이지에서 먼저 시험하지 마라 — 공개 페이지는 많은 사람이 보는 '완성본'이고, 실수가 나면 사용자가 망신을 당한다
    (2026-10-01 같은 날 두 번 지적: *"앱이 우선이고 공지페이지가 앱의 무결성을 본 뒤 적용해야 한다 … 그래야 내가 쪽팔린 상황을 면할 수 있어. 내 입장도 좀 생각해라"*).
  - ✅ **공개 페이지 예비본**: `public-snapshot.yml` 이 30분마다 `dist_snapshot.txt`(분배·공지 묶음)를 커밋하고, 페이지는 그걸로 먼저 그린다 — 서버가 죽어도 빈 칸이 안 된다(2026-10-01 요청 *"꼭 이전 버전이라도 보여야 해"*). 정상 데이터일 때만 덮어쓴다.
  - ⛔ **공개 페이지는 지금도 많은 사람이 본다 — 확인 없이 내보내지 마라**(2026-10-01 사용자: *"먼저 버전은 안전하게 해놓고 업데이트한 게 문제없으면 보내줘야지 … 이게 무슨 망신이야"*).
    그날 백엔드 응답만 보고 '정상'이라 했는데 방문자 화면엔 9월 말이 통째로 '예정'으로 나왔다(묶음 요청이 33초 끝에 404 → 개별 요청 줄서기 → 한두 곳 빈칸).
    ① `dist_notice.html` 은 **실제 응답(fetch.yml 로 받은 getDistributionAll·getEtfNoticesAll)을 넣어 로컬 크롬으로 먼저 그려 본다** — 응답을 비워 넣은 테스트는 테스트가 아니다.
    ② 푸시·배포 뒤엔 **`smoke-public.yml`(실제 공개 주소를 실제 크롬으로 열어 'N곳 최신'·공지 표·달력·JS 오류 검사, 스크린샷은 probe-out/smoke/) 통과를 확인하고서야** 됐다고 말한다. 매시간 자동으로도 돌고, 실패하면 카톡이 온다.
  - ⭐ **공개 주소는 `https://jjk.distributionjn.workers.dev/` 하나다**(2026-09-14에 옮겼다. 옛 주소는 계정명이 드러나서 안 쓰기로 했다 — WORKLOG 156·159). 이 저장소의 `dist_notice.html` 을 **Cloudflare Workers 가 그대로 서빙**하므로 **고쳐서 `main` 에 푸시하면 자동 반영**이고, 미러도 배포도 필요 없다(설정: `wrangler.toml`·`.assetsignore`·`_redirects`. `.assetsignore` 가 이 파일 하나만 올리도록 막는다 — 개인 앱과 일지는 그 주소에서 404).
  - 옛 주소 `https://jaenamking1-collab.github.io/jjk-dist/`(별도 저장소 `jjk-dist` 의 `index.html`)도 **아직 살아 있다** — 아래 미러가 30분마다 돌기 때문이다. 예비로 둔 것이니 굳이 끄지 않지만, **알림·카톡·문서가 가리키는 주소는 위의 것 하나다**(`Code.gs` 의 `_NOTICE_PAGE_URL`).
  - ✅ **미러는 자동이다 — 손으로 복사하지 마라**(2026-09-14). `jjk-dist/.github/workflows/mirror.yml` 이 30분마다 이 파일을 받아 `index.html` 로 커밋한다(비밀값 불필요: `jjk` 가 공개라 raw 로 받고, 쓰기는 그 저장소의 기본 `GITHUB_TOKEN`). 고친 직후 바로 반영하려면 `actions_run_trigger` 로 `jjk-dist` 의 `mirror.yml` 을 돌려라.
  - 이 자동화를 만든 이유: 그전까지 **손으로 복사**해야 했고 **이미 네 번 빠뜨렸다**(7/13·8/27·9/12 두 번). 그때마다 공개 페이지만 조용히 옛 버전으로 남아, 달력 백지·느린 첫 화면이 고쳐진 뒤에도 사용자 눈엔 그대로였다. 2026-09-14에 "모바일 로딩이 한참 걸린다"는 지적으로 드러났다 — 원인은 9/4 판이 스냅샷 첫 화면 코드를 안 갖고 있었던 것.
- `Code.gs` — the Google Apps Script backend (mirror of the deployed script; not auto-deployed).
- `okx_nft_alert.gs` — unrelated to the portfolio app: an OKX NFT (Kaia) Legendary/Mystic listing watcher that writes Google Calendar alerts. Mirror of a **separate** Apps Script project — do not merge it into `Code.gs` (its 30-min trigger would steal the portfolio backend's execution slots; see WORKLOG 85·86). Also not auto-deployed. Design: `docs/superpowers/specs/2026-08-08-okx-nft-legendary-alert-design.md`.
- `WORKLOG.md` — running work log for syncing across the two PCs; append a dated entry each session.
- `README.md` — one line (`# jjk`); no other docs.
