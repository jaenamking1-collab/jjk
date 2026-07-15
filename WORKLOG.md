# WORKLOG

두 대의 Windows PC(직장 / 집)에서 Claude Code로 번갈아 작업할 때 대화 맥락을 이어받기 위한 작업 로그.
각 항목은 장소(직장/집)로 구분한다. **최신 항목이 맨 위**에 오도록 추가한다. 세션 시작 시 이 파일을 먼저 읽고, 세션 종료 시 맨 위에 새 항목을 추가한다. 과거 항목은 수정하지 않는다.

## 루틴
- **작업 시작**: 폴더 열기 → `git pull` → 이 파일 읽기 → Claude Code 작업
- **작업 종료**: 맨 위에 새 항목 추가 → `git add .` → `git commit` → `git push`

---

## 2026-07-15 (14) / 집 — 은경 저축 편의기능 5종 (Tab이동·날짜자동포맷·순서변경·메뉴맨끝)
- **Tab 옆칸 이동**: 편집 시 `renderSavings()` 전체 재렌더 제거가 핵심 — `svSet(id,k,el)`가 해당 칸만 재포맷(`el.value`)하고 합계는 `svUpdateTotals()`로 인플레이스 갱신(합계 셀에 `sv-sum-p/m/e` id). 재렌더 안 하니 네이티브 Tab이 그대로 옆 input으로 이동. ▲▼✕ 버튼은 `tabindex=-1`로 Tab에서 제외 → 데이터 칸끼리만 이동.
- **날짜 자동 포맷**: 날짜칸을 `type=date`→`type=text`(placeholder 20261101)로 교체. `svFmtDate`가 숫자만 추출해 8자리면 `YYYY-MM-DD`로 변환. 텍스트라 월/일 클릭 후 키보드 수정도 됨(수정 후 blur에 재포맷).
- **순서 변경 ▲▼**: `svMove(id,dir)` — **인접 두 행의 데이터를 맞바꿈(id·행위치 고정)** 방식이라 백엔드 `moveRows` 없이 기존 updateSaving 2번으로 처리 → **재배포 불필요**. 첫 행 ▲/끝 행 ▼ disabled. (created_at은 id에 묶여 유지되나 미표시라 무관.)
- **메뉴 맨 오른쪽**: 은경 저축 탭 버튼을 엑셀 뒤(제일 오른쪽)로 이동.
- 표 아래 안내문(날짜 8자리·Tab·▲▼) 추가. 검증: 로컬서버 — nav 마지막·날짜 20261101→2026-11-01(표시+백엔드 실측 persist)·편집후 동일 input 유지(Tab)·금액 재포맷+합계 인플레이스·▲로 순서변경+저장 페이로드 확인·첫끝행 버튼 disabled·콘솔 무에러. 프론트만 — **재배포 불필요.**
- ⚠️ 참고: 백엔드에 사용자 실입력 데이터 발견(`SMART` 원금 5,033만·만기 2026-11-01 + 빈 행 1) — 건드리지 않고 그대로 둠.

