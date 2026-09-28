# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> 먼저 공용 `일하는_방식.md`(기억 폴더)를 읽어라. 이 파일은 이 프로젝트 고유 정보만 담는다.
> 작업 절차는 `.claude/skills/`(배포·화면수정·분배금점검), 검증 항목은 `.claude/CHECKS.md`, 도구는 `tools/INDEX.md`.
> ⚠️ 원격(웹) 세션은 `claude-memory` 를 붙이기 전엔 공용 기억을 못 본다(GitHub 접근이 `jjk` 하나로 묶임). 그래서 아래 두 ⛔ 규칙은 여기에도 남긴다.

## ⛔ 0. 사용자에게는 **존댓말**로 답한다 — 예외 없다

**이 문서 전체가 반말 지시문(`~한다`, `~하지 마라`)인데, 그건 나에게 내리는 지시일 뿐이다.
사용자에게 보내는 답변은 언제나 존댓말이다.** 문서 문체에 끌려가지 마라.

- 사용자는 **계정을 만든 첫날부터** 존댓말을 요구했는데 새 세션마다 반말로 시작해 같은 지적을 반복하게
  만들었다(8/27·9/12·9/14). 원격 세션엔 전역 설정이 없어 **이 파일이 모든 환경에서 읽히는 유일한 곳**이다.
- 대화 중에 정해진 규칙은 그 대화에서만 산다. 규칙을 받으면 **그 자리에서 여기(또는 WORKLOG·매뉴얼)에 적어라.**

## What this is

A single-file personal dividend-portfolio tracker (`portfolio.html`) for two people (재남 / 은경) holding mostly monthly-distribution Korean ETFs across several brokerage accounts. It tracks holdings, valuations, and monthly dividend income, and visualizes progress toward a goal (₩10,000,000/month in distributions by 2029-02-28).

There is **no build system, package manager, test suite, or lint config**. The frontend — HTML, CSS (`<style>`), and JS (`<script>`) — lives in `portfolio.html` (수천 줄). Open the file in a browser to run it; there is nothing to compile. The backend lives in `Code.gs` (Google Apps Script).

## Working in this repo

- **Editing**: The file is large with inline styles and one big script block. Use Grep to locate a function/section by line before editing rather than reading the whole file. Function definitions are plain `function name()` / `async function name()` at column 0, so `^(async )?function <name>` finds them fast.
- **Preview**: Just open `portfolio.html` in a browser (the launch preview panel also renders it). No dev server.
- ⛔ **비밀번호·열쇠 값은 저장소에 절대 적지 않는다.** 일지에도 코드에도 `••••`로만 쓴다. `jjk`는 **공개 저장소**다 — 한 번 커밋하면 지난 기록에 영구히 남고, 나중에 지워도 완전히 안 지워진다. 실제 값은 Apps Script **스크립트 속성**(`APP_TOKEN`)에만 둔다. (2026-08-05에 적힌 진입 비밀번호가 3주간 공개돼 있었다 — WORKLOG 127.)
- **Commit/push**: `main` on `origin` (GitHub `jaenamking1-collab/jjk`) is the live branch — PC sessions commit there directly; cloud sessions work on their assigned branch and open a PR. Commit messages describe the change in Korean. Git identity is set locally as `jaenamking1-collab <jaenamking1@gmail.com>` (not global — new clones must set it). The owner works across **three Windows PCs (home / work desktop / work laptop)** plus cloud sessions and wants every change committed and pushed automatically without asking.
- **Multi-PC routine**: This repo is edited from several machines. Session start/end steps are in **세션 연속성** below.
- **This is also automated — but don't rely on it alone.** `.claude/settings.json` (committed, so both PCs get it) holds two hooks: `SessionStart` rescues a dirty tree then pulls, and `Stop` commits + pushes at the end of every turn. Both are limited to `main` and skip mid-merge/rebase.

## 세션 연속성 (WORKLOG.md)

