# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A single-file personal dividend-portfolio tracker (`portfolio.html`) for two people (재남 / 은경) holding mostly monthly-distribution Korean ETFs across several brokerage accounts. It tracks holdings, valuations, and monthly dividend income, and visualizes progress toward a goal (₩10,000,000/month in distributions by 2029-02-28).

There is **no build system, package manager, test suite, or lint config**. The frontend — HTML, CSS (`<style>`), and JS (`<script>`) — lives in `portfolio.html` (~3900 lines). Open the file in a browser to run it; there is nothing to compile. The backend lives in `Code.gs` (Google Apps Script).

## Working in this repo

- **Editing**: The file is large with inline styles and one big script block. Use Grep to locate a function/section by line before editing rather than reading the whole file. Function definitions are plain `function name()` / `async function name()` at column 0, so `^(async )?function <name>` finds them fast.
- **Preview**: Just open `portfolio.html` in a browser (the launch preview panel also renders it). No dev server.
- **Commit/push**: History is a linear series of single-file "Update portfolio.html" commits on `main`, pushed to `origin` (GitHub `jaenamking1-collab/jjk`). Git identity is set locally as `jaenamking1-collab <jaenamking1@gmail.com>` (not global — new clones must set it). Only commit/push when asked.

## 세션 연속성 (WORKLOG.md)

이 프로젝트는 윈도우/맥북 두 대의 PC에서 Claude Code로 번갈아 작업한다. 대화 맥락이 PC 간에 이어지지 않으므로 `WORKLOG.md`를 Git으로 동기화되는 공유 메모리로 사용한다.

- **세션 시작 시**: `WORKLOG.md`를 먼저 읽어 최근 작업 맥락과 "다음 할 일"을 파악한다.
- **세션 종료 시**: `WORKLOG.md` 맨 위에 새 항목(날짜, PC 구분(Windows/맥북), 한 일, 다음 할 일)을 추가한다. 과거 항목은 수정하지 않는다.
- **갱신 후**: 사용자에게 `git add WORKLOG.md`, `git commit`, `git push` 실행을 안내한다(직접 커밋/푸시는 사용자가 명시적으로 요청했을 때만).

## Architecture

**Frontend (`portfolio.html`)** ⇄ **Google Apps Script backend (`Code.gs`)**, a Google Sheets–backed web app.

- The backend URL is the `API` constant at the top of `portfolio.html`'s `<script>` (`https://script.google.com/macros/s/.../exec`). All persistence lives in Google Sheets behind it — this repo has no database.
- `Code.gs` is the source of the deployed Apps Script. **Editing it here does NOT deploy it** — changes must be pasted back into the Apps Script editor (or pushed via `clasp`) and a new web-app deployment made. Keep `Code.gs` in sync with what's deployed.
- The Apps Script reads/writes two spreadsheets by ID: the app's own DB sheet (`SHEET_ID`, tabs `accounts`/`holdings`/`dividends`/`config`/`stocks` + logs/caches) and an external "주식상황"/"분배금" sheet (hardcoded ID in `getSheetData`/`getDivSheetData`) that the sync features diff against.
- `doGet` routes `?action=` reads; `doPost` routes JSON-body writes. `getDistribution(source)` scrapes six ETF issuers (KODEX/TIGER/ACE/RISE/PLUS/SOL) with per-issuer parsers, a smarttoday.co.kr news fallback, an adaptive sheet cache (`분배캐시`, keyed by billing "cycle"), and optional Google Vision OCR (needs `VISION_API_KEY` script property) for schedules embedded in notice images. `checkAndLogAlerts` fingerprints each parse to detect structure changes and writes to the `알림로그` sheet. Several time-driven triggers exist (`snapshotPrices`, `snapshotPortfolio`, `compactPriceLog`, `refreshAllDistributions`).
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

Adapted from the Karpathy coding guidelines — they matter more than usual here because there is no test suite or type checker to catch mistakes, and the backend must be redeployed by hand.

1. **Think before coding.** State assumptions out loud instead of hiding uncertainty. When a request has multiple valid readings (e.g. "remove the ₩" — every page, or just totals?), lay out the options and recommend the simpler one before editing. If something is genuinely ambiguous, ask rather than guess — a wrong guess here ships to a live personal-finance app.
2. **Simplicity first.** This is a personal two-user tool, not a framework. Write the minimal change that solves the actual request. No speculative features, config toggles, abstractions, or defensive handling for cases that can't occur. Match the existing plain-`function`, inline-style, `api()`/`apiPost()` idiom rather than introducing new patterns.
3. **Surgical changes.** Edit only what the task needs. Don't reformat, rename, or "improve" untouched code in the same file — diffs are reviewed by eye against a ~3900-line file, so noise hides real changes. Flag unrelated dead code (like the stray top-level debug lines that were in `Code.gs`) instead of silently removing it unless asked. Preserve the documented **Conventions** above.
4. **Goal-driven execution.** Since nothing runs automatically, define how you'll verify before you start: for `portfolio.html`, open it in the preview and check the affected tab; for `Code.gs`, remember it only takes effect after a manual redeploy and say so explicitly. State the success criterion ("월합계 셀에 ₩가 사라지고 숫자 크기가 나머지와 같다") and confirm it's met.

## Files

- `portfolio.html` — the entire frontend.
- `Code.gs` — the Google Apps Script backend (mirror of the deployed script; not auto-deployed).
- `README.md` — one line (`# jjk`); no other docs.
