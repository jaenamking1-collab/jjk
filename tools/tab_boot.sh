#!/data/data/com.termux/files/usr/bin/sh
# 갤럭시탭이 켜질 때 Termux:Boot 앱이 이 파일(~/.termux/boot/jjk.sh 로 복사된 것)을 돌려 중계 루프를 띄운다.
# 복사는 tab_loop.sh 가 매시간 알아서 한다 — 탭에 사람이 할 일은 Termux:Boot 앱 설치 + 한 번 열기뿐(2026-11-24).
termux-wake-lock 2>/dev/null
sleep 30   # 와이파이 잡힐 시간
cd "$HOME/jjk" && nohup sh tools/tab_loop.sh >/dev/null 2>&1 &