이 프로젝트는 세 대의 Windows PC(집 / 직장 본체 / 직장 노트북)와 원격 세션에서 Claude Code로 번갈아 작업한다. 대화 맥락이 PC 간에 이어지지 않으므로 `WORKLOG.md`를 Git으로 동기화되는 공유 메모리로 사용한다.

- **세션 시작 시**: ⚠️ **`git status`가 맨 처음이다.** 더러우면 **읽기도 pull도 하기 전에** 먼저 커밋·푸시한다 — 이전 세션이 커밋 없이 끝났다는 뜻이고, 그 작업은 이 PC에만 있다. 그 다음 `git pull`, 그 다음 `WORKLOG.md`를 읽어 맥락과 "다음 할 일"을 파악한다.
- **다른 PC 대화 이어받기**: "집에서/학교에서 한 거 이어가봐"는 **다른 기기의 세션**을 뜻한다. `list_sessions`는 이 PC만 본다 — 그것만 보고 "없다"고 답하지 마라. 먼저 `git fetch origin`으로 넘어온 커밋을 보고, `~/claude-memory/transcripts/INDEX.md`를 읽는다. 필요하면 `python ~/claude-memory/hooks/index_transcripts.py read <PC>/<파일> [검색어]`로 원문을 확인한다.
- **지금 있는 PC에서 할 수 있는 것만 안내한다.** 다른 PC에서 해야 할 일은 **`WORKLOG.md`의 "다음 할 일"에만 남기고, 대화에서 시키지 마라.** 사용자는 지금 그 PC 앞에 없다 — 실행할 수 없는 명령을 받으면 할 일 목록이 아니라 잡음이다. 다른 PC 차례가 되면 그 PC의 세션이 WORKLOG를 읽고 알아서 꺼낸다.
- **원격(클라우드) 세션은 시작할 때 `add_repo`로 `jaenamking1-collab/claude-memory`를 붙이고 clone한다.** 원격 세션의 GitHub 접근은 `jjk` 하나로 묶여 있어 `SessionStart` 훅의 clone이 **조용히 실패**하기 때문이다(`>/dev/null`). 붙이기만 하면 **저장·푸시는 훅이 알아서 한다** — 손으로 `save`를 부르거나 `CLAUDE_MEMORY_PC`를 넘길 필요 없다.
  - 컨테이너는 `USERPROFILE`이 없다는 것으로 자동 판별해 `transcripts/cloud/`에 남는다. 윈도우 PC는 각자 폴더를 쓴다.
  - **부작용**: 저장소를 하나 더 붙이면 그 세션이 앱 목록에서 `jjk` 아래가 아니라 **'기타'로 잡힌다.** 기록을 남기는 값이 더 크므로 감수한다.
  - 반대 방향(읽기)은 이미 자동이다 — 집·학교 PC는 `SessionStart` 훅의 `index_transcripts.py brief`가 다른 PC 대화 목록을 세션 맥락에 넣어준다. 사용자가 명령어를 칠 필요 없다.
- **세션 종료 시**: `WORKLOG.md` 맨 위에 새 항목(날짜, 장소(직장/집/원격), 한 일, 다음 할 일)을 추가한다. 과거 항목은 수정하지 않는다.
- **갱신 후**: `git add .` → `git commit` → `git push`까지 자동으로 수행한다. 사용자가 "항상 커밋·푸시"를 요청했으므로 매번 확인하지 않는다.

## Architecture

**Frontend (`portfolio.html`)** ⇄ **Google Apps Script backend (`Code.gs`)**, a Google Sheets–backed web app.

