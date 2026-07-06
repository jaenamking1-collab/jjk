# WORKLOG

두 대의 Windows PC(직장 / 집)에서 Claude Code로 번갈아 작업할 때 대화 맥락을 이어받기 위한 작업 로그.
각 항목은 장소(직장/집)로 구분한다. **최신 항목이 맨 위**에 오도록 추가한다. 세션 시작 시 이 파일을 먼저 읽고, 세션 종료 시 맨 위에 새 항목을 추가한다. 과거 항목은 수정하지 않는다.

## 루틴
- **작업 시작**: 폴더 열기 → `git pull` → 이 파일 읽기 → Claude Code 작업
- **작업 종료**: 맨 위에 새 항목 추가 → `git add .` → `git commit` → `git push`

---

## 2026-07-06 / 집 — (b) 공개 페이지 작성 완료
**한 일**:
- `dist_notice.html` 생성 — 반응형 공개 페이지(3패널: 공지·캘린더·운용사일정). 보유종목 강조·보유X 태그·하단 보유표 전부 제거. `API` = 공개 프록시 URL placeholder. 6개 운용사 공지를 모두 프록시 `getEtfNotices`로 통일(브라우저 CORS 제거). 모바일은 세로 스택 + 캘린더/일정 좌우 스크롤(min-width로 5열 유지).
- `Code.gs`의 `getEtfNotices`에 **tiger·plus 분기 추가**(서버측 파싱). → **비공개 GAS 재배포 필요**.

**다음 할 일 (배포 · 님 몫)**:
1. 비공개 GAS(`Code.gs`) 재배포 — tiger/plus 공지 반영.
2. 공개 프록시(`public_dist_proxy.gs`) 새 프로젝트 배포 → `PRIVATE_URL` 교체 → 웹앱(모든 사용자) → 공개 exec URL 발급.
3. `dist_notice.html`의 `API` 상수를 그 공개 URL로 교체 후 GitHub Pages 등에 배포.
4. 검증: 공개 URL에 `?action=getHoldings` 쳐서 `Not allowed` 나오는지(개인데이터 차단), `?action=getEtfNotices&source=tiger`/`plus` 정상 파싱되는지 확인. tiger/plus 서버 정규식은 실 HTML 대비 미검증이라 안 나오면 알려주면 조정.

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
