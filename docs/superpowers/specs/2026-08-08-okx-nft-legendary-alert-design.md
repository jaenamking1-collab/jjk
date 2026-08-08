# OKX NFT Legendary/Mystic 리스팅 알림 — 설계

작성일: 2026-08-08

## 목적

OKX Web3 마켓플레이스(Kaia 체인)의 두 컬렉션에서 **Legendary 또는 Mystic 등급 매물이 새로 판매 등록되거나 가격이 내려가면 Google 캘린더 일정으로 알림**을 받는다. 메일은 놓치기 쉬워 캘린더 팝업으로 받는다.

- https://web3.okx.com/nft/collection/kaia/puuvillasociety
- https://web3.okx.com/nft/collection/kaia/puuvilla-fashionista

## 데이터 소스 (실측으로 확인됨, 2026-08-08)

`POST https://web3.okx.com/priapi/v1/nft/secondary/market?t=<epoch_ms>`

요청 본문:

```json
{
  "sources": [], "stateIn": [], "nameSearch": "",
  "sortBy": "makeOrderTimeDesc",
  "ownerAddressIn": [], "contentTypes": [],
  "projectIn": [748906],
  "pageNum": 1, "pageSize": 50, "cursor": ""
}
```

확인된 사실:

- **인증·쿠키·API 키가 전혀 필요 없다.** 브라우저 밖 `curl`로 성공 확인 → Apps Script `UrlFetchApp`으로 그대로 호출 가능.
- 컬렉션 project ID: `puuvillasociety` = **748906**, `puuvilla-fashionista` = **2289945**
- `sortBy: "makeOrderTimeDesc"` = 최근 리스팅순 (번들 상수 `LATEST_LIST`).
- 응답 `data.list[]` 각 항목에서 필요한 필드:
  - `tokenId`, `name`, `projectName`, `contractAddress`
  - `specialProperties` — **JSON 문자열**. 파싱하면 `[{"name":"Rarity","key":"Legendary"}, ...]`
  - `sale` — 현재 판매 등록 정보. **없거나 `price`가 0이면 미등록.** `sale.price`(숫자), `sale.strPrice`, `sale.currency`(KAIA/WKLAY), `sale.usdPrice`, `sale.lastTime`(리스팅 시각 ms)
  - `thumbnailUrl`
- 두 컬렉션 모두 Rarity 6단계: Common / Uncommon / Rare / Epic / **Legendary** / **Mystic**. Mystic이 Legendary보다 희귀(각 100개 / 20개)하므로 둘 다 대상에 포함한다.
- Fashionista에는 `Tier` 트레잇도 있으나 이번 설계에서는 쓰지 않는다.
- 리스팅 빈도가 낮다(확인 시점 기준 최근 Legendary 매물은 Fashionista `#99, 250,000 KAIA, 2026-08-02`). 30분 주기로 충분하다.

상세 페이지 URL 형식: `https://web3.okx.com/nft/asset/kaia/<contractAddress>/<tokenId>`

## 배포 위치 — 별도 Apps Script 프로젝트

기존 `Code.gs`(포트폴리오 백엔드)에 **넣지 않는다.**

- `WORKLOG.md` (85)·(86)에 기록된 대로 포트폴리오 백엔드는 이미 **트리거가 실행 슬롯을 점유해 앱이 수십 초 멈추는 문제**를 겪었다. 30분 트리거를 같은 프로젝트에 추가하면 같은 병을 재발시킨다.
- 별도 프로젝트는 실행 레인과 할당량이 분리된다.
- 웹앱 배포가 필요 없다. 트리거만 걸면 된다.

저장소에는 `okx_nft_alert.gs`로 미러한다. `Code.gs`와 동일하게 **이 파일을 고쳐도 자동 반영되지 않는다** — Apps Script 편집기에 붙여넣어야 한다.

최초 1회 필요한 수동 작업: 새 프로젝트 생성 → 파일 붙여넣기 → `testOnce()` 실행하며 **캘린더 접근 권한 승인** → `setupOkxTriggers()` 실행.

## 구조

파일 하나, 주석 포함 170줄 안팎.

