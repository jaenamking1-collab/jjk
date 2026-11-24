#!/data/data/com.termux/files/usr/bin/sh
# 갤럭시탭(Termux)에서 상시 실행: 매일 08~22시 한 시간마다 TIGER·PLUS·RISE 공지 중계(tiger_relay.py)를 돌리고,
# 돌 때마다 tab_alive.txt 에 시각을 적어 올린다(탭이 죽었는지 밖에서 알 수 있게 — .github/workflows/tab-alive.yml).
# 시작: cd ~/jjk && nohup sh tools/tab_loop.sh >/dev/null 2>&1 &
# 한 시간 쉬고 나면 이 파일을 **다시 읽어** 돈다(exec) — 저장소에서 고친 내용이 탭에 손대지 않고 다음 시간부터 먹는다.
# (2026-11-24 전 판은 한 번 읽은 채 계속 돌아서, 시간대를 바꿔도 탭을 다시 켜야 했다. tiger_relay.py 가 옛 판을 한 번 갈아 끼운다.)
export TAB_LOOP_SELFEXEC=1
cd "$HOME/jjk" || exit 1
# 루프는 하나만: 부팅 스크립트와 손으로 띄운 것이 겹쳐도 둘째는 바로 끝난다(exec 로 다시 읽어도 $$ 는 그대로).
PIDF="$HOME/.tab_loop.pid"
old=$(cat "$PIDF" 2>/dev/null)
if [ -n "$old" ] && [ "$old" != "$$" ] && grep -q tab_loop "/proc/$old/cmdline" 2>/dev/null; then exit 0; fi
echo $$ > "$PIDF"
# 재부팅 뒤 자동 시작(Termux:Boot 앱이 ~/.termux/boot/ 안의 파일을 돌린다)
mkdir -p "$HOME/.termux/boot"
cmp -s tools/tab_boot.sh "$HOME/.termux/boot/jjk.sh" || { cp tools/tab_boot.sh "$HOME/.termux/boot/jjk.sh"; chmod +x "$HOME/.termux/boot/jjk.sh"; }
termux-wake-lock 2>/dev/null
h=$(date +%H | sed 's/^0//')
if [ "$h" -ge 8 ] && [ "$h" -le 22 ]; then
  git rebase --abort 2>/dev/null
  git fetch -q origin && git reset -q --hard origin/main
  python tools/tiger_relay.py
  # 맨 앞 16글자(시각)는 tiger_relay.py 가 읽는다 — 뒤에 붙이는 것만 바꿀 것
  boot=$( (cmd package list packages com.termux.boot 2>/dev/null || pm list packages com.termux.boot 2>/dev/null) | grep -q com.termux.boot && echo 있음 || echo 없음·모름)
  echo "$(date '+%Y-%m-%d %H:%M') 탭 중계 살아있음 · 자동시작앱 $boot · Termux ${TERMUX_APK_RELEASE:-?}" > tab_alive.txt
  git add tab_alive.txt
  git commit -qm "탭 중계 생존 표시"
  git pull -q --rebase --autostash
  git push -q
fi
sleep 3600
git fetch -q origin && git reset -q --hard origin/main
exec sh "$HOME/jjk/tools/tab_loop.sh"
