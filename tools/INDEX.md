# 툴박스 목록

재사용 스크립트 목록이다. 새 도구를 만들면 여기에 한 줄을 더한다. 설명은 각 파일 머리 주석을 보고 적었다(2026-09-28 확인).

## `tools/` — 자체 점검과 시세 위젯

| 파일 | 무엇을 하나 | 언제 쓰나 |
|---|---|---|
| `test_calstatus.js` | 달력 '회차 상태 한 줄' 코드를 `portfolio.html`·`dist_notice.html` 에서 각각 떼어 같은 자료를 먹이고 결과가 같은지 본다 | 달력 상태 줄을 고친 뒤. `node tools/test_calstatus.js` |
| `test_distchg.js` | `dist_notice.html` 의 '전달 대비 증감' 셀 코드를 떼어 돌린다 | 증감 셀을 고친 뒤. `node tools/test_distchg.js` |
| `test_plus_ocr.js` | `Code.gs` 의 `plusOcrItems`(PLUS 공지 OCR 행 파싱)를 실제 공지 원문으로 돌린다 | PLUS 파서를 고친 뒤. `node tools/test_plus_ocr.js` |
| `test_sol_items.js` | `Code.gs` 의 `_solItems`(SOL 블로그 공지 표 파싱)를 형식별로 돌린다. SOL 은 회차마다 표기를 바꾼다 | SOL 파서를 고친 뒤. `node tools/test_sol_items.js` |
| `test_ticker_sources.py` | 가짜 tkinter·가짜 네트워크로 `ticker.pyw` 의 진짜 `refresh()` 를 돌려, 망을 하나씩 막으며 고르는 시세 경로와 화면 출처 문구가 맞는지 본다 | 위젯 시세 경로를 고친 뒤. `python3 tools/test_ticker_sources.py` |
| `ticker.pyw` | 바탕화면 실시간 시세 위젯(윈도우). 스스로 새 버전을 받아 갱신한다 | 위젯 본체. 고치면 푸시만 하면 각 PC 가 받아 간다 |
| `ticker_setup.bat` | 위젯 설치: 파이썬 확인/설치 → 최신 `ticker.pyw` 받기 → 바탕화면 바로가기. ⚠️ **CP949 로 저장**해야 한다(UTF-8 이면 cmd 가 줄을 자른다) | 새 PC 에 위젯을 깔 때 |
| `ticker_check.ps1` | 위젯의 지금 상태를 한 번에 본다: 도는 프로세스·코드 날짜·바로가기·남은 사본 | "위젯 적용됐나?"를 확인할 때. 답하기 전에 이걸 돌린다 |
| `ticker_move.ps1` | 위젯을 구글 드라이브(H:)에서 `%LOCALAPPDATA%\ticker` 로 옮기고 바로가기까지 고친다 | 위젯이 H: 에서 돌고 있을 때(언마운트되면 죽는다) |

`test_*.js` 는 모두 원본 파일에서 코드를 **떼어 와** 돌린다 — 복사본을 따로 두지 않는다. 떼는 기준 문자열을 바꾸면 테스트가 "못 찾음"으로 멈추니 같이 고친다.

## `.github/scripts/` — Actions 에서 도는 점검

원격(클라우드) 세션은 `script.google.com`·공개 주소(`workers.dev`)에 직접 못 닿는다(프록시 403, 2026-09-28 확인). 그래서 백엔드·공개 페이지 점검은 아래 워크플로를 `actions_run_trigger` 로 돌려 로그로 본다.

| 파일 | 무엇을 하나 | 돌리는 워크플로 |
|---|---|---|
| `check_backend.py` | `getDistributionAll` 의 운용사별 `savedAt` 나이를 재서 백엔드가 멈췄는지 판정(공지창 6시간 / 평소 30시간, 한 곳만 48시간 멈춤도 따로 잡는다). 이상하면 이슈를 연다 | `backend-health.yml`(매일 09:30 KST, 손으로도 가능) |
| `linkcheck.py` | `getEtfNoticesAll` 의 공지 링크를 하나씩 눌러 상태·최종 URL·제목을 찍는다 | `linkcheck.yml` |
| `pick.py` | 큰 JSON 응답에서 점 경로(`sources.plus`)만 골라 보여 준다 | `probe.yml` 의 `pick` 입력 |
| `check_clasp_files.py` | `clasp status` 출력이 `Code.gs` 하나뿐인지 확인 | `deploy.yml` |

그 밖의 워크플로: `probe.yml`(공개 액션 또는 임의 URL 을 쳐서 응답을 본다. `pick=size` 면 크기·해시만), `maint.yml`(`runMaint` 로 `MAINT_ALLOW` 함수 실행 — `resetAllTriggers` 등), `deploy.yml`(clasp push + 기존 배포 ID 로 deploy), `latency.yml`(응답 시간·시세 맞물림).
