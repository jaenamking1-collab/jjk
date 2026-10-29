@echo off
chcp 65001 >nul
rem TIGER 공지 중계(tiger_relay.py)를 하루 2회(10:40, 16:40) 돌리도록 작업 스케줄러에 등록한다.
rem 서버가 TIGER 를 직접 읽고 있으면 스크립트가 바로 끝나므로 계속 켜 둬도 된다. 다시 실행하면 덮어쓴다.
set "PYW="
for /f "delims=" %%p in ('where pythonw 2^>nul') do if not defined PYW set "PYW=%%p"
if not defined PYW (echo pythonw 를 찾지 못했습니다 & pause & exit /b 1)
set "SCRIPT=%~dp0tiger_relay.py"
schtasks /Create /F /SC DAILY /ST 10:40 /TN "jjk TIGER relay AM" /TR "\"%PYW%\" \"%SCRIPT%\"" >nul && echo 오전 10:40 등록 완료
schtasks /Create /F /SC DAILY /ST 16:40 /TN "jjk TIGER relay PM" /TR "\"%PYW%\" \"%SCRIPT%\"" >nul && echo 오후 4:40 등록 완료
schtasks /Run /TN "jjk TIGER relay AM" >nul && echo 지금 한 번 실행했습니다
pause
