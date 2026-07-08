# WORKLOG

두 대의 Windows PC(직장 / 집)에서 Claude Code로 번갈아 작업할 때 대화 맥락을 이어받기 위한 작업 로그.
각 항목은 장소(직장/집)로 구분한다. **최신 항목이 맨 위**에 오도록 추가한다. 세션 시작 시 이 파일을 먼저 읽고, 세션 종료 시 맨 위에 새 항목을 추가한다. 과거 항목은 수정하지 않는다.

## 루틴
- **작업 시작**: 폴더 열기 → `git pull` → 이 파일 읽기 → Claude Code 작업
- **작업 종료**: 맨 위에 새 항목 추가 → `git add .` → `git commit` → `git push`

---

## 2026-07-08 / — 시세 로드 안정화 (getSheetData 간헐 지연 대응)
- 증상: 새로고침 시 종목관리 현재가/평가금액/손익이 "-"로 멈춤. 추종·배당주기는 정상.
- 원인: 외부 '주식상황' 시트의 실시간 수식 재계산으로 `getSheetData`가 warm 1.8초 → cold 13.5초/타임아웃. 로드 시 동시 호출 경합으로 실패하면 `.catch(()=>({items:[]}))`가 빈 배열로 삼켜 시세가 "-"로 굳음. 브라우저 실측 확인.
- 수정(A, 프론트): `loadHoldingPrices`에 getSheetData 빈 결과 시 1.5초 후 1회 재시도(`fetchSheet`), 로딩 중 셀을 '…'로 표시. 67개 개별조회 폴백 대신 재시도 경로 사용.
- 즉시 회피책: 종목관리 "📈 시세 불러오기" 단독 클릭 → 정상.
- 다음 할 일(B, 근본): Code.gs `getSheetData`가 '주식상황' 재계산 대신 시세로그/snapshot 캐시에서 현재가 읽게 변경 → 재배포 필요.

## 2026-07-07 / 직장 — 종목관리 괴리율(NAV) 컬럼 추가
**목적**: 리밸런싱 지표로 괴리율(현재가 vs 기준가NAV) — 님 요청, "추세" 다음 컬럼. (지표 논의 결과 스냅샷 괴리율부터, 월별 추이 로깅은 추후 확장 후보)
**데이터 소스 확인**: Naver realtime API(`polling.../stock/`)엔 nav **없음**(시세만). ETF 리스트 API(`finance.naver.com/api/sise/etfItemList.nhn`)엔 **nav 있음**(1142개 ETF, 예: 161510 nav 25449). → 이걸 씀.
**구현**:
- `Code.gs`: `getNavMap()` 추가 — etfItemList 1회 호출 → `{티커:nav}` 맵, 1h 캐시. 라우터 `getNavMap` case 추가. **⚠️ 비공개 GAS 재배포 필요**.
- `portfolio.html`(renderHoldings): "추세" 다음 `괴리율` th + `disc-<id>` td 추가. `loadHoldingPrices`가 navMap 병렬 로딩 → 종목별 `(현재가−NAV)/NAV×100` 렌더(+프리미엄 빨강/−디스카운트 파랑, ±0.05% 이하 회색). 국내 ETF만(navMap에 있을 때), USD·비ETF는 `-`.
- 재배포 전엔 getNavMap 없어 괴리율 전부 `-`(graceful), 재배포 후 값 표시.
**검증**: 재배포 → 종목관리 "시세 불러오기" → 괴리율 값·부호 확인(예 161510 ≈ −0.1%). 표 컬럼 정렬 확인.

---

