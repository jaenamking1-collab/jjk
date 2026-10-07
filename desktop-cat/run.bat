@echo off
chcp 65001 >nul
cd /d "%~dp0"
where python >nul 2>nul
if errorlevel 1 (
  echo 파이썬이 없습니다. https://www.python.org/downloads/ 에서 설치할 때 "Add python.exe to PATH" 를 체크하세요.
  pause
  exit /b 1
)
python -c "import PIL, numpy" 2>nul || python -m pip install --user pillow numpy
start "" pythonw "%~dp0desktop_cat.py"
