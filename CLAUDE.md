# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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
- ⚠️ **원격(클라우드) 세션은 대화 기록이 자동 저장되지 않는다.** `SessionStart` 훅이 `claude-memory`를 clone하려 하지만, 원격 세션의 GitHub 접근은 `jjk` 하나로만 묶여 있어 **조용히 실패**한다(`>/dev/null`). 그대로 두면 그 대화는 어디에도 안 남아 다른 PC가 영영 못 본다(2026-08-27 실제로 그럴 뻔했다).
  - **원격 세션에서 할 일**(에이전트가 알아서 한다, 사용자에게 시키지 마라): ① `add_repo`로 `jaenamking1-collab/claude-memory`를 붙이고 `/home/user/claude-memory`에 clone ② 세션 트랜스크립트(`/root/.claude/projects/-home-user-jjk/<세션>.jsonl`)를 저장:
    ```
    echo '{"transcript_path":"<그 jsonl 경로>"}' | CLAUDE_MEMORY_PC=cloud python3 /home/user/claude-memory/hooks/index_transcripts.py save final
    ```
    ③ `claude-memory`를 커밋·푸시(원격이 앞서 있으면 `pull --rebase`).
  - `CLAUDE_MEMORY_PC=cloud`가 핵심이다 — 안 주면 컨테이너 호스트명(`vm`)으로 학교/집과 구분 안 되는 `pc<해시>` 폴더가 생긴다.
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
    3. `clasp redeploy AKfycbwJS1Fd-sDCVKPLJEpEWZmPQEKAOR9pG7y-nPKZOYty65j3ArOmlDzNX2WFqiGNF_s -d "<note>"` — 라이브 배포에 새 버전을 물린다. **새 배포(`clasp deploy`) 금지 — URL이 새로 생긴다.**
    4. 다시 `clasp pull`로 원격에 변경분이 들어갔는지 확인.
    공개 분배금공지 페이지가 같은 `/exec` URL을 쓴다. 기존 액션의 응답 계약을 바꾸는 배포라면 먼저 알린다(액션 추가처럼 덧붙이기만 하는 변경은 그냥 배포한다).
  - **`clasp run-function` does not work** here — it needs the script deployed as an API executable. **Installing a trigger (`setupKeepWarm()`, `setupWatchdogTrigger()`, …) therefore still requires the owner to click `▶` in the editor.**
  - One-time per machine: `npm i -g @google/clasp`(에이전트가 함) + `clasp login`과 `script.google.com/home/usersettings`의 **Apps Script API** 토글(구글 계정 행위 — 소유자가 함). 자격증명은 `~/.clasprc.json`.
  - **`portfolio.html`·`m.html`·`dist_notice.html`은 배포 대상이 아니다.** 로컬 파일을 브라우저로 열고, `.claspignore`가 push에서 막는다. git push로 끝.
- The Apps Script reads/writes two spreadsheets by ID: the app's own DB sheet (`SHEET_ID`, tabs `accounts`/`holdings`/`dividends`/`config`/`stocks` + logs/caches) and an external "주식상황"/"분배금" sheet (hardcoded ID in `getSheetData`/`getDivSheetData`) that the sync features diff against.
- `doGet` routes `?action=` reads; `doPost` routes JSON-body writes. `getDistribution(source)` scrapes six ETF issuers (KODEX/TIGER/ACE/RISE/PLUS/SOL) with per-issuer parsers, a smarttoday.co.kr news fallback, an adaptive sheet cache (`분배캐시`, keyed by billing "cycle"), and optional Google Vision OCR (needs `VISION_API_KEY` script property) for schedules embedded in notice images. `checkAndLogAlerts` fingerprints each parse to detect structure changes and writes to the `알림로그` sheet. Several time-driven triggers exist (`snapshotPrices`, `snapshotPortfolio`, `compactPriceLog`, `refreshAllDistributions`, `checkDistNotices`).
- **Functions meant to be run by hand from the Apps Script editor go at the very bottom of `Code.gs`, under the `===== 수동 실행 =====` banner** (`setupDistTriggers`, `setupPortfolioTriggers`, `_testNoticeWindow`). Don't scatter them mid-file — they're hard to find in the editor's function dropdown otherwise. Leave a one-line pointer where the code logically belongs.
- Two client helpers wrap every call:
  - `api(params)` — GET via `API + '?' + URLSearchParams`, returns JSON. Used for all reads.
  - `apiPost(data)` — POST with JSON body. Used for all writes.
- `state` (global object) holds the in-memory cache: `accounts`, `holdings`, `dividends`, `exchangeRate`, `currentYear`, `stockList`. Most tabs re-fetch from the API on activation rather than trusting the cache.

### Backend action contract

Reads (`api`): `getExchangeRate`, `getAccounts`, `getHoldings`, `getDividends`, `getSheetData`, `getDivSheetData`, `getDistribution`, `getPortfolioLog`, `getPriceLog`, `getStockPrice`, `getEtfScreener`, `getEtfNotices`, `getStockList`, `getAlerts`, `markAlertRead`.

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
- `dist_notice.html` — public standalone copy of the 분배금공지 tab (calendar + 운용사별 일정 + notices). It duplicates `renderMasterCalendar`/schedule-table code from `portfolio.html`; **any change to the 분배금공지 tab's calendar or schedule rendering must be mirrored here** or the public page silently diverges.
- `Code.gs` — the Google Apps Script backend (mirror of the deployed script; not auto-deployed).
- `okx_nft_alert.gs` — unrelated to the portfolio app: an OKX NFT (Kaia) Legendary/Mystic listing watcher that writes Google Calendar alerts. Mirror of a **separate** Apps Script project — do not merge it into `Code.gs` (its 30-min trigger would steal the portfolio backend's execution slots; see WORKLOG 85·86). Also not auto-deployed. Design: `docs/superpowers/specs/2026-08-08-okx-nft-legendary-alert-design.md`.
- `WORKLOG.md` — running work log for syncing across the two PCs; append a dated entry each session.
- `README.md` — one line (`# jjk`); no other docs.
