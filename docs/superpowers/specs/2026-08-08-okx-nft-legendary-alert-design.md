# OKX NFT Legendary/Mystic 리스팅 알림 — 설계

작성일: 2026-08-08

## 목적

OKX Web3 마켓플레이스(Kaia 체인)의 두 컬렉션에서 **Legendary 또는 Mystic 등급 매물이 새로 판매 등록되거나 가격이 내려가면 Gmail로 알림**을 받는다.

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

## 구조

파일 하나, 주석 포함 150줄 안팎.

```
CONFIG (상수)
  COLLECTIONS = [{ slug, projectId, label }] × 2
  TARGET_RARITY = ['Legendary', 'Mystic']
  MAIL_TO = 'jaenamking1@gmail.com'

checkOnce()                    ← 30분 트리거가 부르는 진입점
  컬렉션마다 (각각 try/catch 분리):
    fetchListings_(projectId)  → UrlFetchApp POST, 실패 시 null
    항목별 extract_(item)      → { tokenId, rarity, price, currency, ... }
    대상 등급 + sale 있는 것만 남김
    diffAgainstState_()        → 알림 대상 산출 + 새 상태 반환
  모은 알림이 있으면 sendMail_(rows) 1통

fetchListings_(projectId)      HTTP + 응답 검증
extract_(item)                 필드 추출 (specialProperties 파싱 포함)
diffAgainstState_(pid, cur)    상태 비교
sendMail_(rows)                Gmail HTML 메일

===== 수동 실행 =====           (편집기에서 손으로 돌리는 것만 맨 아래)
setupOkxTriggers()             기존 트리거 삭제 후 30분 트리거 1개 생성
testOnce()                     메일 없이 현재 대상 매물을 로그로만 출력
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

**가격은 네이티브 `sale.price`로만 비교한다.** `usdPrice`로 비교하면 KAIA 시세가 내릴 때마다 가짜 "가격 인하"가 발생한다. USD는 메일에 참고용으로만 표시한다.

## 엣지 케이스

| 상황 | 처리 |
|---|---|
| **첫 실행** — 상태가 비어 현재 매물 전부가 "신규"로 보임 | 저장된 상태 키 자체가 없으면 **메일 없이 상태만 시드**하고 로그를 남긴다 |
| HTTP 실패, `code !== 0`, `data.list` 없음 | 그 컬렉션은 **이번 회차 건너뛴다. 상태를 건드리지 않는다** (다음 회차에 전부 신규로 오폭하는 것 방지) |
| 한 컬렉션만 실패 | 컬렉션별 try/catch — 나머지 하나는 정상 처리 |
| OKX가 응답 구조를 바꿔 파싱이 0건이 됨 | `data.list`는 왔는데 Rarity 추출이 전부 실패하면 로그에 경고를 남긴다 (`Code.gs`의 `checkAndLogAlerts` 발상) |
| 한 회차에 여러 건 | 메일을 여러 통 보내지 않고 **한 통에 표로** 담는다 |

## 메일 형식

제목: `[OKX] Legendary 신규 2건 · 가격인하 1건`

본문은 HTML 표 한 개. 열:

썸네일 · 컬렉션 · `#tokenId` · 등급 · 가격(KAIA, USD 병기) · 구분(🆕/🔻, 인하면 이전가 병기) · 링크

가격 표기는 이 저장소 관례를 따라 **통화 기호 없이 숫자 + 단위 문자열**(`250,000 KAIA`), USD만 `$` 접두.

## 검증 (완료 판정 기준)

Apps Script 편집기에서 순서대로 실행하고 아래가 전부 성립해야 완료로 본다.

1. `testOnce()` → 로그에 현재 Legendary/Mystic 매물이 **실제 값으로** 찍힌다. 등급이 전부 Legendary 또는 Mystic이고 가격·링크가 채워져 있어야 한다. (2026-08-08 확인 시점에는 Fashionista `#99 / Legendary / 250,000 KAIA`가 목록에 있었다. 그 사이 팔렸을 수 있으므로 이 항목 자체를 조건으로 삼지는 않는다.)
2. `resetState()` → `checkOnce()` 1회 실행: **메일 0통**, 로그에 "시드" 표시.
3. 이어서 `checkOnce()` 재실행: **메일 0통** (변화 없음).
4. 스크립트 속성에서 항목 하나를 지우고 `checkOnce()`: **🆕 메일 1통**, 그 tokenId만 들어있다.
5. 스크립트 속성에서 항목 하나의 `p`를 크게 올리고 `checkOnce()`: **🔻 메일 1통**, 이전가/현재가가 올바르게 표시된다.
6. `setupOkxTriggers()` 실행 후 트리거 화면에 `checkOnce` 30분 트리거가 **정확히 1개**.

## 범위 밖 (하지 않는 것)

- 등급 외 조건(가격 상한, Tier, 랭크) 필터 — 요청에 없다.
- 시트 기록/이력 화면, `portfolio.html` 연동 — 알림만 필요하다.
- 설정 UI. 대상 등급·수신 주소는 파일 상단 상수로 고정한다.
- 텔레그램 등 추가 알림 채널.