- The backend URL is the `API` constant at the top of `portfolio.html`'s `<script>` (`https://script.google.com/macros/s/.../exec`). All persistence lives in Google Sheets behind it — this repo has no database.
- `Code.gs` is the source of the deployed Apps Script. **Editing it here does NOT deploy it.**
  - ⛔ **배포는 에이전트가 한다 — 사용자에게 "편집기에 붙여넣고 재배포하세요"라고 시키지 마라.** 절차·결정 규칙·과거 사고는 **`.claude/skills/배포/SKILL.md`**. `clasp` 가 없으면 묻지 말고 설치한다.
  - **`doGet`/`doPost` 경로가 바뀌면 재배포**(`clasp deploy -i <라이브 ID>`, `-i` 없이 치면 URL 이 새로 생긴다). 트리거·편집기 ▶ 만 쓰는 변경은 `clasp push` 로 끝난다 — **배포를 습관처럼 하지 마라**(버전 200개 상한, 지우는 API 없음).
  - ⛔ **배포 직후 `maint.yml` 로 `resetAllTriggers` — 배포의 일부다.** 빼먹어서 트리거가 조용히 죽은 게 세 번이다(WORKLOG 149). 사용자에게 ▶ 를 부탁하지 않는다. 확인 항목은 `.claude/CHECKS.md`.
  - 원격(클라우드) 세션은 `deploy.yml`(Actions)로 배포한다. "원격이라 배포 못 한다"고 말하지 마라.
  - 진단·복구 함수는 `runMaint`/`maint.yml` 로 에이전트가 돌린다(`MAINT_ALLOW` 에 등록). `clasp run-function` 은 안 된다.
  - 공개 분배금공지 페이지가 같은 `/exec` 를 쓴다. 기존 액션의 응답 형식을 바꾸는 배포라면 먼저 알린다.
  - **`portfolio.html`·`m.html`·`dist_notice.html`은 배포 대상이 아니다.** git push로 끝.
- The Apps Script reads/writes two spreadsheets by ID: the app's own DB sheet (`SHEET_ID`, tabs `accounts`/`holdings`/`dividends`/`config`/`stocks` + logs/caches) and an external "주식상황"/"분배금" sheet (hardcoded ID in `getSheetData`/`getDivSheetData`) that the sync features diff against.
- `doGet` routes `?action=` reads; `doPost` routes JSON-body writes. `getDistribution(source)` scrapes six ETF issuers (KODEX/TIGER/ACE/RISE/PLUS/SOL) with per-issuer parsers, a smarttoday.co.kr news fallback, an adaptive sheet cache (`분배캐시`, keyed by billing "cycle"), and optional Google Vision OCR (needs `VISION_API_KEY` script property) for schedules embedded in notice images. `checkAndLogAlerts` fingerprints each parse to detect structure changes and writes to the `알림로그` sheet. Several time-driven triggers exist (`snapshotPrices`, `snapshotPortfolio`, `compactPriceLog`, `refreshAllDistributions`, `checkDistNotices`).
- **Functions meant to be run by hand from the Apps Script editor go at the very bottom of `Code.gs`, under the `===== 수동 실행 =====` banner** (`setupDistTriggers`, `setupPortfolioTriggers`, `_testNoticeWindow`). Don't scatter them mid-file — they're hard to find in the editor's function dropdown otherwise. Leave a one-line pointer where the code logically belongs.
- Two client helpers wrap every call:
  - `api(params)` — GET via `API + '?' + URLSearchParams`, returns JSON. Used for all reads.
  - `apiPost(data)` — POST with JSON body. Used for all writes.
- `state` (global object) holds the in-memory cache: `accounts`, `holdings`, `dividends`, `exchangeRate`, `currentYear`, `stockList`. Most tabs re-fetch from the API on activation rather than trusting the cache.

### Backend action contract

The authoritative action list is the `switch` in `doGet` (reads, `api`) and `doPost` (writes, `apiPost`) in `Code.gs` — grep `case '` there rather than trusting a copy here.

**공개 액션**(`Code.gs` 맨 위 `PUBLIC_ACTIONS`, `APP_TOKEN` 없이 열림 — 공개 분배금공지 페이지 등이 쓴다). 나머지는 전부 토큰이 필요하고, 서로 다른 오답 5개가 쌓이면 5분간 잠긴다.

