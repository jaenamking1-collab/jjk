#!/data/data/com.termux/files/usr/bin/sh
# 갤럭시탭(Termux)에서 상시 실행: 매일 08~22시 한 시간마다 TIGER·PLUS·RISE 공지 중계(tiger_relay.py)를 돌리고,
# 돌 때마다 tab_alive.txt 에 시각을 적어 올린다(탭이 죽었는지 밖에서 알 수 있게 — .github/workflows/tab-alive.yml).
# 시작: cd ~/jjk && nohup sh tools/tab_loop.sh >/dev/null 2>&1 &
# 한 시간 쉬고 나면 이 파일을 **다시 읽어** 돈다(exec) — 저장소에서 고친 내용이 탭에 손대지 않고 다음 시간부터 먹는다.
# (2026-11-24 전 판은 한 번 읽은 채 계속 돌아서, 시간대를 바꿔도 탭을 다시 켜야 했다. tiger_relay.py 가 옛 판을 한 번 갈아 끼운다.)
export TAB_LOOP_SELFEXEC=1
cd "$HOME/jjk" || exit 1
termux-wake-lock 2>/dev/null
h=$(date +%H | sed 's/^0//')
if [ "$h" -ge 8 ] && [ "$h" -le 22 ]; then
  git rebase --abort 2>/dev/null
  git fetch -q origin && git reset -q --hard origin/main
  python tools/tiger_relay.py
  date '+%Y-%m-%d %H:%M 탭 중계 살아있음' > tab_alive.txt
  git add tab_alive.txt
  git commit -qm "탭 중계 생존 표시"
  git pull -q --rebase --autostash
  git push -q
fi
sleep 3600
git fetch -q origin && git reset -q --hard origin/main
exec sh "$HOME/jjk/tools/tab_loop.sh"
