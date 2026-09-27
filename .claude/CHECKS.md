# jjk 검증 목록

주기별로 무엇을 확인하고, **무엇으로** 확인하는지. 설계: `docs/superpowers/specs/2026-09-20-working-manual-system-design.md`.
결과는 두 줄로 보고한다(무엇을 고쳤다 / 무엇으로 확인했다). **이상 없으면 한 줄도 쓰지 않는다.**

⚠️ **원격(클라우드) 세션은 `script.google.com`·`jjk.distributionjn.workers.dev` 에 직접 못 닿는다**(프록시 403, 2026-09-28 실측). 그래서 아래 백엔드·공개 페이지 점검은 **Actions 워크플로를 `actions_run_trigger` 로 돌리고 `get_job_logs` 로 읽는다.** PC 세션은 `curl` 로 바로 쳐도 된다.
로그는 `tail_lines` 를 작게(20~40) 준다 — 판정 줄은 끝 쪽(정리 로그 약 15줄 위)에 있다.

## 매번 (내가 만진 것 + 돈 숫자)

- [ ] **바꾼 화면을 실제로 열어 확인, 콘솔 오류 0.** 로컬: 파일을 브라우저로 연다(개발 서버 없음). 원격: Playwright(Chromium 설치돼 있음)로 열고 `page.on('console')` 오류를 센다. 브라우저를 못 열면 최소한 `<script>` 를 떼어 `node --check` 로 문법만이라도 본다 — 그땐 "화면 확인"이라고 쓰지 않는다.
- [ ] **계산을 건드렸으면 PC·모바일 요약 숫자 여섯 개 대조.** `portfolio.html` 의 `renderAccountStats` 와 `m.html` 의 `⚠️ 계산 블록` 에 같은 보유/분배 자료를 먹여 요약 여섯 칸(투자금·평가금액·손익 등)이 같은지 본다. 같은 식이 두 파일에 따로 있어 어긋나도 화면엔 안 보인다.
- [ ] **분배 날짜 해석을 건드렸으면** `node tools/test_calstatus.js`(앱·공개 페이지 대조), 파서를 건드렸으면 해당 `tools/test_*.js`. 목록: `tools/INDEX.md`.

## 배포 직후 (`Code.gs` 를 올렸을 때)

- [ ] **원격이 로컬과 같아졌는지.** PC: 임시 폴더에 `.clasp.json` 만 복사해 `clasp pull` → `diff` 로 저장소 `Code.gs` 와 비교(**저장소 안에서 `clasp pull` 금지**). Actions 배포(`deploy.yml`)면 그 로그의 '올라갈 파일 확인' 단계가 `Code.gs` 하나만 통과했고 `clasp push`·`clasp deploy` 가 성공인지.
- [ ] **트리거 재설치 — 빼먹지 마라.** `maint.yml` 을 `fn=resetAllTriggers` 로 돌리고 로그에 `✅` 13줄(트리거 15개)이 찍히는지 본다. 빠뜨려서 조용히 멈춘 적 세 번(WORKLOG 128 부근·149).
- [ ] **실제 주소로 액션 하나 호출해 옛 버전이 아닌지.** `probe.yml` 을 고친 액션으로(공개 액션이면 `action=<이름>`) 돌려 `HTTP 200` 과 새 응답 형식을 확인. 토큰 액션은 `latency.yml`.
- [ ] `/exec` 가 **드라이브식 'You need access' 403** 이면 코드 탓이 아니라 소유자 실행 승인이 풀린 것이다(WORKLOG 180). 이땐 `maint` 도 못 쓴다 — 편집기에서 `resetAllTriggers` ▶ 재승인을 사용자에게 부탁한다.

## 매일 첫 세션 (가볍게 — 이상 없으면 말하지 않는다)

- [ ] **안 올라간 커밋.** `git status --short` 와 `git fetch origin main && git rev-list --count origin/main..HEAD`. 둘 다 비어 있거나 0 이어야 한다.
- [ ] **분배 데이터가 멈췄는지.** `backend-health.yml` 을 돌린다(매일 09:30 KST 에도 자동으로 돈다). 로그의 `status=ok` 와 `partial=`(빈 값)을 본다. 기준은 공지창(8~12일, 23일~) 6시간, 평소 30시간, 한 운용사만 48시간. 열린 이슈(`🚨 백엔드 정지 감지`, `⚠️ 운용사 한 곳의 분배 데이터가 멈춤`)가 있는지도 본다 — 9/18 에 열린 이슈를 이틀간 아무도 안 봤다(WORKLOG 165).
- [ ] **공개 페이지가 저장소 최신판과 같은지.** `probe.yml` 을 `url=https://jjk.distributionjn.workers.dev/`, `pick=size` 로 돌려 `HTTP 200 · N bytes`·줄 수·`sha256` 을 `wc -c -l dist_notice.html`·`sha256sum dist_notice.html` 과 비교. 다르면 Cloudflare 반영이 막힌 것이다. 미러 누락으로 사용자만 옛 화면을 본 적 네 번.
  - `pick=size` 를 빼면 90KB 본문이 통째로 로그에 찍힌다 — 읽지 마라.

## 주간

- [ ] **6개 운용사 파서 전수.** `probe.yml` 을 `action=getDistributionAll`, `pick=sched` 로 돌려 운용사별 회차 일정을 본다. 빈손(0건)이거나 지난달 회차만 물고 있으면 캐시가 아니라 **파서**를 본다. 절차: `.claude/skills/분배금점검/SKILL.md`.
- [ ] **알림로그에 구조 변경 경고가 쌓였는지.** `알림로그` 시트(`getAlerts`)는 토큰 액션이라 공개 통로가 없다. 지금은 앱 **분배금공지 탭의 🔔 패널**로 본다. 에이전트가 직접 보려면 `Code.gs` 에 요약 함수를 만들어 `MAINT_ALLOW` 에 넣고 `maint` 로 돌려야 한다(아직 없음).