## 2026-07-07 / 직장 — "운용사별 일정" 표 디자인 개선
**증상**: 표가 지저분·선이 이상함. 원인: 모든 셀 1px 격자테두리(#d4c2e8) + 한 셀에 월중/월말 두 값을 내부 border-bottom으로 우겨넣어 인접 셀 분할선이 안 맞음 + 행 테두리와 겹침.
**수정**: 회차(월중/월말)마다 **실제 `<tr>`로 분리**, 운용사명은 `rowspan`으로 묶음. 셀 내부 분할선 제거 → 가로선만. 회차는 알약(pill) 배지(월중 파랑/월말 핑크), 날짜는 종류별 색(공시 amber·분배락 green·기준 blue·지급 red), 빈 값은 muted `·`. 격자테두리 → 운용사 그룹 사이 1px solid var(--border)만. 하드코딩 색(#d4c2e8/#c9b3e0) 제거하고 CSS 변수 사용.
- `portfolio.html`(renderMasterCalendar) + `dist_notice.html`(공개, 보유X만 뺀 동일본) 둘 다 적용 → jjk-dist 재배포.
- **후속**: portfolio.html은 전역 CSS(`td{border:1px solid #64748b}`, `th{background:#e2e8f0;text-transform:uppercase}`, `tr:last-child td{border-bottom:none}`)가 표에 세로격자·회색헤더·맨아래선제거를 강제 → 각 셀에 인라인 `border:0`+필요한 밑줄만 박아 눌러씀(인라인이 전역 선택자 이김). "맨 아래 선 혼자 없음"도 이 `tr:last-child` 규칙 때문 → 해결. 공개페이지 룩에 맞춤.

---

## 2026-07-07 / 직장 — PLUS 월말 해결 ✅ (원인=결제 미연결, 코드 아님)
**결론**: PLUS 월말 누락은 **비공개 GAS 프로젝트에 Google Cloud 결제(billing)가 연결 안 돼 Vision OCR이 막혀 있던 것**. 코드 문제 아님.
- 진단 경로(`diagPlus`/`diagPlus2` GAS에서 실행): ① `VISION_API_KEY` 존재(길이39)하나 OCR 0 → ② Vision 직접호출 원본응답 확인: 처음 `API_KEY_INVALID`(키 무효, 다른 대화서 삭제→복구) → 키 교체 후 `BILLING_DISABLED`(403, 결제 미연결) → 프로젝트#610977749646에 결제계정 연결(무료체험 크레딧 있음) → Vision HTTP 200, OCR 정상.
- 결과: `getDistribution('plus', true)` 재파싱 → **items 15개(월중 3 + 월말 12)**, 월말 sched 정확(공시 6/25·분배락 6/29·기준 6/30·지급 7/2). PLUS는 분배내역을 이미지로 공지하므로 OCR 필수 — OCR 살아나니 `fetchDist_plus`가 정상 동작(코드 수정 0).
- **최종 상태**: 오늘 프론트 cycleOf 월-경계 수정(별도 커밋) + 이 결제 연결 = PLUS 월중·월말 둘 다 정상.
**선택 개선(미적용)**: `ocrImageText`가 Vision 에러를 조용히 `''`로 삼켜 진단이 어려웠음 → 실패 시 `console.log`로 에러 남기면 다음 OCR 장애 진단이 쉬움(백엔드 수정+재배포 필요, 지금은 안 함).

---

## 2026-07-07 / 직장 — CLAUDE.md 작업원칙 보강
**한 일**: karpathy-guidelines 스킬(github.com/multica-ai/andrej-karpathy-skills) 읽고 CLAUDE.md "Working principles" 보강. `.claude/`는 gitignore라 스킬 파일 설치는 두 PC 동기화 안 됨 → git 동기화되는 CLAUDE.md에 원칙을 박는 방식 채택.
- 추가: 원문 링크 + "속도보다 신중, 사소한 건 재량" 단서 / 원칙2 자가체크("시니어가 과하다 할까? 200줄→50줄이면 재작성") / 원칙3 "내 변경이 만든 고아만 제거, 기존 죽은 코드는 신고" + "바뀐 줄이 요청에 직결되는가" 테스트 / 원칙4 "재현 먼저(테스트러너 없으니 콘솔로 `_distData` 덤프 등) → 수정 → 재현 소멸 확인".

---

## 2026-07-07 / 직장 — PLUS 월중/월말 오분류 버그 수정 🐛
**증상**: portfolio.html "운용사별 일정" 표에서 PLUS가 월중 줄 비고 월말 줄에만 뜸(`플러스가 안나오네`).
**원인 확정**(브라우저 콘솔 `_distData` 덤프로): PLUS 데이터는 smarttoday 기사 출처라 아이템에 `cycle` 없음 + 대표 `schedule` 기준일 = **6월 15일**(명백한 월중, title도 "PLUS 월중 배당"). 그런데 `cycleOf`가 `base.m === month(=7월) ? 월중 : 월말` → 6≠7이라 **월말로 오분류**. **월 경계 버그**(7월에 6월 데이터 보면 전부 월말).
**수정**: 회차 판정을 월 비교 → **기준일의 '일(day)' 기준**(20일 이하=월중, 초과=월말)으로 변경. 월 경계 안전. 명시 `it.cycle` 있으면 그대로 우선.
- `portfolio.html` 2곳: `renderMasterCalendar`의 cycleOf(캘린더+일정표), `renderDistGrid`의 cycleOf(보유종목 그리드).
- `dist_notice.html` 1곳(공개 페이지) 동일 수정 → jjk-dist repo에도 재배포.
**검증**: PLUS 기준일 6/15 → 월중 ✓. **portfolio.html 열린 탭 하드새로고침(Ctrl+Shift+R)으로 육안 확인 필요.**
**남은 것(선택)**: 백엔드 `fetchDist_smarttoday`가 PLUS **월말 일정은 아예 안 가져옴**(왼쪽 공지엔 PLUS 월말배당 06.25 있음). 완전하려면 백엔드에서 월말 기사도 파싱 필요(GAS 수정+재배포). 분류 버그와는 별개.
**→ 월말 누락 근본원인 규명(같은 세션)**: PLUS는 분배내역을 **이미지로만** 공지(plusetf 상세 n=30821 = 텍스트표 아님, `hwadm.plusetf.co.kr/webapp/upload/*.PNG`). `fetchDist_plus`는 OCR(`ocrImageText`, Google Vision) 의존인데 `ocrImageText`는 `VISION_API_KEY` 스크립트 속성 없으면 즉시 `''` 반환 → PLUS 0건 → `fetchDist_smarttoday` 폴백. 근데 smarttoday는 **브랜드당 기사 1건**만 수집(브랜드당 첫 매칭에서 `found[brand]` 차단) → PLUS는 월중 기사 하나만 → **월말 구조적 누락**.
- **조치(님 몫)**: 비공개 GAS 편집기 → 프로젝트 설정 → 스크립트 속성에서 `VISION_API_KEY` 확인. 진단함수 `diagOcrPlus`(PLUS 이미지 OCR 테스트)로 키/OCR 상태 확인. 키 없으면 Google Vision 키 발급·등록 → `getDistribution('plus', true)`로 강제 새로고침. 키는 자격증명이라 Claude가 안 만짐.
- OCR 살아나면 fetchDist_plus가 월중·월말 둘 다 정상 파싱 예상. 그래도 안 되면 parsePlusNotice regex 조정 필요.

---

## 2026-07-07 / 직장 — 공개 페이지 호스팅 완료 ✅
**한 일**: 호스팅 결정 → **(a) 별도 공개 repo** 채택. 새 repo `jjk-dist` 생성 → `dist_notice.html`을 `index.html`로 복사해 push → GitHub Pages 켬.
- **공개 URL**: https://jaenamking1-collab.github.io/jjk-dist/ (HTTP 200 확인).
- 안전 재검증: 공개 페이지에 개인 GAS/`portfolio` 노출 **없음**, `getHoldings` → `Not allowed` 확인.
- 구조: `jjk-dist`는 **배포 전용**. 원본 수정 시 `jjk/dist_notice.html` → `jjk-dist/index.html` 재복사 후 push 필요(해당 repo README에 명시). 폴더: `C:\Users\azsxd\Documents\jjk-dist`.

**⚠️ 새 이슈 (다음 세션 백엔드에서 확인 필요)**: 07-07 현재 백엔드 데이터가 **전부 빈값**.
- `getDistribution` → `{"items":[],"error":"기사/사이트 모두 실패"}`
- `getEtfNotices`(kodex·ace·tiger·plus) → `{"success":true,"items":[]}` (전부 빈 배열)
- 07-06 "검증 완료"였는데 하루 뒤 빈값 → **캐시 만료 후 재-깨짐 / 비공개 GAS 배포 되돌아감 / 상류 사이트 차단** 중 하나 의심.
- 조치 후보: 비공개 GAS 편집기에서 실 fetch 로그로 재검증, 필요 시 재배포 + `clearNoticeCache` + `notices_*` 캐시 제거. 공개 프록시는 중계만 하므로 손댈 것 없음.

**남은 것**: 호스팅은 완료. 위 **데이터 빈값 이슈만 비공개 GAS에서 확인**.

---

## 2026-07-06 / 집 — 검증 완료 ✅
**한 일**: 비공개 GAS 재배포 + 공지 캐시 초기화(`clearNoticeCache`) 후 공개 프록시로 재검증.
- **6개 운용사 공지 전부 정상**(TIGER·PLUS 서버 파서도 실 HTML과 일치 확인).
- 캘린더(`getDistribution`)·운용사 일정·보안 차단(`getHoldings`→`Not allowed`) 모두 정상.
- 백엔드/페이지 기능 완료.

**남은 것 (다음 세션 재개 지점)**: `dist_notice.html` 호스팅만 남음.
- ⚠️ 이 repo로 GitHub Pages 켜면 `portfolio.html`도 `.../jjk/portfolio.html`로 공개돼 **개인 GAS URL 노출**(우리가 막으려던 문제). → **공개용 별도 repo에 `dist_notice.html`만 올리는 것 권장.**
- 결정 필요: (a) 공개 전용 새 repo 만들어 그 파일만 배포, (b) 이 repo Pages + 위험 감수.
- 공개 프록시 URL은 `dist_notice.html`의 `API`에 이미 박힘(안전 — 개인데이터 차단 검증됨).

---

## 2026-07-06 / 집 — (b) 공개 페이지 작성 완료
**한 일**:
- `dist_notice.html` 생성 — 반응형 공개 페이지(3패널: 공지·캘린더·운용사일정). 보유종목 강조·보유X 태그·하단 보유표 전부 제거. `API` = 공개 프록시 URL placeholder. 6개 운용사 공지를 모두 프록시 `getEtfNotices`로 통일(브라우저 CORS 제거). 모바일은 세로 스택 + 캘린더/일정 좌우 스크롤(min-width로 5열 유지).
- `Code.gs`의 `getEtfNotices`에 **tiger·plus 분기 추가**(서버측 파싱). → **비공개 GAS 재배포 필요**.

**진행 상황**:
- ✅ 공개 프록시 배포됨. URL을 `dist_notice.html` `API`에 반영(커밋).
- ✅ 보안 검증: 공개 URL `?action=getHoldings` → `{"error":"Not allowed"}` (개인데이터 차단).
- ✅ `getDistribution`(캘린더) + KODEX/ACE/RISE/SOL 공지 정상.
- ⚠️ TIGER·PLUS 공지 `items:[]`. 원인: 비공개 GAS 재배포 전이거나, `getEtfNotices` 6h 캐시에 옛 빈결과가 남음(getEtfNotices는 force 없음 → 빈값도 캐시).

**다음 할 일**:
1. 비공개 GAS(`Code.gs`, tiger/plus 분기 포함) **재배포**(기존 배포 "새 버전").
2. 비공개 GAS 편집기에서 공지 캐시 비우기: `CacheService.getScriptCache().removeAll(['notices_tiger','notices_plus'])` 한 줄 함수 실행.
3. 재검증 — tiger/plus 공지 뜨는지. 안 뜨면 서버 정규식 조정(실 HTML 확인).
4. `dist_notice.html` GitHub Pages 배포.

---

## 2026-07-06 / 집
**한 일**: 분배금공지 공개 페이지 착수 — 백엔드 (a) 단계. `public_dist_proxy.gs` 생성(얇은 프록시: `getDistribution`·`getEtfNotices`만 화이트리스트로 비공개 GAS에 중계, 개인 데이터 액션 차단, force 무시, 30분 캐시). 실제 비공개 URL은 파일에 넣지 않고 placeholder 유지(공개 repo 대비).
- 구조 결정: **얇은 프록시** 채택(파서 복붙 대신) → 단일 소스, 복제 0.
- 권한 프롬프트 감소 위해 `.claude/settings.local.json` allow/deny 정비(로컬 전용).

**다음 할 일 (b 단계)**:
1. `portfolio.html` 분배탭 렌더 로직(`loadDistributions`·`renderMasterCalendar`·`loadAllNotices`·`normalizeNoticeTitle`) 읽고, **보유종목 강조·보유X 태그 제거**한 반응형 공개 페이지 `dist_notice.html` 1개 작성. API 상수 = 공개 프록시 URL.
2. TIGER·PLUS 좌측 공지 서버화: 비공개 `Code.gs`의 `getEtfNotices`에 tiger/plus 분기 추가(실 HTML 구조 확인 후) → 재배포.
3. 폐기: `dist_notice_pc.html`/`dist_notice_mobile.html`(이전 세션 산출물, GAS URL 노출) — repo엔 없음.
**배포(님 몫)**: 공개 프록시 GAS 새 프로젝트에 붙여넣기 → PRIVATE_URL 교체 → 웹앱(모든 사용자) 배포 → 공개 URL을 페이지에 사용.

---

## 2026-07-06 / Windows (직장) — 후속
**한 일**: `Code.gs` 변경분(top-level 디버그 2줄 제거)을 Apps Script 편집기에서 **재배포 완료**. 실제 백엔드에 반영됨.

**다음 할 일**: (없음)

---

## 2026-07-06 / Windows (직장)
**한 일**:
- CLAUDE.md 정비: init 형식 재작성, 백엔드 API 계약 문서화, Karpathy 작업 원칙, 두 PC 루틴/세션 연속성 규칙 추가.
- `Code.gs`(Google Apps Script 백엔드)를 repo에 추가 + 매 호출마다 ACE 파싱을 돌리던 top-level 디버그 2줄 제거.
- `portfolio.html`: 분배금 그리드 소계/합계 폰트 크기를 `1em`으로 통일, 표시되는 ₩ 기호 전부 제거(파싱용 정규식 속 ₩는 유지).
- 로컬 설치 도구용 `.gitignore` 추가(`.agents/`, `.claude/`, `skills-lock.json`).
- WORKLOG.md 신설 + 두 PC 동기화 루틴 확립.

**다음 할 일**: `Code.gs` 변경분을 Apps Script 편집기에 붙여넣고 **새 배포**(재배포)해야 실제 백엔드에 반영됨.
