const SHEET_ID = '1iNlOU1YBRyJ6redmVoLDE4q6VfnWqL22s32IQHdSKN8';

// ── 접근 토큰 ─────────────────────────────
// Script 속성 'APP_TOKEN'에 값을 넣으면, 아래 PUBLIC_ACTIONS를 제외한 모든 요청은
// 올바른 token 파라미터가 있어야 통과한다(개인 계좌·보유·분배 데이터 보호).
// 속성이 비어 있으면(미설정) 전부 허용 → 재배포 전까지 기존 앱이 끊기지 않음(하위호환).
// getDistribution·getEtfNotices는 공개 분배금 페이지(프록시)가 쓰므로 토큰 없이 허용.
const PUBLIC_ACTIONS = ['getDistribution', 'getEtfNotices'];
function _authOk(action, token) {
  if (PUBLIC_ACTIONS.indexOf(action) !== -1) return true;
  const secret = PropertiesService.getScriptProperties().getProperty('APP_TOKEN');
  if (!secret) return true;           // 미설정 시 허용(하위호환)
  return token === secret;
}
function _unauthorized() {
  return ContentService.createTextOutput(JSON.stringify({ error: 'unauthorized' }))
    .setMimeType(ContentService.MimeType.JSON);
}

function doGet(e) {
  if (!e || !e.parameter) return ContentService.createTextOutput('ok');
  const action = e.parameter.action;
  if (!_authOk(action, e.parameter.token)) return _unauthorized();
  let result;
  try {
    switch(action) {
      case 'getAccounts':     result = getAccounts(); break;
      case 'getHoldings':     result = getHoldings(e.parameter.account_id); break;
      case 'getDividends':    result = getDividends(e.parameter.year, e.parameter.account_id); break;
      case 'getSavings':      result = getSavings(); break;
      case 'maturityAlertPreview': result = previewMaturityAlerts(); break;
      case 'getExchangeRate': result = { rate: fetchExchangeRate() }; break;
      case 'getStockInfo':    result = getStockInfo(e.parameter.ticker, e.parameter.currency); break;
      case 'getStockList':    result = getStockList(); break;
      case 'getStockPrice':   result = getStockPrice(e.parameter.ticker, e.parameter.currency); break;
      case 'getStockHistory': result = getStockHistory(e.parameter.ticker, e.parameter.currency, e.parameter.days); break;
      case 'getEtfNotices':   result = getEtfNotices(e.parameter.source); break;
      case 'getSheetData':    result = getSheetData(e.parameter.force === '1'); break;
      case 'getPriceLog':     result = getPriceLog(); break;
      case 'getEtfScreener':  result = getEtfScreener(); break;
      case 'getNavMap':       result = getNavMap(); break;
      case 'getDistribution': result = getDistribution(e.parameter.source, e.parameter.force === '1'); break;
      case 'getDivSheetData': result = getDivSheetData(); break;
      case 'getPortfolioLog': result = getPortfolioLog(); break;
      case 'getAlerts':       result = getAlerts(e.parameter.limit ? parseInt(e.parameter.limit) : 30); break;
      case 'checkAlerts':     result = checkAndLogAlerts(); break;
      case 'markAlertRead':   result = markAlertRead(parseInt(e.parameter.row)); break;
      default: result = { error: 'Unknown action' };
    }
  } catch(err) {
    result = { error: err.toString() };
  }
  const output = ContentService.createTextOutput(JSON.stringify(result));
  output.setMimeType(ContentService.MimeType.JSON);
  return output;
}

function doPost(e) {
  const data = JSON.parse(e.postData.contents);
  const action = data.action;
  const token = (e.parameter && e.parameter.token) || data.token;
  if (!_authOk(action, token)) return _unauthorized();
  let result;
  try {
    switch(action) {
      case 'addAccount':     result = addAccount(data); break;
      case 'updateAccount':  result = updateAccount(data); break;
      case 'deleteAccount':  result = deleteAccount(data.id); break;
      case 'addHolding':     result = addHolding(data); break;
      case 'updateHolding':  result = updateHolding(data); break;
      case 'deleteHolding':  result = deleteHolding(data.id); break;
      case 'saveDividend':   result = saveDividend(data); break;
      case 'deleteDividend': result = deleteDividend(data.id); break;
      case 'addSaving':      result = addSaving(data); break;
      case 'updateSaving':   result = updateSaving(data); break;
      case 'deleteSaving':   result = deleteSaving(data.id); break;
      default: result = { error: 'Unknown action' };
    }
  } catch(err) {
    result = { error: err.toString() };
  }
  return ContentService.createTextOutput(JSON.stringify(result)).setMimeType(ContentService.MimeType.JSON);
}

// ── ACCOUNTS ──────────────────────────────
function getAccounts() {
  const sheet = getSheet('accounts');
  const rows = sheet.getDataRange().getValues();
  if (rows.length <= 1) return [];
  return rows.slice(1).filter(r => r[0]).map(r => ({ id: r[0], name: r[1], type: r[2], created_at: r[3] }));
}
function addAccount(data) {
  const sheet = getSheet('accounts');
  const id = new Date().getTime().toString();
  sheet.appendRow([id, data.name, data.type, new Date().toISOString()]);
  return { success: true, id };
}
function updateAccount(data) {
  const sheet = getSheet('accounts');
  const rows = sheet.getDataRange().getValues();
  for (let i = 1; i < rows.length; i++) {
    if (rows[i][0].toString() === data.id.toString()) {
      sheet.getRange(i+1, 2).setValue(data.name);
      sheet.getRange(i+1, 3).setValue(data.type);
      return { success: true };
    }
  }
  return { error: 'Not found' };
}
function deleteAccount(id) {
  deleteRowById('accounts', id);
  const sheet = getSheet('holdings');
  const rows = sheet.getDataRange().getValues();
  for (let i = rows.length - 1; i >= 1; i--) {
    if (rows[i][1].toString() === id.toString()) sheet.deleteRow(i+1);
  }
  return { success: true };
}

// ── HOLDINGS ──────────────────────────────
function getHoldings(account_id) {
  const sheet = getSheet('holdings');
  const rows = sheet.getDataRange().getValues();
  if (rows.length <= 1) return [];
  let data = rows.slice(1).filter(r => r[0]);
  if (account_id) data = data.filter(r => r[1].toString() === account_id.toString());
  return data.map(r => ({
    id: r[0],
    account_id: r[1],
    ticker: r[2] != null ? r[2].toString().trim() : '',
    name: r[3],
    avg_price: r[4],
    quantity: r[5],
    currency: r[6],
    div_cycle: r[7],
    created_at: r[8]
  }));
}
function addHolding(data) {
  const sheet = getSheet('holdings');
  const id = new Date().getTime().toString();
  sheet.appendRow([id, data.account_id, "'" + (data.ticker||'').toString().trim(), data.name, data.avg_price, data.quantity, data.currency, data.div_cycle, new Date().toISOString()]);
  return { success: true, id };
}
function updateHolding(data) {
  const sheet = getSheet('holdings');
  const rows = sheet.getDataRange().getValues();
  for (let i = 1; i < rows.length; i++) {
    if (rows[i][0].toString() === data.id.toString()) {
      sheet.getRange(i+1, 3).setValue("'" + (data.ticker||'').toString().trim());
      sheet.getRange(i+1, 4).setValue(data.name);
      sheet.getRange(i+1, 5).setValue(data.avg_price);
      sheet.getRange(i+1, 6).setValue(data.quantity);
      sheet.getRange(i+1, 7).setValue(data.currency);
      sheet.getRange(i+1, 8).setValue(data.div_cycle);
      return { success: true };
    }
  }
  return { error: 'Not found' };
}
function deleteHolding(id) { deleteRowById('holdings', id); return { success: true }; }

// ── DIVIDENDS ──────────────────────────────
function getDividends(year, account_id) {
  const sheet = getSheet('dividends');
  const rows = sheet.getDataRange().getValues();
  if (rows.length <= 1) return [];
  let data = rows.slice(1).filter(r => r[0]);
  if (year) data = data.filter(r => r[2].toString() === year.toString());
  return data.map(r => ({ id: r[0], holding_id: r[1], year: r[2], month: r[3], amount: r[4], currency: r[5] }));
}
function saveDividend(data) {
  const sheet = getSheet('dividends');
  const rows = sheet.getDataRange().getValues();
  for (let i = 1; i < rows.length; i++) {
    if (rows[i][1].toString() === data.holding_id.toString() &&
        rows[i][2].toString() === data.year.toString() &&
        rows[i][3].toString() === data.month.toString()) {
      sheet.getRange(i+1, 5).setValue(data.amount);
      sheet.getRange(i+1, 6).setValue(data.currency);
      return { success: true, updated: true };
    }
  }
  const id = new Date().getTime().toString();
  sheet.appendRow([id, data.holding_id, data.year, data.month, data.amount, data.currency]);
  return { success: true, id };
}
function deleteDividend(id) { deleteRowById('dividends', id); return { success: true }; }

// ── 은경 저축 ──────────────────────────────
function getSavingsSheet() {
  const ss = SpreadsheetApp.openById(SHEET_ID);
  let sheet = ss.getSheetByName('은경저축');
  if (!sheet) {
    sheet = ss.insertSheet('은경저축');
    sheet.appendRow(['id','계좌명','원금','만기이율','만기날짜','만기시금액','중도해지이율','중도해지일','중도해지금액','created_at','비고','가입일']);
  } else if (!sheet.getRange(1, 12).getValue()) {
    sheet.getRange(1, 12).setValue('가입일'); // 기존 시트에 가입일 열 헤더 보강
  }
  return sheet;
}
function getSavings() {
  const sheet = getSavingsSheet();
  const rows = sheet.getDataRange().getValues();
  if (rows.length <= 1) return [];
  return rows.slice(1).filter(r => r[0]).map(r => ({
    id: r[0], name: r[1], principal: r[2], mat_rate: r[3], mat_date: r[4],
    mat_amount: r[5], early_rate: r[6], early_date: r[7], early_amount: r[8], created_at: r[9],
    memo: r[10] != null ? r[10] : '', join_date: r[11] != null ? r[11] : ''
  }));
}
function addSaving(data) {
  const sheet = getSavingsSheet();
  const id = new Date().getTime().toString();
  sheet.appendRow([id, data.name||'', data.principal||'', data.mat_rate||'',
    data.mat_date ? "'"+data.mat_date : '', data.mat_amount||'', data.early_rate||'',
    data.early_date ? "'"+data.early_date : '', data.early_amount||'', new Date().toISOString(), data.memo||'',
    data.join_date ? "'"+data.join_date : '']);
  return { success: true, id };
}
function updateSaving(data) {
  const sheet = getSavingsSheet();
  const rows = sheet.getDataRange().getValues();
  for (let i = 1; i < rows.length; i++) {
    if (rows[i][0].toString() === data.id.toString()) {
      sheet.getRange(i+1, 2).setValue(data.name||'');
      sheet.getRange(i+1, 3).setValue(data.principal||'');
      sheet.getRange(i+1, 4).setValue(data.mat_rate||'');
      sheet.getRange(i+1, 5).setValue(data.mat_date ? "'"+data.mat_date : '');
      sheet.getRange(i+1, 6).setValue(data.mat_amount||'');
      sheet.getRange(i+1, 7).setValue(data.early_rate||'');
      sheet.getRange(i+1, 8).setValue(data.early_date ? "'"+data.early_date : '');
      sheet.getRange(i+1, 9).setValue(data.early_amount||'');
      sheet.getRange(i+1, 11).setValue(data.memo||'');
      sheet.getRange(i+1, 12).setValue(data.join_date ? "'"+data.join_date : '');
      return { success: true };
    }
  }
  return { error: 'Not found' };
}
function deleteSaving(id) { deleteRowById('은경저축', id); return { success: true }; }

// ── 만기 알림 이메일 ─────────────────────────
// installMaturityTrigger()로 매일 1회 실행 → 만기 임박 저축 계좌를 이메일로 알림.
// 시점: 6개월·3개월·1개월 진입 시 각 1회 → 만기 7일 전부터 매일(7·6·…·1일) → 만기 당일.
// (계좌 + 만기일 + 시점) 조합마다 1회만 발송. 발송 이력은 Script 속성에 기록해 중복 방지.
const MATURITY_ALERT_TO = 'azsxdcd@naver.com,divayeyo@gmail.com';

// 오늘(KST) 자정 기준 만기까지 남은 일수. 날짜 형식 아니면 null.
function _matDaysLeft(matDate) {
  const raw = String(matDate == null ? '' : matDate).trim();
  if (!/^\d{4}-\d{2}-\d{2}$/.test(raw)) return null;
  const todayStr = Utilities.formatDate(new Date(), 'Asia/Seoul', 'yyyy-MM-dd');
  const t = Date.parse(todayStr + 'T00:00:00Z');
  const m = Date.parse(raw + 'T00:00:00Z');
  if (isNaN(m) || isNaN(t)) return null;
  return Math.round((m - t) / 86400000);
}

// 남은 일수 → 알림 시점 라벨. 알림 불필요(6개월 초과)면 null.
function maturityBucket(days) {
  if (days <= 0)   return '만기일';
  if (days <= 7)   return days + '일';   // 7·6·5·4·3·2·1일: 마지막 주 매일
  if (days <= 30)  return '1개월';
  if (days <= 91)  return '3개월';
  if (days <= 183) return '6개월';
  return null;
}

function _matAlertBody(s, days, bucket) {
  const won = n => { const v = parseFloat(String(n == null ? '' : n).replace(/,/g, '')); return isFinite(v) && v ? Math.round(v).toLocaleString() + '원' : '-'; };
  const head = bucket === '만기일'
    ? (days < 0 ? '만기일이 ' + (-days) + '일 지났습니다.' : '오늘이 만기일입니다.')
    : '만기까지 ' + days + '일 남았습니다.';
  const lines = [head, '',
    '계좌: ' + (s.name || '-'),
    '원금: ' + won(s.principal),
    '만기일: ' + (s.mat_date || '-')];
  if (s.mat_rate)   lines.push('만기이율: ' + s.mat_rate + '%');
  if (s.mat_amount) lines.push('만기시 금액: ' + won(s.mat_amount));
  return lines.join('\n');
}

// 실제 발송(트리거·수동 실행). 조건 충족 + 미발송 건만 메일. 제목 예:[만기알림][SMART-1][6개월]
function sendMaturityAlerts() {
  const savings = getSavings();
  const props = PropertiesService.getScriptProperties();
  let sent; try { sent = JSON.parse(props.getProperty('sentMaturityAlerts') || '{}'); } catch(e) { sent = {}; }
  const outbox = [];
  savings.forEach(function(s) {
    const days = _matDaysLeft(s.mat_date);
    if (days == null) return;
    const bucket = maturityBucket(days);
    if (!bucket) return;
    const key = s.id + '|' + s.mat_date + '|' + bucket; // 만기일 포함 → 재예치 시 새 회차로 재알림
    if (sent[key]) return;
    const subject = '[만기알림][' + (s.name || '이름없음') + '][' + bucket + ']';
    MailApp.sendEmail({ to: MATURITY_ALERT_TO, subject: subject, body: _matAlertBody(s, days, bucket) });
    sent[key] = new Date().toISOString();
    outbox.push(subject);
  });
  if (outbox.length) props.setProperty('sentMaturityAlerts', JSON.stringify(sent));
  return { success: true, sent: outbox };
}

// 드라이런: 메일 안 보내고 '지금 보낼 대상'만 반환(테스트용, 계좌 데이터 포함이라 token 필요).
function previewMaturityAlerts() {
  const savings = getSavings();
  const props = PropertiesService.getScriptProperties();
  let sent; try { sent = JSON.parse(props.getProperty('sentMaturityAlerts') || '{}'); } catch(e) { sent = {}; }
  const rows = savings.map(function(s) {
    const days = _matDaysLeft(s.mat_date);
    const bucket = days == null ? null : maturityBucket(days);
    const key = bucket ? (s.id + '|' + s.mat_date + '|' + bucket) : null;
    return { name: s.name, mat_date: s.mat_date, days: days, bucket: bucket, willSend: !!(bucket && !sent[key]) };
  });
  return { to: MATURITY_ALERT_TO, candidates: rows.filter(function(r){ return r.willSend; }), all: rows };
}

// 하루 1회(오전 8시 KST) 트리거 설치 — 편집기에서 한 번만 실행하면 됨.
function installMaturityTrigger() {
  ScriptApp.getProjectTriggers().forEach(function(t) {
    if (t.getHandlerFunction() === 'sendMaturityAlerts') ScriptApp.deleteTrigger(t);
  });
  ScriptApp.newTrigger('sendMaturityAlerts').timeBased().everyDays(1).atHour(8).create();
  return { success: true, msg: '매일 오전 8시(KST) 만기 알림 트리거 설치됨' };
}

// ── 환율 ──────────────────────────────────
function fetchExchangeRate() {
  const cache = CacheService.getScriptCache();
  const hit = cache.get('exchange_rate');
  if (hit) return parseFloat(hit);
  try {
    const url = 'https://query1.finance.yahoo.com/v8/finance/chart/USDKRW=X?interval=1d&range=1d';
    const res = UrlFetchApp.fetch(url, { muteHttpExceptions: true, headers: { 'User-Agent': 'Mozilla/5.0' } });
    const json = JSON.parse(res.getContentText());
    const rate = json.chart.result[0].meta.regularMarketPrice;
    cache.put('exchange_rate', String(rate), 3600);
    const config = getSheet('config');
    const rows = config.getDataRange().getValues();
    for (let i = 1; i < rows.length; i++) {
      if (rows[i][0] === 'exchange_rate') { config.getRange(i+1, 2).setValue(rate); return rate; }
    }
    return rate;
  } catch(e) {
    const config = getSheet('config');
    const rows = config.getDataRange().getValues();
    for (let i = 1; i < rows.length; i++) { if (rows[i][0] === 'exchange_rate') return rows[i][1]; }
    return 1447;
  }
}

// ── 유틸 ──────────────────────────────────
function getSheet(name) { return SpreadsheetApp.openById(SHEET_ID).getSheetByName(name); }
function deleteRowById(sheetName, id) {
  const sheet = getSheet(sheetName);
  const rows = sheet.getDataRange().getValues();
  for (let i = rows.length - 1; i >= 1; i--) {
    if (rows[i][0].toString() === id.toString()) { sheet.deleteRow(i+1); return; }
  }
}

// KRW 티커 6자리 leading zero 보정 유틸
function padTicker(ticker, currency) {
  if (currency === 'KRW' && ticker && ticker.length < 6) {
    return ticker.padStart(6, '0');
  }
  return ticker;
}

