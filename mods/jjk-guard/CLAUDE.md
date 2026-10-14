# mods/jjk-guard — Claude Code mod

> CLAUDE.md 에서 옮김(2026-10-14).

- `mods/jjk-guard/` — Claude Code mod(2026-10-12). 실행 직전에 막는다: `clasp deploy` 에 라이브 `-i` 없음, 저장소 안 `clasp pull`, 명령·파일의 토큰 모양 문자열. `deploy.yml` 실행 뒤 `maint resetAllTriggers` 전까지 상태줄 경고. 상태줄에 `컨텍스트 N% · 5시간 N% · 7일 N%`(80%·90% 첫 도달 때 알림). 시험 `claude plugin test mods/jjk-guard`. `.claude/settings.json` 의 `enabledPlugins` 로 자동 설치된다(안 깔렸으면 `/plugin install jjk-guard --marketplace jaenamking1-collab/jjk`).