```
CONFIG (상수)
  COLLECTIONS = [{ slug, projectId, label }] × 2
  TARGET_RARITY = ['Legendary', 'Mystic']
  CALENDAR_NAME = 'OKX NFT'
  FALLBACK_USDKRW = 1450

checkOnce()                    ← 30분 트리거가 부르는 진입점
  컬렉션마다 (각각 try/catch 분리):
    fetchListings_(projectId)  → UrlFetchApp POST, 실패 시 null
    항목별 extract_(item)      → { tokenId, rarity, price, currency, usd, ... }
    대상 등급 + sale 있는 것만 남김
    diffAgainstState_()        → 알림 대상 산출 + 새 상태 반환
  모은 알림이 있으면 매물당 createEvent_(row) 1개

fetchListings_(projectId)      HTTP + 응답 검증
extract_(item)                 필드 추출 (specialProperties 파싱 포함)
diffAgainstState_(pid, cur)    상태 비교
fetchUsdKrw_()                 야후 환율 (1시간 캐시, 실패 시 폴백)
getCalendar_()                 'OKX NFT' 캘린더 조회, 없으면 생성
createEvent_(row)              캘린더 일정 1건 생성 + 팝업 리마인더

===== 수동 실행 =====           (편집기에서 손으로 돌리는 것만 맨 아래)
setupOkxTriggers()             기존 트리거 삭제 후 30분 트리거 1개 생성
testOnce()                     일정 생성 없이 현재 대상 매물을 로그로만 출력
resetState()                   저장된 상태 삭제
```

## 상태 저장

`PropertiesService.getScriptProperties()`, 키 `okx_state_<projectId>`, 값은 JSON:

```json
{ "99": { "p": 250000, "c": "KAIA" }, "1175": { "p": 112000, "c": "KAIA" } }
```

- 대상이 Legendary/Mystic 매물뿐이라 항목 수가 매우 적다. 스프레드시트가 필요 없다.
- **매 회차 현재 목록으로 통째로 덮어쓴다.** 그래서 내려갔다 다시 올라온 매물이 자연스럽게 다시 "신규"로 잡힌다.

## 알림 판정

각 대상 매물에 대해:

| 조건 | 결과 |
|---|---|
| 이전 상태에 tokenId 없음 | 🆕 **신규 리스팅** |
| 있고, 통화 같고, `현재가 < 이전가` | 🔻 **가격 인하** (이전가 → 현재가 함께 표시) |
| 있고, 통화가 다름 | 🆕 신규로 취급 |
| 그 외 (같거나 오름) | 알림 없음 |

**가격은 네이티브 `sale.price`로만 비교한다.** `usdPrice`로 비교하면 KAIA 시세가 내릴 때마다 가짜 "가격 인하"가 발생한다. USD·KRW는 일정 본문에 참고용으로만 표시한다.

## 환율 (달러·원화 환산)

- **KAIA → USD**: OKX 응답의 `sale.usdPrice`를 그대로 쓴다. 별도 조회가 필요 없다.
- **USD → KRW**: `Code.gs:370 fetchExchangeRate()`와 같은 소스인 야후 `USDKRW=X`
  (`https://query1.finance.yahoo.com/v8/finance/chart/USDKRW=X?interval=1d&range=1d`,
  `chart.result[0].meta.regularMarketPrice`).
- `CacheService.getScriptCache()`에 **1시간** 캐시. 실패하면 **1450** 폴백 (`Code.gs:2918`과 같은 값).
- 이 프로젝트에는 config 시트가 없으므로 `Code.gs`처럼 시트에 되쓰지 않는다.
- 환율 조회는 **알릴 매물이 있을 때만** 한다. 대부분의 회차는 알릴 것이 없으므로 요청이 낭비되지 않는다.

## 엣지 케이스

| 상황 | 처리 |
|---|---|
| **첫 실행** — 상태가 비어 현재 매물 전부가 "신규"로 보임 | 저장된 상태 키 자체가 없으면 **일정을 만들지 않고 상태만 시드**하고 로그를 남긴다 |
| HTTP 실패, `code !== 0`, `data.list` 없음 | 그 컬렉션은 **이번 회차 건너뛴다. 상태를 건드리지 않는다** (다음 회차에 전부 신규로 오폭하는 것 방지) |
| 한 컬렉션만 실패 | 컬렉션별 try/catch — 나머지 하나는 정상 처리 |
| OKX가 응답 구조를 바꿔 파싱이 0건이 됨 | `data.list`는 왔는데 Rarity 추출이 전부 실패하면 로그에 경고를 남긴다 (`Code.gs`의 `checkAndLogAlerts` 발상) |
| 한 회차에 여러 건 | 매물마다 일정 1개. 제목에 정보가 담겨야 폰 알림에서 바로 보이기 때문이다. 리스팅 빈도가 낮아 스팸이 되지 않는다 |
| 환율 조회 실패 | 폴백 1450으로 계산하고 일정 본문에 `(환율 추정치)`를 덧붙인다. **알림 자체는 거른다거나 미루지 않는다** |
| 캘린더 생성 실패 / 권한 미승인 | 예외를 로그에 남기고 그 회차 종료. **상태는 저장하지 않는다** — 다음 회차에 다시 알리도록 |

