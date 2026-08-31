@echo off
chcp 65001 >nul
setlocal enabledelayedexpansion
title 실시간 시세 위젯 설치

rem 남에게 이 파일 하나만 주면 되게 만든 설치기.
rem 파이썬 확인/설치 → 최신 ticker.pyw 내려받기 → 바탕화면 바로가기 → 실행.
rem 설정 파일은 아래 DEST 폴더에만 생기므로, 내 보유 종목이 딸려 가지 않는다.

set "DEST=%LOCALAPPDATA%\ticker"
set "SRC=https://raw.githubusercontent.com/jaenamking1-collab/jjk/main/tools/ticker.pyw"

echo.
echo   실시간 시세 위젯을 설치합니다.
echo   설치 위치: %DEST%
echo.

rem ── 파이썬 찾기 ─────────────────────────────────────────
set "PYW="
for /f "delims=" %%p in ('where pythonw 2^>nul') do if not defined PYW set "PYW=%%p"
if not defined PYW (
  for /f "delims=" %%p in ('dir /b /s "%LOCALAPPDATA%\Programs\Python\pythonw.exe" 2^>nul') do if not defined PYW set "PYW=%%p"
)

if not defined PYW (
  echo   파이썬이 없어서 먼저 설치합니다. 1~3분 걸립니다...
  winget install -e --id Python.Python.3.12 --silent --accept-package-agreements --accept-source-agreements
  rem 설치 위치는 계정용/전체용에 따라 갈린다. 한 곳만 보면 방금 깐 것을 못 찾는다.
  for /f "delims=" %%p in ('dir /b /s "%LOCALAPPDATA%\Programs\Python\pythonw.exe" 2^>nul') do if not defined PYW set "PYW=%%p"
  for /d %%d in ("%ProgramFiles%\Python3*") do if not defined PYW if exist "%%d\pythonw.exe" set "PYW=%%d\pythonw.exe"
  for /d %%d in ("C:\Python3*") do if not defined PYW if exist "%%d\pythonw.exe" set "PYW=%%d\pythonw.exe"
)

if not defined PYW (
  echo.
  echo   자동 설치가 안 됐습니다. python.org 페이지를 엽니다.
  echo   설치 첫 화면의 "Add python.exe to PATH"를 꼭 체크하시고,
  echo   설치가 끝나면 이 파일을 다시 실행해 주세요.
  start "" https://www.python.org/downloads/
  pause
  exit /b 1
)

rem ── 프로그램 내려받기 ───────────────────────────────────
if not exist "%DEST%" mkdir "%DEST%"
curl -L -f -s -o "%DEST%\ticker.pyw" "%SRC%"
if errorlevel 1 (
  echo   내려받기에 실패했습니다. 인터넷 연결을 확인하고 다시 실행해 주세요.
  pause
  exit /b 1
)

rem ── 바탕화면 바로가기 (원드라이브로 옮겨진 바탕화면도 찾는다) ──
powershell -NoProfile -Command "$d=[Environment]::GetFolderPath('Desktop'); $s=(New-Object -COM WScript.Shell).CreateShortcut($d+'\실시간 시세.lnk'); $s.TargetPath='%PYW%'; $s.Arguments='\"%DEST%\ticker.pyw\"'; $s.WorkingDirectory='%DEST%'; $s.Description='실시간 시세 위젯'; $s.Save()"

start "" "%PYW%" "%DEST%\ticker.pyw"

echo.
echo   설치 완료. 바탕화면에 「실시간 시세」 바로가기를 만들었습니다.
echo   창 오른쪽 위 톱니(⚙)를 눌러 보고 싶은 종목으로 바꾸세요.
echo   (국내 종목은 6자리 코드, 예: 069500 / 해외는 티커, 예: AAPL)
echo.
timeout /t 8 >nul