function getStockInfo(ticker, currency) {
  ticker = padTicker(ticker, currency);
  try {
    if (currency === 'KRW') {
      const res = UrlFetchApp.fetch('https://polling.finance.naver.com/api/realtime/domestic/stock/' + ticker, { muteHttpExceptions: true });
      const json = JSON.parse(res.getContentText());
      if (json.datas && json.datas[0]) {
        const d = json.datas[0];
        return { name: d.stockName || d.itemName || d.name || '', success: true };
      }
    } else {
      const res = UrlFetchApp.fetch('https://query1.finance.yahoo.com/v8/finance/chart/' + ticker + '?interval=1d&range=1d', { muteHttpExceptions: true, headers: { 'User-Agent': 'Mozilla/5.0' } });
      const json = JSON.parse(res.getContentText());
      if (json.chart.result && json.chart.result[0]) return { name: json.chart.result[0].meta.longName || ticker, success: true };
    }
  } catch(e) {}
  return { success: false };
}

function getStockList() {
  const sheet = getSheet('stocks');
  const rows = sheet.getDataRange().getValues();
  if (rows.length <= 1) return [];
  return rows.slice(1).filter(r => r[0]).map(r => ({ ticker: r[0], name: r[1], currency: r[2], div_cycle: r[3] || '' }));
}

function getStockPrice(ticker, currency) {
  ticker = padTicker(ticker, currency);
  const cache = CacheService.getScriptCache();
  const cacheKey = 'price_' + ticker;
  const cached = cache.get(cacheKey);
  if (cached) return JSON.parse(cached);
  try {
    if (currency === 'KRW') {
      const res = UrlFetchApp.fetch('https://polling.finance.naver.com/api/realtime/domestic/stock/' + ticker, { muteHttpExceptions: true });
      const json = JSON.parse(res.getContentText());
      if (json.datas && json.datas[0]) {
        const d = json.datas[0];
        const stripComma = v => parseFloat((v||'0').toString().replace(/,/g,'')) || 0;
        const current = stripComma(d.closePrice || d.tradePrice);
        const diff    = stripComma(d.compareToPreviousClosePrice);
        const prev    = current - diff;
        const result  = { success: true, current, prev: prev > 0 ? prev : 0, change: parseFloat(d.fluctuationsRatio || 0) };
        cache.put(cacheKey, JSON.stringify(result), 21600);
        return result;
      }
    } else {
      const res = UrlFetchApp.fetch('https://query1.finance.yahoo.com/v8/finance/chart/' + ticker + '?interval=1d&range=1d', { muteHttpExceptions: true, headers: { 'User-Agent': 'Mozilla/5.0' } });
      const json = JSON.parse(res.getContentText());
      if (json.chart.result && json.chart.result[0]) {
        const meta = json.chart.result[0].meta;
        const result = { success: true, current: meta.regularMarketPrice, prev: meta.chartPreviousClose, change: ((meta.regularMarketPrice - meta.chartPreviousClose) / meta.chartPreviousClose * 100).toFixed(2) };
        cache.put(cacheKey, JSON.stringify(result), 21600);
        return result;
      }
    }
  } catch(e) {}
  return { success: false };
}

function getStockHistory(ticker, currency, days) {
  ticker = padTicker(ticker, currency);
  days = parseInt(days) || 5;
  const cache = CacheService.getScriptCache();
  const cacheKey = 'hist_' + ticker + '_' + days;
  const cached = cache.get(cacheKey);
  if (cached) return JSON.parse(cached);
  try {
    if (currency === 'KRW') {
      const text = UrlFetchApp.fetch('https://fchart.stock.naver.com/siseJson.naver?symbol=' + ticker + '&requestType=1&count=' + days + '&timeframe=day', { muteHttpExceptions: true }).getContentText();
      const prices = (text.match(/\[([^\]]+)\]/g) || []).slice(1).map(m => parseFloat(m.replace(/[[\]]/g,'').split(',')[4]) || 0).filter(p => p > 0);
      const result = { success: true, prices };
      cache.put(cacheKey, JSON.stringify(result), 21600);
      return result;
    } else {
      const res = UrlFetchApp.fetch('https://query1.finance.yahoo.com/v8/finance/chart/' + ticker + '?interval=1d&range=' + days + 'd', { muteHttpExceptions: true, headers: { 'User-Agent': 'Mozilla/5.0' } });
      const json = JSON.parse(res.getContentText());
      if (json.chart.result && json.chart.result[0]) {
        const result = { success: true, prices: (json.chart.result[0].indicators.quote[0].close || []).filter(p => p != null) };
        cache.put(cacheKey, JSON.stringify(result), 21600);
        return result;
      }
    }
  } catch(e) {}
  return { success: false, prices: [] };
}

// ACE papi 공용 fetch: 브라우저형 헤더로 WAF 회피 시도. HTML이 오면 차단으로 보고 명시적 에러.
// (2026-07: papi가 Apps Script 요청에 HTML 차단 페이지를 반환하기 시작 — 원인 추정: 구글 IP/봇 필터)
function fetchAceApi(url) {
  const res = UrlFetchApp.fetch(url, { muteHttpExceptions: true, headers: {
    'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36',
    'Accept': 'application/json, text/plain, */*',
    'Accept-Language': 'ko-KR,ko;q=0.9',
    'Referer': 'https://www.aceetf.co.kr/'
  }});
  const text = res.getContentText('UTF-8');
  if (text.trim().charAt(0) === '<') throw new Error('ACE API 차단 (HTTP ' + res.getResponseCode() + ', HTML 응답)');
  return JSON.parse(text);
}

