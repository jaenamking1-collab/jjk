@echo off
rem Registers tools\tiger_relay.py in Task Scheduler: every day 09:05-18:05, hourly.
rem The script only opens the Mirae (TIGER) site when other issuers already posted a newer notice.
rem Keep this file ASCII + CRLF: cmd mis-parses UTF-8 Korean and LF line endings (2026-10-29).
set "PYW="
for /f "delims=" %%p in ('where pythonw 2^>nul') do if not defined PYW set "PYW=%%p"
if not defined PYW (echo pythonw not found & pause & exit /b 1)
set "SCRIPT=%~dp0tiger_relay.py"
schtasks /Create /F /SC DAILY /ST 09:05 /RI 60 /DU 09:00 /TN "jjk TIGER relay" /TR "\"%PYW%\" \"%SCRIPT%\""
if errorlevel 1 (echo FAILED - please tell Claude the message above & pause & exit /b 1)
echo OK - registered: every day 09:05-18:05, hourly
schtasks /Run /TN "jjk TIGER relay" >nul && echo OK - ran once now
pause