`getDistributionAll` 은 6개사를 한 번에 준다(개별 호출 6번 대신 시트 1회 읽기). 운용사별로 `stale`(캐시가 낡음)과 `savedAt`(분배캐시에 쓰인 시각)이 함께 오고, 화면 헤더의 '데이터 기준 시각'이 이 값을 쓴다. ⚠️ `_파서메타`의 `itemCount` 는 **직전 2회차가 병합된** 건수라 분배캐시 행의 건수(이번 회차만)와 다르다.

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
- **회색은 '살아 있지 않은 값' 전용이다.** 회색(`var(--text3)`·`muted`)은 **못 받은 값·아직 안 된 값·누락**에만 쓴다 — 시세없음, 예정(공시 전), 상류 응답 없음 같은 것. **살아 있는 실제 값은 작게 쓰더라도 본문색(`var(--text)`)으로 둔다.** 사용자는 회색을 "죽었거나 아직 안 됐거나 빠진 것"으로 읽는다(본인 표현: *"회색은 죽은거나 아직 안된거나 누락일때 써"*).
- **국내/해외는 나눠 보여준다.** 증권사 앱은 '국내 잔고'와 '해외 잔고'를 다른 화면에 둔다. 계좌 표는 **합계 / ㄴ국내 / ㄴ해외 세 줄**이고 수익금·분배금·분배율까지 각 줄의 투자금 기준으로 따로 낸다. 해외가 섞인 계좌만 세 줄이고 국내뿐이면 한 줄이다. 합쳐서만 보여주면 증권사 화면과 대조가 안 돼 **값이 맞는데도 틀린 것처럼 보인다**(2026-09-21).

## ⛔ 사용자에게 시키기 전에 — 먼저 해보고 말해라

- **"못 한다 / 해주세요"는 실제로 시도해 본 뒤에만 말한다.** 넘겨짚은 '못 함'은 전부 틀렸다 — 원격 배포(WORKLOG 141), `clasp` 미설치(설치하면 그만), "PC에서 `git pull` 하세요"(2026-09-11 한 세션에 네 번).
  - **사용자는 앱을 웹 주소(GitHub Pages / 공개 페이지는 Cloudflare)로 본다.** 프론트를 고쳐 푸시했으면 사용자가 할 일은 **새로고침뿐**이다. `git pull` 을 부탁해도 되는 건 그 PC에서 작업·배포할 때뿐이다.
  - 저장소 안 기록끼리 어긋나면 **사용자에게 확인하고 하나로 정리해라.** 한쪽을 골라 시키지 마라.
- **순서**: ① 직접 해본다 → ② 막히면 *무엇이* 왜 막혔는지 명령·응답 코드로 확인한다 → ③ 그래도 사람만 할 수 있는 것(구글 계정 '허용' 클릭, 브라우저 로그인)만 부탁한다. ③에 해당하는지 스스로 증거를 못 대면 아직 ①이 안 끝난 것이다.
- **사용자의 되물음("배포가 안 된다고?", "그거 왜 안 돼?")은 점검 지시다.** 그 자리에서 다시 확인하고 답한다 — 앞서 한 말을 반복하지 않는다.
- 부탁을 할 때는 **왜 나는 못 하는지**(막힌 호스트, 없는 자격증명 등 구체적 근거)를 한 줄로 같이 준다. 근거를 못 쓰겠으면 부탁하지 마라.

## Working principles

