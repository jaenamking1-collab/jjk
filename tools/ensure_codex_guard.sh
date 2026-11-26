#!/usr/bin/env bash
# 세션 시작 때: 코덱스가 깔린 PC면 코덱스 사용량 감시(codex_guard.pyw)를 시작프로그램에 등록하고 켠다.
# 2026-11-26 사용자: "집 PC에도 등록해줘" — PC마다 손으로 등록하지 않게 jjk 훅으로 퍼뜨린다.
h="${USERPROFILE:-$HOME}"
[ -d "$h/.codex" ] || exit 0
s="$(cygpath -w "$PWD/tools/codex_guard.pyw")"
lnk="$(cygpath -w "${APPDATA:-$h/AppData/Roaming}")\\Microsoft\\Windows\\Start Menu\\Programs\\Startup\\코덱스 사용량 감시.lnk"
new=""
if [ ! -f "$(cygpath -u "$lnk")" ]; then
  powershell.exe -NoProfile -Command "\$p=(Get-Command pythonw -EA Stop).Source; \$l=(New-Object -ComObject WScript.Shell).CreateShortcut('$lnk'); \$l.TargetPath=\$p; \$l.Arguments='\"$s\"'; \$l.WorkingDirectory='$(dirname "$s")'; \$l.Save()" >/dev/null 2>&1 && new=1
fi
# 켜져 있지 않으면 켠다. explorer 로 띄워야 Claude 앱 밖에서 돈다(앱을 닫아도 안 꺼짐). 중복은 프로그램이 스스로 막는다.
powershell.exe -NoProfile -Command "if(-not (Get-CimInstance Win32_Process -Filter \"Name='pythonw.exe'\" | ? CommandLine -like '*codex_guard*')){ exit 1 }" >/dev/null 2>&1 \
  || { [ -f "$(cygpath -u "$lnk")" ] && explorer.exe "$lnk"; }
[ -n "$new" ] && printf '{"systemMessage":"이 PC에 코덱스 사용량 감시를 등록했습니다(90%%에서 끊음)."}'
exit 0