## 2026-07-15 (13) / 집 — 은경 저축 표 디자인 개편 (그룹 구분·컴팩트·중앙정렬)
- **3구역 그룹화 + 2단 헤더**: 계좌명 | **원금**(파랑) | **만기**(초록: 이율·날짜·금액) | **중도해지**(앰버: 이율·날짜·금액). 그룹 경계마다 3px 굵은 색 세로선(`.bd-b/.bd-g/.bd-a`), 각 구역 옅은 색 배경(body 5~7%, header 16~20%), 헤더 글자색도 구역별(파랑#2563eb/초록#0f766e/앰버#b45309). 색은 테마가 안 바꾸는 고정 시맨틱 색이라 전 테마 안전.
- **컴팩트·중앙정렬**: `width:fit-content;margin:0 auto`로 표를 내용 폭(약 930px)으로 줄이고 카드 중앙 배치(이전엔 카드폭 꽉 채워 늘어남). 입력칸 고정폭(이름118·금액92·이율46·날짜118), 숫자 오른쪽정렬이라 빈 공간 없음. 둥근 모서리12px+그림자. `.pp-plain` 대신 저축 전용 `.sv-in` 도입.
- **제목 가운데 정렬**: 카드 헤더를 position:relative 중앙정렬 제목(18px)+추가 버튼 우측 절대배치로 교체.
- 검증: 로컬 서버 computed-style 측정 — 그룹 테두리 색·배경·헤더색·2단 colspan·표너비 932(카드1232)·좌우여백 150=150 중앙정렬·셀 내용맞춤·콘솔 무에러 확인. (브라우저 스크린샷 서브시스템이 세션 내내 타임아웃이라 측정값으로 검증) 프론트만 — 재배포 불필요.

## 2026-07-15 (12) / 집 — 은경 저축 백엔드 재배포 완료 + CRUD 실측 검증
- Code.gs 재배포 완료(배포 관리에서 앱 URL과 배포 ID 일치하는 활성 배포를 "새 버전"으로 갱신 — "제목 없음" 활성 배포가 여러 개라 이전에 엉뚱한 걸 눌러 헛돌았던 것). 재배포 후 `/exec?action=getSavings`가 `Unknown action`→`[]`로 정상.
- 실제 앱 경로(브라우저 fetch/apiPost)로 add·update·delete 왕복 실측: 추가 후 전 필드 정확, 한글 이름 정상, 빈 날짜 `""` 보존, 수정 시 전 필드 갱신, 삭제 후 `[]`. 시트탭 `은경저축` 자동생성 확인. 테스트 행은 모두 삭제해 시트 비움.
- 참고: curl 직접 POST는 302 리다이렉트 재전송 때 411/한글깨짐 발생 — curl 아티팩트일 뿐 앱 fetch 경로는 정상. **이제 은경 저축 탭 실사용 가능.**

## 2026-07-15 (11) / 집 — "은경 저축" 탭 신설 (정기예금·적금 만기/중도해지 관리)
- 새 탭 **💵 은경 저축**: 계좌명·원금·만기이율·만기날짜·만기시금액·중도해지이율·중도해지일·중도해지금액 8컬럼 인라인 편집 표(`.sv-tbl` + `.pp-plain` 재사용). ➕추가로 빈 행 생성, 셀 onchange 자동 저장, 행마다 ✕삭제. 맨 아래 합계 행(원금·만기시금액·중도해지금액). 금액은 ₩ 없이 콤마 숫자, 이율은 %.
- 금액/이율은 **직접 입력**(요청). 나중에 원금·이율로 자동 계산 전환 여지 열어둠 — 컬럼 구조는 그대로.
- **저장은 구글 시트 백엔드**(두 PC 동기화). `Code.gs`에 시트탭 `은경저축` 자동생성 + `getSavings`/`addSaving`/`updateSaving`/`deleteSaving` 추가(holdings CRUD와 동일 패턴). 날짜는 `'` 프리픽스로 텍스트 저장(시트 날짜 자동변환 방지).
- **⚠️ 다음 할 일: Code.gs 수동 재배포 필요.** 재배포 전엔 저장·불러오기 안 됨(프론트는 빈 표로 안전하게 폴백). 재배포 후 추가/수정/삭제 실측 확인할 것.
- 검증: 로컬 서버로 띄워 탭 활성·헤더 8개·빈 상태·가짜데이터 렌더·합계(15,000,000/15,550,000/15,070,000)·입력 파싱(콤마·% 제거)·콘솔 무에러 확인. (백엔드는 미배포라 CRUD 왕복은 재배포 후 검증)
- 두 표 모두 [계좌명 20 | 월 배당금(평균) 21 | 연간 합계 21 | 평가금액 21 | 개시년도 9 | 액션 8%] 동일 그리드. 연금계좌 헤더 월 수령액→월 배당금(평균), 연 수령액→연간 합계. ISA 표의 개시년도·액션 칸은 테두리 없는 빈 칸(표가 평가금액에서 끝나 보임).
- 연금계좌에 평가금액 추가: loadPensionPlan에서 전 계좌 `valByAccount` 집계(현재가×수량, USD 환산) → 계좌 연결 행만 표시, 수동 행(우체국)은 빈칸.
- 액션(✏️ 수정=계좌명 포커스, ✕ 삭제)은 테두리 밖 무테두리 칸으로 이동. 안세공 배지 회색→앰버(#f59e0b) — **회색 금지 피드백 메모리 저장함**. ISA 표 값 색 text2→text.
- 검증: 콘솔에서 두 표 th 너비 완전 일치·헤더 통일·평가금액 표시·수동행 빈칸·✏️/✕ 무테두리 확인, 무에러.

## 2026-07-15 (9) / — ISA·일반 표 계좌명도 인라인 수정 가능
- ISA/일반계좌 라벨을 pp-plain 인라인 입력으로 교체. 표시 이름은 `ppCfg.baseNames[k]` 오버라이드(로컬 저장), 비우면 기본(ISA/일반계좌) 복귀 — `ppSetBaseName`. 집계 키(재남_ISA 등)는 그대로라 계산엔 영향 없음.
- 검증: 콘솔에서 기본 렌더·이름 변경·빈값 복귀 3단계 확인, 무에러.

## 2026-07-15 (8) / — 연금플랜 ISA·일반 표 헤더 '구분' → '계좌명'
- 한 단어 수정. 콘솔 렌더 확인, 무에러.

## 2026-07-15 (7) / — 연금계좌 구분 컬럼 → 계좌명 옆 배지
- 세공/안세공 구분 컬럼 삭제 → 계좌명 셀 안 오른쪽 배지(pill)로 이동(세공=accent 파랑, 안세공=회색 테두리, 클릭 토글 동일). 계좌명 셀은 flex(input min-width:0 + 배지 flex:none).
- 연금계좌 표 th 너비를 위 ISA·일반 표와 정렬: 계좌명 19% / 월 수령액 27% / 연 수령액 27%(= 구분·월배당·연간합계 열과 동일) / 개시년도 17% / 삭제 10%. 참고 문구 갱신.
- 검증: 콘솔에서 두 표 th 너비 [19,27,27,…] 일치·배지 렌더 확인, 무에러.

## 2026-07-15 (6) / — 연금플랜 표 개편 (ISA·일반 컬럼 교체 + 연금계좌 표 평문화)
- **ISA·일반 표**: 컬럼을 [구분 | 월 배당금(평균) | 연간 합계 | 평가금액]으로 교체. 직접입력(월)·적용(월) 삭제 → `ppCfg.base` 오버라이드·`ppSetBase` 제거, ppCalcAt은 자동 월평균만 사용. 평가금액은 getSheetData 현재가(없으면 평단가)×수량, USD는 환율 환산 — loadPensionPlan에서 계산해 `ppAuto.byBase[k].val`에 저장.
- **연금계좌 표**: 셀을 위 표처럼 평문 스타일로(`.pp-plain` — 평소 투명, hover/focus 때만 입력칸 표시). 🔄 버튼·'자동' 배지 삭제(연 수령액 비우면 자동 복귀, title 힌트). 구분(세공/안세공)은 select → 클릭 토글(`ppToggleTax`), ➕추가 시 confirm으로 설정. 계좌명은 계속 인라인 수정 가능.
- 두 표 모두 `table-layout:fixed` + th 너비 % 지정으로 컬럼 폭 통일. 참고 문구도 갱신. 검증: 콘솔에서 ppRenderOwnerCol 출력 검사(새 헤더/옛 컬럼 제거/토글/평가금액 표시) + 무에러. 프론트만 — 재배포 불필요.

## 2026-07-15 (5) / — 연금플랜 ISA 행 0원 버그 수정
- `ppAccClass`가 type `'일반'`도 표준값으로 신뢰해 계좌명 판별을 건너뜀 → DB의 계좌 type이 전부 `'일반'`(기본값)이라 ISA·연금저축 계좌까지 일반으로 분류, 연금플랜 탭 ISA 행이 0이 되고 그 몫이 일반계좌 행에 합산되던 버그. `'일반'`을 신뢰 목록에서 빼서 계좌명 판별로 넘어가게 수정 (portfolio.html 한 줄).
- 검증: 콘솔에서 `ppAccClass({type:'일반',name:'재남 키움 ISA'})` → `'ISA'`, 일반계좌·연금저축·명시적 type ISA 모두 정상, 콘솔 무에러. 프론트만 수정 — 재배포 불필요.

## 2026-07-15 (4) / — 공지 페이지: 종목 목록을 하단 별도 섹션으로 (앱과 동일 레이아웃)
- 공지 카드 안 접이식 "공지 종목 전체 N개"(stk-details) 제거 → 페이지 하단에 **📋 공지 종목 전체** 섹션 신설. 앱의 '내 보유 종목 분배 내역'과 동일한 운용사별 카드 3열 그리드(모바일 1열) + 주기/코드/종목명/분배금/분배율 표. 카드 본문 max-height 340px 스크롤.
- 표시 대상: 현재 공지 종목만(hist 이력·기준일 전월~익월 밖 제외). 앱 renderDistGrid에도 `it.hist` 제외 가드 추가(이력 쌓이면 지난 회차 섞임 방지).
- 프론트만 수정 — Code.gs 재배포 불필요. 실측: KODEX 45/TIGER 33/ACE 19/PLUS 16/RISE 26/SOL 6종목, 콘솔 무에러.

## 2026-07-15 (3) / — 지난 회차 이력 + NEW 7일 + force 타임아웃 90초
- **분배캐시 회차별 이력(Code.gs)**: `writeDistCache`를 source당 회차별 행(최근 3회차 유지)으로 변경, `attachDistHistory`가 이전 2회차 종목을 items에 병합(종목 sched 없으면 그 회차 대표 일정 주입, `hist:true` 표시). 모든 반환 경로를 `finishDist` 헬퍼로 통일 — 기존엔 kodex~plus 분기가 시트캐시를 아예 안 썼음(버그). → **지난 달 운용사별 일정·공지 종목이 계속 보임**(이력은 지금부터 쌓임 — 6월 월중 소급은 ACE 등 시트에 6월 행 남은 소스만 가능). **⚠️ 재배포 필요.**
- **NEW 배지 5일→7일**(양쪽 프론트): 월중 공시(10~13일) 흐름을 다 덮게. 7/10 등록 PLUS·RISE가 5일째에 꺼지던 문제.
- **force 새로고침 타임아웃 25→90초**(양쪽 프론트): KODEX OCR 재파싱이 25초 초과 → 새로고침 직후 달력·표에서 KODEX 통째 누락되던 원인.

## 2026-07-15 (2) / — ACE 파서 수정 (papi WAF 차단 대응)
- **원인 조사**: papi.aceetf.co.kr가 Apps Script 요청에만 HTML 차단 페이지 반환(로컬 PC에선 UA 무관 200, 타 사이트 Origin 헤더면 403). 구글 IP/봇 필터로 추정.
- **수정**: `fetchAceApi()` 헬퍼 신설 — 브라우저형 헤더(Chrome UA, Accept, Accept-Language, Referer)로 요청, HTML 응답이면 `ACE API 차단 (HTTP xxx)` 명시적 에러. `getEtfNotices(ace)`·`fetchDist_ace`(목록/본문/body 3곳) 모두 이 헬퍼로 교체.
- **재배포 완료 + 복구 확인**: 헤더 우회 성공(IP 차단 아니었음 — 봇 핑거프린트 필터). getEtfNotices(ace) 공지 5건, getDistribution(ace) 19종목·일정(공시 7/13·기준 7/15·지급 7/16) 정상. 공지 페이지에서 ACE 카드·달력·일정 표 복귀 실측.

## 2026-07-15 / — 분배금 공지 카드: 월중/월말 빈 슬롯 유지
- **공지 카드 월중/월말 슬롯 고정**: 해당 회차 공지가 없어도 자리를 비워둠(회색 라벨 + 날짜 `-`) — 카드 높이·순서가 달마다 안 흔들림. 특별/기타는 있을 때만. `dist_notice.html`·`portfolio.html` 둘 다 반영, jjk-dist/index.html 복사·푸시.
- **"6월 월중 없음" 원인 규명**: 📢 공지 패널의 6월 월중은 정상 표시됨(전 운용사, 브라우저 실측). 비는 건 🏢 **운용사별 일정 표** — getDistribution이 운용사 사이트의 "현재 공지"만 스크랩해 지난 회차(6월 월중) 일정 데이터 자체가 없음. 분배캐시도 source당 1행 덮어쓰기라 이력 없음. 근본 해결은 Code.gs에 회차별 이력 저장 필요(+수동 재배포) — 미착수.
- **발견(미수정)**: ACE 공지 API(papi.aceetf.co.kr)가 HTML 반환 → `getEtfNotices(ace)` 파싱 실패로 ACE 카드 "불러오기 실패". Code.gs 수정+재배포 필요.

## 2026-07-14 (3) / — 연금플랜 좌우 재남/은경 완전 분리 + ISA 자동추정 미표시 수정
- **레이아웃 전면 개편**: 좌 재남 / 우 은경 2컬럼(grid auto-fit 360px, 모바일은 세로 스택). 사람마다 한 카드에 ISA·일반 표 + 연금계좌 표(+ **인별 1,500만 한도 게이지** — 한도는 1인당이 세법상 맞음) + 기타 연금 표. 연도별 표도 재남(월)/은경(월)/합계/인별 연금계좌 연간으로 분리. 요약 4번째 카드도 인별 표시.
- **ISA 자동추정 0 원인 추정 수정**: 기존엔 계좌 `type` 필드가 정확히 ISA/연금저축/IRP/일반일 때만 집계 → 실계좌 type이 비었거나 다른 값이면 누락. `ppAccClass()` 추가: type이 표준값 아니면 **계좌명으로 판별**(ISA/IRP/연금 포함 여부, 대소문자 무시). 자동 행 생성도 동일 로직.
- **연간 합계 컬럼 추가**: ISA·일반 표가 연간합계 → 월평균(n개월) → 직접입력 → 적용값 순. 월평균 근거가 보이게.
- 데이터 cfg **v3**: pension/others 행에 `owner`(재남/은경) 필드. v1·v2 → v3 마이그레이션(이름에 은경 포함 시 은경, 아니면 재남). 사학 입력도 콤마 처리.
- 검증: mock(type 빈 값·'중개형' 포함)으로 이름 폴백, 인별 분리 합계(재남 1,729,000/은경 1,398,000), 2037 우체국 전환, 인별 한도 게이지 실측. 콘솔 무에러.

## 2026-07-14 (2) / — 연금플랜 피드백 반영 (재남/은경 분리 + 우체국 통합 + 콤마)
- **ISA·일반 → 재남/은경 4행 분리** (재남 ISA/재남 일반/은경 ISA/은경 일반). 자동추정 = 올해 분배금의 계좌유형×소유자(계좌명에 재남/은경 포함)별 월합산 평균, 행마다 (n개월 평균) 표기. 오버라이드 키도 `재남_ISA` 형식으로 변경.
- **우체국 연금 별도 섹션 삭제 → 연금계좌 표에 행으로 통합**. 기본 2행(재남 연 6,348,000 / 은경 연 4,896,000, 세공, **개시 2037년** — 실제 개시일 2037-10-25, 둘 다 동일). "1,500만 합산" 체크 제거 — 세공/안세공 구분만으로 한도 합산. 연금계좌 표에 **월 수령액 컬럼 추가**(월↔연 자동 환산).
- **금액 입력칸 전부 콤마 표시** — type=number → type=text(inputmode=numeric), `ppFmtIn`로 콤마 렌더, `ppNum`이 콤마 제거 파싱.
- 연도별 표에서 우체국 컬럼 제거(연금계좌에 포함). cfg **v2 마이그레이션**(`ppMigrateCfg`): 구 postal→pension 행 변환, 구 base 오버라이드는 폐기(자동 복귀).
- 검증: mock으로 4행 분리값(은경 ISA $100×1400=140,000), 2037년부터 연금계좌 월 1,937,000 전환, 콤마 입력("550,000") 파싱, v1→v2 마이그레이션 실측. 콘솔 무에러.

## 2026-07-14 / — 연금플랜 탭 신설 (월 수령액 시뮬레이션)
- **새 탭 🪙 연금플랜** (`tab-pension`, 대시보드 다음). 프론트만 수정 — Code.gs 변경 없음(재배포 불필요).
- 구성: ① **ISA·일반계좌** — 올해 분배금 데이터에서 계좌유형(`a.type`)별 월평균 자동 추정(채워진 달 기준), 직접입력 시 오버라이드(비우면 자동 복귀). ② **우체국 연금** — 직접입력(월↔연 자동 환산), 기본값 재남 52.9만/은경 40.8만(비율 추정, 채팅 근거 주석 포함), "1,500만 합산" 체크(기본 ON). ③ **연금계좌(연금저축·IRP)** — 연금저축/IRP 계좌마다 자동 행 생성(연금저축=분배금 연환산 자동, IRP=직접입력 0), 세공/안세공 분류, 섹션 상단에 **연 1,500만 종합과세 한도 게이지**(세공 행+우체국 포함분 합산, 초과 시 빨간 경고). 삭제한 자동 행은 `pensionRemoved`로 재생성 방지. ④ **기타 연금** — ➕추가로 제목/월액/개시년도 (국민연금·사학연금용), 사학연금 납입누계(참고) 필드. ⑤ **연도별 표** — 퇴직월(기본 2029-03, month input) 행 하이라이트, 각 행 개시년도 반영한 연도×소스별 월 수령액 + 한도체크 컬럼. 요약 stat 카드 4장(지금/퇴직시점/전체개시/한도합산).
- **저장은 localStorage(`jjk_pension_plan`)** — ⚠️ PC·폰 간 동기화 안 됨. 기기 간 공유가 필요해지면 Code.gs에 config 저장 액션 추가+재배포 필요(다음 할 일 후보).
- 검증: node 문법 체크 OK, 로컬 서버+mock api로 자동추정 산식(ISA 640k=KRW500k+$100×1400, 연금계좌 연환산), 한도 초과 경고(17,244,000 시 ⚠️), 개시년도별 연도표(2026/2033/2040), 기타 추가 모두 실측 확인. 콘솔 무에러.

## 2026-07-09 (2) / — 저장소 다시 public + 백엔드 토큰 인증 도입
- **private화가 GitHub Pages(jjk/portfolio.html URL)를 꺼뜨림** → 사용자가 URL로 못 열게 됨. 무료 Pages는 public 저장소만 가능. **jjk를 다시 public으로 되돌리고 Pages 재생성**(API POST, main/root). URL 복구 확인(200).
- 대신 **데이터는 백엔드 토큰으로 보호**하도록 변경:
  - `Code.gs`: `_authOk(action, token)` 추가. Script 속성 `APP_TOKEN`과 일치해야 통과. `PUBLIC_ACTIONS`(getDistribution·getEtfNotices)는 토큰 없이 허용(공개 분배 페이지 프록시용). **속성 미설정 시 전부 허용(하위호환)** → 재배포 전 앱 안 끊김. doGet/doPost 양쪽에 가드.
  - `portfolio.html`: 하드코딩 비번 제거. 게이트가 입력값을 `APP_TOKEN`으로 삼아 `getAccounts` 호출로 검증. `api`/`apiPost`/getAlerts/markAlertRead 모두 `token` 파라미터 전송. 세션 저장키 `jjk_token`. 프리뷰로 통과·부팅·계좌7개 로드 검증.
- ⚠️ **다음 할 일(사용자 수동)**: Apps Script 편집기 → 프로젝트 설정 → 스크립트 속성 `APP_TOKEN=1231` 추가 → Code.gs 붙여넣기 → **기존 배포 편집(새 버전)으로 재배포**(URL 유지). 이걸 해야 실제 강제됨. 그 전까지는 아무 비번이나 통과(하위호환).

## 2026-07-09 / — 보안(저장소 private화) + 진입 비밀번호 + ACE 표시 버그
- **`jjk` 저장소를 private으로 전환** (GitHub API). public일 때 portfolio.html·Code.gs 소스가 통째로 노출됐고, 그 안의 개인 GAS API URL로 외부에서 `getAccounts`/`getHoldings` 직접 호출해 실명 계좌·보유내역이 그대로 뽑히는 것 실증됨. 공개 분배금 페이지는 `jjk-dist`(별도 저장소)에서 서빙되므로 영향 없음. 확인: jjk API 404, github.io/jjk-dist 200.
- **portfolio.html 진입 비밀번호 잠금 추가**: body 최상단 #gate 오버레이 + 하단 스크립트 `APP_PASSWORD`(현재 '1231') / `submitGate` / sessionStorage `jjk_auth`. init()은 인증 후에만 호출. 프리뷰로 오답 차단·정답 통과·콘솔 무에러 검증 완료.
- **ACE 분배금 안 뜨는 버그 수정**: renderDistGrid의 "묵은 자료 숨김" 필터가 기준일 月을 당월/익월로만 비교 → 7월에 ACE 최신(6월, cycle 태그 없음) 항목이 전부 걸러짐. 전월도 허용하도록 수정.
- ⚠️ 한계: 비밀번호는 클라이언트 잠금일 뿐, 개인 API는 여전히 열려 있어 API 직접 호출은 차단 못 함. 다음 할 일(원하면): Code.gs doGet/doPost에 토큰 검사 추가 후 재배포.

## 2026-07-08 / — 시세 13초 지연 근본수정 (getSheetData 캐시)
- 사용자 캡처의 "…"는 직전 커밋에서 추가한 로딩 표시. 멈춘 게 아니라 getSheetData 응답(최대 13초) 대기 중 상태였음. 라이브 실측: 은경 미래에셋 연금저축 12종목 전부(494300=10,270/+0.08%) 결국 채워짐.
- **Code.gs `getSheetData`에 CacheService 3분 캐시 추가** → 첫 호출만 느리고 이후 로드/탭전환/일괄적용은 즉시. ⚠️ **Apps Script 수동 재배포 필요.**
- portfolio.html: 로딩 '…'를 빈 셀에만 적용(기존 값 유지) → 새로고침 시 깜빡임 제거.
- 남은 개선: 첫 호출도 빠르게 하려면 time-trigger(snapshotPrices 등)로 캐시 warm-up.

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
