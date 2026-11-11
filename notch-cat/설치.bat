@echo off
chcp 65001 >nul
rem 노치 고양이 설치·업데이트: 최신 NotchCat.exe 를 받아 %LOCALAPPDATA%\NotchCat 에 두고 실행한다.
rem 다시 누르면 최신판으로 바꿔 끼운다. 자동 실행은 프로그램이 스스로 등록한다.
set "DIR=%LOCALAPPDATA%\NotchCat"
set "URL=https://github.com/jaenamking1-collab/jjk/releases/download/notch/NotchCat.exe"
if not exist "%DIR%" mkdir "%DIR%"
echo 노치 고양이 최신판을 받는 중...
curl -sSfL -o "%DIR%\NotchCat.new" "%URL%"
if errorlevel 1 (
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
