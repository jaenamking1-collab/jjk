#!/data/data/com.termux/files/usr/bin/sh
# 갤럭시탭(Termux)에서 상시 실행: 매일 09~18시 한 시간마다 TIGER·PLUS 공지 중계(tiger_relay.py)를 돌리고,
# 돌 때마다 tab_alive.txt 에 시각을 적어 올린다(탭이 죽었는지 밖에서 알 수 있게).
# 시작: cd ~/jjk && nohup sh tools/tab_loop.sh >/dev/null 2>&1 &
cd "$HOME/jjk" || exit 1
termux-wake-lock 2>/dev/null
while true; do
  h=$(date +%H | sed 's/^0//')
  if [ "$h" -ge 9 ] && [ "$h" -le 18 ]; then
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
done
