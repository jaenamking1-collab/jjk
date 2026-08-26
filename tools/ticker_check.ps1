# 위젯이 "지금" 어떤 상태인지 한 번에 본다.
# 실행 중인 파일 / 코드 날짜 / ticker를 가리키는 모든 바로가기 / 남아 있는 사본.
#
# 대화 열 번 대신 이 한 줄로 확인하기 위한 것이다 — WORKLOG 120에서
# "적용됐나?"를 확인 없이 답했다가 3시간을 헛돌았다.

$dest = "$env:LOCALAPPDATA\ticker"

"=== 도는 프로세스 ==="
$procs = @(Get-CimInstance Win32_Process -Filter "Name='pythonw.exe'" |
           Where-Object { $_.CommandLine -match 'ticker\.pyw' })
if ($procs.Count -eq 0) { "  (없음 - 위젯이 꺼져 있습니다)" }
foreach ($p in $procs) {
  $path = if ($p.CommandLine -match '([A-Za-z]:\\[^"]*)\\ticker\.pyw') { $Matches[1] } else { "?" }
  $flag = if ($path -like "$dest*") { "OK" } else { "<-- 잘못된 위치" }
  "  [{0}] {1}" -f $flag, $p.CommandLine
}
if ($procs.Count -gt 1) { "  !! 위젯이 " + $procs.Count + "개 떠 있습니다 - 옛 사본이 같이 도는 중" }

""
"=== 파일 ==="
foreach ($f in @("$dest\ticker.pyw") + @(Get-ChildItem "H:\*\ticker\ticker.pyw" -EA 0 |
                 ForEach-Object { $_.FullName })) {
  if (Test-Path $f) {
    $i = Get-Item $f
    "  {0:MM-dd HH:mm}  {1,7:N0} bytes  {2}" -f $i.LastWriteTime, $i.Length, $f
  }
}

""
"=== ticker를 가리키는 바로가기 ==="
# 바탕화면이 원드라이브로 리디렉션된 PC가 있어 두 자리를 다 본다(WORKLOG 120).
$sh = New-Object -ComObject WScript.Shell
$hits = @(Get-ChildItem ([Environment]::GetFolderPath('Desktop') + '\*.lnk'),
                        "$env:USERPROFILE\OneDrive\*\*.lnk",
                        "$env:USERPROFILE\Desktop\*.lnk",
                        "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup\*.lnk",
                        "$env:ProgramData\Microsoft\Windows\Start Menu\Programs\StartUp\*.lnk" -EA 0 |
          Sort-Object FullName -Unique)
$found = $false
foreach ($h in $hits) {
  $l = $sh.CreateShortcut($h.FullName)
  if (($l.TargetPath + ' ' + $l.Arguments) -match 'ticker') {
    $found = $true
    $flag = if ($l.Arguments -match [regex]::Escape($dest)) { "OK" } else { "<-- 옛 위치" }
    "  [{0}] {1}" -f $flag, $h.FullName
    "        -> {0} {1}" -f $l.TargetPath, $l.Arguments
  }
}
if (-not $found) { "  (없음)" }
