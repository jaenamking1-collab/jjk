@echo off
chcp 65001 >nul
rem TIGER 공지 중계(tiger_relay.py)를 매일 9시~18시 1시간마다 깨우도록 작업 스케줄러에 등록한다.
rem 깨워도 다른 운용사 공지가 TIGER 보다 앞설 때만 미래에셋을 열고, 서버가 TIGER 를 직접 읽으면 바로 끝난다.
rem 다시 실행하면 덮어쓴다. ⚠️ 이 파일은 CRLF 여야 한다(.gitattributes) — LF 면 명령창이 줄을 못 읽고 조용히 끝난다.
set "PYW="
for /f "delims=" %%p in ('where pythonw 2^>nul') do if not defined PYW set "PYW=%%p"
if not defined PYW (echo pythonw 를 찾지 못했습니다 & pause & exit /b 1)
set "SCRIPT=%~dp0tiger_relay.py"
schtasks /Create /F /SC DAILY /ST 09:05 /RI 60 /DU 09:00 /TN "jjk TIGER relay" /TR "\"%PYW%\" \"%SCRIPT%\""
if errorlevel 1 (echo 등록 실패 - 위 메시지를 알려 주세요 & pause & exit /b 1)
echo 매일 9시~18시 1시간마다 등록 완료
schtasks /Run /TN "jjk TIGER relay" >nul && echo 지금 한 번 실행했습니다
pause
