@echo off
chcp 65001 >nul
cd /d "%~dp0"
where node >nul 2>&1
if errorlevel 1 (
  echo Node.js가 필요합니다. https://nodejs.org 에서 LTS를 설치해 주세요.
  pause
  exit /b 1
)
node -e "if (!AbortSignal.any) process.exit(1)"
if errorlevel 1 (
  echo Node.js 20.3 이상이 필요합니다. https://nodejs.org 에서 LTS를 설치해 주세요.
  pause
  exit /b 1
)
powershell.exe -NoProfile -Command "Start-Process -FilePath (Get-Command node.exe).Source -ArgumentList 'app.js' -WorkingDirectory (Get-Location).Path -WindowStyle Hidden"
if errorlevel 1 pause
