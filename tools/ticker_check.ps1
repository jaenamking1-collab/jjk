# 위젯이 "지금" 어떤 상태인지 한 번에 본다.
# 실행 중인 파일 / 코드 날짜 / ticker를 가리키는 모든 바로가기 / 남아 있는 사본.
#
# 대화 열 번 대신 이 한 줄로 확인하기 위한 것이다 — WORKLOG 120에서
# "적용됐나?"를 확인 없이 답했다가 3시간을 헛돌았다.

$dest = "$env:LOCALAPPDATA\ticker"

"=== 도는 프로세스 ==="
# python.exe 로 켠 경우(콘솔 확인용)도 잡아야 "(없음)"으로 헷갈리지 않는다.
$procs = @(Get-CimInstance Win32_Process |
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

""
"=== 설정 (위젯이 배운 값) ==="
$cfgPath = "$dest\ticker_config.json"
if (-not (Test-Path $cfgPath)) {
  "  (없음) - 설정이 저장되지 않고 있습니다"
} else {
  $i = Get-Item $cfgPath
  "  {0:MM-dd HH:mm} 에 저장됨  ({1:N0} bytes)" -f $i.LastWriteTime, $i.Length
  try {
    $c = Get-Content $cfgPath -Raw -Encoding UTF8 | ConvertFrom-Json
    "  종목 {0}개 · 갱신 {1}초" -f @($c.tickers).Count, $c.refresh_sec
    if ($c.coin_prem) {
      $t = @(); foreach ($n in $c.coin_prem.PSObject.Properties) {
        $t += "{0} {1:N4}" -f $n.Name.Split('-')[0], $n.Value }
      "  김치프리미엄 : " + ($t -join "  ")
    } else { "  김치프리미엄 : (아직 없음)" }
    if ($c.coin_base) {
      $t = @(); foreach ($n in $c.coin_base.PSObject.Properties) {
        $t += "{0} {1}({2})" -f $n.Name.Split('-')[0], $n.Value[1], $n.Value[0] }
      "  등락률 기준가: " + ($t -join "  ")
    } else { "  등락률 기준가: (아직 없음)" }
    if ($c.ratio_band) {
      foreach ($n in $c.ratio_band.PSObject.Properties) {
        "  30일 구간    : {0}  최저 {1:N2} ~ 최고 {2:N2}" -f $n.Name, $n.Value[1], $n.Value[2] }
    } else { "  30일 구간    : (아직 없음)" }
  } catch { "  (설정 파일을 읽지 못했습니다: " + $_.Exception.Message + ")" }
}

""
"=== 이 망에서 뚫리는 곳 ==="
# 학교망은 raw.githubusercontent 와 거래소를 막는다. 어디가 막혔는지 한 번에 본다.
# ⚠️ Windows PowerShell 5.1 은 옛 TLS 를 기본으로 써서, 멀쩡한 주소도 실패로 보인다.
try { [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 } catch {}
$urls = [ordered]@{
  "코드갱신 raw     " = "https://raw.githubusercontent.com/jaenamking1-collab/jjk/main/tools/ticker.pyw"
  "코드갱신 api     " = "https://api.github.com/repos/jaenamking1-collab/jjk/contents/tools/ticker.pyw?ref=main"
  "코드갱신 jsdelivr" = "https://cdn.jsdelivr.net/gh/jaenamking1-collab/jjk@main/tools/ticker.pyw"
  "빗썸 (국내코인)  " = "https://api.bithumb.com/public/ticker/BTC_KRW"
  "업비트 (대체)    " = "https://api.upbit.com/v1/market/all"
  "코인게코 (중계)  " = "https://api.coingecko.com/api/v3/ping"
  "야후 (해외·환율) " = "https://query1.finance.yahoo.com/v7/finance/spark?range=1d&interval=1d&symbols=PLTR"
  "네이버 (국내주식)" = "https://polling.finance.naver.com/api/realtime/domestic/stock/005930"
}
foreach ($k in $urls.Keys) {
  try {
    $r = Invoke-WebRequest -Uri $urls[$k] -UseBasicParsing -TimeoutSec 8 `
                           -Headers @{ "User-Agent" = "Mozilla/5.0" }
    "  [열림] {0}  ({1})" -f $k, $r.StatusCode
  } catch {
    $code = ""
    if ($_.Exception.Response) { $code = [int]$_.Exception.Response.StatusCode }
    if ($code) { "  [열림] {0}  ({1} - 응답은 옴)" -f $k, $code }
    else       { "  [막힘] {0}" -f $k }
  }
}

""
"점검 끝. 이 화면을 그대로 캡처해서 보여주면 됩니다."