Adapted from the [Karpathy coding guidelines](https://x.com/karpathy/status/2015883857489522876) — they matter more than usual here because there is no test suite or type checker to catch mistakes, and the backend must be redeployed by hand. They bias toward caution over speed; for trivial edits, use judgment.

1. **Think before coding.** State assumptions out loud instead of hiding uncertainty. When a request has multiple valid readings (e.g. "remove the ₩" — every page, or just totals?), lay out the options and recommend the simpler one before editing. If something is genuinely ambiguous, stop and ask rather than guess — a wrong guess here ships to a live personal-finance app.
2. **Simplicity first.** This is a personal two-user tool, not a framework. Write the minimal change that solves the actual request — no speculative features, config toggles, abstractions, or defensive handling for cases that can't occur. Match the existing plain-`function`, inline-style, `api()`/`apiPost()` idiom rather than introducing new patterns. *Self-check: "Would a senior engineer call this overcomplicated?" If 200 lines could be 50, rewrite it.*
3. **Surgical changes.** Edit only what the task needs. Don't reformat, rename, or "improve" untouched code in the same file — diffs are reviewed by eye against a file of several thousand lines, so noise hides real changes. Remove imports/variables/functions that *your* change orphaned, but flag pre-existing dead code (like the stray top-level debug lines that were in `Code.gs`) instead of silently deleting it unless asked. Preserve the documented **Conventions** above. *The test: every changed line should trace directly to the request.*
4. **Goal-driven execution.** Define how you'll verify before you start, then loop until it holds. Reframe vague tasks as checkable goals: "fix the bug" → reproduce it first — there's no test runner, so reproduce in the browser console or a scratch fetch (e.g. dumping `_distData` to pin down a mis-render), then confirm the reproduction is gone. `Code.gs` changes reach the web app only after the agent redeploys (`.claude/skills/배포/SKILL.md`) — say which verification you ran — and state the concrete success criterion ("월합계 셀에 ₩가 사라지고 숫자 크기가 나머지와 같다") and confirm it's met.

## Files

- `portfolio.html` — the entire frontend (desktop). Redirects to `m.html` when the viewport is ≤768px unless `?pc=1` is present; the redirect sits in `<head>` so it runs before the password gate.
- `m.html` — **phone-only frontend** (bottom tab bar; 홈 / 종목 / 분배금 / 더보기). Same backend, same actions, same origin — so it reuses the `jjk_pw_v1` password fingerprint and needs no separate login. Design: `docs/superpowers/specs/2026-08-13-mobile-view-design.md`.
  - ⚠️ **The calculation formulas are duplicated here.** The server does *not* return 평가금액/손익 — the browser computes them (`renderAccountStats` in `portfolio.html`). `m.html` has its own copy in a single block marked `⚠️ 계산 블록`. **Change one, change both** — a layout drift is visible, a number drift is not. Verify by feeding both files the same holdings/dividends and comparing the six summary figures.
- `dist_notice.html` — public standalone copy of the 분배금공지 tab (calendar + 운용사별 일정 + notices). `renderMasterCalendar`·`renderDistGrid`·`renderNotices`·`distIssueNotices` 가 `portfolio.html` 에도 같은 이름으로 있다.
  - ⚠️ 두 파일은 **일부러** 다르다(앱만 보유종목을 안다, 공개만 `shortLabel`). **같아야 하는 건 데이터 해석 규칙뿐** — 회차 판정, `roundBest.n >= 3`, `uncertain`→`?`, `fillRakil`·`parseDay`, `pubAfterBase`. 두 화면이 달라 보이면 **데이터부터 의심해라.**
  - ⭐ 공개 주소는 **`https://jjk.distributionjn.workers.dev/`** — Cloudflare Workers 가 이 파일을 그대로 서빙하므로 `main` 에 푸시하면 자동 반영. 옛 주소(`jjk-dist`)는 30분 자동 미러. **손으로 복사하지 마라.** 세부: `.claude/skills/화면수정/SKILL.md`.
- `Code.gs` — the Google Apps Script backend (mirror of the deployed script; not auto-deployed).
- `okx_nft_alert.gs` — unrelated to the portfolio app: an OKX NFT (Kaia) Legendary/Mystic listing watcher that writes Google Calendar alerts. Mirror of a **separate** Apps Script project — do not merge it into `Code.gs` (its 30-min trigger would steal the portfolio backend's execution slots; see WORKLOG 85·86). Also not auto-deployed. Design: `docs/superpowers/specs/2026-08-08-okx-nft-legendary-alert-design.md`.
- `WORKLOG.md` — running work log for syncing across PCs; append a dated entry each session.
- `README.md` — one line (`# jjk`); no other docs.
