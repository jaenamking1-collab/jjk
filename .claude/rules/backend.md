---
paths:
  - "Code.gs"
  - "portfolio.html"
  - "m.html"
  - "dist_notice.html"
  - "calc.js"
---
# 구조 — 백엔드 액션·시트·탭

> CLAUDE.md 에서 옮김(2026-10-14).

- The Apps Script reads/writes two spreadsheets by ID: the app's own DB sheet (`SHEET_ID`, tabs `accounts`/`holdings`/`dividends`/`config`/`stocks` + logs/caches) and an external "주식상황"/"분배금" sheet (hardcoded ID in `getSheetData`/`getDivSheetData`) that the sync features diff against.
- `doGet` routes `?action=` reads; `doPost` routes JSON-body writes. `getDistribution(source)` scrapes eight ETF issuers (KODEX/TIGER/ACE/RISE/PLUS/SOL + 2026-10-01 HANARO/KIWOOM) with per-issuer parsers (HANARO = 공지 본문 HTML 표, KIWOOM = '분배금 정보' 종목별 JSON API — 인증서 체인이 깨져 `validateHttpsCertificates:false`), a smarttoday.co.kr news fallback, an adaptive sheet cache (`분배캐시`, keyed by billing "cycle"), and optional Google Vision OCR (needs `VISION_API_KEY` script property) for schedules embedded in notice images. `checkAndLogAlerts` fingerprints each parse to detect structure changes and writes to the `알림로그` sheet. Several time-driven triggers exist (`snapshotPrices`, `snapshotPortfolio`, `compactPriceLog`, `refreshAllDistributions`, `checkDistNotices`).
- Two client helpers wrap every call:
  - `api(params)` — GET via `API + '?' + URLSearchParams`, returns JSON. Used for all reads.
  - `apiPost(data)` — POST with JSON body. Used for all writes.
- `state` (global object) holds the in-memory cache: `accounts`, `holdings`, `dividends`, `exchangeRate`, `currentYear`, `stockList`. Most tabs re-fetch from the API on activation rather than trusting the cache.

### Backend action contract

Reads (`api`): `getExchangeRate`, `getAccounts`, `getHoldings`, `getDividends`, `getSheetData`, `getDivSheetData`, `getDistribution`, `getDistributionAll`, `getPortfolioLog`, `getPriceLog`, `getStockPrice`, `getEtfScreener`, `getEtfNotices`, `getEtfNoticesAll`, `getStockList`, `getAlerts`, `markAlertRead`, `checkAlerts`, `hitCounter`.

**공개 액션**(`PUBLIC_ACTIONS`, `APP_TOKEN` 없이 열림 — 공개 분배금공지 페이지가 쓴다): `getDistribution`, `getDistributionAll`, `getDistributionMonth`, `getEtfNotices`, `getEtfNoticesAll`, `hitCounter`, `getPricesCsv`, `getNoticeChanges`(알림로그 중 공지변경만·24시간). 나머지는 전부 토큰이 필요하고, 서로 다른 오답 5개가 쌓이면 5분간 잠긴다.

`getDistributionAll` 은 `DIST_SOURCE_IDS` 의 운용사 전부를 한 번에 준다(개별 호출 대신 시트 1회 읽기). 운용사별로 `stale`(캐시가 낡음)과 `savedAt`(분배캐시에 쓰인 시각)이 함께 오고, 화면 헤더의 '데이터 기준 시각'이 이 값을 쓴다. ⚠️ `_파서메타`의 `itemCount` 는 **직전 2회차가 병합된** 건수라 분배캐시 행의 건수(이번 회차만)와 다르다.

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
