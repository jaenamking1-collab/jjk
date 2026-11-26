@echo off
chcp 65001 >nul
rem 노치 고양이 설치·업데이트: 최신 NotchCat.exe 를 받아 %LOCALAPPDATA%\NotchCat 에 두고 실행한다.
rem 다시 누르면 최신판으로 바꿔 끼운다. 자동 실행은 프로그램이 스스로 등록한다.
set "DIR=%LOCALAPPDATA%\NotchCat"
set "URL=https://github.com/jaenamking1-collab/jjk/releases/download/notch/NotchCat.exe"
set "MIRROR=https://jjk-app.distributionjn.workers.dev/NotchCat.exe"
if not exist "%DIR%" mkdir "%DIR%"
echo 노치 고양이 최신판을 받는 중...
del "%DIR%\NotchCat.new" >nul 2>&1
curl -sSfL --max-time 60 -o "%DIR%\NotchCat.new" "%URL%" || del "%DIR%\NotchCat.new" >nul 2>&1
rem 학교망은 윈도우 curl 의 GitHub 받기를 끊는다(2026-11-23) - 그때는 파워셸로 다시 받는다
if not exist "%DIR%\NotchCat.new" powershell -NoProfile -Command "Invoke-WebRequest -UseBasicParsing -TimeoutSec 60 -Uri '%URL%' -OutFile '%DIR%\NotchCat.new'"
rem GitHub 파일 서버가 통째로 막히면 Cloudflare 중계로 받는다(2026-11-26)
if not exist "%DIR%\NotchCat.new" curl -sSfL --max-time 120 -o "%DIR%\NotchCat.new" "%MIRROR%" || del "%DIR%\NotchCat.new" >nul 2>&1
if not exist "%DIR%\NotchCat.new" powershell -NoProfile -Command "Invoke-WebRequest -UseBasicParsing -Uri '%MIRROR%' -OutFile '%DIR%\NotchCat.new'"
if not exist "%DIR%\NotchCat.new" (
  echo 받기에 실패했습니다. 인터넷 연결을 확인한 뒤 다시 눌러 주세요.
  pause
  exit /b 1
)
taskkill /im NotchCat.exe /f >nul 2>&1
timeout /t 1 /nobreak >nul
move /y "%DIR%\NotchCat.new" "%DIR%\NotchCat.exe" >nul
start "" "%DIR%\NotchCat.exe"
echo 설치 끝 - 화면 위 가운데에 노치가 떴습니다.
timeout /t 3 >nul
