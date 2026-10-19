---
paths:
  - "portfolio.html"
  - "m.html"
  - "calc.js"
  - "tools/calc_test.js"
  - "Code.gs"
---
# 앱 화면(PC·모바일) 규칙

> CLAUDE.md 에서 옮김(2026-10-14). PC 를 고치면 같은 턴에 모바일도 — 묻지 않는다.

### Conventions to preserve

- **Currency display**: KRW amounts are shown as plain numbers (no ₩ symbol); USD amounts keep a `$` prefix. The `USD ? '$' : ''` ternary and bare `toLocaleString()` are intentional — do not reintroduce a ₩ prefix on displayed values. The `₩` still inside the two `replace(/[₩$,↑↓▲▼+\s]/g,'')` regexes is functional (strips symbols before parsing a price) and must stay.
- **Font sizing**: dividend-grid cells use `font-size:1em` so the "글자" range slider (`applyDivFont`) can scale the whole grid uniformly. Avoid hardcoding px font sizes inside the grid.
- **CDN dependencies**: SheetJS (`xlsx.full.min.js`) and Pretendard font, both loaded from CDN in `<head>`.
- ⛔ **회색은 '살아 있지 않은 값' 전용이다.** 회색(`var(--text3)`·`muted`)은 **못 받은 값·아직 안 된 값·누락**에만 쓴다 — 시세없음, 예정(공시 전), 상류 응답 없음 같은 것. **살아 있는 실제 값은 작게 쓰더라도 본문색(`var(--text)`)으로 둔다.** 사용자는 회색을 "죽었거나 아직 안 됐거나 빠진 것"으로 읽는다(2026-09-21 본인 확인: *"회색은 죽은거나 아직 안된거나 누락일때 써"*). 2026-09-21에 계좌 국내/해외 금액을 회색으로 썼다가 지적받았다. **2026-10-01 또 지적: 분배금공지 '공지 종목 전체' 표의 머리글·회사명·종목 수·0.0% 가 회색이었다 → "그냥 검정색해". 표의 글자는 기본 검정으로 쓰고, 회색을 쓰고 싶으면 '이게 정말 죽은 값인가'를 먼저 따져라.**
- **계좌 그래프(대시보드) = 보유 평가액 + 기준일 뒤 현금**(2026-10-02 사용자 결정). 분배금은 **입력한 날이 아니라 지급일**에 더하고, 매수는 빼고 매도는 더한다(`loadPlCash`).
  사용자 원칙: *"다른 곳에서 입금하면 말해 줄게, 그 외에는 다 분배금 예수금으로 산 거야."* · *"매도나 외부에서 돈 넣었으면 앞으로 너에게 얘기할게"*(2026-10-02).
  ⛔ 들은 금액을 **저장소·Actions 로그(maint 의 arg 는 로그에 찍힌다)에 남기지 마라** — 공개다. ✅ **통로 있음(2026-10-13)**: 들으면 비공개 시트 **'외부입금'**(`EXT_DEP_SHEET_ID`, 열: 계좌이름·날짜·금액(+입금 −출금)·메모)에 한 줄 넣는다 — 드라이브 커넥터는 칸 수정이 안 되니 새 줄이 필요하면 시트를 새로 만들어 ID 를 바꾸거나 사용자에게 한 줄 추가를 부탁한다. 성적표 투자원금·예수금·XIRR 과 계좌 그래프 현금에 들어간다. 확인은 `maint _diagExtDeposits`(건수·이름 불일치만). ⛔ **말씀 안 하신 매수는 전부 분배금 예수금으로 산 것이다 — 예수금이 모자라도 투자원금에 더하지 마라**(2026-10-13 사용자: *"따로 입금하고 사는 상황이면 항상 얘기할 테니까 분배금 받은 걸로 사는 건 니가 계산해야 한다"*). 모자라면 예수금이 마이너스로 남고 화면에 ⚠ '분배금 입력 누락?' 으로 밝힌다.
