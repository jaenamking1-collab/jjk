#!/usr/bin/env bash
# 세션 시작 때 jjk-guard 모드가 이 PC에 깔려 있고 저장소 판과 같은지 보고, 아니면 설치·갱신한다.
# enabledPlugins 만으로는 설치되지 않았다(2026-10-14 집 PC에 한 번도 안 깔려 있었음).
want=$(grep -o '"version": *"[^"]*"' mods/jjk-guard/.claude-plugin/plugin.json | grep -o '[0-9][0-9.]*')
reg="${USERPROFILE:-$HOME}/.claude/plugins/installed_plugins.json"
have=$(grep -A4 '"jjk-guard@jjk"' "$reg" 2>/dev/null | grep -o '"version": *"[^"]*"' | head -1 | grep -o '[0-9][0-9.]*')
[ -n "$want" ] && [ "$want" = "$have" ] && exit 0
# PATH 의 claude 는 옛판일 수 있다(2.1.201 은 이 모드 형식을 모른다) — 앱에 든 최신 claude.exe 를 먼저 쓴다
c=$(ls -d "${APPDATA:-/nonexistent}"/Claude/claude-code/*/*/claude.exe 2>/dev/null | sort -V | tail -1)
[ -n "$c" ] || c=$(command -v claude) || exit 0
cp .claude/settings.json "${TMPDIR:-/tmp}/jjk-settings.bak" 2>/dev/null
"$c" plugin marketplace update jjk >/dev/null 2>&1
if [ -z "$have" ]; then "$c" plugin install jjk-guard@jjk --scope project >/dev/null 2>&1
else "$c" plugin update jjk-guard@jjk --scope project >/dev/null 2>&1; fi
# 설치가 settings.json 키 순서만 바꿔 놓는다 — 되돌려 쓸데없는 커밋을 막는다
cp "${TMPDIR:-/tmp}/jjk-settings.bak" .claude/settings.json 2>/dev/null
now=$(grep -A4 '"jjk-guard@jjk"' "$reg" 2>/dev/null | grep -o '"version": *"[^"]*"' | head -1 | grep -o '[0-9][0-9.]*')
if [ "$now" = "$want" ]; then m="jjk-guard 모드 $want 를 이 PC에 설치했습니다 — 상태줄은 다음 세션부터 보입니다."
else m="jjk-guard 모드 자동 설치 실패(원하는 판 $want, 지금 ${now:-없음}) — 세션에서 확인해 주세요."; fi
printf '{"systemMessage":"🛡 %s"}' "$m"
exit 0
