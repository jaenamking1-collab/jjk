# mods/jjk-guard — Claude Code mod

> CLAUDE.md 에서 옮김(2026-10-14).

- `mods/jjk-guard/` — Claude Code mod(2026-10-12). 실행 직전에 막는다: `clasp deploy` 에 라이브 `-i` 없음, 저장소 안 `clasp pull`, 명령·파일의 토큰 모양 문자열. `deploy.yml` 실행 뒤 `maint resetAllTriggers` 전까지 상태줄 경고. 상태줄에 `컨텍스트 N% · 5시간 N% · 7일 N%`(80%·90% 첫 도달 때 알림). 2026-10-14: Token Weather 처럼 `이번 답 +N%`(이번 질문이 쓴 5시간 한도)·`캐시 N분`(마지막 응답 뒤 1시간, 1분마다 줄어듦)을 덧붙였다. 컨텍스트 앞에 날씨(☀️<30 ⛅<50 ☁️<70 🌧️<85 ⛈️), 50% 첫 도달 때 `/compact`·새 대화 권유 알림 — 디렉터리의 Token Weather Usage 가 앱에 안 떠서 직접 넣음. 시험은 PATH 의 옛 claude(2.1.201)가 아니라 앱 쪽 최신 claude.exe 로 돌린다. 시험 `claude plugin test mods/jjk-guard`. ⚠️ `enabledPlugins` 만으로는 설치되지 않는다(2026-10-14 집 PC에 한 번도 안 깔려 있었다). 그래서 `SessionStart` 훅이 `tools/ensure_jjk_guard.sh` 로 설치 여부·판을 보고 없거나 옛 판이면 설치·갱신한다. 공식 모드 hookify·session-report(사용자 범위)도 없으면 같이 깐다(2026-10-14 사용자 승인)(앱 쪽 최신 claude.exe 사용, settings.json 키 순서 변경은 되돌림). 설치된 세션 다음 세션부터 상태줄이 보인다.
- 0.5.0(2026-10-14): **할 일 띠** — 세션을 열면 입력창 위에 WORKLOG 맨 위 항목의 제목 + '다음 할 일'·'확인 못 한 것' 줄(최대 2개). 첫 질문을 보내면 사라진다. 모듈은 JSX 때문에 `register.tsx`.
- '됐습니다' 검사는 이 모드가 아니라 전역 Stop 훅 `~/claude-memory/hooks/stop_claim_check.py`(아래 참고): 이번 답이 '됐습니다·고쳤습니다…'로 끝났는데 도구를 한 번도 안 썼으면 한 번 되돌려 확인시킨다(hookify 는 대화 전체만 봐서 못 씀).
- 0.6.0: 할 일 띠에 `🆕 새 모드 N개`(tools/mods/new.txt — `new-mods.yml` 이 매일 공식 목록과 비교해 쌓음). 사용자가 "새 모드 봐줘" 하면 new.txt 를 읽고 쓸 만한 것 2~3개만 추천 → 설치는 승인 뒤 `tools/ensure_jjk_guard.sh` 의 EXTRA 에 추가 → new.txt 비우기.
- 2026-10-14: **모든 프로젝트에 적용**(사용자 요청). jjk-guard 는 사용자 범위로 설치(`ensure_jjk_guard.sh` 가 예전 프로젝트 범위 설치를 걷고 다시 깜). '됐습니다' 검사는 `~/claude-memory/hooks/stop_claim_check.py` 로 옮겨 `install_global.py` 가 전역 Stop 훅으로 건다(jjk 의 settings.json 에서는 뺐다 — 두 번 돌지 않게). 다른 프로젝트에선 WORKLOG·tools/mods 가 없으면 띠가 안 뜬다.