- **없는 종목(수량 0)은 숨긴다**(2026-10-02 사용자 두 번 지적: *"없는 주식은 숨김처리하라고 했잖아"* — 0으로 바꾸는 게 아니라 숨김). 앱 올해 화면은 수량 0 **무조건** 숨김(원금월별 보유 달이 늦게 남아 '마지막 배당 전'으로 오판했던 `soldDone` 은 버림). 시트 분배금 탭은 `hideSoldDivRows`(매일 08:30)가 마지막 배당이 들어온 판 종목 줄을 숨긴다. 지우지 않는다.
- **IRP 분배금은 자동 입력**(`autoFillIrpDivs`, 매일 08:30 markInputCells 끝): 분배락 전 수량 × 좌당 분배금, 빈 칸만, 메모 '자동: N주 × M원'. 월배당은 기준월 칸, 분기·반기는 지급월 칸(사용자 시트 방식).
- **국내/해외는 나눠 보여준다.** 증권사 앱은 '국내 잔고'와 '해외 잔고'를 다른 화면에 둔다. 계좌 표는 **합계 / ㄴ국내 / ㄴ해외 세 줄**이고 수익금·분배금·분배율까지 각 줄의 투자금 기준으로 따로 낸다. 해외가 섞인 계좌만 세 줄이고 국내뿐이면 한 줄이다. 합쳐서만 보여주면 증권사 화면과 대조가 안 돼 **값이 맞는데도 틀린 것처럼 보인다**(2026-09-21).

## 파일

- `portfolio.html` — the entire frontend (desktop). Redirects to `m.html` when the viewport is ≤768px unless `?pc=1` is present; the redirect sits in `<head>` so it runs before the password gate.
- ⛔ **PC 화면(`portfolio.html`)에 기능·값을 넣으면 같은 턴에 모바일(`m.html`)에도 넣는다. 묻지 마라**(2026-10-06 사용자: *"모바일은 당연히 해야지 묻지도 마라"*). 마우스 롤오버(title)는 폰에서 안 보이니 모바일은 탭해서 펼치는 상세(`detail`)에 넣는다.
- `m.html` — **phone-only frontend** (bottom tab bar; 홈 / 종목 / 분배금 / 더보기). Same backend, same actions, same origin — so it reuses the `jjk_pw_v1` password fingerprint and needs no separate login. Design: `docs/superpowers/specs/2026-08-13-mobile-view-design.md`.
  - ⚠️ 계산식은 `calc.js` 한 곳에 있다(PC·폰 공용, 2026-10-12). 서버는 평가금액/손익을 주지 않고 브라우저가 계산한다. 고치면 `node tools/calc_test.js` 를 돌리고,
    숫자를 일부러 바꿨으면 `--update` 로 정답을 갱신한다. 푸시마다 `calc-test.yml` 도 돈다. 공개 페이지(`dist_notice.html`)는 `calc.js` 를 쓰지 않는다.
    `calc.js` 를 고치면 두 화면의 `calc.js?v=N` 숫자를 올린다(브라우저가 옛 파일을 쓰지 않게).
- **분배금 월합계 = 시트 분배금 탭 3행**(2026-10-19 사용자 결정). 둘 다 **IRP 포함·아버지 철저히 제외·판 종목(0주, 숨김) 배당 포함**. 앱은 판 종목을 줄로는 숨기되 '판 종목 N개(숨김)' 한 줄로 합계에 넣는다(`soldDivRow`). 시트 3행은 줄 번호를 박지 않은 SCAN 수식(`fixDivSheetTotals`)이다 — 줄 번호를 박으면 새 줄이 또 빠진다. 매일 `_divSheetVsApp` 이 계좌·종목·월로 대조해 어긋나면 카톡. 달러 배당은 양쪽 다 **받은 달 환율**(시트 4행, `_syncDivRates`)이다 — 오늘 환율은 평가액에만 쓴다(2026-10-19 사용자).
