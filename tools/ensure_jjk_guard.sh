#!/usr/bin/env bash
# 세션 시작 때 jjk-guard 모드가 이 PC에 깔려 있고 저장소 판과 같은지 보고, 아니면 설치·갱신한다.
# enabledPlugins 만으로는 설치되지 않았다(2026-10-14 집 PC에 한 번도 안 깔려 있었음).
reg="${USERPROFILE:-$HOME}/.claude/plugins/installed_plugins.json"
# 등록 파일에서 jjk-guard 항목의 version (항목 안 필드 순서가 바뀌어도 되게 ']' 까지 본다)
ver_of() { awk '/"jjk-guard@jjk"/{f=1} f&&/"version"/{match($0,/[0-9][0-9.]*/);print substr($0,RSTART,RLENGTH);exit} f&&/\]/{exit}' "$reg" 2>/dev/null; }
want=$(grep -o '"version": *"[^"]*"' mods/jjk-guard/.claude-plugin/plugin.json | grep -o '[0-9][0-9.]*')
have=$(ver_of)
# 두 PC 공통으로 쓰는 공식 모드(사용자 범위 — 모든 프로젝트). 2026-10-14 사용자 승인: hookify·session-report
EXTRA="hookify session-report"
missing=""; for x in $EXTRA; do grep -q "\"$x@claude-plugins-official\"" "$reg" 2>/dev/null || missing="$missing $x"; done
# 모든 프로젝트에서 쓰이게 사용자 범위로 깐다(2026-10-14). 예전 프로젝트 범위 설치면 다시 깐다
scope=$(awk '/"jjk-guard@jjk"/{f=1} f&&/"scope"/{print;exit} f&&/\]/{exit}' "$reg" 2>/dev/null | grep -o 'user\|project')
[ "$scope" = "user" ] || have=""
[ -n "$want" ] && [ "$want" = "$have" ] && [ -z "$missing" ] && exit 0
# PATH 의 claude 는 옛판일 수 있다(2.1.201 은 이 모드 형식을 모른다) — 앱에 든 최신 claude.exe 를 먼저 쓴다
c=$(ls -d "${APPDATA:-/nonexistent}"/Claude/claude-code/*/*/claude.exe 2>/dev/null | sort -V | tail -1)
[ -n "$c" ] || c=$(command -v claude) || exit 0
cp .claude/settings.json "${TMPDIR:-/tmp}/jjk-settings.bak" 2>/dev/null
added=""; for x in $missing; do "$c" plugin install "$x@claude-plugins-official" --scope user >/dev/null 2>&1 && added="$added $x"; done
if [ -n "$want" ] && [ "$want" = "$have" ]; then
  [ -n "$added" ] && printf '{"systemMessage":"🛡 이 PC에 모드 설치:%s — 다음 세션부터 쓰입니다."}' "$added"
  exit 0
fi
"$c" plugin marketplace update jjk >/dev/null 2>&1
if [ "$scope" = "project" ]; then "$c" plugin uninstall jjk-guard@jjk --scope project >/dev/null 2>&1; fi
if [ -z "$have" ]; then "$c" plugin install jjk-guard@jjk --scope user >/dev/null 2>&1
else "$c" plugin update jjk-guard@jjk --scope user >/dev/null 2>&1; fi
# 설치가 settings.json 키 순서만 바꿔 놓는다 — 되돌려 쓸데없는 커밋을 막는다
cp "${TMPDIR:-/tmp}/jjk-settings.bak" .claude/settings.json 2>/dev/null
now=$(ver_of)
[ -n "$added" ] && added=" (+$added)"
if [ "$now" = "$want" ]; then m="jjk-guard 모드 $want$added 를 이 PC에 설치했습니다 — 상태줄은 다음 세션부터 보입니다."
else m="jjk-guard 모드 자동 설치 실패(원하는 판 $want, 지금 ${now:-없음}) — 세션에서 확인해 주세요."; fi
printf '{"systemMessage":"🛡 %s"}' "$m"
exit 0
