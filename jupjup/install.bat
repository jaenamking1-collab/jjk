@echo off
cd /d "%~dp0"
echo chrome://extensions| clip
start "" explorer "%~dp0"
start "" "C:\Program Files\Google\Chrome\Application\chrome.exe" --profile-directory="Profile 14"
