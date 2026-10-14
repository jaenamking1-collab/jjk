# mods/jjk-guard — Claude Code mod

> CLAUDE.md 에서 옮김(2026-10-14).

- `mods/jjk-guard/` — Claude Code mod(2026-10-12). 실행 직전에 막는다: `clasp deploy` 에 라이브 `-i` 없음, 저장소 안 `clasp pull`, 명령·파일의 토큰 모양 문자열. `deploy.yml` 실행 뒤 `maint resetAllTriggers` 전까지 상태줄 경고. 상태줄에 `컨텍스트 N% · 5시간 N% · 7일 N%`(80%·90% 첫 도달 때 알림). 2026-10-14: Token Weather 처럼 `이번 답 +N%`(이번 질문이 쓴 5시간 한도)·`캐시 N분`(마지막 응답 뒤 1시간, 1분마다 줄어듦)을 덧붙였다 — 디렉터리의 Token Weather Usage 가 앱에 안 떠서 직접 넣음. 시험은 PATH 의 옛 claude(2.1.201)가 아니라 앱 쪽 최신 claude.exe 로 돌린다. 시험 `claude plugin test mods/jjk-guard`. `.claude/settings.json` 의 `enabledPlugins` 로 자동 설치된다(안 깔렸으면 `/plugin install jjk-guard --marketplace jaenamking1-collab/jjk`).