## 캘린더 알림 형식

- **캘린더**: 전용 `OKX NFT` 캘린더. `getCalendar_()`가 이름으로 찾고 없으면 만든다(최초 1회).
- **일정 1건 = 매물 1건.**
- **시각**: 시작 = 현재 시각 + 2분, 길이 15분. 팝업 리마인더 **0분 전**.
  (시작을 정확히 '지금'으로 두면 리마인더가 발화하지 않을 수 있어 2분 여유를 둔다.)

**제목** — 폰 알림에 뜨는 줄이므로 요청하신 세 가지를 전부 여기에 담는다:

```
🆕 [Fashionista] Legendary #99 · 250,000 KAIA · $6,760 · 9,400,000원
🔻 [Society] Mystic #8093 · 60,000 KAIA ↓73,000 · $1,620 · 2,260,000원
```

**본문**:

```
컬렉션: Puuvilla Fashionista
등급:   Legendary
가격:   250,000 KAIA
달러:   $6,760
원화:   9,400,000원  (환율 1,392.00)
구분:   신규 리스팅            ← 인하면 "가격 인하 300,000 → 250,000 KAIA"
리스팅: 2026-08-02 18:01
https://web3.okx.com/nft/asset/kaia/0xe3d5.../99
```

숫자 표기는 이 저장소 관례를 따라 **통화 기호를 앞에 붙이지 않는다**. KAIA·원화는 `250,000 KAIA` / `9,400,000원`처럼 단위를 뒤에, USD만 `$` 접두.

## 검증 (완료 판정 기준)

Apps Script 편집기에서 순서대로 실행하고 아래가 전부 성립해야 완료로 본다.

1. `testOnce()` → 로그에 현재 Legendary/Mystic 매물이 **실제 값으로** 찍힌다. 등급이 전부 Legendary 또는 Mystic이고 가격·링크가 채워져 있어야 한다. (2026-08-08 확인 시점에는 Fashionista `#99 / Legendary / 250,000 KAIA`가 목록에 있었다. 그 사이 팔렸을 수 있으므로 이 항목 자체를 조건으로 삼지는 않는다.)
2. `resetState()` → `checkOnce()` 1회 실행: **일정 0건**, 로그에 "시드" 표시.
3. 이어서 `checkOnce()` 재실행: **일정 0건** (변화 없음).
4. 스크립트 속성에서 항목 하나를 지우고 `checkOnce()`: **🆕 일정 1건**이 `OKX NFT` 캘린더에 생기고, 제목에 **등급 · KAIA · $USD · 원화**가 모두 들어 있다.
5. 스크립트 속성에서 항목 하나의 `p`를 크게 올리고 `checkOnce()`: **🔻 일정 1건**, 제목에 `↓이전가`, 본문에 `가격 인하 이전가 → 현재가`가 올바르게 표시된다.
6. 원화 검산: `sale.usdPrice × 환율`(= 일정의 달러 × 환율)이 일정에 적힌 원화와 일치한다. 환율은 로그에 찍어 야후 값과 대조한다.
7. 4·5에서 만든 일정의 **팝업 알림이 실제로 폰에 떴는지** 확인한다. (Google 캘린더 앱에서 `OKX NFT` 캘린더 동기화가 켜져 있어야 한다 — 새 캘린더는 기본으로 꺼져 있을 수 있다.)
8. `setupOkxTriggers()` 실행 후 트리거 화면에 `checkOnce` 30분 트리거가 **정확히 1개**.

## 범위 밖 (하지 않는 것)

- 등급 외 조건(가격 상한, Tier, 랭크) 필터 — 요청에 없다.
- 시트 기록/이력 화면, `portfolio.html` 연동 — 알림만 필요하다.
- 설정 UI. 대상 등급·캘린더 이름은 파일 상단 상수로 고정한다.
- 메일·텔레그램 등 추가 알림 채널. 캘린더 하나만 쓴다.
- 지난 일정 정리/삭제. 일정이 쌓이면 캘린더에서 직접 지운다.
