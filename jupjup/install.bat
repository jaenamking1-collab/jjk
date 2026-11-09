@echo off
chcp 65001 >nul
rem 줍줍 설치·업데이트. 어느 PC 에서든 더블클릭.
rem  - jjk 저장소가 있는 PC: 그 안의 jupjup 폴더를 git pull 로 최신으로
rem  - 없는 PC: GitHub Pages(학교망은 raw.githubusercontent 가 막힘)에서 %USERPROFILE%\jupjup 로 받고, 매일 12시에 다시 받아 최신 유지(확장이 1분 안에 스스로 다시 켜짐)
rem  "install.bat update" 는 받기만 한다(예약 작업용).
setlocal
set "JJK=%USERPROFILE%\Documents\jjk"
if exist "%JJK%\jupjup\manifest.json" (
  set "DIR=%JJK%\jupjup"
  git -C "%JJK%" pull -q
) else (
  set "DIR=%USERPROFILE%\jupjup"
  powershell -NoProfile -ExecutionPolicy Bypass -Command ^
    "$d=$env:USERPROFILE+'\jupjup'; $r='https://jaenamking1-collab.github.io/jjk/jupjup/';" ^
    "foreach($f in 'manifest.json','background.js','content.js','store.js','board.html','board.css','board.js','fonts/jua.woff2','icons/16.png','icons/48.png','icons/128.png'){" ^
    "  $p=Join-Path $d $f; New-Item -ItemType Directory -Force (Split-Path $p) | Out-Null;" ^
    "  Invoke-WebRequest -UseBasicParsing ($r+$f+'?t='+[DateTime]::Now.Ticks) -OutFile $p }"
  if errorlevel 1 ( echo 받기 실패 - 인터넷 연결을 확인해 주세요. & pause & exit /b 1 )
  rem 실행 중인 bat 를 덮어쓰면 cmd 가 꼬이므로 bat 는 받지 않고 자기 자신을 복사해 둔다(예약 작업이 쓴다)
  if /i not "%~f0"=="%USERPROFILE%\jupjup\install.bat" copy /y "%~f0" "%USERPROFILE%\jupjup\install.bat" >nul
  if not defined JUPJUP_TEST schtasks /create /f /sc daily /st 12:00 /tn "jupjup-update" /tr "\"%USERPROFILE%\jupjup\install.bat\" update" >nul
)
if /i "%~1"=="update" exit /b 0

echo %DIR%| clip
set "CH=%ProgramFiles%\Google\Chrome\Application\chrome.exe"
if not exist "%CH%" set "CH=%LOCALAPPDATA%\Google\Chrome\Application\chrome.exe"
start "" "%CH%" chrome://extensions
echo.
echo  크롬 확장 화면이 열렸습니다.
echo  1. 오른쪽 위 '개발자 모드' 켜기
echo  2. '압축해제된 확장 프로그램을 로드합니다' 누르기
echo  3. 폴더 창 위쪽 주소칸에 Ctrl+V 하고 '폴더 선택'
echo     (폴더 주소 %DIR% 는 이미 복사해 두었습니다)
echo.
echo  이미 설치돼 있으면 할 일 없습니다. 최신 파일로 바뀌었고 1분 안에 줍줍이 다시 켜집니다.
pause