// ── ETF 공지 프록시 ──
function getEtfNotices(source) {
  const cache = CacheService.getScriptCache();
  const cacheKey = 'notices_' + source;
  const cached = cache.get(cacheKey);
  if (cached) return JSON.parse(cached);
  try {
    let items = [];
    if (source === 'kodex') {
      const html = UrlFetchApp.fetch('https://www.samsungfund.com/etf/lounge/notice.do?category=DIVIDEND', { muteHttpExceptions: true }).getContentText('UTF-8');
      const matches = [...html.matchAll(/notice-view\.do\?no=(\d+)[^"]*"[^>]*>([\s\S]*?)<\/a>/g)];
      matches.slice(0, 5).forEach(m => {
        const inner = m[2].replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ').trim();
        const date = (inner.match(/\d{4}\.\d{2}\.\d{2}/) || [''])[0];
        items.push({ title: inner.replace(date, '').trim(), date, url: 'https://www.samsungfund.com/etf/lounge/notice-view.do?no=' + m[1] });
      });
    } else if (source === 'ace') {
      const json = fetchAceApi('https://papi.aceetf.co.kr/api/notices?categoryNo=61&page=1&searchValue=');
      (json.data || []).slice(0, 5).forEach(n => items.push({ title: n.title || '', date: (n.regDate || '').replace(/-/g, '.'), url: 'https://www.aceetf.co.kr/cs/notice/' + n.id }));
    } else if (source === 'rise') {
      const html = UrlFetchApp.fetch('https://www.riseetf.co.kr/cust/notice?searchText=%EB%B6%84%EB%B0%B0%EA%B8%88&searchType4=tab', { muteHttpExceptions: true }).getContentText('UTF-8');
      html.split('<li class=').slice(1).forEach(block => {
        if (items.length >= 5) return;
        const idM = block.match(/href="(\/cust\/notice\/\d+)/);
        const titleM = block.match(/class="body01">([\s\S]*?)<\/p>/);
        const dateM = block.match(/class="body02">\s*([\d.]+)/);
        if (idM && titleM && titleM[1].includes('분배금')) items.push({ title: titleM[1].replace(/<[^>]+>/g, '').trim(), date: dateM ? dateM[1].trim() : '', url: 'https://www.riseetf.co.kr' + idM[1] });
      });
    } else if (source === 'sol') {
      const json = JSON.parse(UrlFetchApp.fetch('https://www.soletf.com/api/cs/notice?keyword=%EB%B6%84%EB%B0%B0%EA%B8%88', { muteHttpExceptions: true, headers: { 'User-Agent': 'Mozilla/5.0', 'Accept': 'application/json' } }).getContentText('UTF-8'));
      (json.items || []).filter(n => (n.TITLE||'').includes('분배금') && !(n.TITLE||'').includes('이벤트')).slice(0, 5).forEach(n => items.push({ title: n.TITLE || '', date: (n.REG_DATE||'').slice(0,10).replace(/-/g,'.'), url: 'https://www.soletf.com/ko/cs/notice?no=' + n.NO }));
    } else if (source === 'tiger') {
      // 서버측 파싱(브라우저 CORS 회피). 공개 프록시가 이 액션을 중계한다.
      const fd = 'firstIndex=0&listCnt=20&pageIndex=1&detailsKey=&q=';
      const html = UrlFetchApp.fetch('https://investments.miraeasset.com/tigeretf/ko/customer/notice/list.ajax', { method:'post', payload:fd, headers:{'Content-Type':'application/x-www-form-urlencoded','User-Agent':'Mozilla/5.0'}, muteHttpExceptions:true }).getContentText('UTF-8');
      html.split('<li').slice(1).forEach(b => {
        if (items.length >= 5 || !b.includes('분배금')) return;
        const titleM = b.match(/class="txt"[^>]*>([\s\S]*?)<\//);
        if (!titleM) return;
        const title = titleM[1].replace(/<[^>]+>/g,'').replace(/\s+/g,' ').trim();
        if (!title.includes('분배금')) return;
        const keyM = b.match(/'(\d+)'/);
        const dateM = b.match(/class="item-date"[^>]*>([\s\S]*?)<\//);
        items.push({ title, date: dateM ? dateM[1].replace(/<[^>]+>/g,'').trim() : '', url: keyM ? 'https://investments.miraeasset.com/tigeretf/ko/customer/notice/view.do?detailsKey=' + keyM[1] : '#' });
      });
    } else if (source === 'plus') {
      const html = UrlFetchApp.fetch('https://www.plusetf.co.kr/customer/notice/list', { headers:{'User-Agent':'Mozilla/5.0'}, muteHttpExceptions:true }).getContentText('UTF-8');
      const re = /href="(\/customer\/notice\/detail\?n=\d+)"[^>]*>([\s\S]*?)<\/a>/g;
      let m;
      while ((m = re.exec(html)) && items.length < 5) {
        const txt = m[2].replace(/<[^>]+>/g,' ').replace(/\s+/g,' ').trim();
        if (!txt.includes('분배금')) continue;
        const date = (txt.match(/\d{4}\.\d{2}\.\d{2}/) || [''])[0];
        items.push({ title: txt.replace(date,'').trim(), date, url: 'https://www.plusetf.co.kr' + m[1] });
      }
    }
    const result = { success: true, items };
    if (items.length) cache.put(cacheKey, JSON.stringify(result), distCacheTtlSec()); // 0건은 캐시 안 함(일시 실패 장기 캐시 방지)
    return result;
  } catch(e) {
    return { success: false, items: [], error: e.toString() };
  }
}

// ── 분배금 공지 ──
function getDistribution(source, force) {
  const cache = CacheService.getScriptCache();
  const cacheKey = 'dist2_' + source;

  // 적응형 시트캐시: 이번 달 현재 회차를 이미 받았으면 파싱 스킵하고 시트값 반환
  if (!force) {
    const sc = readDistCache(source);
    if (sc && sc.payload) {
      const need = currentCycleKey();               // 예: '2026-06-말'
      const parsedItems = sc.payload.items || [];
      // 회차 일치 + 신선(TTL 내)할 때만 캐시 반환. 공시 임박 구간엔 2시간마다 재파싱해
      // 새로 올라온 공지(예: KODEX 월중)를 놓치지 않는다.
      if (parsedItems.length && sc.cycleKey === need) {
        const saved = sc.savedAt instanceof Date
          ? sc.savedAt
          : new Date(String(sc.savedAt).replace(' ', 'T') + ':00+09:00');
        if (!isNaN(saved) && (Date.now() - saved.getTime()) < distCacheTtlSec() * 1000) return attachDistHistory(source, sc.payload);
      }
    }
    const cached = cache.get(cacheKey);              // 스크립트캐시 폴백
    if (cached) return JSON.parse(cached);
  }
  let result;
  try {
    if (source === 'kodex') {
      result = fetchDist_kodex();
      if (!result || !result.items || !result.items.length) {
        const all = fetchDist_smarttoday(force);
        result = all[source] || { items: [], error: 'KODEX 양쪽 실패' };
      } else {
        try {
          const all = fetchDist_smarttoday(force);
          const st = all['kodex'];
          if (st && st.schedule) {
            result.schedule = result.schedule || {};
            if (!result.schedule['공시일'] && st.schedule['공시일']) result.schedule['공시일'] = st.schedule['공시일'];
            if (!result.schedule['분배락일'] && st.schedule['분배락일']) result.schedule['분배락일'] = st.schedule['분배락일'];
          }
        } catch(e) {}
      }
      return finishDist(source, result, cache, cacheKey);
    }
    if (source === 'tiger') {
      result = fetchDist_tiger();
      if (!result || !result.items || !result.items.length) {
        const all = fetchDist_smarttoday(force);
        result = all[source] || { items: [], error: 'TIGER 양쪽 실패' };
      }
      return finishDist(source, result, cache, cacheKey);
    }
    if (source === 'ace') {
      result = fetchDist_ace();
      if (!result || !result.items || !result.items.length) {
        const all = fetchDist_smarttoday(force);
        result = all[source] || { items: [], error: 'ACE 양쪽 실패' };
      }
      return finishDist(source, result, cache, cacheKey);
    }
    if (source === 'rise') {
      result = fetchDist_rise();
      if (!result || !result.items || !result.items.length) {
        const all = fetchDist_smarttoday(force);
        result = all[source] || { items: [], error: 'RISE 양쪽 실패' };
      }
      return finishDist(source, result, cache, cacheKey);
    }
    if (source === 'plus') {
      result = fetchDist_plus();
      if (!result || !result.items || !result.items.length) {
        const plusErr = (result && result.error) || 'PLUS 자사 파서 empty';
        const all = fetchDist_smarttoday(force);
        result = all[source] || { items: [], error: 'PLUS 양쪽 실패' };
        result._plusErr = plusErr; // 자사(OCR) 파서 실패 사유 보존 → API 응답으로 진단
      }
      return finishDist(source, result, cache, cacheKey);
    }
    const all = fetchDist_smarttoday(force);
    result = all[source];
    if (!result || !result.items || result.items.length === 0) {
      const fb = fetchDist_fallback(source);
      if (fb && fb.items && fb.items.length > 0) { fb.fallback = true; result = fb; }
      else if (!result) result = { items: [], error: '기사/사이트 모두 실패' };
    }
    if (result && result.items && result.items.length > 0) {
      const sched = result.schedule || {};
      const need = ['공시일','분배락일','기준일','지급일'].some(k => !sched[k]);
      if (need) {
        try {
          const fb = fetchDist_fallback(source);
          if (fb && fb.schedule) {
            ['공시일','분배락일','기준일','지급일'].forEach(k => {
              if (!sched[k] && fb.schedule[k]) sched[k] = fb.schedule[k];
            });
            result.schedule = sched;
            result.merged = true;
          }
        } catch(e) {}
      }
    }
  } catch(e) {
    try {
      result = fetchDist_fallback(source) || { items: [], error: e.toString() };
      if (result.items && result.items.length) result.fallback = true;
    } catch(e2) {
      result = { items: [], error: e.toString() };
    }
  }
  return finishDist(source, result, cache, cacheKey);
}

// ── 적응형 분배캐시 유틸 ──
// 공시 임박 구간(매달 8~15일, 21일~말일)의 주간(08~20시 KST)엔 2시간마다 재파싱.
// 그 외에는 6시간(CacheService 최대 TTL). 야간엔 공지 올라올 일 없으니 길게.
function distCacheTtlSec() {
  const now = new Date();
  const h = parseInt(Utilities.formatDate(now, 'Asia/Seoul', 'H'), 10);
  const d = parseInt(Utilities.formatDate(now, 'Asia/Seoul', 'd'), 10);
  const noticeWindow = (d >= 8 && d <= 15) || d >= 21;
  const daytime = h >= 8 && h < 20;
  return (noticeWindow && daytime) ? 7200 : 21600;
}
function currentCycleKey() {
  const now = new Date();
  const ym = Utilities.formatDate(now, 'Asia/Seoul', 'yyyy-MM');
  const day = parseInt(Utilities.formatDate(now, 'Asia/Seoul', 'd'), 10);
  return ym + '-' + (day <= 20 ? '중' : '말');
}
function _distCacheSheet() {
  const ss = SpreadsheetApp.openById(SHEET_ID);
  let sh = ss.getSheetByName('분배캐시');
  if (!sh) { sh = ss.insertSheet('분배캐시'); sh.appendRow(['source','payload','savedAt','cycleKey']); }
  return sh;
}
// 회차키 → 시간순 정렬용 숫자 ('2026-07-중' < '2026-07-말')
function cycleRank(key) {
  const m = String(key || '').match(/(\d{4})-(\d{2})-(중|말)/);
  if (!m) return -1;
  return (parseInt(m[1]) * 12 + parseInt(m[2])) * 2 + (m[3] === '말' ? 1 : 0);
}
// 분배캐시는 source당 회차별 여러 행(최근 3회차) — 읽기는 최신 회차 행.
function readDistCache(source) {
  try {
    const sh = _distCacheSheet();
    const rows = sh.getDataRange().getValues();
    let best = null;
    for (let i = 1; i < rows.length; i++) {
      if (rows[i][0] !== source) continue;
      if (!best || cycleRank(rows[i][3]) > cycleRank(best.cycleKey)) {
        best = { raw: rows[i][1], savedAt: rows[i][2], cycleKey: rows[i][3], row: i + 1 };
      }
    }
    if (best) return { payload: JSON.parse(best.raw), savedAt: best.savedAt, cycleKey: best.cycleKey, row: best.row };
  } catch(e) {}
  return null;
}
function writeDistCache(source, payload, cycleKey) {
  try {
    const sh = _distCacheSheet();
    const rows = sh.getDataRange().getValues();
    const now = Utilities.formatDate(new Date(), 'Asia/Seoul', 'yyyy-MM-dd HH:mm');
    const rec = [source, JSON.stringify(payload), now, cycleKey];
    const mine = [];   // 같은 source의 다른 회차 행들
    for (let i = 1; i < rows.length; i++) {
      if (rows[i][0] !== source) continue;
      if (rows[i][3] === cycleKey) { sh.getRange(i + 1, 1, 1, 4).setValues([rec]); return; }
      mine.push({ row: i + 1, rank: cycleRank(rows[i][3]) });
    }
    sh.appendRow(rec);
    // source당 최근 3회차만 유지(방금 쓴 행 + 이전 2회차) — 나머지는 아래쪽 행부터 삭제
    if (mine.length > 2) {
      mine.sort((a, b) => b.rank - a.rank);
      mine.slice(2).sort((a, b) => b.row - a.row).forEach(r => sh.deleteRow(r.row));
    }
  } catch(e) {}
}
// 이전 회차(최근 2개) 종목을 items 뒤에 병합 — 지난 달 일정 표·공지 종목이 계속 보이게.
// 종목별 sched가 없으면 그 회차 대표 일정을 채워 현재 회차 일정과 섞이지 않게 한다.
function attachDistHistory(source, payload) {
  try {
    if (!payload || !payload.items) return payload;
    const cur = currentCycleKey();
    const sh = _distCacheSheet();
    const rows = sh.getDataRange().getValues();
    const hist = [];
    for (let i = 1; i < rows.length; i++) {
      if (rows[i][0] !== source || rows[i][3] === cur) continue;
      hist.push({ rank: cycleRank(rows[i][3]), raw: rows[i][1] });
    }
    if (!hist.length) return payload;
    hist.sort((a, b) => b.rank - a.rank);
    const out = JSON.parse(JSON.stringify(payload));
    const keyOf = (it, sched) => (it.ticker || it.name) + '|' + (it.cycle || '') + '|' + ((sched || {})['기준일'] || '');
    const seen = new Set(out.items.map(it => keyOf(it, it.sched)));
    hist.slice(0, 2).forEach(h => {
      let p; try { p = JSON.parse(h.raw); } catch(e) { return; }
      (p.items || []).forEach(it => {
        const sched = (it.sched && Object.keys(it.sched).length) ? it.sched : (p.schedule || {});
        const key = keyOf(it, sched);
        if (seen.has(key)) return;
        seen.add(key);
        const copy = JSON.parse(JSON.stringify(it));
        copy.sched = sched;
        copy.hist = true;
        out.items.push(copy);
      });
    });
    return out;
  } catch(e) { return payload; }
}
// 파싱 결과 마무리: 시트캐시(회차별) 저장 → 이력 병합 → 스크립트캐시 → 반환
function finishDist(source, result, cache, cacheKey) {
  if (result && result.items && result.items.length) {
    writeDistCache(source, result, currentCycleKey());
    result = attachDistHistory(source, result);
    cache.put(cacheKey, JSON.stringify(result), distCacheTtlSec());
  }
  return result;
}

function fetchDist_fallback(source) {
  switch(source) {
    case 'kodex': return fetchDist_kodex();
    case 'tiger': return fetchDist_tiger();
    case 'ace':   return fetchDist_ace();
    case 'plus':  return fetchDist_plus();
    case 'rise':  return fetchDist_rise();
    case 'sol':   return fetchDist_sol();
    default: return null;
  }
}

function fetchDist_smarttoday(force) {
  const cache = CacheService.getScriptCache();
  if (!force) {
    const cached = cache.get('dist2_ALL');
    if (cached) return JSON.parse(cached);
  }
  const BRANDS = {
    KODEX:{id:'kodex',label:'삼성자산운용'}, TIGER:{id:'tiger',label:'미래에셋자산운용'},
    ACE:{id:'ace',label:'한국투자신탁운용'}, RISE:{id:'rise',label:'KB자산운용'},
    PLUS:{id:'plus',label:'한화자산운용'}, SOL:{id:'sol',label:'신한자산운용'}
  };
  const out = {};
  let ids = [];
  for (let page = 1; page <= 2; page++) {
    try {
      const u = 'https://www.smarttoday.co.kr/ko-kr/articles?q=' + encodeURIComponent('분배금') + (page>1?'&page='+page:'');
      const h = UrlFetchApp.fetch(u, { muteHttpExceptions:true, headers:{'User-Agent':'Mozilla/5.0'} }).getContentText('UTF-8');
      ids = ids.concat([...h.matchAll(/\/ko-kr\/articles\/(\d+)/g)].map(m => m[1]));
    } catch(e) {}
  }
  ids = [...new Set(ids)];
  const found = {};
  for (const id of ids) {
    if (Object.keys(found).length >= 6) break;
    let html;
    try {
      html = UrlFetchApp.fetch('https://www.smarttoday.co.kr/ko-kr/articles/' + id, { muteHttpExceptions:true, headers:{'User-Agent':'Mozilla/5.0'} }).getContentText('UTF-8');
    } catch(e) { continue; }
    const titleM = html.match(/<title>([^<]+)<\/title>/);
    const title = titleM ? titleM[1] : '';
    if (!/월\s*배당\s*ETF.*분배금\s*내역|월중\s*배당\s*ETF.*분배금\s*내역/.test(title)) continue;
    const brand = Object.keys(BRANDS).find(b => new RegExp(b).test(title));
    if (!brand || found[brand]) continue;
    found[brand] = { id, html, title };
  }
  Object.keys(BRANDS).forEach(brand => {
    const f = found[brand];
    if (!f) { out[BRANDS[brand].id] = { items:[], error:'최근 기사 없음', label:BRANDS[brand].label }; return; }
    const parsed = parseSmartTodayArticle(f.html);
    out[BRANDS[brand].id] = {
      success: true, items: parsed.items, schedule: parsed.schedule,
      title: f.title.replace(/^\[표\]\s*/, ''), label: BRANDS[brand].label,
      articleUrl: 'https://www.smarttoday.co.kr/ko-kr/articles/' + f.id
    };
  });
  cache.put('dist2_ALL', JSON.stringify(out), distCacheTtlSec());
  return out;
}

function parseSmartTodayArticle(html) {
  const items = [];
  const tableM = html.match(/<table[\s\S]*?<\/table>/);
  if (tableM) {
    const trs = [...tableM[0].matchAll(/<tr[^>]*>([\s\S]*?)<\/tr>/g)]
      .map(tr => [...tr[1].matchAll(/<t[dh][^>]*>([\s\S]*?)<\/t[dh]>/g)]
        .map(c => c[1].replace(/<[^>]+>/g,'').replace(/&nbsp;/g,' ').trim()));
    if (trs.length >= 2) {
      const header = trs[0];
      const codeIdx = header.findIndex(h => /종목코드|코드/.test(h));
      const nameIdx = header.findIndex(h => /종목명/.test(h));
      const rateIdx = header.findIndex(h => /분배율/.test(h));
      const amtIdx  = header.findIndex(h => /분배금/.test(h));
      trs.slice(1).forEach(cells => {
        if (cells.length < 2) return;
        const ticker = codeIdx >= 0 ? (cells[codeIdx]||'').trim() : '';
        const name   = nameIdx >= 0 ? (cells[nameIdx]||'').trim() : cells[0];
        const rate   = rateIdx >= 0 ? (parseFloat((cells[rateIdx]||'').replace(/,/g,'')) || null) : null;
        const amount = amtIdx  >= 0 ? (parseFloat((cells[amtIdx]||'').replace(/,/g,''))  || null) : null;
        if (name && amount != null) items.push({ name, ticker, rate, amount });
      });
    }
  }
  const text = html.replace(/<[^>]+>/g,' ').replace(/&nbsp;/g,' ').replace(/\s+/g,' ');
  if (items.length === 0) {
    const re = /((?:SOL|KODEX|TIGER|ACE|RISE|PLUS|Kodex)[가-힣A-Za-z0-9()+&·]+?)(?:은|는|이|가)?\s*좌당\s*([\d,]+)\s*원/g;
    let m;
    while ((m = re.exec(text))) {
      const name = m[1].trim();
      const amount = parseFloat(m[2].replace(/,/g,'')) || null;
      if (name && amount != null && !items.some(it => it.name === name)) items.push({ name, ticker:'', rate:null, amount });
    }
  }
  const schedule = {};
  const pubM = html.match(/article:published_time"?\s*content="(\d{4})-(\d{2})-(\d{2})/);
  const pubMonth = pubM ? parseInt(pubM[2]) : (new Date().getMonth()+1);
  if (pubM) schedule['공시일'] = parseInt(pubM[2]) + '월 ' + parseInt(pubM[3]) + '일';
  const baseM = text.match(/지급기준일은\s*(\d{1,2})일/) || text.match(/(\d{1,2})일을?\s*기준일/) || text.match(/(\d{1,2})일이?\s*기준일/);
  if (baseM) schedule['기준일'] = pubMonth + '월 ' + baseM[1] + '일';
  let payM = text.match(/분배금은?\s*(\d{1,2})월\s*(\d{1,2})일\s*지급/) || text.match(/(\d{1,2})월\s*(\d{1,2})일\s*지급/);
  if (payM) {
    schedule['지급일'] = parseInt(payM[1]) + '월 ' + parseInt(payM[2]) + '일';
  } else {
    payM = text.match(/오는\s*(\d{1,2})일\s*분배금이\s*지급/)
        || text.match(/(\d{1,2})일\s*분배금이[^.]*입금/)
        || text.match(/분배금은?\s*(?:오는\s*)?(\d{1,2})일\s*지급/)
        || text.match(/(\d{1,2})일\s*분배금이\s*지급/);
    if (payM) schedule['지급일'] = pubMonth + '월 ' + payM[1] + '일';
  }
  return { items, schedule };
}

// 공지가 몰리는 날: 중순①은 10일, 월말②는 25일 전후 → 앞뒤 이틀씩
function _inNoticeWindow(day) { return (day >= 8 && day <= 12) || (day >= 23 && day <= 27); }

// 트리거용(매일 10시·14시). 공지 창 밖의 날은 그냥 넘어가 쿼터를 아낀다.
function checkDistNotices() {
  const day = Number(Utilities.formatDate(new Date(), 'Asia/Seoul', 'd'));
  if (!_inNoticeWindow(day)) return;
  return checkAndLogAlerts();
}

// 트리거 설정은 파일 맨 아래 '수동 실행' 섹션의 setupDistTriggers() 참고.

// 트리거용. checkAndLogAlerts가 6개 운용사를 강제 갱신하면서 신규공지·구조변경까지 감지해 알림로그에 남긴다.
function refreshAllDistributions() {
  try { return checkAndLogAlerts(); } catch(e) { console.log('checkAndLogAlerts', e); }
  ['kodex','tiger','ace','plus','rise','sol'].forEach(s => {
    try { getDistribution(s, true); } catch(e) { console.log(s, e); }
  });
}
// ===== OCR 공용 함수 (Google Cloud Vision) =====
// 이미지 URL을 받아 OCR 텍스트 반환. 실패 시 '' 반환.
// 실패 사유는 _ocrDbg 에 남겨 호출측(fetchDist_plus 등)이 에러 응답에 실어 진단할 수 있게 한다.
var _ocrDbg = '';
function ocrImageText(imgUrl) {
  _ocrDbg = '';
  try {
    const key = PropertiesService.getScriptProperties().getProperty('VISION_API_KEY');
    if (!key) { _ocrDbg = 'no VISION_API_KEY'; return ''; }
    const resp = UrlFetchApp.fetch(imgUrl, { muteHttpExceptions: true, headers: { 'User-Agent': 'Mozilla/5.0' } });
    const code = resp.getResponseCode();
    const bytes = resp.getBlob().getBytes();
    if (code !== 200 || !bytes.length) { _ocrDbg = 'img fetch code=' + code + ' bytes=' + bytes.length + ' url=' + imgUrl; return ''; }
    const b64 = Utilities.base64Encode(bytes);
    const payload = { requests: [{ image: { content: b64 }, features: [{ type: 'DOCUMENT_TEXT_DETECTION' }] }] };
    const res = UrlFetchApp.fetch('https://vision.googleapis.com/v1/images:annotate?key=' + key, {
      method: 'post', contentType: 'application/json', payload: JSON.stringify(payload), muteHttpExceptions: true
    });
    const json = JSON.parse(res.getContentText('UTF-8'));
    if (json.error) { _ocrDbg = 'vision err ' + JSON.stringify(json.error).slice(0, 180); return ''; }
    const r0 = json.responses && json.responses[0];
    if (r0 && r0.error) { _ocrDbg = 'vision resp err ' + JSON.stringify(r0.error).slice(0, 180); return ''; }
    const text = r0 && r0.fullTextAnnotation ? r0.fullTextAnnotation.text : '';
    _ocrDbg = 'ok textLen=' + (text ? text.length : 0);
    return text;
  } catch(e) { _ocrDbg = 'ocr exception ' + e; return ''; }
}

// OCR 텍스트에서 일정 추출. 형식 예: "6.26 분배금 공시일", "6.30 분배금 지급기준일", "7.2 분배금 지급일"
// 반환: { 공시일, 분배락일, 기준일, 지급일 } (찾은 것만)
function parseScheduleFromOcr(text) {
  if (!text) return {};
  const t = text.replace(/\s+/g, ' ');
  const out = {};

  // 핵심: 설명표의 날짜는 '요일'이 붙는다 ("6.26 금", "6월 26일(금)").
  // 달력 그리드 숫자엔 요일이 없으므로, 요일 붙은 날짜만 신뢰해 그리드 노이즈 배제.
  const dateTokens = [];
  let dm;
  // 형식1: "6.26 금" 또는 "6.26 (금)" / "6/26 금"
  const re1 = /(\d{1,2})[.\/](\d{1,2})\s*\(?([월화수목금토일])\)?/g;
  while ((dm = re1.exec(t))) dateTokens.push({ m: parseInt(dm[1]), d: parseInt(dm[2]), end: dm.index + dm[0].length });
  // 형식2: "6월 26일(금)" / "6월 26일 금"
  const re2 = /(\d{1,2})월\s*(\d{1,2})일\s*\(?([월화수목금토일])\)?/g;
  while ((dm = re2.exec(t))) dateTokens.push({ m: parseInt(dm[1]), d: parseInt(dm[2]), end: dm.index + dm[0].length });

  // 라벨 직전, 요일붙은 날짜 중 가장 가까운 것
  // 라벨은 '마지막 출현' 위치 사용 (앞쪽 달력 그리드의 라벨이 아닌 뒤쪽 설명표 라벨)
  const lastIndexOf = (re) => {
    const g = new RegExp(re.source, 'g');
    let m, last = -1;
    while ((m = g.exec(t))) last = m.index;
    return last;
  };
  const findNearestBefore = (labelRe) => {
    const labelPos = lastIndexOf(labelRe);
    if (labelPos < 0) return null;
    let best = null;
    for (const dt of dateTokens) { if (dt.end <= labelPos) { if (!best || dt.end > best.end) best = dt; } }
    return best ? (best.m + '월 ' + best.d + '일') : null;
  };

  out['공시일'] = findNearestBefore(/분배금\s*공시일|공시일/);
  out['분배락일'] = findNearestBefore(/분배락일|분배락(?!\s*전일)/);
  out['기준일'] = findNearestBefore(/분배금\s*지급기준일|지급기준일/);
  out['지급일'] = findNearestBefore(/분배금\s*지급일(?!정)|(?<!기준)지급일/);
  Object.keys(out).forEach(k => { if (!out[k]) delete out[k]; });
  return out;
}

// 공지 본문 HTML에서 '일정표 이미지' URL을 찾아 OCR → 일정 반환
// 일정표 식별: OCR 결과에 지급기준일/분배락 키워드가 있는 이미지
function ocrScheduleFromNotice(html, baseUrl) {
  try {
    const imgs = [...html.matchAll(/<img[^>]+src=["']([^"']+\.(?:png|jpg|jpeg|gif))["']/gi)]
      .map(m => m[1])
      .filter(src => /upload|attach|board|notice|file/i.test(src)); // 첨부 이미지만(배너/아이콘 제외)
    for (const src of imgs) {
      const full = src.startsWith('http') ? src : (baseUrl + src);
      const text = ocrImageText(full);
      if (text && /(지급기준일|분배락|지급일)/.test(text)) {
        const sched = parseScheduleFromOcr(text);
        if (sched['기준일'] || sched['지급일']) return sched; // 유효 일정 발견
      }
    }
  } catch(e) {}
  return {};
}

// base64 이미지(또는 data URI) 직접 OCR → 일정. ACE처럼 본문에 base64가 박힌 경우.
function ocrScheduleFromBase64Html(content) {
  try {
    const key = PropertiesService.getScriptProperties().getProperty('VISION_API_KEY');
    if (!key) return {};
    const m = content.match(/data:image\/(?:png|jpeg|jpg);base64,([A-Za-z0-9+/=]+)/);
    if (!m) return {};
    const payload = { requests: [{ image: { content: m[1] }, features: [{ type: 'DOCUMENT_TEXT_DETECTION' }] }] };
    const res = UrlFetchApp.fetch('https://vision.googleapis.com/v1/images:annotate?key=' + key, {
      method: 'post', contentType: 'application/json', payload: JSON.stringify(payload), muteHttpExceptions: true
    });
    const json = JSON.parse(res.getContentText('UTF-8'));
    if (json.error) return {};
    const text = json.responses && json.responses[0] && json.responses[0].fullTextAnnotation ? json.responses[0].fullTextAnnotation.text : '';
    if (text && /(지급기준일|분배락|지급일)/.test(text)) return parseScheduleFromOcr(text);
    return {};
  } catch(e) { return {}; }
}

function fetchDist_kodex() {
  try {
    // ── 1) 공지글 목록에서 월중/월말 최신 글 no 추출 ──
    const listHtml = UrlFetchApp.fetch('https://www.samsungfund.com/etf/lounge/notice.do?category=DIVIDEND',
      { muteHttpExceptions:true, headers:{'User-Agent':'Mozilla/5.0'} }).getContentText('UTF-8');
    // 견고 전략: no 링크와 (NN월_월중/월말배당) 제목을 각각 위치(index)로 수집한 뒤,
    // 제목마다 "가장 가까운 no"를 짝지음. 링크가 제목 앞이든 뒤든(raw HTML/마크다운) 무관하게 동작.
    const linkPos = [];   // { no, idx }
    const titlePos = [];  // { mon, cycle, idx }
    let mm;
    const reLink = /notice-view\.do\?no=(\d+)/g;
    while ((mm = reLink.exec(listHtml))) linkPos.push({ no: mm[1], idx: mm.index });
    const reTitle = /\(\s*(?:'|&#39;|&apos;)?\s*(\d{2})\.(\d{1,2})월_(월중배당|월말배당)\s*\)/g;
    while ((mm = reTitle.exec(listHtml))) titlePos.push({ mon: parseInt(mm[2]), cycle: mm[3], idx: mm.index });

    const entries = [];
    const usedNo = {};
    titlePos.forEach(t => {
      // 가장 가까운(미사용) 링크 선택
      let best = null, bestDist = Infinity;
      linkPos.forEach(l => {
        const d = Math.abs(l.idx - t.idx);
        if (d < bestDist && !usedNo[l.no + '@' + t.idx]) { best = l; bestDist = d; }
      });
      if (best && bestDist <= 400) {  // 한 항목(li/링크+제목) 범위 내로 제한
        entries.push({ no: best.no, mon: t.mon, cycle: t.cycle });
        usedNo[best.no + '@' + t.idx] = true;
      }
    });
    if (!entries.length) return fetchDist_kodex_api(); // 공지 파싱 실패 → 기존 API 방식 폴백

    // 회차별 최신 글 1건씩 (목록은 최신순이라 먼저 등장한 것이 최신).
    // 전체 최신월로 거르면 새 월중 공지가 뜨는 순간 지난 월말 일정이 통째로
    // 사라지므로(7월 월중 + 6월 월말 공존 못 함), 회차별로 독립 선택한다.
    const pick = cyc => entries.find(e => e.cycle === cyc);
    const midE = pick('월중배당');
    const endE = pick('월말배당');

    // ── 2) API에서 종목별 일정(basicD/payD) 맵 구성 (이미지 일정 대체) ──
    const schedMap = {};
    try {
      const all = [];
      for (let page = 1; page <= 6; page++) {
        const res = UrlFetchApp.fetch('https://www.samsungfund.com/api/v1/kodex/distribution.do?pageNo=' + page + '&pageSize=100',
          { muteHttpExceptions:true, headers:{'User-Agent':'Mozilla/5.0','Accept':'application/json'} });
        if (res.getResponseCode() !== 200) break;
        const j = JSON.parse(res.getContentText('UTF-8'));
        const list = j.dividList || [];
        if (!list.length) break;
        all.push(...list);
        if (all.length >= (j.totalCnt || 9999)) break;
      }
      const ymd = s => { if(!s||String(s).length<8) return ''; const t=String(s); return parseInt(t.substr(4,2))+'월 '+parseInt(t.substr(6,2))+'일'; };
      all.forEach(it => {
        const tk = (it.stkTicker||'').toString().trim();
        if (!tk) return;
        // 종목별 최신 회차 basicD 우선
        if (!schedMap[tk] || String(it.basicD) > String(schedMap[tk]._b)) {
          schedMap[tk] = { '기준일': ymd(it.basicD), '지급일': ymd(it.payD), _b: String(it.basicD||'') };
        }
      });
    } catch(eApi) {}

    // ── 3) 공지글 본문 표 파싱 ──
    // 그 달 마지막 영업일(주말 제외, 공휴일 미반영 근사)
    const lastBizDay = (year, mon) => {
      const d = new Date(year, mon, 0); // mon월 마지막 날
      while (d.getDay() === 0 || d.getDay() === 6) d.setDate(d.getDate() - 1);
      return { m: d.getMonth() + 1, d: d.getDate() };
    };
    // 다음 영업일
    const nextBizDay = (year, mon, day) => {
      const d = new Date(year, mon - 1, day);
      do { d.setDate(d.getDate() + 1); } while (d.getDay() === 0 || d.getDay() === 6);
      return { m: d.getMonth() + 1, d: d.getDate() };
    };
    const curYear = new Date().getFullYear();

    const parseNoticeTable = (no, cycleLabel) => {
      const html = UrlFetchApp.fetch('https://www.samsungfund.com/etf/lounge/notice-view.do?no=' + no,
        { muteHttpExceptions:true, headers:{'User-Agent':'Mozilla/5.0'} }).getContentText('UTF-8');
      const out = [];
      // 본문 텍스트의 분배율 기준일: "* 분배율 : 6월 25일 종가 기준" → 기준월 확정
      const plain = html.replace(/<[^>]+>/g,' ').replace(/&nbsp;/g,' ').replace(/\s+/g,' ');
      const refM = plain.match(/분배율\s*[:：]?\s*(\d{1,2})월\s*(\d{1,2})일\s*종가/);
      const refMonth = refM ? parseInt(refM[1]) : null;
      const refDay   = refM ? parseInt(refM[2]) : null;

      // 공시일: 공지 게시일 (본문 상단 "2026.06.26" 또는 메타 published_time)
      let pubMD = null;
      const pubM = plain.match(/(\d{4})\.(\d{2})\.(\d{2})/) || html.match(/article:published_time"?\s*content="(\d{4})-(\d{2})-(\d{2})/);
      if (pubM) pubMD = parseInt(pubM[2]) + '월 ' + parseInt(pubM[3]) + '일';

      // 이 회차의 일정 계산 (본문 기준월 기반, API 미반영 대비)
      let calcSched = {};
      if (pubMD) calcSched['공시일'] = pubMD;
      if (refMonth) {
        if (cycleLabel === '월말') {
          // 월말배당: 지급기준일 = 기준월 마지막 영업일, 지급일 = 다음 영업일
          const base = lastBizDay(curYear, refMonth);
          const pay  = nextBizDay(curYear, base.m, base.d);
          calcSched['기준일'] = base.m + '월 ' + base.d + '일';
          calcSched['지급일'] = pay.m + '월 ' + pay.d + '일';
        } else {
          // 월중배당: 분배율 기준일 다음 영업일을 지급기준일로 근사
          if (refDay) {
            const pay = nextBizDay(curYear, refMonth, refDay);
            calcSched['기준일'] = refMonth + '월 ' + refDay + '일';
            calcSched['지급일'] = pay.m + '월 ' + pay.d + '일';
          }
        }
      }

      // 실제 일정은 본문 첨부 이미지에 있음(월중·월말 공통). 계산 근사는 부정확
      // (월중 지급일이 기준일 다음 영업일이 아니라 며칠 뒤인 경우 多) → OCR 실제값으로 덮어쓰기.
      {
        const ocrSched = ocrScheduleFromNotice(html, 'https://www.samsungfund.com');
        if (ocrSched && (ocrSched['기준일'] || ocrSched['지급일'])) {
          // OCR로 읽은 값으로 덮어쓰기 (공시일은 OCR에 있으면 우선, 없으면 게시일 유지)
          if (ocrSched['공시일']) calcSched['공시일'] = ocrSched['공시일'];
          if (ocrSched['분배락일']) calcSched['분배락일'] = ocrSched['분배락일'];
          if (ocrSched['기준일']) calcSched['기준일'] = ocrSched['기준일'];
          if (ocrSched['지급일']) calcSched['지급일'] = ocrSched['지급일'];
          calcSched['_ocr'] = true; // OCR 확정 표시
        }
      }

      const trRe = /<tr[^>]*>([\s\S]*?)<\/tr>/g;
      let tr;
      while ((tr = trRe.exec(html))) {
        const cols = [...tr[1].matchAll(/<t[dh][^>]*>([\s\S]*?)<\/t[dh]>/g)]
          .map(c => c[1].replace(/<[^>]+>/g,'').replace(/&nbsp;/g,' ').replace(/,/g,'').trim());
        // 종목코드 패턴: 6자리 영숫자 (494300, 0005A0 등)
        const tickerIdx = cols.findIndex(c => /^[0-9A-Z]{6}$/.test(c));
        if (tickerIdx < 0) continue;
        const ticker = cols[tickerIdx];
        const name = (cols[tickerIdx+1] || '').replace(/\s*ETF\s*$/,'').trim();
        const nums = cols.slice(tickerIdx+2).filter(c => /^[\d.]+$/.test(c)).map(Number);
        if (!nums.length) continue;
        // 분배율(작은 값, 0~30) + 분배금(큰 값, >=10 보통이나 12원 등 예외 있어 마지막 숫자를 금액으로)
        const rate   = nums.find(n => n > 0 && n < 30) ?? null;
        const amount = nums[nums.length-1];
        if (amount == null) continue;
        // 일정: OCR로 실제 일정을 읽었으면 그것이 공식 공지값이므로 최우선.
        // 없으면 API값(schedMap)이 본문 기준월과 일치할 때 우선, 아니면 계산값(calcSched).
        let sched = calcSched;
        const apiS = calcSched['_ocr'] ? null : schedMap[ticker];
        if (apiS && apiS['기준일']) {
          const apiMonM = apiS['기준일'].match(/(\d{1,2})월/);
          const apiMon = apiMonM ? parseInt(apiMonM[1]) : null;
          // 월말이면 API 기준월이 refMonth와 같을 때만(=API에 이번 월말 반영됨) 채택
          // 월중이면 API 기준월이 refMonth와 같을 때 채택
          if (apiMon === refMonth) sched = { '공시일': calcSched['공시일'], '기준일': apiS['기준일'], '지급일': apiS['지급일'] };
        }
        out.push({ name, ticker, amount: Number(amount), rate, cycle: cycleLabel, sched });
      }
      return out;
    };

    let items = [];
    if (midE) { try { items = items.concat(parseNoticeTable(midE.no, '월중')); } catch(e) {} }
    if (endE) { try { items = items.concat(parseNoticeTable(endE.no, '월말')); } catch(e) {} }

    if (!items.length) return fetchDist_kodex_api(); // 표 파싱 0건 → API 폴백

    // ── 4) 대표 일정(schedule): 월중 글 기준일/지급일을 대표로 (API 보강) ──
    const repItem = items.find(it => it.cycle === '월중' && it.sched['기준일']) || items.find(it => it.sched['기준일']);
    const schedule = repItem ? { '기준일': repItem.sched['기준일'], '지급일': repItem.sched['지급일'] } : {};

    return { success: true, items, schedule, title: 'KODEX 분배금 (공지글 파싱)' };
  } catch(e) {
    try { return fetchDist_kodex_api(); } catch(e2) { return { items: [], error: 'KODEX: ' + e.toString() }; }
  }
}

// 기존 자사 API 방식 (폴백용으로 분리 보존)
function fetchDist_kodex_api() {
  try {
    const all = [];
    for (let page = 1; page <= 6; page++) {
      const res = UrlFetchApp.fetch('https://www.samsungfund.com/api/v1/kodex/distribution.do?pageNo=' + page + '&pageSize=100',
        { muteHttpExceptions:true, headers:{'User-Agent':'Mozilla/5.0','Accept':'application/json'} });
      if (res.getResponseCode() !== 200) break;
      const j = JSON.parse(res.getContentText('UTF-8'));
      const list = j.dividList || [];
      if (!list.length) break;
      all.push(...list);
      if (all.length >= (j.totalCnt || 9999)) break;
    }
    const ymd = s => { if(!s||String(s).length<8) return ''; const t=String(s); return parseInt(t.substr(4,2))+'월 '+parseInt(t.substr(6,2))+'일'; };
    const byTicker = {};
    all.forEach(it => {
      const ticker = (it.stkTicker||'').toString().trim();
      if (it.dividA == null) return;
      if (!byTicker[ticker] || String(it.basicD) > String(byTicker[ticker].basicD)) byTicker[ticker] = it;
    });
    const items = [];
    Object.keys(byTicker).forEach(ticker => {
      const it = byTicker[ticker];
      items.push({ name: it.fNm || '', ticker, amount: Number(it.dividA), rate: it.dividY != null ? Math.round(Number(it.dividY)*100)/100 : null, sched: { '기준일': ymd(it.basicD), '지급일': ymd(it.payD) } });
    });
    if (!items.length) return { items: [], error: 'KODEX: API 결과 없음' };
    const cnt = {};
    items.forEach(it => { const k=it.sched['기준일']; if(k) cnt[k]=(cnt[k]||0)+1; });
    const topBase = Object.keys(cnt).sort((a,b)=>cnt[b]-cnt[a])[0] || '';
    const rep = items.find(it => it.sched['기준일'] === topBase);
    const schedule = rep ? { '기준일': rep.sched['기준일'], '지급일': rep.sched['지급일'] } : {};
    return { success: true, items, schedule, title: 'KODEX 분배금 (자사 API)' };
  } catch(e) {
    return { items: [], error: 'KODEX: ' + e.toString() };
  }
}
function parseKodexSchedule(html) {
  const schedule = {};
  const thRe = /<th[^>]*>([\s\S]*?)<\/th>\s*<td[^>]*>([\s\S]*?)<\/td>/g;
  let m;
  while ((m = thRe.exec(html))) {
    const key = m[1].replace(/<[^>]+>/g,'').replace(/&nbsp;/g,' ').trim();
    const val = m[2].replace(/<[^>]+>/g,'').replace(/&nbsp;/g,' ').trim();
    if (!val) continue;
    const dateM = val.match(/\d{4}[.\-\/]\d{1,2}[.\-\/]\d{1,2}|\d{1,2}월\s*\d{1,2}일|\d{1,2}[\/]\d{1,2}/);
    if (!dateM) continue;
    if (/공시/.test(key))    schedule['공시일']  = dateM[0];
    if (/분배락/.test(key))  schedule['분배락일'] = dateM[0];
    if (/기준/.test(key))    schedule['기준일']  = dateM[0];
    if (/지급/.test(key))    schedule['지급일']  = dateM[0];
  }
  if (!Object.keys(schedule).length) return parseScheduleFromText(html.replace(/<[^>]+>/g,' '));
  return schedule;
}

function fetchDist_tiger() {
  try {
    const fd = 'firstIndex=0&listCnt=20&pageIndex=1&detailsKey=&q=';
    const listRes = UrlFetchApp.fetch('https://investments.miraeasset.com/tigeretf/ko/customer/notice/list.ajax', {
      method: 'post', payload: fd,
      headers: { 'Content-Type': 'application/x-www-form-urlencoded', 'User-Agent': 'Mozilla/5.0' },
      muteHttpExceptions: true
    });
    const listHtml = listRes.getContentText('UTF-8');
    const allKeys = [...listHtml.matchAll(/'detailsKey',\s*'(\d+)'/g)].map(m => m[1]);
    if (!allKeys.length) return { items: [], error: 'TIGER: detailsKey 파싱 실패' };

    // 분배금 글들의 본문에서 공시일(월/일) 파싱 → 최신 월의 월중/월말 가르기
    // TIGER 컨벤션: 월중 공시 ≈ 11일, 월말 공시 ≈ 26일 (같은 달 두 번)
    const curYear = new Date().getFullYear();
    const found = []; // { key, pubMon, pubDay, cycle }
    for (const key of allKeys.slice(0, 12)) {
      const html = UrlFetchApp.fetch('https://investments.miraeasset.com/tigeretf/ko/customer/notice/view.do?detailsKey=' + key,
        { headers: { 'User-Agent': 'Mozilla/5.0' }, muteHttpExceptions: true }).getContentText('UTF-8');
      if (!(html.includes('분배금') && (html.includes('분배율') || html.includes('좌당')))) continue;
      const text = html.replace(/<[^>]+>/g,' ').replace(/&nbsp;/g,' ').replace(/\s+/g,' ');
      const pubM = text.match(/(\d{1,2})\/(\d{1,2})\s*\([월화수목금토일]\)\s*분배금\s*공시일/);
      if (!pubM) continue;
      const pubMon = parseInt(pubM[1]), pubDay = parseInt(pubM[2]);
      const cycle = pubDay >= 20 ? '월말' : '월중'; // 공시 20일 이후=월말, 이전=월중
      found.push({ key, pubMon, pubDay, cycle });
      if (found.length >= 8) break;
    }
    if (!found.length) return { items: [], error: 'TIGER: 분배금 공지 미발견' };

    // 회차별 최신 글 1건씩 (목록 최신순) — 새 월중이 떠도 지난 월말 일정 유지
    const midE = found.find(f => f.cycle === '월중');
    const endE = found.find(f => f.cycle === '월말');

    let items = [];
    let schedule = {};
    if (midE) {
      const r = fetchDist_tiger_detail(midE.key, '월중');
      items = items.concat(r.items);
      if (Object.keys(r.schedule).length) schedule = r.schedule; // 월중 일정을 대표로
    }
    if (endE) {
      const r = fetchDist_tiger_detail(endE.key, '월말');
      items = items.concat(r.items);
      if (!Object.keys(schedule).length && Object.keys(r.schedule).length) schedule = r.schedule;
    }
    if (!items.length) return { items: [], error: 'TIGER: 표 파싱 0건' };
    return { success: true, items, schedule, title: 'TIGER 분배금 (자사 공지 파싱)' };
  } catch(e) {
    return { items: [], error: 'TIGER: ' + e.toString() };
  }
}

function fetchDist_tiger_detail(key, cycleLabel) {
  const detailRes = UrlFetchApp.fetch('https://investments.miraeasset.com/tigeretf/ko/customer/notice/view.do?detailsKey=' + key, { headers: { 'User-Agent': 'Mozilla/5.0' }, muteHttpExceptions: true });
  const html = detailRes.getContentText('UTF-8');
  const items = [];
  const trRe = /<tr[^>]*>([\s\S]*?)<\/tr>/g;
  let tr;
  while ((tr = trRe.exec(html))) {
    // TIGER 표 컬럼 순서: [종목코드, 종목명, 분배금(원), 분배율(%)]
    const cols = [...tr[1].matchAll(/<td[^>]*>([\s\S]*?)<\/td>/g)]
      .map(x => x[1].replace(/<[^>]+>/g,'').replace(/&nbsp;/g,' ').replace(/&#37;/g,'%').replace(/,/g,'').trim());
    const tickerIdx = cols.findIndex(c => /^[0-9A-Z]{6}$/.test(c));
    if (tickerIdx < 0) continue;
    const ticker = cols[tickerIdx];
    const name = (cols[tickerIdx+1] || '').replace(/\s*ETF\s*$/,'').trim();
    // tickerIdx 이후 숫자들: 분배금(정수, %없음) / 분배율(%붙음)
    const after = cols.slice(tickerIdx+2);
    let amount = null, rate = null;
    after.forEach(c => {
      if (/%/.test(c)) { const r = parseFloat(c.replace('%','')); if (!isNaN(r) && rate == null) rate = r; }
      else if (/^[\d.]+$/.test(c)) { const a = parseFloat(c); if (!isNaN(a) && amount == null) amount = a; }
    });
    if (amount == null) continue;
    items.push({ name, ticker, amount: Number(amount), rate, cycle: cycleLabel });
  }
  const schedule = parseTigerSchedule(html);
  // 일정에 cycle 정보 부여 위해 각 item.sched에도 동일 일정 복사(달력 표시용)
  items.forEach(it => { it.sched = { '공시일': schedule['공시일'], '분배락일': schedule['분배락일'], '기준일': schedule['기준일'], '지급일': schedule['지급일'] }; });
  return { success: true, items, schedule };
}

function parseTigerSchedule(html) {
  const schedule = {};
  let text = html.replace(/<[^>]+>/g,' ').replace(/&nbsp;/g,' ').replace(/\s+/g,' ');
  const secIdx = text.indexOf('분배금 지급 일정');
  if (secIdx >= 0) text = text.slice(secIdx);
  const rules = [
    ['공시일',   /(\d{1,2}[\/\.]\d{1,2})\s*\([월화수목금토일]\)\s*분배금\s*공시일/],
    ['분배락일', /(\d{1,2}[\/\.]\d{1,2})\s*\([월화수목금토일]\)\s*분배락일/],
    ['기준일',   /(\d{1,2}[\/\.]\d{1,2})\s*\([월화수목금토일]\)\s*분배금\s*지급기준일/],
    ['지급일',   /(\d{1,2}[\/\.]\d{1,2})\s*\([월화수목금토일]\)\s*분배금\s*지급일/],
  ];
  rules.forEach(([label, re]) => {
    const m = text.match(re);
    if (m) schedule[label] = m[1].replace('.', '/');
  });
  return schedule;
}

function parseAceSchedule(html) {
  const schedule = {};
  const text = html.replace(/<[^>]+>/g,' ').replace(/&nbsp;/g,' ').replace(/\s+/g,' ');
  const now = new Date();
  const curMonth = now.getMonth() + 1;
  const noticeM = text.match(/분배금\s*공시일\s*\(?\s*(\d{1,2})월\s*(\d{1,2})일/);
  if (noticeM) schedule['공시일'] = noticeM[1] + '월 ' + noticeM[2] + '일';
  const baseM = text.match(/매월\s*(\d{1,2})일을?\s*지급기준일/);
  if (baseM) {
    schedule['기준일'] = curMonth + '월 ' + baseM[1] + '일';
  } else {
    const baseM2 = text.match(/(\d{1,2})월\s*(\d{1,2})일.{0,10}지급기준일|지급기준일.{0,10}(\d{1,2})월\s*(\d{1,2})일/);
    if (baseM2) {
      const mo = baseM2[1] || baseM2[3], dy = baseM2[2] || baseM2[4];
      if (mo && dy) schedule['기준일'] = mo + '월 ' + dy + '일';
    }
  }
  return schedule;
}

function fetchDist_ace() {
  try {
    const listJson = fetchAceApi('https://papi.aceetf.co.kr/api/notices?categoryNo=61&page=1&searchValue=');
    const notices = (listJson.data || []).filter(n => (n.title||'').includes('분배금'));
    if (!notices.length) return { items: [], error: 'ACE: 분배금 공지 없음' };

    // regDate(게시일)로 월중/월말 가르기: 게시 20일 이후=월말, 이전=월중
    // 단발 비정기(국고채 등, 게시 5일경)는 제외 위해 월중은 11~16일만 인정
    const parsed = notices.map(n => {
      const m = (n.regDate || '').match(/(\d{4})-(\d{2})-(\d{2})/);
      if (!m) return null;
      return { id: n.id, title: n.title || '', y: parseInt(m[1]), mon: parseInt(m[2]), day: parseInt(m[3]) };
    }).filter(Boolean);
    if (!parsed.length) return { items: [], error: 'ACE: 게시일 파싱 실패' };

    // 회차별 최신 글 1건씩 (목록 최신순) — 새 월중이 떠도 지난 월말 일정 유지
    const midE = parsed.find(p => p.day >= 10 && p.day <= 16);
    const endE = parsed.find(p => p.day >= 20);

    // 영업일 계산 헬퍼
    const lastBizDay = (year, mon) => { const d = new Date(year, mon, 0); while (d.getDay()===0||d.getDay()===6) d.setDate(d.getDate()-1); return { m:d.getMonth()+1, d:d.getDate() }; };
    const nextBizDay = (year, mon, day) => { const d = new Date(year, mon-1, day); do { d.setDate(d.getDate()+1); } while (d.getDay()===0||d.getDay()===6); return { m:d.getMonth()+1, d:d.getDate() }; };
    const curYear = new Date().getFullYear();

    const fetchAceBody = (id) => {
      const dj = fetchAceApi('https://papi.aceetf.co.kr/api/notices/' + id);
      const item = dj.current || dj.data || {};
      let content = item.content || '';
      if (!content) {
        try {
          const bj = fetchAceApi('https://papi.aceetf.co.kr/api/notices/' + id + '/body');
          content = bj.data || bj.content || '';
        } catch(e) {}
      }
      return content;
    };

    const parseAceTable = (entry, cycleLabel) => {
      const content = fetchAceBody(entry.id);
      const out = [];
      // 본문 분배율 기준월: "분배율 : 6월 26일 종가 기준"
      const plain = content.replace(/<[^>]+>/g,' ').replace(/&nbsp;/g,' ').replace(/\s+/g,' ');
      const refM = plain.match(/분배율\s*[:：]?\s*(\d{1,2})월\s*(\d{1,2})일\s*종가/);
      const refMonth = refM ? parseInt(refM[1]) : entry.mon;
      const refDay   = refM ? parseInt(refM[2]) : null;

      // 일정 계산 (근사치) — 실제값은 아래 OCR로 덮어씀
      let calcSched = { '공시일': entry.mon + '월 ' + entry.day + '일' };
      if (cycleLabel === '월말') {
        const base = lastBizDay(curYear, refMonth);
        const pay  = nextBizDay(curYear, base.m, base.d);
        calcSched['기준일'] = base.m + '월 ' + base.d + '일';
        calcSched['지급일'] = pay.m + '월 ' + pay.d + '일';
      } else {
        // ACE 월중: 본문 "매월 15일을 지급기준일" → 기준일=15일(휴일이면 직전 영업일), 지급일=다음 영업일
        const baseMon = refMonth || entry.mon;
        let bd = new Date(curYear, baseMon - 1, 15);
        while (bd.getDay() === 0 || bd.getDay() === 6) bd.setDate(bd.getDate() - 1); // 15일 휴일이면 직전 영업일
        const base = { m: bd.getMonth() + 1, d: bd.getDate() };
        const pay = nextBizDay(curYear, base.m, base.d);
        calcSched['기준일'] = base.m + '월 ' + base.d + '일';
        calcSched['지급일'] = pay.m + '월 ' + pay.d + '일';
      }

      // 월중·월말 공통: 실제 일정은 본문 base64 이미지에 있음. 계산값은 근사치라
      // (특히 월중 지급일은 기준일 다음 영업일이 아니라 T+3인 경우 多) OCR 실제값을 우선한다.
      const ocrSched = ocrScheduleFromBase64Html(content);
      if (ocrSched && (ocrSched['기준일'] || ocrSched['지급일'])) {
        if (ocrSched['공시일']) calcSched['공시일'] = ocrSched['공시일'];
        if (ocrSched['분배락일']) calcSched['분배락일'] = ocrSched['분배락일'];
        if (ocrSched['기준일']) calcSched['기준일'] = ocrSched['기준일'];
        if (ocrSched['지급일']) calcSched['지급일'] = ocrSched['지급일'];
        calcSched['_ocr'] = true;
      }

      // 표: [종목명, 종목코드, 분배금, 분배율, 기호]
      const trRe = /<tr[^>]*>([\s\S]*?)<\/tr>/g;
      let tr;
      while ((tr = trRe.exec(content))) {
        const cols = [...tr[1].matchAll(/<td[^>]*>([\s\S]*?)<\/td>/g)].map(x => x[1].replace(/<[^>]+>/g,'').replace(/&nbsp;/g,' ').replace(/,/g,'').trim());
        const tickerIdx = cols.findIndex(c => /^[0-9A-Z]{6}$/.test(c));
        if (tickerIdx < 0) continue;
        const ticker = cols[tickerIdx];
        const name = (cols[tickerIdx-1] || cols[tickerIdx+1] || '').replace(/\s*ETF\s*$/,'').trim();
        // ACE 표 순서: [명, 코드, 금액, 율, 기호] → 코드 다음 숫자 = [금액, 율]
        const after = cols.slice(tickerIdx+1).filter(c => /^[\d.]+$/.test(c)).map(Number);
        if (after.length < 1) continue;
        const amount = after[0];        // 첫 숫자 = 분배금(원)
        const rate   = after.length >= 2 ? after[1] : null; // 둘째 숫자 = 분배율(%)
        if (amount == null) continue;
        out.push({ name, ticker, amount: Number(amount), rate, cycle: cycleLabel, sched: { ...calcSched } });
      }
      return out;
    };

    let items = [], schedule = {};
    if (midE) { const r = parseAceTable(midE, '월중'); items = items.concat(r); if (r[0]) schedule = r[0].sched; }
    if (endE) { const r = parseAceTable(endE, '월말'); items = items.concat(r); if (!Object.keys(schedule).length && r[0]) schedule = r[0].sched; }
    if (!items.length) return { items: [], error: 'ACE: 표 파싱 0건' };
    return { success: true, items, schedule, title: 'ACE 분배금 (자사 공지 파싱)' };
  } catch(e) {
    return { items: [], error: 'ACE: ' + e.toString() };
  }
}

function fetchDist_plus() {
  // PLUS 사이트는 정상인데도(curl 0.2초) Apps Script에서 간헐적으로 Timeout이 난다.
  // 목록·상세 어느 쪽이 죽어도 6월짜리 뉴스 폴백으로 밀리므로 둘 다 재시도한다.
  const fetchPlus = (url) => {
    let lastErr;
    for (let a = 0; a < 3; a++) {
      try {
        return UrlFetchApp.fetch(url, { headers: { 'User-Agent': 'Mozilla/5.0' }, muteHttpExceptions: true }).getContentText('UTF-8');
      } catch (e) { lastErr = e; Utilities.sleep(1000 * (a + 1)); }
    }
    throw lastErr;
  };
  try {
    const listHtml = fetchPlus('https://www.plusetf.co.kr/customer/notice/list');
    // 분배금 공지 목록: 제목에 (월말)/(월중) 명시
    const rowRe = /href="\/customer\/notice\/detail\?n=(\d+)"[\s\S]{0,300}?<\/a>/g;
    const cands = [];
    let rm;
    while ((rm = rowRe.exec(listHtml))) {
      const block = rm[0];
      const titleM = block.match(/PLUS ETF[^<]*분배금[^<]*/);
      if (!titleM) continue;
      const title = titleM[0].trim();
      const dateM = block.match(/(\d{4})\.(\d{2})\.(\d{2})/);
      let cycle = null;
      if (/\(월말\)/.test(title)) cycle = '월말';
      else if (/\(월중\)/.test(title)) cycle = '월중';
      const monM = title.match(/(\d{1,2})월/);
      cands.push({ n: rm[1], title, cycle, mon: monM ? parseInt(monM[1]) : 0,
        pubMon: dateM ? parseInt(dateM[2]) : 0, pubDay: dateM ? parseInt(dateM[3]) : 0 });
    }
    const dated = cands.filter(c => c.cycle && c.mon);
    if (!dated.length) return { items: [], error: 'PLUS: 월중/월말 공지 미발견' };
    // 회차별 최신 글 1건씩 (목록 최신순) — 새 월중이 떠도 지난 월말 일정 유지
    const midE = dated.find(c => c.cycle === '월중');
    const endE = dated.find(c => c.cycle === '월말');

    const parsePlusNotice = (entry) => {
      const html = fetchPlus('https://www.plusetf.co.kr/customer/notice/detail?n=' + entry.n);
      const out = [];
      let sched = {};
      if (entry.pubMon) sched['공시일'] = entry.pubMon + '월 ' + entry.pubDay + '일';
      let usedOcr = false;

      // 방어 1: 텍스트 <table>에 종목코드 행이 있으면 우선 파싱
      const tables = [...html.matchAll(/<table[\s\S]*?<\/table>/g)];
      for (const tb of tables) {
        const trs = [...tb[0].matchAll(/<tr[^>]*>([\s\S]*?)<\/tr>/g)];
        for (const tr of trs) {
          const cols = [...tr[1].matchAll(/<td[^>]*>([\s\S]*?)<\/td>/g)].map(x => x[1].replace(/<[^>]+>/g,'').replace(/&nbsp;/g,' ').replace(/,/g,'').trim());
          const ti = cols.findIndex(c => /^([0-9]{6}|[0-9]{4}[A-Z][0-9])$/.test(c));
          if (ti < 0) continue;
          const after = cols.slice(ti+1).filter(c => /^[\d.]+%?$/.test(c)).map(c => parseFloat(c.replace('%','')));
          if (!after.length) continue;
          const amount = after[0];
          const rate = after.length >= 2 ? after[after.length-1] : null;
          out.push({ name: (cols[ti-1]||cols[ti+1]||'').replace(/\s*ETF\s*$/,'').trim(), ticker: cols[ti], amount: Number(amount), rate, cycle: entry.cycle });
        }
      }

      // 방어 2: 텍스트 표에서 못 얻으면 본문 이미지 OCR
      let dbg = 'n=' + entry.n + ' htmlLen=' + html.length + ' tbl=' + tables.length + ' out1=' + out.length;
      if (!out.length) {
        const imgM = html.match(/<img[^>]+src=["'](https?:\/\/[^"']*\/upload\/[^"']+\.(?:png|jpg|jpeg|PNG|JPG))["']/i);
        dbg += ' img=' + (imgM ? imgM[1].slice(-40) : 'NONE');
        if (imgM) {
          const ocrText = ocrImageText(imgM[1]);
          dbg += ' ocr[' + _ocrDbg + ']';
          if (ocrText) {
            usedOcr = true;
            const t = ocrText.replace(/\s+/g, ' ');
            // 종목코드 위치 기준으로 분할, 각 구간에서 금액+분배율
            const codeRe = /\b([0-9]{6}|[0-9]{4}[A-Z][0-9])\b/g;
            const codes = [];
            let cm;
            while ((cm = codeRe.exec(t))) codes.push({ code: cm[1], idx: cm.index, end: cm.index + cm[0].length });
            for (let i = 0; i < codes.length; i++) {
              const seg = t.slice(codes[i].end, i+1 < codes.length ? codes[i+1].idx : codes[i].end + 80);
              const rateM = seg.match(/(\d+\.\d+)\s*%/);
              const rate = rateM ? parseFloat(rateM[1]) : null;
              let amount = null;
              const amtM = seg.match(/(\d+)\s+\d+\.\d+\s*%/);
              if (amtM) amount = parseInt(amtM[1]);
              if (amount == null) continue;
              // 종목명: 코드와 금액 사이 한글/영문 (기호 ●·. 제거)
              let name = seg.slice(0, amtM ? seg.indexOf(amtM[0]) : seg.length).replace(/[●•·.]/g,'').replace(/\s+/g,' ').trim();
              out.push({ name, ticker: codes[i].code, amount: Number(amount), rate, cycle: entry.cycle });
            }
            // 일정도 같은 OCR 텍스트에서 시도
            const ocrSched = parseScheduleFromOcr(ocrText);
            if (ocrSched['기준일'] || ocrSched['지급일']) {
              if (ocrSched['공시일']) sched['공시일'] = ocrSched['공시일'];
              if (ocrSched['분배락일']) sched['분배락일'] = ocrSched['분배락일'];
              if (ocrSched['기준일']) sched['기준일'] = ocrSched['기준일'];
              if (ocrSched['지급일']) sched['지급일'] = ocrSched['지급일'];
            }
          }
        }
      }

      // 텍스트 본문에서 일정 보강 (이미지 OCR 실패 대비)
      if (!sched['기준일']) {
        const text = html.replace(/<[^>]+>/g,' ').replace(/&nbsp;/g,' ').replace(/\s+/g,' ');
        const baseM = text.match(/지급기준일\s*[:：]?\s*\d{0,4}년?\s*(\d{1,2})월\s*(\d{1,2})일/);
        const payM = text.match(/지급(?:예정)?일\s*[:：]?\s*\d{0,4}년?\s*(\d{1,2})월\s*(\d{1,2})일/);
        if (baseM) sched['기준일'] = parseInt(baseM[1]) + '월 ' + parseInt(baseM[2]) + '일';
        if (payM) sched['지급일'] = parseInt(payM[1]) + '월 ' + parseInt(payM[2]) + '일';
      }

      if (usedOcr) sched['_ocr'] = true;
      dbg += ' outFinal=' + out.length;
      out.forEach(it => { it.sched = { ...sched }; });
      return { items: out, schedule: sched, usedOcr, dbg };
    };

    let items = [], schedule = {}, anyOcr = false, dbgs = [];
    // 회차별로 격리: 한쪽이 타임아웃/파싱 실패해도 나머지 회차는 살린다.
    if (midE) { try { const r = parsePlusNotice(midE); items = items.concat(r.items); if (Object.keys(r.schedule).length) schedule = r.schedule; if (r.usedOcr) anyOcr = true; dbgs.push('월중 ' + r.dbg); } catch(e) { dbgs.push('월중 EXC ' + e); } }
    if (endE) { try { const r = parsePlusNotice(endE); items = items.concat(r.items); if (!Object.keys(schedule).length && Object.keys(r.schedule).length) schedule = r.schedule; if (r.usedOcr) anyOcr = true; dbgs.push('월말 ' + r.dbg); } catch(e) { dbgs.push('월말 EXC ' + e); } }
    if (!items.length) return { items: [], error: 'PLUS: 종목 파싱 0건 || ' + dbgs.join(' || ') };
    return { success: true, items, schedule, title: 'PLUS 분배금 (자사 공지 파싱)' + (anyOcr ? ' [OCR]' : ''), _usedOcr: anyOcr };
  } catch(e) {
    return { items: [], error: 'PLUS: ' + e.toString() };
  }
}

function fetchDist_rise() {
  try {
    const listHtml = UrlFetchApp.fetch('https://www.riseetf.co.kr/cust/notice?searchText=%EB%B6%84%EB%B0%B0%EA%B8%88&searchType4=tab', { muteHttpExceptions: true }).getContentText('UTF-8');
    const liBlocks = listHtml.split('<li').slice(1);
    const cands = [];
    for (const block of liBlocks) {
      const idM = block.match(/href="(\/cust\/notice\/(\d+))/);
      const titleM = block.match(/class="body01">([\s\S]*?)<\/p>/);
      const dateM = block.match(/(\d{4})[.\-](\d{2})[.\-](\d{2})/);
      if (idM && titleM) {
        const title = titleM[1].replace(/<[^>]+>/g,'').trim();
        if (title.includes('분배금')) {
          // 제목으로 월중/월말 구분: "6월 말"=월말, "중순"=월중, "초"=비정기(제외)
          let cycle = null;
          const monM = title.match(/(\d{1,2})월/);
          if (/말/.test(title)) cycle = '월말';
          else if (/중순|중/.test(title)) cycle = '월중';
          cands.push({ id: parseInt(idM[2]), title, url: idM[1], cycle,
            mon: monM ? parseInt(monM[1]) : 0,
            pubMon: dateM ? parseInt(dateM[2]) : 0, pubDay: dateM ? parseInt(dateM[3]) : 0 });
        }
      }
    }
    cands.sort((a,b) => b.id - a.id);
    // 최신 월의 월중/월말 각 1건
    const dated = cands.filter(c => c.cycle && c.mon);
    if (!dated.length) return { items: [], error: 'RISE: 월중/월말 공지 미발견' };
    // 회차별 최신 글 1건씩 (목록 최신순) — 새 월중이 떠도 지난 월말 일정 유지
    const midE = dated.find(c => c.cycle === '월중');
    const endE = dated.find(c => c.cycle === '월말');

    const parseRiseNotice = (entry) => {
      const html = UrlFetchApp.fetch('https://www.riseetf.co.kr' + entry.url, { muteHttpExceptions: true }).getContentText('UTF-8');
      const text = html.replace(/<[^>]+>/g,' ').replace(/&nbsp;/g,' ').replace(/\s+/g,' ');
      // 일정: 텍스트로 명시 ("지급기준일 : 2026년 6월 30일")
      const sched = {};
      if (entry.pubMon) sched['공시일'] = entry.pubMon + '월 ' + entry.pubDay + '일';
      const baseM = text.match(/지급기준일\s*[:：]\s*\d{4}년\s*(\d{1,2})월\s*(\d{1,2})일/);
      const payM = text.match(/지급예정일\s*[:：]\s*\d{4}년\s*(\d{1,2})월\s*(\d{1,2})일/);
      if (baseM) sched['기준일'] = parseInt(baseM[1]) + '월 ' + parseInt(baseM[2]) + '일';
      if (payM) sched['지급일'] = parseInt(payM[1]) + '월 ' + parseInt(payM[2]) + '일';
      // 표: [종목명, 코드, 좌당예상분배금, 좌당과세분배금, 분배율]
      const out = [];
      const trRe = /<tr[^>]*>([\s\S]*?)<\/tr>/g;
      let tr;
      while ((tr = trRe.exec(html))) {
        const cols = [...tr[1].matchAll(/<td[^>]*>([\s\S]*?)<\/td>/g)].map(x => x[1].replace(/<[^>]+>/g,'').replace(/&nbsp;/g,' ').replace(/,/g,'').trim());
        const ti = cols.findIndex(c => /^[0-9A-Z]{6}$/.test(c));
        if (ti < 0) continue;
        const ticker = cols[ti];
        const name = (cols[ti-1] || cols[0] || '').replace(/\s*ETF\s*$/,'').trim();
        const after = cols.slice(ti+1).filter(c => /^[\d.]+$/.test(c)).map(Number);
        if (!after.length) continue;
        const amount = after[0];          // 좌당 예상분배금
        const rate = after[after.length-1]; // 분배율(마지막)
        if (amount == null) continue;
        out.push({ name, ticker, amount: Number(amount), rate, cycle: entry.cycle, sched: { ...sched } });
      }
      return { items: out, schedule: sched };
    };

    let items = [], schedule = {};
    if (midE) { const r = parseRiseNotice(midE); items = items.concat(r.items); if (Object.keys(r.schedule).length) schedule = r.schedule; }
    if (endE) { const r = parseRiseNotice(endE); items = items.concat(r.items); if (!Object.keys(schedule).length && Object.keys(r.schedule).length) schedule = r.schedule; }
    if (!items.length) return { items: [], error: 'RISE: 표 파싱 0건' };
    return { success: true, items, schedule, title: 'RISE 분배금 (자사 공지 파싱)' };
  } catch(e) {
    return { items: [], error: 'RISE: ' + e.toString() };
  }
}

// SOL 공지 목록 [{title, date:'yyyy-MM-dd HH:mm', logNo}] — 네이버 블로그 RSS 우선, 실패 시 홈페이지 공지 API
// logNo는 블로그 글 번호. 본문 파싱(_solParsePost)에 쓴다.
function _solNotices() {
  try {
    const xml = UrlFetchApp.fetch('https://rss.blog.naver.com/soletf.xml', { muteHttpExceptions:true, headers:{'User-Agent':'Mozilla/5.0'} });
    if (xml.getResponseCode() === 200) {
      const out = [];
      String(xml.getContentText('UTF-8')).split('<item>').slice(1).forEach(chunk => {
        const t = chunk.match(/<title>\s*(?:<!\[CDATA\[)?([\s\S]*?)(?:\]\]>)?\s*<\/title>/);
        const d = chunk.match(/<pubDate>\s*([\s\S]*?)\s*<\/pubDate>/);
        const l = chunk.match(/blog\.naver\.com\/soletf\/(\d{6,})/);
        if (t && d) out.push({ title: t[1], date: Utilities.formatDate(new Date(d[1]), 'Asia/Seoul', 'yyyy-MM-dd HH:mm'), logNo: l ? l[1] : '' });
      });
      if (out.length) return out;
    }
  } catch(e) {}
  try {
    const res = UrlFetchApp.fetch('https://www.soletf.com/api/cs/notice?pageNo=1&pageSize=20', { muteHttpExceptions:true, headers:{'User-Agent':'Mozilla/5.0','Accept':'application/json'} });
    if (res.getResponseCode() === 200) {
      return (JSON.parse(res.getContentText('UTF-8')).items||[]).map(it => {
        const l = String(it.CONTENT||'').match(/blog\.naver\.com\/soletf\/(\d{6,})/);
        return { title: String(it.TITLE||''), date: String(it.REG_DATE||''), logNo: l ? l[1] : '' };
      });
    }
  } catch(e) {}
  return [];
}

// 블로그 이름 → 종목코드. 표시용이라 없는 종목은 빈칸으로 두면 된다.
const SOL_TICKER = {
  'SOL 팔란티어커버드콜OTM채권혼합':'0040Y0', 'SOL 팔란티어미국채커버드콜혼합':'0040X0',
  'SOL 금융지주플러스고배당':'484880', 'SOL 미국30년국채커버드콜(합성)':'473330',
  'SOL 미국배당다우존스':'446720', 'SOL 미국배당다우존스2호':'493420'
};

// 블로그 분배금 공지 1건 파싱 → {sched, items:[{name,amount,rate}]}
// 본문이 HTML 표(텍스트)라 OCR 불필요. 일정·종목별 분배금·분배율이 전부 여기 들어있다.
function _solParsePost(logNo) {
  const url = 'https://blog.naver.com/PostView.naver?blogId=soletf&logNo=' + logNo + '&redirect=Dlog&widgetTypeCall=true';
  const res = UrlFetchApp.fetch(url, { muteHttpExceptions:true, headers:{'User-Agent':'Mozilla/5.0'} });
  if (res.getResponseCode() !== 200) return null;
  const html = res.getContentText('UTF-8');
  const bi = html.indexOf('se-main-container'); // 본문 컨테이너
  if (bi < 0) return null;
  const txt = html.slice(bi).replace(/<[^>]+>/g, ' ')
    .replace(/&nbsp;/g, ' ').replace(/&amp;/g, '&').replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&quot;/g, '"').replace(/&#39;/g, "'")
    .replace(/[\s​]+/g, ' ');
  const md = re => { const m = txt.match(re); return m ? (parseInt(m[1]) + '월 ' + parseInt(m[2]) + '일') : ''; };
  const sched = {};
  const set = (k, v) => { if (v) sched[k] = v; };
  set('공시일',   md(/분배금\s*공시일\s*:?\s*(?:\d+\s*년)?\s*(\d{1,2})월\s*(\d{1,2})일/));
  set('분배락일', md(/분배락\s*(\d{1,2})월\s*(\d{1,2})일/));
  set('기준일',   md(/지급\s*기준일\s*(\d{1,2})월\s*(\d{1,2})일/));
  set('지급일',   md(/지급\s*예정일\s*(\d{1,2})월\s*(\d{1,2})일/));
  // '분배금 내역' 표: "1 SOL 코리아고배당 60 0.46" — 종목명에도 숫자가 들어가므로 끝의 분배율(소수)로 행을 끊는다
  const seg = txt.split(/분배금\s*내역/)[1] || '';
  const items = [];
  const re = /(?:^|\s)\d{1,2}\s+(SOL\s.+?)\s+([\d,]+)\s+(\d{1,3}\.\d{1,2})(?=\s|$)/g;
  let m;
  while ((m = re.exec(seg)) !== null) {
    items.push({ name: m[1].trim(), amount: Number(m[2].replace(/,/g, '')), rate: Number(m[3]) });
  }
  return { sched, items };
}

// SOL은 홈페이지 대신 네이버 블로그에 분배금 공지를 올린다(홈페이지는 늦거나 누락).
// 최신 월중(①)·월말(②) 공지 본문을 각각 파싱해 회차별 종목을 만든다.
function fetchDist_sol() {
  try {
    const notices = _solNotices().filter(n => n.logNo && /분배금\s*안내/.test(n.title));
    const latest = f => notices.filter(f).sort((a,b) => b.date.localeCompare(a.date))[0];
    const rounds = [
      { cycle:'월중', n: latest(n => /중순/.test(n.title)) },
      { cycle:'월말', n: latest(n => /②/.test(n.title) && !/중순/.test(n.title)) }
    ];
    const items = [];
    let schedule = {};
    rounds.forEach(r => {
      if (!r.n) return;
      const p = _solParsePost(r.n.logNo);
      if (!p || !p.items.length) return;
      p.items.forEach(it => items.push({
        name: it.name, ticker: SOL_TICKER[it.name] || '', amount: it.amount, rate: it.rate,
        cycle: r.cycle, sched: p.sched
      }));
      if (r.cycle === '월중' || !Object.keys(schedule).length) schedule = p.sched;
    });
    if (!items.length) return { items: [], error: 'SOL: 블로그 공지 파싱 0건' };
    return { success: true, items, schedule, title: 'SOL 월배당 분배금 (블로그 공지)', _source: 'blog' };
  } catch(e) {
    return { items: [], error: 'SOL: ' + e.toString() };
  }
}

// ===== 알림 엔진 =====
// 시트: '알림로그'(이력 누적), '_파서메타'(직전 상태 저장→변경 감지)
const ALERT_SHEET_ID = '1iNlOU1YBRyJ6redmVoLDE4q6VfnWqL22s32IQHdSKN8';

function _getOrCreateSheet(name, headers) {
  const ss = SpreadsheetApp.openById(ALERT_SHEET_ID);
  let sh = ss.getSheetByName(name);
  if (!sh) {
    sh = ss.insertSheet(name);
    if (headers) sh.appendRow(headers);
  }
  return sh;
}

// 운용사별 현재 파싱 상태의 "지문" 생성 (변경 감지용)
function _fingerprint(source, result) {
  const items = (result && result.items) || [];
  const sched = (result && result.schedule) || {};
  return {
    source: (result && result._source) || (source === 'sol' ? 'api' : 'page'),
    isOcr: !!(result && (result._usedOcr || (sched && sched._ocr))),
    itemCount: items.length,
    cycles: [...new Set(items.map(it => it.cycle).filter(Boolean))].sort().join(','),
    pubDate: sched['공시일'] || '',
    hasItems: items.length > 0,
    error: (result && result.error) || ''
  };
}

// 알림 1건 추가 (중복 방지: 같은 운용사+종류+메시지가 최근 있으면 skip)
function _addAlert(sheet, source, kind, message, level) {
  const now = Utilities.formatDate(new Date(), 'Asia/Seoul', 'yyyy-MM-dd HH:mm');
  // 최근 50행 내 동일 알림 중복 체크
  const last = sheet.getLastRow();
  if (last > 1) {
    const start = Math.max(2, last - 49);
    const rows = sheet.getRange(start, 1, last - start + 1, 5).getValues();
    for (const r of rows) {
      if (r[1] === source && r[2] === kind && r[3] === message && r[5] !== '확인') return; // 이미 있음
    }
  }
  sheet.appendRow([now, source, kind, message, level || '정보', '신규']);
}

// 6개 운용사 파싱 후 알림 감지·생성 (getDistribution 호출하며 비교)
function checkAndLogAlerts() {
  const logSheet = _getOrCreateSheet('알림로그', ['시각','운용사','종류','메시지','중요도','상태']);
  const metaSheet = _getOrCreateSheet('_파서메타', ['운용사','source','isOcr','itemCount','cycles','pubDate','updated']);

  // 직전 메타 로드
  const metaRows = metaSheet.getLastRow() > 1 ? metaSheet.getRange(2,1,metaSheet.getLastRow()-1,7).getValues() : [];
  const prevMeta = {};
  metaRows.forEach(r => { prevMeta[r[0]] = { source:r[1], isOcr:r[2]===true||r[2]==='TRUE'||r[2]===true, itemCount:r[3], cycles:r[4], pubDate:r[5] }; });

  const SRC_LABEL = { kodex:'KODEX', tiger:'TIGER', ace:'ACE', rise:'RISE', plus:'PLUS', sol:'SOL' };
  const newMeta = [];

  ['kodex','tiger','ace','rise','plus','sol'].forEach(source => {
    let result;
    try { result = getDistribution(source, true); } catch(e) { result = { items:[], error:e.toString() }; }
    const fp = _fingerprint(source, result);
    const label = SRC_LABEL[source];
    const prev = prevMeta[source];

    // 1) 파싱 경고
    if (!fp.hasItems) {
      _addAlert(logSheet, label, '파싱경고', `${label} 종목 0건 — 파싱 실패 또는 공지 없음`, '경고');
    } else if (fp.isOcr) {
      _addAlert(logSheet, label, '파싱경고', `${label} 이미지 OCR로 처리됨 — 정확도 확인 권장`, '정보');
    }

    if (prev) {
      // 2) 구조 변경 감지
      if (prev.source && prev.source !== fp.source) {
        _addAlert(logSheet, label, '구조변경', `${label} 데이터 출처 변경: ${prev.source} → ${fp.source} — 파서 수정 필요`, '중요');
      }
      if (prev.isOcr && !fp.isOcr) {
        _addAlert(logSheet, label, '구조변경', `${label} 이미지→텍스트 전환됨 — 페이지에 직접 작성 시작, 파서 점검 권장`, '중요');
      }
      if (!prev.isOcr && fp.isOcr) {
        _addAlert(logSheet, label, '구조변경', `${label} 텍스트→이미지 전환됨 — OCR로 처리 중`, '정보');
      }
      // 3) 신규 공지 (공시일이 직전과 다름)
      if (fp.pubDate && prev.pubDate && fp.pubDate !== prev.pubDate) {
        _addAlert(logSheet, label, '신규공지', `${label} 새 분배금 공지: 공시일 ${fp.pubDate} (${fp.cycles})`, '정보');
      }
    }
    newMeta.push([source, fp.source, fp.isOcr, fp.itemCount, fp.cycles, fp.pubDate, Utilities.formatDate(new Date(),'Asia/Seoul','yyyy-MM-dd HH:mm')]);
  });

  // 메타 갱신 (전체 덮어쓰기)
  if (metaSheet.getLastRow() > 1) metaSheet.getRange(2,1,metaSheet.getLastRow()-1,7).clearContent();
  if (newMeta.length) metaSheet.getRange(2,1,newMeta.length,7).setValues(newMeta);

  return { checked: 6, time: Utilities.formatDate(new Date(),'Asia/Seoul','yyyy-MM-dd HH:mm') };
}

// 화면에 줄 알림 목록 반환 (최근 N건, 미확인 우선)
function getAlerts(limit) {
  try {
    const sh = _getOrCreateSheet('알림로그', ['시각','운용사','종류','메시지','중요도','상태']);
    const last = sh.getLastRow();
    if (last <= 1) return { success: true, alerts: [] };
    const n = Math.min(limit || 30, last - 1);
    const rows = sh.getRange(last - n + 1, 1, n, 6).getValues();
    const alerts = rows.map((r, i) => ({
      row: last - n + 1 + i,
      time: r[0], source: r[1], kind: r[2], message: r[3], level: r[4], status: r[5]
    })).reverse(); // 최신 먼저
    return { success: true, alerts };
  } catch(e) {
    return { success: false, error: e.toString(), alerts: [] };
  }
}

// 알림 확인 처리 (상태→확인)
function markAlertRead(row) {
  try {
    const sh = _getOrCreateSheet('알림로그');
    if (row >= 2 && row <= sh.getLastRow()) sh.getRange(row, 6).setValue('확인');
    return { success: true };
  } catch(e) { return { success: false, error: e.toString() }; }
}

function parseScheduleFromText(text) {
  const schedule = {};
  const keyMap = [
    ['공시일',    /공시일|공지일/],
    ['분배락일',  /분배락/],
    ['기준일',    /기준일/],
    ['지급일',    /지급일|지급\(예정\)|지급예정/],
  ];
  keyMap.forEach(([label, re]) => {
    const idx = text.search(re);
    if (idx < 0) return;
    const sub = text.slice(idx, idx + 80);
    const datePatterns = [
      /\d{4}[.\-\/]\d{1,2}[.\-\/]\d{1,2}/,
      /\d{1,2}월\s*\d{1,2}일/,
      /\d{1,2}[\/]\d{1,2}/,
    ];
    for (const dp of datePatterns) {
      const m = sub.match(dp);
      if (m) { schedule[label] = m[0].trim(); break; }
    }
  });
  return schedule;
}

function getSheetData(force) {
  // '주식상황' 시트는 실시간 시세 수식 재계산 때문에 읽기에 최대 10초+ 걸림.
  // 프론트가 로드/탭전환/일괄적용마다 반복 호출하므로 3분 캐시 → 첫 호출 외에는 즉시 응답.
  // 단 동기화(force=1)는 방금 수정한 시트 값을 봐야 하므로 캐시를 건너뛴다.
  const cache = CacheService.getScriptCache();
  const hit = force ? null : cache.get('sheetData_v1');
  if (hit) return JSON.parse(hit);

  const sheet = SpreadsheetApp.openById('19UsD0Tz6YL2eDoLdocL0ify8NLbUYSHaOOV-jtDqNLU').getSheetByName('주식상황');
  const rows = sheet.getDataRange().getValues();
  const items = [];
  let account = '';
  for (let i = 2; i < rows.length; i++) {
    const r = rows[i];
    if (r[0]) account = r[0].toString().trim();
    const ticker = r[1] ? r[1].toString().trim() : '';
    if (!ticker) continue;
    const currentKrw = parseFloat(r[6]) || 0;
    const currentUsd = parseFloat(r[7]) || 0;
    const change = r[13] ? r[13].toString().replace('▼','-').replace('%','').trim() : '';
    const qty = parseFloat(r[3]) || 0;
    const avgKrw = parseFloat(r[4]) || 0;
    const avgUsd = parseFloat(r[5]) || 0;
    const currency = currentKrw ? 'KRW' : 'USD';
    items.push({
      account, ticker,
      name: r[2] ? r[2].toString().trim() : '',
      quantity: qty,
      avg_price: currency === 'KRW' ? avgKrw : avgUsd,
      current: currentKrw || currentUsd,
      currency,
      change: parseFloat(change) || 0
    });
  }
  const out = { success: true, items };
  try { cache.put('sheetData_v1', JSON.stringify(out), 180); } catch(e) {} // 100KB 초과 시 캐시 생략
  return out;
}

function getDivSheetData() {
  const sheet = SpreadsheetApp.openById('19UsD0Tz6YL2eDoLdocL0ify8NLbUYSHaOOV-jtDqNLU').getSheetByName('분배금');
  const rows = sheet.getDataRange().getValues();
  const items = [];
  let account = '', ticker = '', currency = 'KRW';
  for (let i = 3; i < rows.length; i++) {
    const r = rows[i];
    if (r[0] && r[0].toString().trim()) account = r[0].toString().trim();
    if (r[1] && r[1].toString().trim()) ticker = r[1].toString().trim();
    if (!ticker || !account) continue;
    currency = /^[A-Z]+$/.test(ticker) ? 'USD' : 'KRW';
    const item = { account, ticker, currency };
    for (let m = 1; m <= 12; m++) {
      const val = parseFloat(r[5 + m]) || 0;
      item['m' + m] = val;
    }
    if (Object.keys(item).some(k => k.startsWith('m') && item[k] > 0)) items.push(item);
  }
  return { success: true, items };
}

function getPriceLog() {
  const log = SpreadsheetApp.openById(SHEET_ID).getSheetByName('시세로그');
  if (!log) return { success: true, items: {} };
  const rows = log.getDataRange().getValues();
  const map = {};
  for (let i = 1; i < rows.length; i++) {
    let t = rows[i][1] ? rows[i][1].toString().trim().toUpperCase() : '';
    if (!t) continue;
    if (/^\d+$/.test(t) && t.length < 6) t = t.padStart(6, '0');
    (map[t] = map[t] || []).push({ d: rows[i][0].toString(), p: parseFloat(rows[i][2]) || 0 });
  }
  const items = {};
  Object.keys(map).forEach(k => {
    map[k].sort((a, b) => a.d < b.d ? -1 : 1);
    // 같은 날짜 중복행이 있으면 마지막 값만 유지 (기존 누적 중복 방어)
    const byDate = {};
    map[k].forEach(x => { byDate[x.d] = x.p; });
    const days = Object.keys(byDate).sort();
    items[k] = days.map(d => byDate[d]).slice(-30);
  });
  return { success: true, items };
}

function snapshotPrices() {
  const src = SpreadsheetApp.openById('19UsD0Tz6YL2eDoLdocL0ify8NLbUYSHaOOV-jtDqNLU').getSheetByName('주식상황');
  const rows = src.getDataRange().getValues();
  const ss = SpreadsheetApp.openById(SHEET_ID);
  let log = ss.getSheetByName('시세로그');
  if (!log) { log = ss.insertSheet('시세로그'); log.appendRow(['date','ticker','price']); }
  const today = Utilities.formatDate(new Date(), 'Asia/Seoul', 'yyyy-MM-dd');
  if (log.getDataRange().getValues().some(r => r[0] && r[0].toString() === today)) return;
  const out = [];
  const seen = {};                                    // 티커 중복 제거 (주식상황이 계좌별로 종목 중복 나열)
  for (let i = 2; i < rows.length; i++) {
    let t = rows[i][1] ? rows[i][1].toString().trim() : '';
    if (!t) continue;
    if (/^\d+$/.test(t) && t.length < 6) t = t.padStart(6, '0');
    if (seen[t]) continue;                            // 이미 기록한 티커면 건너뜀
    const price = (parseFloat(rows[i][6]) || 0) || (parseFloat(rows[i][7]) || 0);
    if (price) { out.push([today, t, price]); seen[t] = true; }
  }
  if (out.length) log.getRange(log.getLastRow() + 1, 1, out.length, 3).setValues(out);
}

// [주1회 트리거용] 시세로그 압축: 원본은 백업시트로 이관, 본시트는 다운샘플만 유지
//  - 30일 이내: 일별 원본 유지
//  - 30~90일: 주1회(월요일)만 유지
//  - 90일 이상: 월1회(1일)만 유지
function compactPriceLog() {
  const ss = SpreadsheetApp.openById(SHEET_ID);
  const log = ss.getSheetByName('시세로그');
  if (!log) return { success: false, msg: '시세로그 없음' };
  const rows = log.getDataRange().getValues();
  if (rows.length < 2) return { success: true, msg: '데이터 없음' };
  const header = rows[0];
  const body = rows.slice(1);

  const today = new Date();
  const dayMs = 86400000;
  const ageDays = (dStr) => {
    const d = new Date(dStr);
    if (isNaN(d)) return 0;
    return Math.floor((today - d) / dayMs);
  };

  // 1) 원본 전체를 백업시트에 이관(누적 append)
  let bak = ss.getSheetByName('시세로그_백업');
  if (!bak) { bak = ss.insertSheet('시세로그_백업'); bak.appendRow(header); }
  if (body.length) bak.getRange(bak.getLastRow() + 1, 1, body.length, header.length).setValues(body);

  // 2) 본시트는 규칙에 맞는 행만 남김 (날짜별 마지막값으로 dedup)
  const keep = {};   // key = ticker|date → row
  body.forEach(r => {
    const dStr = r[0] ? r[0].toString() : '';
    if (!dStr) return;
    const age = ageDays(dStr);
    const d = new Date(dStr);
    let ok = false;
    if (age <= 30) ok = true;                          // 30일 이내: 전부
    else if (age <= 90) ok = (d.getDay() === 1);       // 30~90일: 월요일만
    else ok = (d.getDate() === 1);                     // 90일↑: 매월 1일만
    if (!ok) return;
    let t = r[1] ? r[1].toString().trim().toUpperCase() : '';
    if (!t) return;
    if (/^\d+$/.test(t) && t.length < 6) t = t.padStart(6, '0');
    keep[t + '|' + dStr] = [dStr, t, r[2]];            // 같은 티커·날짜 마지막값 유지
  });
  const compact = Object.keys(keep).sort().map(k => keep[k]);

  // 3) 본시트 재작성
  log.clearContents();
  log.getRange(1, 1, 1, header.length).setValues([header]);
  if (compact.length) log.getRange(2, 1, compact.length, 3).setValues(compact);

  return { success: true, before: body.length, after: compact.length, backedUp: body.length };
}

// 시트 컬럼 순서 (rowToScreener와 반드시 일치). 첫 컬럼명에 스키마 버전을 넣어
// 로직 변경 시(예: aum 단위 수정) 당일 캐시를 무효화한다.
var SCREENER_HEADER = ['date_v4','ticker','name','provider','category','baseIndex','divRate','divPay','divPayDt','price','change','wk','mo','yld1y','expense','aum','deviation','buyInd','buyFor','buyOrg','listedDate','top5'];

function getEtfScreener() {
  const ss = SpreadsheetApp.openById(SHEET_ID);
  let sh = ss.getSheetByName('ETF스크리너');
  const today = Utilities.formatDate(new Date(), 'Asia/Seoul', 'yyyy-MM-dd');
  if (sh) {
    const rows = sh.getDataRange().getValues();
    // 헤더가 현재 스키마 버전이고 당일자면 캐시 사용 (+시세만 실시간 덮어쓰기)
    if (rows.length > 1 && rows[1][0] && rows[1][0].toString() === today &&
        rows[0].length >= SCREENER_HEADER.length && rows[0][0].toString() === SCREENER_HEADER[0]) {
      const cached = rows.slice(1).map(rowToScreener);
      try { overlayLivePrices(cached); } catch(e) {}
      return { success: true, date: today, items: cached };
    }
  }
  const items = [];
  try {
    const url = 'https://finance.naver.com/api/sise/etfItemList.nhn?etfType=0&targetColumn=market_sum&sortOrder=desc';
    const res = UrlFetchApp.fetch(url, { muteHttpExceptions: true, headers: { 'User-Agent': 'Mozilla/5.0', 'Referer': 'https://finance.naver.com/fund/etf/etfMain.naver' } });
    if (res.getResponseCode() !== 200) return staleOr(sh, 'ETF API 응답 오류: ' + res.getResponseCode());
    const json = JSON.parse(res.getContentText('EUC-KR'));
    const list = (json.result && json.result.etfItemList) || [];
    if (!list.length) return staleOr(sh, '결과 없음');
    list.forEach(r => {
      const name = r.itemname || '';
      if (!name.match(/커버드콜|고배당|배당|위클리|데일리|월배당|인컴|리츠/)) return;
      items.push({
        ticker:    r.itemcode || '',
        name,
        provider:  guessProvider(name),
        category:  etfCategory(name),   // enrichScreener에서 기초지수 기반으로 재계산
        baseIndex: '',                  // 실제 기초지수 (hover 표시)
        divRate:   null,                // 연 분배율
        divPay:    null,                // 최근 좌당 분배금(원) — Seibro/공지 조인
        divPayDt:  '',                  // 최근 분배금 기준일(yyyyMMdd)
        price:     parseFloat(r.nowVal) || 0,   // enrich에서 실시간 종가로 갱신
        change:    null,                // 등락률 (etfItemList는 장 마감 후 0 → integration 사용)
        wk:        null,                // 주간 수익
        mo:        null,                // 월간 수익
        yld1y:     null,                // 1년 수익
        expense:   null,                // 총보수
        aum:       Math.round(parseFloat(r.marketSum) || 0),  // 순자산(억) — marketSum이 이미 억 단위
        deviation: null,                // 괴리율
        buyInd:    null,                // 개인 순매수금액(원)
        buyFor:    null,                // 외인 순매수금액(원)
        buyOrg:    null,                // 기관 순매수금액(원)
        listedDate:'',                  // 상장일
        top5:      []                   // 구성종목 top5 [{n,w}]
      });
    });
  } catch(e) { return staleOr(sh, e.toString()); }
  if (!items.length) return staleOr(sh, '결과 없음');
  // 네이버 모바일 종목 API로 상세 보강 (분배율·보수·수익률·기초지수·상장일·구성종목·투자자별)
  try { enrichScreener(items); } catch(e) { /* 보강 실패해도 기본 리스트는 반환 */ }
  // 분배금공지 캐시(분배캐시 시트)에서 당월 좌당 분배금 조인
  try { applyDistAmounts(items); } catch(e) {}
  if (!sh) sh = ss.insertSheet('ETF스크리너');
  sh.clearContents();
  const out = [SCREENER_HEADER];
  items.forEach(it => out.push([today, it.ticker, it.name, it.provider, it.category, it.baseIndex,
    it.divRate, it.divPay, it.divPayDt, it.price, it.change, it.wk, it.mo, it.yld1y, it.expense, it.aum, it.deviation,
    it.buyInd, it.buyFor, it.buyOrg, it.listedDate, (it.top5 && it.top5.length) ? JSON.stringify(it.top5) : '']));
  sh.getRange(1, 1, out.length, SCREENER_HEADER.length).setValues(out);
  try { overlayLivePrices(items); } catch(e) {}   // 응답에는 조회 시점 시세 반영 (시트엔 종가 유지)
  return { success: true, date: today, items };
}

// 조회 시점 실시간 시세 덮어쓰기: etfItemList 1회 호출로 현재가·등락률·괴리율 갱신.
// 장 마감 후엔 etfItemList의 changeRate가 전부 0으로 초기화되므로,
// 0이 아닌 종목 비율로 장중 여부를 판별해 마감 후엔 캐시된 등락(전일 확정치)을 유지한다.
function overlayLivePrices(items) {
  const url = 'https://finance.naver.com/api/sise/etfItemList.nhn?etfType=0&targetColumn=market_sum&sortOrder=desc';
  const res = UrlFetchApp.fetch(url, { muteHttpExceptions: true, headers: { 'User-Agent': 'Mozilla/5.0', 'Referer': 'https://finance.naver.com/fund/etf/etfMain.naver' } });
  if (res.getResponseCode() !== 200) return;
  const list = (JSON.parse(res.getContentText('EUC-KR')).result || {}).etfItemList || [];
  if (!list.length) return;
  const map = {};
  let liveCnt = 0;
  list.forEach(r => { map[r.itemcode] = r; if (parseFloat(r.changeRate)) liveCnt++; });
  const isLive = liveCnt > list.length * 0.05;   // 5% 이상 등락이 있으면 장중으로 판단
  items.forEach(it => {
    const r = map[it.ticker];
    if (!r) return;
    const p = parseFloat(r.nowVal);
    if (p) it.price = p;
    if (isLive) it.change = parseFloat(r.changeRate) || 0;
    const nav = parseFloat(r.nav);
    if (p && nav > 0) it.deviation = Math.round((p - nav) / nav * 10000) / 100;
  });
}

// 콤마/부호 포함 문자열 → 숫자. "+431,438" → 431438, "15,480" → 15480
function scrNum(s) { if (s == null) return null; const n = parseFloat(String(s).replace(/[+,\s]/g, '')); return isNaN(n) ? null : n; }

// 최근 좌당 분배금(원) 채우기.
// 1순위: Seibro(예탁원) 분배금지급현황 — 전 운용사 커버 (KIWOOM·TIME·FOCUS 포함)
// 2순위: 운용사 공지 조인(getDistribution) — Seibro 실패/누락분 폴백
function applyDistAmounts(items) {
  let seibro = {};
  try { seibro = fetchSeibroDist(); } catch(e) {}
  items.forEach(it => {
    const s = seibro[it.ticker];
    if (s) { it.divPay = s.amount; it.divPayDt = s.dt; }
  });
  if (items.every(it => it.divPay != null)) return;
  // 폴백: 운용사 공지 (Seibro에 아직 안 뜬 이번 회차 공지 포함)
  const norm = s => (s || '').toString().toUpperCase().replace(/\s+/g, '');
  const byT = {}, byN = {};
  ['kodex','tiger','ace','rise','sol','plus'].forEach(src => {
    let its = [];
    try { const r = getDistribution(src, false); its = (r && r.items) || []; } catch(e) {}
    its.forEach(d => {
      if (d.amount == null) return;
      const t = (d.ticker || '').toString().trim();
      if (t && byT[t] == null) byT[t] = d.amount;
      const nk = norm(d.name);
      if (nk && byN[nk] == null) byN[nk] = d.amount;
    });
  });
  items.forEach(it => {
    if (it.divPay != null) return;
    const v = byT[it.ticker] != null ? byT[it.ticker] : byN[norm(it.name)];
    if (v != null) { const n = parseFloat(v); if (!isNaN(n)) it.divPay = n; }
  });
}

// Seibro 분배금지급현황(최근 95일)을 페이지(30행)씩 긁어 { 티커: {amount, dt} } 반환.
// 티커 = ISIN 4~9번째 자리. 같은 티커는 기준일 최신 것만. 청산분배 제외.
function fetchSeibroDist() {
  const now = new Date();
  const f = d => Utilities.formatDate(d, 'Asia/Seoul', 'yyyyMMdd');
  const from = f(new Date(now.getTime() - 95 * 24 * 3600 * 1000)), to = f(now);
  const map = {};
  for (let p = 1; p <= 25; p++) {
    const xml = '<reqParam action="exerInfoDtramtPayStatPlist" task="ksd.safe.bip.cnts.etf.process.EtfExerInfoPTask">'
      + '<etf_sort_cd value=""/><etf_big_sort_cd value=""/><isin value=""/><mngco_custno value=""/>'
      + '<RGT_RSN_DTAIL_SORT_CD value=""/><fromRGT_STD_DT value="' + from + '"/><toRGT_STD_DT value="' + to + '"/>'
      + '<START_PAGE value="' + ((p - 1) * 30 + 1) + '"/><END_PAGE value="' + (p * 30) + '"/>'
      + '<MENU_NO value="179"/><CMM_BTN_ABBR_NM value=""/><W2XPATH value="/IPORTAL/user/etf/BIP_CNTS06030V.xml"/></reqParam>';
    let res;
    try {
      res = UrlFetchApp.fetch('https://seibro.or.kr/websquare/engine/proworks/callServletService.jsp', {
        method: 'post', contentType: 'application/xml', payload: xml, muteHttpExceptions: true,
        headers: { 'User-Agent': 'Mozilla/5.0', 'Referer': 'https://seibro.or.kr/websquare/control.jsp',
                   'submissionid': 'submission_exerInfoDtramtPayStatPlist' }
      });
    } catch(e) { break; }
    if (res.getResponseCode() !== 200) break;
    const rows = res.getContentText('UTF-8').split('<result>').slice(1);
    rows.forEach(r => {
      const g = k => { const m = r.match(new RegExp('<' + k + ' value="([^"]*)"')); return m ? m[1] : ''; };
      if ((g('RGT_RSN_DTAIL_NM') || '').indexOf('청산') !== -1) return;   // 청산분배(상환금) 제외
      const isin = g('ISIN');
      if (!isin || isin.length < 9) return;
      const t = isin.substr(3, 6);
      const amt = parseFloat(g('ESTM_STDPRC'));
      if (!amt || isNaN(amt)) return;
      const dt = g('RGT_STD_DT');
      if (!map[t] || dt > map[t].dt) map[t] = { amount: Math.round(amt * 100) / 100, dt: dt };
    });
    if (rows.length < 30) break;
  }
  return map;
}

// 각 ETF를 네이버 모바일 종목 API 2종으로 보강. fetchAll 병렬(청크 45) — 하루 1회 빌드 시에만 실행.
//   etfAnalysis : 기초지수·상장일·보수·괴리율·분배율·주간/월간/1년수익·구성종목 top10
//   integration : 실시간 종가·등락·투자자별 순매수(개인/외인/기관)
function enrichScreener(items) {
  const CHUNK = 45;
  const opt = { muteHttpExceptions: true, headers: { 'User-Agent': 'Mozilla/5.0', 'Referer': 'https://m.stock.naver.com/' } };
  const fetchChunked = (buildUrl, handle) => {
    for (let i = 0; i < items.length; i += CHUNK) {
      const batch = items.slice(i, i + CHUNK);
      const reqs = batch.map(it => Object.assign({ url: buildUrl(it.ticker) }, opt));
      let resps;
      try { resps = UrlFetchApp.fetchAll(reqs); } catch(e) { continue; }
      resps.forEach((res, j) => {
        try {
          if (res.getResponseCode() !== 200) return;
          const body = res.getContentText('UTF-8');
          if (body.charAt(0) !== '{' && body.charAt(0) !== '[') return;  // HTML(상장폐지 등) 스킵
          handle(batch[j], JSON.parse(body));
        } catch(e) { /* 개별 실패 무시 */ }
      });
      if (i + CHUNK < items.length) Utilities.sleep(150);
    }
  };

  // 1) etfAnalysis
  fetchChunked(t => 'https://m.stock.naver.com/api/stock/' + t + '/etfAnalysis', (it, d) => {
    if (!d) return;
    if (d.dividend && d.dividend.dividendYieldTtm != null) it.divRate = parseFloat(d.dividend.dividendYieldTtm);
    if (d.totalFee != null)      it.expense   = parseFloat(d.totalFee);
    if (d.deviationRate != null) it.deviation = parseFloat(d.deviationRate);
    if (d.etfBaseIndex)          it.baseIndex = String(d.etfBaseIndex);
    if (d.issuerName) it.provider = d.issuerName.replace(/\(ETF\)\s*$/, '').replace(/자산운용.*$/, '').trim() || it.provider;
    if (d.listedDate) { const s = String(d.listedDate); it.listedDate = s.length === 8 ? s.slice(0,4)+'-'+s.slice(4,6)+'-'+s.slice(6,8) : s; }
    if (it.baseIndex) it.category = etfCategory(it.baseIndex + ' ' + it.name);  // 기초지수 기반 재분류
    (d.returnPerformanceList || []).forEach(p => {
      if (p.periodTypeCode === 'W1') it.wk    = parseFloat(p.value);
      if (p.periodTypeCode === 'M1') it.mo    = parseFloat(p.value);
      if (p.periodTypeCode === 'Y1') it.yld1y = parseFloat(p.value);
    });
    const top = (d.etfTop10MajorConstituentAssets || []).slice(0, 5).map(h => {
      const w = scrNum(h.etfWeight);
      return { n: h.itemName, w: w == null ? null : w };
    }).filter(x => x.n);
    if (top.length) it.top5 = top;   // 배열로 유지 (시트 저장 시에만 stringify)
  });

  // 2) integration — 실시간 종가·등락·투자자별 (+etfAnalysis 실패 종목 폴백)
  fetchChunked(t => 'https://m.stock.naver.com/api/stock/' + t + '/integration', (it, d) => {
    // etfAnalysis가 없는 종목은 etfKeyIndicator로 지표 폴백
    const k = (d && d.etfKeyIndicator) || {};
    if (it.divRate   == null && k.dividendYieldTtm != null) it.divRate   = parseFloat(k.dividendYieldTtm);
    if (it.expense   == null && k.totalFee         != null) it.expense   = parseFloat(k.totalFee);
    if (it.yld1y     == null && k.returnRate1y     != null) it.yld1y     = parseFloat(k.returnRate1y);
    if (it.mo        == null && k.returnRate1m     != null) it.mo        = parseFloat(k.returnRate1m);
    if (it.deviation == null && k.deviationRate    != null) it.deviation = parseFloat(k.deviationRate);
    const arr = d && d.dealTrendInfos;
    if (!arr || !arr.length) return;
    const x = arr[0];
    const close = scrNum(x.closePrice);
    const chg   = scrNum(x.compareToPreviousClosePrice);
    if (close) {
      it.price = close;
      if (chg != null && (close - chg) !== 0) it.change = Math.round(chg / (close - chg) * 10000) / 100;
      const ind = scrNum(x.individualPureBuyQuant), fo = scrNum(x.foreignerPureBuyQuant), or = scrNum(x.organPureBuyQuant);
      if (ind != null) it.buyInd = Math.round(ind * close);
      if (fo  != null) it.buyFor = Math.round(fo  * close);
      if (or  != null) it.buyOrg = Math.round(or  * close);
    }
  });
}

// 국내 ETF 기준가(NAV) 맵: { 티커(6자리): nav } — 괴리율 계산용. etfItemList 1회 호출, 1h 캐시.
function getNavMap() {
  const cache = CacheService.getScriptCache();
  const cached = cache.get('navmap');
  if (cached) return JSON.parse(cached);
  try {
    const url = 'https://finance.naver.com/api/sise/etfItemList.nhn?etfType=0&targetColumn=market_sum&sortOrder=desc';
    const res = UrlFetchApp.fetch(url, { muteHttpExceptions: true, headers: { 'User-Agent': 'Mozilla/5.0', 'Referer': 'https://finance.naver.com/fund/etf/etfMain.naver' } });
    if (res.getResponseCode() !== 200) return { success: false, error: 'ETF API 응답 오류: ' + res.getResponseCode(), navs: {} };
    const json = JSON.parse(res.getContentText('EUC-KR'));
    const list = (json.result && json.result.etfItemList) || [];
    const navs = {};
    list.forEach(r => { const nav = parseFloat(r.nav); if (r.itemcode && nav > 0) navs[r.itemcode] = nav; });
    const result = { success: true, navs };
    if (Object.keys(navs).length) cache.put('navmap', JSON.stringify(result), 3600);
    return result;
  } catch(e) { return { success: false, error: e.toString(), navs: {} }; }
}

function guessProvider(name) {
  if (/^KODEX/.test(name)) return '삼성';
  if (/^TIGER/.test(name)) return '미래에셋';
  if (/^ACE/.test(name))   return '한투';
  if (/^RISE/.test(name))  return 'KB';
  if (/^SOL/.test(name))   return '신한';
  if (/^PLUS/.test(name))  return '한화';
  if (/^KBSTAR/.test(name))return 'KB';
  if (/^HANARO/.test(name))return 'NH';
  return '';
}
function staleOr(sh, err) {
  if (sh) { const rows = sh.getDataRange().getValues(); if (rows.length > 1) return { success: true, stale: true, date: rows[1][0], items: rows.slice(1).map(rowToScreener) }; }
  return { success: false, error: err, items: [] };
}
function rowToScreener(r) {
  const num = v => (v === '' || v == null) ? null : (isNaN(parseFloat(v)) ? null : parseFloat(v));
  let top5 = [];
  try { if (r[21]) top5 = JSON.parse(r[21]); } catch(e) {}
  return {
    ticker: r[1], name: r[2], provider: r[3], category: r[4], baseIndex: r[5] || '',
    divRate: num(r[6]), divPay: num(r[7]), divPayDt: (r[8] || '').toString(),
    price: parseFloat(r[9]) || 0, change: num(r[10]),
    wk: num(r[11]), mo: num(r[12]), yld1y: num(r[13]),
    expense: num(r[14]), aum: parseFloat(r[15]) || 0, deviation: num(r[16]),
    buyInd: num(r[17]), buyFor: num(r[18]), buyOrg: num(r[19]),
    listedDate: r[20] || '', top5: top5
  };
}
function etfCategory(n) {
  if (/레버리지|선물단일종목|2X/.test(n)) return '레버리지/단일';
  if (/CD금리|KOFR|SOFR|머니마켓|CD1년|단기채|단기변동금리|초단기|MMF/.test(n)) return '금리/현금';
  if (/혼합|밸런스|TRF|멀티에셋|목표헤지/.test(n)) return '혼합/자산배분';
  if (/국채|국고채/.test(n)) return '채권-국채';
  if (/회사채|크레딧|투자등급|하이일드|금융채/.test(n)) return '채권-회사채';
  if (/리츠|부동산|오피스|인프라/.test(n)) return '리츠/부동산';
  if (/국제금|금커버드콜|골드|천연가스/.test(n)) return '원자재';
  if (/차이나|중국|항셍/.test(n)) return '중국';
  if (/미국배당|배당퀄리티|배당귀족|배당킹|캐시카우|미국고배당|배당100|배당증가/.test(n)) return '미국배당';
  if (/나스닥|테크100|미국테크|빅테크|AI테크|AI빅테크/.test(n)) return '나스닥/테크';
  if (/S&P500|미국500|미국S&P/.test(n)) return 'S&P500';
  if (/밸류업/.test(n)) return '코리아밸류업';
  if (/고배당|배당성장|주주환원|배당주|은행|금융지주|K고배당|코리아고배당/.test(n)) return '한국고배당';
  if (/200|코스피/.test(n)) return '코스피200';
  if (/반도체|AI|엔비디아|팔란티어|테슬라/.test(n)) return '테마';
  return '기타';
}

function testDistribution() {
  ['kodex','ace','rise','sol','tiger','plus'].forEach(s => {
    const r = getDistribution(s, true);
    console.log('[' + s + '] items:' + (r.items?r.items.length:0) + ' err:' + (r.error||''));
    if (r.items && r.items[0]) console.log('  샘플:', JSON.stringify(r.items[0]));
    if (r.schedule) console.log('  일정:', JSON.stringify(r.schedule));
  });
}

function debugSmartToday() {
  ['kodex','tiger','ace','rise','plus','sol'].forEach(src => {
    const d = getDistribution(src, true);
    console.log('[' + src + '] 종목:' + (d.items||[]).length + ' | 일정:' + JSON.stringify(d.schedule||{}));
  });
}

// ── 수익로그 ──────────────────────────────
function snapshotPortfolio() {
  const day = new Date().getDay();
  if (day === 0 || day === 6) return;
  const ss = SpreadsheetApp.openById(SHEET_ID);
  let log = ss.getSheetByName('수익로그');
  if (!log) { log = ss.insertSheet('수익로그'); log.appendRow(['date','account_name','value','slot']); }
  // 하루 3회(10·13·16시) 스냅샷. 슬롯은 실제 실행 시각(KST)으로 판정하며 16시가 그날 확정값.
  const hour = parseInt(Utilities.formatDate(new Date(), 'Asia/Seoul', 'HH'), 10);
  const slot = hour < 12 ? 10 : hour < 15 ? 13 : 16;
  const today = Utilities.formatDate(new Date(), 'Asia/Seoul', 'yyyy-MM-dd');
  if (log.getRange(1, 4).getValue() !== 'slot') log.getRange(1, 4).setValue('slot');
  const existing = log.getDataRange().getValues();
  // 같은 날짜의 '같은 슬롯'만 교체 → 다른 슬롯 값은 보존.
  // slot이 빈 과거 행(구 1회/일 트리거)은 확정값으로 보고 16으로 간주한다.
  for (let i = existing.length - 1; i >= 1; i--) {
    if (!existing[i][0] || existing[i][0].toString() !== today) continue;
    const rowSlot = (existing[i][3] === '' || existing[i][3] == null) ? 16 : Number(existing[i][3]);
    if (rowSlot === slot) log.deleteRow(i + 1);
  }
  const src = SpreadsheetApp.openById('19UsD0Tz6YL2eDoLdocL0ify8NLbUYSHaOOV-jtDqNLU').getSheetByName('주식상황');
  const srcRows = src.getDataRange().getValues();
  const priceMap = {};
  for (let i = 2; i < srcRows.length; i++) {
    const t = (srcRows[i][1] || '').toString().trim().toUpperCase();
    if (!t) continue;
    const price = parseFloat(srcRows[i][6]) || parseFloat(srcRows[i][7]) || 0;
    if (price) priceMap[t] = price;
  }
  const er = fetchExchangeRate() || 1450;
  const accounts = getAccounts();
  const allHoldings = getHoldings();
  const out = [];
  accounts.forEach(acc => {
    const h = allHoldings.filter(x => x.account_id === acc.id);
    if (!h.length) return;
    let value = 0;
    h.forEach(x => {
      const qty = parseFloat(x.quantity) || 0;
      const avg = parseFloat(x.avg_price) || 0;
      const cur = priceMap[(x.ticker||'').toString().toUpperCase()] || avg;
      value += x.currency === 'USD' ? cur * qty * er : cur * qty;
    });
    if (value > 0) out.push([today, acc.name, Math.round(value), slot]);
  });
  if (out.length) log.getRange(log.getLastRow() + 1, 1, out.length, 4).setValues(out);
}

// 수익로그 스냅샷 트리거를 10·13·16시 3개로 재설정. 스크립트 편집기에서 1회만 실행하면 된다.
// setupPortfolioTriggers()는 파일 맨 아래 '수동 실행' 섹션으로 옮김.

function getPortfolioLog() {
  const ss = SpreadsheetApp.openById(SHEET_ID);
  const log = ss.getSheetByName('수익로그');
  if (!log) return { success: true, items: [] };
  const rows = log.getDataRange().getValues();
  if (rows.length <= 1) return { success: true, items: [] };
  const items = rows.slice(1).filter(r => r[0] && r[1]).map(r => ({
    date:         r[0].toString(),
    account_name: r[1].toString(),
    value:        parseFloat(r[2]) || 0,
    // slot 없는 과거 행은 확정값(16)으로 간주
    slot:         (r[3] === '' || r[3] == null) ? 16 : (parseInt(r[3]) || 16)
  }));
  return { success: true, items };
}

function importHistoricalData() {
  console.log('이미 실행 완료 (472행). 재실행 불필요.');
}
function testScreener() {
  const r = getEtfScreener();
  console.log('success:', r.success);
  console.log('items:', (r.items||[]).length);
  console.log('error:', r.error||'없음');
  if (r.items && r.items[0]) console.log('샘플:', JSON.stringify(r.items[0]));
}
function testEtfCheckApi() {
  const res = UrlFetchApp.fetch('https://www.etfcheck.co.kr/api/screener?type=basic',
    { muteHttpExceptions: true, headers: { 'User-Agent': 'Mozilla/5.0' } });
  console.log('status:', res.getResponseCode());
  console.log('body:', res.getContentText('UTF-8').slice(0, 300));
}

function testKrxEtf2() {
  const today = Utilities.formatDate(new Date(), 'Asia/Seoul', 'yyyyMMdd');
  const payload = 'bld=dbms/MDC/STAT/standard/MDCSTAT04301&trdDd=' + today + '&share=1&money=1&csvxls_isNo=false';
  const res = UrlFetchApp.fetch('https://data.krx.co.kr/comm/bldAttendant/getJsonData.cmd', {
    method: 'post',
    payload: payload,
    headers: {
      'User-Agent': 'Mozilla/5.0',
      'Referer': 'https://data.krx.co.kr/contents/MDC/MDI/mdiLoader/index.cmd?menuId=MDC020103010901',
      'Content-Type': 'application/x-www-form-urlencoded; charset=UTF-8',
      'Accept': 'application/json, text/javascript, */*',
      'X-Requested-With': 'XMLHttpRequest'
    },
    muteHttpExceptions: true
  });
  console.log('status:', res.getResponseCode());
  console.log('body:', res.getContentText('UTF-8').slice(0, 500));
}

function testEtfCheckJang() {
  const res = UrlFetchApp.fetch('https://www.etfcheck.co.kr/user/common/getJangGubun', {
    muteHttpExceptions: true,
    headers: { 'User-Agent': 'Mozilla/5.0', 'Referer': 'https://www.etfcheck.co.kr/' }
  });
  console.log('status:', res.getResponseCode());
  console.log('sample:', res.getContentText('UTF-8').slice(0, 200));
}
function ck(){
  const r=UrlFetchApp.fetch('https://www.samsungfund.com/api/v1/kodex/distribution.do?pageNo=1&pageSize=100',{muteHttpExceptions:true});
  const j=JSON.parse(r.getContentText('UTF-8'));
  console.log((j.dividList||[]).slice(0,5).map(x=>x.basicD+' '+x.fNm));
}

function ckSolCycle(){
  const cache=CacheService.getScriptCache(); cache.remove('dist2_sol');
  const r=getDistribution('sol',true);
  console.log('전체:', r.items.length);
  r.items.forEach(it=>console.log(`  ${it.ticker} ${it.name.slice(0,20)} | ${it.cycle} | 기준 ${it.sched?.기준일||'?'}`));
}

function debugSheetAcc(){
  const r = getSheetData().items;
  const s = {};
  r.forEach(i => s[i.account] = (s[i.account]||0) + 1);
  console.log(JSON.stringify(s, null, 2));
}

// ===================================================================
// ===== 수동 실행 (편집기에서 함수 골라 ▶실행. 재배포와 별개) =====
// 새로 만드는 "손으로 한 번 돌려야 하는" 함수는 전부 여기 아래에 둘 것.
// ===================================================================

// 분배금 공지 탐지 트리거: 매일 10·14·18시.
// 실행 대상 checkDistNotices()가 공지 몰리는 날(8~12, 23~27)만 통과시킨다.
// 18시는 SOL 게시가 17시 전후라 당일에 잡기 위한 슬롯.
function setupDistTriggers() {
  ScriptApp.getProjectTriggers()
    .filter(t => t.getHandlerFunction() === 'checkDistNotices')
    .forEach(t => ScriptApp.deleteTrigger(t));
  [10, 14, 18].forEach(h => ScriptApp.newTrigger('checkDistNotices').timeBased().atHour(h).nearMinute(5).everyDays(1).create());
  console.log('checkDistNotices 트리거 3개(10·14·18시) 재설정 완료');
}

// 수익로그 스냅샷 트리거: 매일 10·13·16시.
function setupPortfolioTriggers() {
  ScriptApp.getProjectTriggers()
    .filter(t => t.getHandlerFunction() === 'snapshotPortfolio')
    .forEach(t => ScriptApp.deleteTrigger(t));
  [10, 13, 16].forEach(h => {
    ScriptApp.newTrigger('snapshotPortfolio').timeBased().atHour(h).nearMinute(5).everyDays(1).create();
  });
  console.log('snapshotPortfolio 트리거 3개(10·13·16시) 재설정 완료');
}

// 공지 창 경계값 자체 점검. 통과하면 'ok' 반환, 틀리면 예외.
function _testNoticeWindow() {
  [8,10,12,23,25,27].forEach(d => { if (!_inNoticeWindow(d)) throw new Error('창 안인데 false: ' + d); });
  [1,7,13,22,28,31].forEach(d => { if (_inNoticeWindow(d)) throw new Error('창 밖인데 true: ' + d); });
  console.log('ok');
  return 'ok';
}
