#!/data/data/com.termux/files/usr/bin/sh
# 갤럭시탭(Termux)에서 상시 실행. 1분마다 깨어 이 파일을 **다시 읽어** 돈다(exec) — 저장소에서 고친 내용이 탭에 손대지 않고 1분 안에 먹는다.
#  · 공지 중계(tiger_relay.py: TIGER·PLUS·RISE·IRP 기준가): 08~22시 한 시간마다, 평일 13~18시(공시가 몰리는 때)는 10분마다.
#    올린 사본은 relay-push.yml 이 서버에 바로 다시 읽힌다.
#  · 생존 표시 tab_alive.txt: 08~22시 한 시간마다(.github/workflows/tab-alive.yml 이 본다).
#  · 대신 열기(tab_fetch.py): 하루 종일 1분마다 'tab-fetch' 브랜치 요청을 본다(tools/tabget.py 와 짝).
# 시작: cd ~/jjk && nohup sh tools/tab_loop.sh >/dev/null 2>&1 &   (재부팅 뒤엔 Termux:Boot 가 tab_boot.sh 로 띄운다)
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

now=$(date +%s); h=$(date +%H | sed 's/^0//'); dow=$(date +%u)
gap=3600
if [ "$dow" -le 5 ] && [ "$h" -ge 13 ] && [ "$h" -le 18 ]; then gap=600; fi
last=$(cat "$HOME/.tab_last_relay" 2>/dev/null || echo 0)
if [ "$h" -ge 8 ] && [ "$h" -le 22 ] && [ $((now - last)) -ge $((gap - 30)) ]; then
  echo "$now" > "$HOME/.tab_last_relay"
  git rebase --abort 2>/dev/null
  git fetch -q origin && git reset -q --hard origin/main
  python tools/tiger_relay.py
  lastA=$(cat "$HOME/.tab_last_alive" 2>/dev/null || echo 0)
  if [ $((now - lastA)) -ge 3570 ]; then
    echo "$now" > "$HOME/.tab_last_alive"
    # 맨 앞 16글자(시각)는 tiger_relay.py 가 읽는다 — 뒤에 붙이는 것만 바꿀 것
    boot=$( (cmd package list packages com.termux.boot 2>/dev/null || pm list packages com.termux.boot 2>/dev/null) | grep -q com.termux.boot && echo 있음 || echo 없음·모름)
    echo "$(date '+%Y-%m-%d %H:%M') 탭 중계 살아있음 · 자동시작앱 $boot · Termux ${TERMUX_APK_RELEASE:-?}" > tab_alive.txt
    git add tab_alive.txt
    git commit -qm "탭 중계 생존 표시"
    git pull -q --rebase --autostash
    git push -q
  fi
fi
python tools/tab_fetch.py
sleep 60
# 1분마다 다시 읽는 건 작업 폴더의 파일이다 — 최신 코드는 중계 때 받아 오고, 밤처럼 중계가 쉬는 때를 위해 10분마다 한 번 더 받는다.
if [ $(( $(date +%M | sed 's/^0//') % 10 )) -eq 0 ]; then git fetch -q origin && git reset -q --hard origin/main; fi
exec sh "$HOME/jjk/tools/tab_loop.sh"
