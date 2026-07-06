# WORKLOG

두 PC(윈도우=직장 / 맥북=집)에서 Claude Code로 번갈아 작업할 때 대화 맥락을 이어받기 위한 작업 로그.
**최신 항목이 맨 위**에 오도록 추가한다. 세션 시작 시 이 파일을 먼저 읽고, 세션 종료 시 맨 위에 새 항목을 추가한다. 과거 항목은 수정하지 않는다.

## 루틴
- **작업 시작**: 폴더 열기 → `git pull` → 이 파일 읽기 → Claude Code 작업
- **작업 종료**: 맨 위에 새 항목 추가 → `git add .` → `git commit` → `git push`

---

## 2026-07-06 / Windows (직장)
**한 일**:
- CLAUDE.md 정비: init 형식 재작성, 백엔드 API 계약 문서화, Karpathy 작업 원칙, 두 PC 루틴/세션 연속성 규칙 추가.
- `Code.gs`(Google Apps Script 백엔드)를 repo에 추가 + 매 호출마다 ACE 파싱을 돌리던 top-level 디버그 2줄 제거.
- `portfolio.html`: 분배금 그리드 소계/합계 폰트 크기를 `1em`으로 통일, 표시되는 ₩ 기호 전부 제거(파싱용 정규식 속 ₩는 유지).
- 로컬 설치 도구용 `.gitignore` 추가(`.agents/`, `.claude/`, `skills-lock.json`).
- WORKLOG.md 신설 + 두 PC 동기화 루틴 확립.

**다음 할 일**: `Code.gs` 변경분을 Apps Script 편집기에 붙여넣고 **새 배포**(재배포)해야 실제 백엔드에 반영됨.
