# mods/jjk-guard — Claude Code mod

> CLAUDE.md 에서 옮김(2026-10-14).

- `mods/jjk-guard/` — Claude Code mod(2026-10-12). 실행 직전에 막는다: `clasp deploy` 에 라이브 `-i` 없음, 저장소 안 `clasp pull`, 명령·파일의 토큰 모양 문자열. `deploy.yml` 실행 뒤 `maint resetAllTriggers` 전까지 상태줄 경고. 상태줄에 `컨텍스트 N% · 5시간 N% · 7일 N%`(80%·90% 첫 도달 때 알림). 2026-10-14: Token Weather 처럼 `이번 답 +N%`(이번 질문이 쓴 5시간 한도)·`캐시 N분`(마지막 응답 뒤 1시간, 1분마다 줄어듦)을 덧붙였다. 컨텍스트 앞에 날씨(☀️<30 ⛅<50 ☁️<70 🌧️<85 ⛈️), 50% 첫 도달 때 `/compact`·새 대화 권유 알림 — 디렉터리의 Token Weather Usage 가 앱에 안 떠서 직접 넣음. 시험은 PATH 의 옛 claude(2.1.201)가 아니라 앱 쪽 최신 claude.exe 로 돌린다. 시험 `claude plugin test mods/jjk-guard`. ⚠️ `enabledPlugins` 만으로는 설치되지 않는다(2026-10-14 집 PC에 한 번도 안 깔려 있었다). 그래서 `SessionStart` 훅이 `tools/ensure_jjk_guard.sh` 로 설치 여부·판을 보고 없거나 옛 판이면 설치·갱신한다. 공식 모드 hookify·session-report(사용자 범위)도 없으면 같이 깐다(2026-10-14 사용자 승인)(앱 쪽 최신 claude.exe 사용, settings.json 키 순서 변경은 되돌림). 설치된 세션 다음 세션부터 상태줄이 보인다.
