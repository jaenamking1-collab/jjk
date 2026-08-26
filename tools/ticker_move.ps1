# 위젯을 구글 드라이브(H:)에서 %LOCALAPPDATA%\ticker 로 이사시킨다.
#
# H:는 가상 드라이브라 언마운트되면 ① 실행 중인 파일이 사라져 위젯이 죽고
# ② self_update가 새 코드를 써도 남지 않아 옛 코드를 돌리면서 "최신"이라고 표시한다
# (WORKLOG 120·122). 바로가기까지 같이 고쳐야 다음 부팅에 다시 H:로 돌아가지 않는다.
#
# 실행: PowerShell 창에 이 파일 내용을 붙여넣거나
#       powershell -NoProfile -ExecutionPolicy Bypass -File ticker_move.ps1

$dest = "$env:LOCALAPPDATA\ticker"
$raw  = "https://raw.githubusercontent.com/jaenamking1-collab/jjk/main/tools/ticker.pyw"
New-Item -ItemType Directory -Force -Path $dest | Out-Null

# ── 1. 지금 도는 위젯의 위치를 알아둔다 (아직 끄지 않는다) ──────────────
$procs = @(Get-CimInstance Win32_Process -Filter "Name='pythonw.exe'" |
           Where-Object { $_.CommandLine -match 'ticker\.pyw' })
$oldDir = $null
if ($procs.Count -and $procs[0].CommandLine -match '([A-Za-z]:\\[^"]*)\\ticker\.pyw') {
  $oldDir = $Matches[1]
  "지금 도는 위치: $oldDir"
} else {
  "도는 위젯 없음 (설치만 진행)"
}

# ── 2. 최신 코드부터 받는다. 실패하면 아무것도 건드리지 않는다 ─────────
$tmp = "$dest\ticker.pyw.new"
curl.exe -L -f -s -o $tmp $raw
if (-not (Test-Path $tmp) -or (Get-Item $tmp).Length -lt 10000) {
  Remove-Item $tmp -EA 0
  throw "내려받기 실패 — 인터넷을 확인하고 다시 실행하세요. 위젯은 그대로 둡니다."
}
Move-Item $tmp "$dest\ticker.pyw" -Force
"코드 받음: {0:N0} bytes" -f (Get-Item "$dest\ticker.pyw").Length

# ── 3. 창 위치·투명도·바탕색 이어받기 ──────────────────────────────────
# 종목 목록은 claude-memory 저장소에서 오므로 안 챙겨도 된다. 이 파일에 남는 건
# 그 PC 사정인 값들뿐이다(WORKLOG 119에서 일부러 갈라놓았다).
$oldCfg = if ($oldDir) { "$oldDir\ticker_config.json" }
          else { @(Get-ChildItem "H:\*\ticker\ticker_config.json" -EA 0)[0].FullName }
if ($oldCfg -and (Test-Path $oldCfg) -and -not (Test-Path "$dest\ticker_config.json")) {
  Copy-Item $oldCfg "$dest\ticker_config.json"
  "설정 이어받음: $oldCfg"
}

# ── 4. 옛 위젯 종료 ────────────────────────────────────────────────────
$procs | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA 0 }

# ── 5. 파이썬 찾기 ─────────────────────────────────────────────────────
$pyw = (Get-Command pythonw.exe -EA 0).Source
if (-not $pyw) {
  $pyw = @(Get-ChildItem "C:\Python*\pythonw.exe",
                         "$env:LOCALAPPDATA\Programs\Python\*\pythonw.exe" -EA 0)[0].FullName
}
if (-not $pyw) { throw "pythonw.exe를 못 찾았습니다. tools\ticker_setup.bat 을 실행하세요." }

# ── 6. ticker를 가리키는 바로가기를 전부 새 위치로 ─────────────────────
# 바탕화면이 원드라이브로 리디렉션돼 있을 수 있어 GetFolderPath로 찾는다(WORKLOG 120).
$sh = New-Object -ComObject WScript.Shell
@([Environment]::GetFolderPath('Desktop'),
  "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup") |
  ForEach-Object { Get-ChildItem $_ -Filter *.lnk -EA 0 } |
  ForEach-Object {
    $l = $sh.CreateShortcut($_.FullName)
    if (($l.TargetPath + ' ' + $l.Arguments) -match 'ticker\.pyw') {
      $l.TargetPath       = $pyw
      $l.Arguments        = '"' + $dest + '\ticker.pyw"'
      $l.WorkingDirectory = $dest
      $l.Save()
      "바로가기 고침: " + $_.FullName
    }
  }

# ── 7. 새 위치에서 실행 ────────────────────────────────────────────────
Start-Process $pyw -ArgumentList "`"$dest\ticker.pyw`""
""
"완료 → $dest\ticker.pyw"
"⚙ 설정창 맨 아래 경로가 %LOCALAPPDATA%\ticker 로 바뀌었는지 확인하세요."
