@echo off
cd /d "%~dp0"
where node >nul 2>&1
if errorlevel 1 (
  echo Install Node.js LTS from https://nodejs.org
  pause
  exit /b 1
)
node -e "if (!AbortSignal.any) process.exit(1)"
if errorlevel 1 (
  echo Node.js 20.3 or newer is required. https://nodejs.org
  pause
  exit /b 1
)
if /i "%~1"=="--check" (
  node --check app.js
  exit /b
)
powershell.exe -NoProfile -Command "Start-Process -FilePath (Get-Command node.exe).Source -ArgumentList 'app.js' -WorkingDirectory (Get-Location).Path -WindowStyle Hidden"
if errorlevel 1 pause
