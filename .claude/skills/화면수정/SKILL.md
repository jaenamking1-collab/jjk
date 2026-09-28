---
name: jjk-ui-edit
description: jjk 의 portfolio.html·m.html·dist_notice.html 화면을 고치고 확인해 푸시한다. 화면·표시·계산 관련 요청을 받았을 때 쓴다.
---

# 화면 수정·푸시

## 언제 쓰나

- 화면 모양·표시·프론트 계산을 바꾸는 요청. 백엔드(`Code.gs`)도 바뀌면 `배포` 매뉴얼을 이어서 쓴다.

## 순서

1. **검색으로 위치를 찾는다.** 파일이 크다(수천 줄) — 통째로 읽지 않는다. 함수는 0열에 `function 이름()` 이라 `^(async )?function <이름>` 으로 바로 찾는다.
2. **고친다.** Edit 로 필요한 줄만. 주변 코드를 다듬거나 이름을 바꾸지 않는다.
3. **확인한다** — `.claude/CHECKS.md` 의 **매번** 항목. 브라우저로 열어 콘솔 오류 0, 계산을 건드렸으면 PC·모바일 숫자 대조, 날짜 해석을 건드렸으면 `node tools/test_calstatus.js`.
4. **커밋·푸시.** 사용자는 웹 주소(GitHub Pages / 공개 페이지는 Cloudflare Workers)로 보므로 푸시하면 끝이다. 사용자 몫은 **새로고침뿐**.

## 결정 규칙

- **계산을 건드리면 `portfolio.html` 과 `m.html` 둘 다.** 서버는 평가금액·손익을 안 준다 — 브라우저가 계산하고, 같은 식이 `m.html` 의 `⚠️ 계산 블록` 에 따로 있다. 모양이 어긋나면 보이지만 숫자가 어긋나면 안 보인다.
- **공개 페이지(`dist_notice.html`)와 앱 화면은 일부러 다르다.** 앱만 보유종목을 안다(`isHeld`·`보유X`), 공개 페이지만 `shortLabel` 을 쓴다. "동기화"한다고 합치지 마라 — 공개 페이지에 남의 보유 개념이 들어간다.
- **같아야 하는 건 데이터 해석 규칙뿐:** 회차 판정(기준일의 '일', 20일 이하=월중), 예정 합의 문턱(`roundBest.n >= 3`), `uncertain`→`?` 배지, `fillRakil`·`parseDay`, `pubAfterBase`. 이걸 고치면 두 파일 모두.
- 표시 관례: KRW 는 기호 없이 숫자, USD 는 `$`. 분배 격자 안에 px 글꼴 크기 금지(`1em`). **회색은 못 받은 값·예정·누락에만** — 살아 있는 값은 작아도 본문색. 국내/해외가 섞인 계좌는 합계 / ㄴ국내 / ㄴ해외 세 줄.

## 공개 페이지 배포 경로

- 공개 주소는 **`https://jjk.distributionjn.workers.dev/` 하나**다(옛 주소는 계정명이 드러나 안 쓴다 — WORKLOG 156·159). **Cloudflare Workers 가 이 저장소의 `dist_notice.html` 을 그대로 서빙**하므로 `main` 에 푸시하면 자동 반영이다(설정 `wrangler.toml`·`.assetsignore`·`_redirects`. `.assetsignore` 가 이 파일 하나만 올린다 — 개인 앱·일지는 그 주소에서 404).
- 알림·카톡·문서가 가리키는 주소도 위 하나다(`Code.gs` 의 `_NOTICE_PAGE_URL`).
- 옛 주소 `https://jaenamking1-collab.github.io/jjk-dist/`(저장소 `jjk-dist` 의 `index.html`)는 예비로 살아 있다. `jjk-dist/.github/workflows/mirror.yml` 이 30분마다 이 파일을 받아 커밋한다(비밀값 불필요). 고친 직후 바로 반영하려면 `actions_run_trigger` 로 그 `mirror.yml` 을 돌린다. **손으로 복사하지 않는다.**
- 두 파일에 같은 이름으로 있는 함수: `renderMasterCalendar`·`renderDistGrid`·`renderNotices`·`distIssueNotices`. 2026-09-15 대조 때 `renderMasterCalendar` 만 159줄 달랐고 전부 의도된 차이였다(WORKLOG 164).

## 완료 확인

- 바꾼 화면을 실제로 열어 봤고 콘솔 오류가 0이다. 못 열었으면 "화면 확인"이라고 쓰지 않고 무엇으로 대신 확인했는지 적는다.
- 계산을 건드렸으면 두 파일의 요약 숫자가 같다.
- 공개 페이지를 고쳤으면 `CHECKS.md` 의 공개 페이지 대조(`probe` `pick=size`)가 저장소와 같다.

## 과거에 틀렸던 것

- 사용자에게 "PC 에서 `git pull` 하세요"라고 시켰다 — 한 세션에서 네 번(2026-09-11). 화면은 웹 주소로 보므로 필요 없다. WORKLOG 루틴의 틀린 기록을 근거로 삼은 것이었다.
- 공개 페이지 미러를 손으로 복사하다 네 번 빠뜨렸다(7/13·8/27·9/12 두 번). 지금은 Cloudflare 가 `main` 을 그대로 서빙하고, 옛 주소(`jjk-dist`)는 30분마다 자동 미러된다 — 손으로 복사하지 않는다.
- 두 화면이 `PLUS?` 와 `PLUS` 로 달라 렌더 코드를 의심했는데 그 순간 받은 데이터가 달랐던 것이었다(WORKLOG 164). 두 화면이 다르면 **데이터 먼저.**
- 계좌 국내/해외 금액을 회색으로 썼다가 지적받았다(2026-09-21) — 살아 있는 값이다.
- 시세를 못 받았는데 평단가로 계산해 평가금액이 총매입과 똑같아 보였다(WORKLOG 180). **못 받은 값을 멀쩡한 값처럼 그리지 않는다** — 낡은 값이면 낡았다고 표시한다.
