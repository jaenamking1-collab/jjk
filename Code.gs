const SHEET_ID = '1iNlOU1YBRyJ6redmVoLDE4q6VfnWqL22s32IQHdSKN8';

// ── 접근 토큰 ─────────────────────────────
// Script 속성 'APP_TOKEN'에 값을 넣으면, 아래 PUBLIC_ACTIONS를 제외한 모든 요청은
// 올바른 token 파라미터가 있어야 통과한다(개인 계좌·보유·분배 데이터 보호).
// 속성이 비어 있으면(미설정) 전부 허용 → 재배포 전까지 기존 앱이 끊기지 않음(하위호환).
// getDistribution·getEtfNotices는 공개 분배금 페이지(프록시)가 쓰므로 토큰 없이 허용.
const PUBLIC_ACTIONS = ['getDistribution', 'getDistributionAll', 'getEtfNotices', 'getEtfNoticesAll', 'hitCounter'];
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
      case 'getBootstrap':    result = getBootstrap(e.parameter.year); break;
      case 'getAccounts':     result = getAccounts(); break;
      case 'getHoldings':     result = getHoldings(e.parameter.account_id); break;
      case 'getDividends':    result = getDividends(e.parameter.year, e.parameter.account_id); break;
      case 'getSavings':      result = getSavings(); break;
      case 'maturityAlertPreview': result = previewMaturityAlerts(); break;
      case 'getExchangeRate': result = { rate: fetchExchangeRate() }; break;
      case 'getStockInfo':    result = getStockInfo(e.parameter.ticker, e.parameter.currency); break;
      case 'getStockList':    result = getStockList(); break;
      case 'getStockPrice':   result = getStockPrice(e.parameter.ticker, e.parameter.currency); break;
      case 'getLivePrices':   result = getLivePrices(); break;
      case 'getStockHistory': result = getStockHistory(e.parameter.ticker, e.parameter.currency, e.parameter.days); break;
      case 'getEtfNotices':   result = getEtfNotices(e.parameter.source); break;
      case 'getEtfNoticesAll': result = getEtfNoticesAll(); break;
      // 공개 페이지 방문자 카운터: bump='1'이면 +1, 아니면 현재 값만 반환.
      // 누적(VISIT_COUNT)과 오늘(VISIT_TODAY, 날짜는 VISIT_TODAY_DATE로 판별해 자정에 리셋)을 함께 반환.
      case 'hitCounter': {
        const _p = PropertiesService.getScriptProperties();
        const _today = Utilities.formatDate(new Date(), 'Asia/Seoul', 'yyyy-MM-dd');
        let _n = parseInt(_p.getProperty('VISIT_COUNT') || '0', 10);
        let _t = parseInt(_p.getProperty('VISIT_TODAY') || '0', 10);
        if (_p.getProperty('VISIT_TODAY_DATE') !== _today) _t = 0; // 날짜 바뀌면 오늘 카운트 리셋
        if (e.parameter.bump === '1') {
          _n += 1; _t += 1;
          _p.setProperties({ VISIT_COUNT: String(_n), VISIT_TODAY: String(_t), VISIT_TODAY_DATE: _today });
        } else if (_p.getProperty('VISIT_TODAY_DATE') !== _today) {
          _p.setProperties({ VISIT_TODAY: '0', VISIT_TODAY_DATE: _today }); // 표시만 해도 리셋은 저장
        }
        result = { count: _n, today: _t };
        break;
      }
      case 'getSheetData':    result = getSheetData(e.parameter.force === '1'); break;
      case 'getPriceLog':     result = getPriceLog(); break;
      case 'getEtfScreener':  result = getEtfScreener(); break;
      case 'getNavMap':       result = getNavMap(); break;
      case 'getDistribution': result = getDistribution(e.parameter.source, e.parameter.force === '1'); break;
      case 'getDistributionAll': result = getDistributionAll(); break;
      case 'getDivSheetData': result = getDivSheetData(e.parameter.year); break;
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

// ── 부팅 통합 조회 ─────────────────────────
// 첫 로딩이 느렸던 이유: 요청 1건당 왕복 고정비가 1.5~2초인데 부팅에 9건을 쐈고,
// 그중 7건이 같은 스프레드시트를 열어 서로 막는 바람에 프론트의 Promise.all이 무력화됐다
// (실측: 시트 액션 4건 병렬 12.6초 / 순차 14.5초 — 병렬 이득 없음. 시트 안 쓰는 액션은 병렬 잘 됨).
// 여기서 한 실행 안에 필요한 탭을 모두 읽어 한 번에 돌려준다. getSheetData는 다른
// 스프레드시트라 경합이 없으므로 프론트가 이 호출과 동시에 따로 친다(9왕복 → 2왕복).
function getBootstrap(year) {
  const divsAll = getDividends();
  return {
    accounts:  getAccounts(),
    holdings:  getHoldings(),
    dividends: year ? divsAll.filter(d => d.year.toString() === year.toString()) : divsAll,
    divsAll:   divsAll,
    stockList: getStockList(),
    rate:      fetchExchangeRate(),
    alerts:    getAlerts(30)
  };
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
// 스프레드시트 핸들은 실행 1회당 한 번만 연다. openById는 호출마다 수백ms~초 단위라
// 탭을 여러 개 읽는 getBootstrap에서 그대로 누적됐다. 전역은 실행 끝나면 사라진다.
let _ss = null;
function getSheet(name) {
  if (!_ss) _ss = SpreadsheetApp.openById(SHEET_ID);
  return _ss.getSheetByName(name);
}
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
      // SOL은 홈페이지가 아니라 네이버 블로그에 분배금 공지를 올린다(홈페이지는 늦거나 누락).
      // 그래서 목록도 블로그에서 받고, 클릭하면 블로그 글로 이동시킨다.
      _solNotices().filter(n => /분배금/.test(n.title) && !/이벤트/.test(n.title)).slice(0, 5).forEach(n => items.push({
        title: n.title,
        date: String(n.date).slice(0, 10).replace(/-/g, '.'),
        url: n.logNo ? 'https://blog.naver.com/soletf/' + n.logNo : 'https://blog.naver.com/soletf'
      }));
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

// 6개사 공지를 한 실행에서 반환 — 스크립트캐시에 있는 것만 담고, 없는 곳은 stale:true로 표시만 한다.
// 왜: 프론트가 운용사별로 6번 치면 캐시 미스 1건이 10초씩 걸려 탭 진입이 11초가 됐다. 그렇다고 한 실행에서
// 6개사를 순차 스크랩하면 오히려 더 느리므로, stale인 곳만 프론트가 병렬로 개별 요청해 캐시를 채운다.
function getEtfNoticesAll() {
  let hits = {};
  try { hits = CacheService.getScriptCache().getAll(DIST_SOURCE_IDS.map(s => 'notices_' + s)) || {}; } catch(e) {}
  const sources = {};
  DIST_SOURCE_IDS.forEach(s => {
    let v = null;
    if (hits['notices_' + s]) { try { v = JSON.parse(hits['notices_' + s]); } catch(e) {} }
    sources[s] = (v && (v.items || []).length) ? v : { items: [], stale: true };
  });
  return { success: true, sources: sources };
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
// rows를 넘기면 시트를 다시 읽지 않는다(getDistributionAll이 6개사 몫을 1회 읽기로 처리).
function readDistCache(source, rows) {
  try {
    if (!rows) rows = _distCacheSheet().getDataRange().getValues();
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
function attachDistHistory(source, payload, rows) {
  try {
    if (!payload || !payload.items) return payload;
    const cur = currentCycleKey();
    if (!rows) rows = _distCacheSheet().getDataRange().getValues();
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

// 6개사 분배 데이터를 한 실행에서 한꺼번에 반환.
// 왜: getDistribution(source)는 캐시 히트여도 readDistCache + attachDistHistory가 각각
// 분배캐시 시트를 통째로 읽어(=호출당 2회), 프론트가 6개사를 동시에 치면 시트 전체 읽기가
// 12회 발생해 3~20초씩 걸렸다. 여기선 시트를 1회만 읽고 6개사를 메모리에서 처리한다.
// 캐시가 없거나 오래된 source는 stale:true로 표시만 한다 — 한 실행에서 6개사를 스크랩하면
// 실행시간 제한에 걸리므로, 프론트가 그 source만 기존 getDistribution으로 개별 재요청한다.
const DIST_SOURCE_IDS = ['kodex', 'tiger', 'ace', 'plus', 'rise', 'sol'];
function getDistributionAll() {
  let rows;
  try { rows = _distCacheSheet().getDataRange().getValues(); } catch(e) { rows = []; }
  const need = currentCycleKey();
  const ttlMs = distCacheTtlSec() * 1000;
  const sources = {};
  DIST_SOURCE_IDS.forEach(s => {
    // 신선도 판정은 getDistribution의 캐시 분기와 동일한 조건을 쓴다(동작 차이 방지).
    const sc = readDistCache(s, rows);
    let fresh = null;
    if (sc && sc.payload && (sc.payload.items || []).length && sc.cycleKey === need) {
      const saved = sc.savedAt instanceof Date
        ? sc.savedAt
        : new Date(String(sc.savedAt).replace(' ', 'T') + ':00+09:00');
      if (!isNaN(saved) && (Date.now() - saved.getTime()) < ttlMs) fresh = attachDistHistory(s, sc.payload, rows);
    }
    sources[s] = fresh || { items: [], stale: true };
  });
  return { success: true, sources: sources };
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

// 공지가 몰리는 날: 중순①은 10일 전후, 월말②는 25일부터 말일까지(28~29일 게시가 잦아 23일~월말 전체를 연다)
function _inNoticeWindow(day) { return (day >= 8 && day <= 12) || (day >= 23); }

// 트리거용(30분 간격). 공지 창 밖의 날·주간 09~18시 밖은 그냥 넘어가 쿼터를 아낀다.
// (OCR은 이미지 URL 캐시(ocrImageText)로 재사용되므로 폴링이 잦아도 Vision 호출은 새 이미지에만 나간다.)
function checkDistNotices() {
  const now = new Date();
  const day = Number(Utilities.formatDate(now, 'Asia/Seoul', 'd'));
  const hour = Number(Utilities.formatDate(now, 'Asia/Seoul', 'H'));
  if (!_inNoticeWindow(day)) return;
  if (hour < 9 || hour > 18) return;
  return checkAndLogAlerts();
}

// 트리거 설정은 파일 맨 아래 '수동 실행' 섹션의 setupDistTriggers() 참고.

// 트리거용. checkAndLogAlerts가 6개 운용사를 강제 갱신하면서 신규공지·구조변경까지 감지해 알림로그에 남긴다.
// 분배캐시(분배캐시 시트) 선갱신 전용 — 사용자가 앱을 열기 전에 6개사 데이터를 미리 채워둔다.
// ⚠️ 예전엔 첫 줄이 `return checkAndLogAlerts();`라 아래 루프가 한 번도 실행되지 않았다(선갱신이 사실상
// 안 돌고 있었음). 게다가 공지 감지·카톡은 checkDistNotices(30분·공지창·09~18시)가 이미 같은
// checkAndLogAlerts를 돌리므로 완전한 중복이었다. → 여기서는 캐시 선갱신만 한다.
function refreshAllDistributions() {
  // ⚠️ 6개사 force 스크랩이라 실행이 수 분 단위로 길다. Apps Script는 긴 실행이 슬롯을 물면
  // 그동안 앱 요청이 전부 큐에 쌓이므로(2026-08-05 keepWarm 93초 사례), 트리거가 잘못된 주기로
  // 걸려 있어도 낮에는 절대 안 돌도록 함수 안에서도 새벽만 통과시킨다(checkDistNotices와 같은 방식).
  const hour = parseInt(Utilities.formatDate(new Date(), 'Asia/Seoul', 'H'), 10);
  if (hour < 3 || hour > 6) return;
  DIST_SOURCE_IDS.forEach(s => {
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
    // 같은 이미지 URL은 재OCR하지 않고 캐시 재사용 → 폴링이 잦아도 Vision 호출은 '새 이미지'에만 나간다.
    const _oc = CacheService.getScriptCache();
    const _ock = 'ocr_' + Utilities.base64EncodeWebSafe(Utilities.computeDigest(Utilities.DigestAlgorithm.MD5, imgUrl));
    const _hit = _oc.get(_ock);
    if (_hit !== null) { _ocrDbg = 'cache hit len=' + _hit.length; return _hit; }
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
    _oc.put(_ock, text, 21600); // 6시간(CacheService 최대 TTL) 동안 같은 이미지 재사용
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
          // OCR 실제값으로 덮어쓰기. 단 공시일은 게시일(등록일)이 정확하므로 OCR로 안 덮는다(OCR 28→29 오독 방지).
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
      else if (/^\d[\d.]*\s*원?$/.test(c)) { const a = parseFloat(c); if (!isNaN(a) && amount == null) amount = a; } // 월말표 '83원' 형식 허용
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
        // 공시일은 게시일(등록일)이 정확하므로 OCR로 안 덮는다(OCR 오독 방지).
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
              // PLUS 월말 공지 이미지는 분배율 칸에 % 기호가 없다(헤더에만) → % 를 선택적으로.
              // 월중 이미지는 "1.27%"처럼 % 가 붙어 있어 그대로 매칭된다.
              const rateM = seg.match(/(\d+\.\d+)\s*%?/);
              const rate = rateM ? parseFloat(rateM[1]) : null;
              let amount = null;
              const amtM = seg.match(/(\d+)\s+\d+\.\d+\s*%?/);
              if (amtM) amount = parseInt(amtM[1]);
              if (amount == null) continue;
              // 종목명: 코드와 금액 사이 한글/영문 (기호 ●·. 제거)
              let name = seg.slice(0, amtM ? seg.indexOf(amtM[0]) : seg.length).replace(/[●•·.]/g,'').replace(/\s+/g,' ').trim();
              out.push({ name, ticker: codes[i].code, amount: Number(amount), rate, cycle: entry.cycle });
            }
            // 일정도 같은 OCR 텍스트에서 시도
            const ocrSched = parseScheduleFromOcr(ocrText);
            if (ocrSched['기준일'] || ocrSched['지급일']) {
              // 공시일은 게시일(등록일)이 정확하므로 OCR로 안 덮는다(OCR 오독 방지).
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
  'SOL 미국배당다우존스':'446720', 'SOL 미국배당다우존스2호':'493420',
  'SOL 미국배당다우존스(H)':'452360', 'SOL 코리아고배당':'0105E0',
  'SOL 미국배당미국채혼합50':'490490', 'SOL 미국S&P500미국채혼합50':'0080X0',
  'SOL CD금리&머니마켓액티브':'497880', 'SOL 국제금커버드콜액티브':'0022T0',
  'SOL 200타겟위클리커버드콜':'0167B0', 'SOL 코스피200채권혼합50':'0192S0',
  'SOL 미국500타겟데일리커버드콜액티브':'494210', 'SOL 미국S&P500':'433330',
  'SOL 미국S&P500엔화노출(H)':'499150', 'SOL 배당성향탑픽액티브':'0152E0'
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
  // '분배금 내역' 표: "1 SOL 코리아고배당 60 0.46" — 종목명에도 숫자가 들어가므로 행 끝을 다음 행 번호(또는 표 끝 ※)로 끊는다.
  // ⚠️ 2026-08-12 실측으로 확인된 두 가지 형식 차이 때문에 예전 정규식은 두 회차 다 실패했다.
  //   ① 월중 공지는 분배율 칸을 **통째로 비운다**(예전엔 '업데이트 예정'이라고 써 줬다) → 분배율을 필수로 두면 0건.
  //   ② 월말 공지는 분배율에 **%가 붙는다**('0.28%') → 소수만 받으면 0건.
  // 게다가 이름을 `.+?`로 열어둔 채 분배율을 필수로 두면, 매칭 실패 시 `.+?`가 페이지 끝까지 달아나
  // **종목명에 본문·댓글·스크립트가 다 들어간 쓰레기 1건**이 나왔다(실측 name 길이 28,810자).
  // → %를 먼저 떼고, 분배율은 선택으로, 이름은 길이·※로 묶고, 행 끝은 다음 행 번호나 ※로 확정한다.
  // 분배율이 없으면 rate=null(화면엔 '-'). SOL이 나중에 채우면 다음 파싱에서 자동 갱신된다.
  const seg = (txt.split(/분배금\s*내역/)[1] || '').replace(/(\d)\s*%/g, '$1');
  const items = [];
  const re = /(?:^|\s)\d{1,2}\s+(SOL\s[^※]{1,40}?)\s+([\d,]+)(?:\s+(\d{1,3}\.\d{1,2})|\s+업데이트\s*예정)?(?=\s+\d{1,2}\s+SOL\s|\s*※|\s*$)/g;
  let m;
  while ((m = re.exec(seg)) !== null) {
    items.push({ name: m[1].trim(), amount: Number(m[2].replace(/,/g, '')), rate: m[3] ? Number(m[3]) : null });
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

// 공시일 문자열에서 '월' 숫자만 추출 ('7월 28일'→7, '7/29'→7, '2026-07-28'→7). 못 찾으면 0.
function _pubMonth(g) {
  g = String(g || '');
  let m = g.match(/(\d{1,2})\s*월/); if (m) return parseInt(m[1], 10);
  m = g.match(/(\d{1,2})\/\d{1,2}/); if (m) return parseInt(m[1], 10);
  m = g.match(/\d{4}[.\-\/](\d{1,2})[.\-\/]\d{1,2}/); if (m) return parseInt(m[1], 10);
  return 0;
}

// 공시일 표기를 '7월 29일' 한글 형식으로 통일.
// TIGER처럼 '7/29' 숫자형은 시트에 저장하면 날짜값으로 자동변환돼, 다음 회차 비교(fp!==prev)가 매번
// 어긋나 중복 알림이 발송됨. 저장·비교 전에 이 형식으로 맞춰 왕복을 안정화한다. Date로 읽힌 값도 여기서 복원.
function _normPubDate(v) {
  if (!v) return '';
  if (v instanceof Date) return Utilities.formatDate(v, 'Asia/Seoul', 'M월 d일');
  const s = String(v).trim();
  let mo, day, m;
  if (m = s.match(/(\d{1,2})\s*월\s*(\d{1,2})/))                    { mo = m[1]; day = m[2]; } // 7월 28일
  else if (m = s.match(/(\d{4})[.\-\/](\d{1,2})[.\-\/](\d{1,2})/))  { mo = m[2]; day = m[3]; } // 2026-07-28
  else if (m = s.match(/(\d{1,2})[.\-\/](\d{1,2})/))               { mo = m[1]; day = m[2]; } // 7/29
  else return s;
  return parseInt(mo, 10) + '월 ' + parseInt(day, 10) + '일';
}

// 운용사별 현재 파싱 상태의 "지문" 생성 (변경 감지용)
// 공시일·회차는 최상위 schedule(월중/지난달로 stale할 수 있음) 대신 '현재 회차(월중/월말)+이번 달' 종목들의
// sched.공시일에서 뽑는다. → 월말엔 이번 달 월말 공시일만, 지난달 잔여·다른 회차는 자동으로 걸러진다.
function _fingerprint(source, result) {
  const items = (result && result.items) || [];
  const sched = (result && result.schedule) || {};
  const curCycle = currentCycleKey().slice(-1) === '말' ? '월말' : '월중';
  const curMonth = parseInt(Utilities.formatDate(new Date(), 'Asia/Seoul', 'M'), 10);
  const cnt = {};
  items.forEach(it => {
    if (it.cycle !== curCycle) return;
    const g = (it.sched && it.sched['공시일']) || '';
    if (g && _pubMonth(g) === curMonth) cnt[g] = (cnt[g] || 0) + 1;
  });
  let pubDate = '', best = 0;
  Object.keys(cnt).forEach(g => { if (cnt[g] > best) { best = cnt[g]; pubDate = g; } }); // 최빈 공시일
  return {
    source: (result && result._source) || (source === 'sol' ? 'api' : 'page'),
    isOcr: !!(result && (result._usedOcr || (sched && sched._ocr))),
    itemCount: items.length,
    cycles: pubDate ? curCycle : '',
    pubDate: _normPubDate(pubDate),
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
    // 6열까지 읽는다. 예전엔 5열만 읽어 r[5](상태)가 항상 undefined였고, 그래서
    // "확인 처리한 알림은 다시 알릴 수 있다"는 의도가 작동하지 않았다(항상 true로 통과).
    const rows = sheet.getRange(start, 1, last - start + 1, 6).getValues();
    for (const r of rows) {
      if (r[1] === source && r[2] === kind && r[3] === message && r[5] !== '확인') return false; // 이미 있음
    }
  }
  sheet.appendRow([now, source, kind, message, level || '정보', '신규']);
  return true; // 새로 추가됨 → 호출부에서 카톡 발송 판단에 사용
}

// 6개 운용사 파싱 후 알림 감지·생성 (getDistribution 호출하며 비교)
function checkAndLogAlerts() {
  const logSheet = _getOrCreateSheet('알림로그', ['시각','운용사','종류','메시지','중요도','상태']);
  const metaSheet = _getOrCreateSheet('_파서메타', ['운용사','source','isOcr','itemCount','cycles','pubDate','updated']);

  // 직전 메타 로드
  const metaRows = metaSheet.getLastRow() > 1 ? metaSheet.getRange(2,1,metaSheet.getLastRow()-1,7).getValues() : [];
  const prevMeta = {};
  metaRows.forEach(r => { prevMeta[r[0]] = { source:r[1], isOcr:r[2]===true||r[2]==='TRUE'||r[2]===true, itemCount:r[3], cycles:r[4], pubDate:_normPubDate(r[5]) }; });

  const SRC_LABEL = { kodex:'KODEX', tiger:'TIGER', ace:'ACE', rise:'RISE', plus:'PLUS', sol:'SOL' };
  const newMeta = [];
  const kakaoMsgs = [];  // 이번 실행에서 새로 감지된 공지 → 카톡 발송용

  ['kodex','tiger','ace','rise','plus','sol'].forEach(source => {
    let result;
    try { result = getDistribution(source, true); } catch(e) { result = { items:[], error:e.toString() }; }
    const fp = _fingerprint(source, result);
    const label = SRC_LABEL[source];
    const prev = prevMeta[source];

    // 1) 파싱 경고 — 실패·이상은 로그뿐 아니라 카톡으로도 즉시 알림(새로 감지된 것만: _addAlert가 true 반환할 때).
    if (!fp.hasItems) {
      if (_addAlert(logSheet, label, '파싱경고', `${label} 종목 0건 — 파싱 실패 또는 공지 없음`, '경고'))
        kakaoMsgs.push(`⚠️ ${label} 파싱 0건 (실패/공지없음)`);
    } else {
      // 자사 파서 사망 → 뉴스사이트 우회(result.fallback). 지금은 나오지만 뉴스에서도 빠지면 통째로 사라짐.
      if (result.fallback && _addAlert(logSheet, label, '파싱경고', `${label} 자사 파서 실패 — 뉴스사이트로 우회 중, 파서 점검 권장`, '경고'))
        kakaoMsgs.push(`⚠️ ${label} 자사 파서 실패(뉴스 우회)`);
      // 월중/월말 어디에도 분류 안 된 항목 → 달력·일정표에서 누락됨.
      const noCyc = (result.items || []).filter(it => !it.cycle || it.cycle === '?').length;
      if (noCyc && _addAlert(logSheet, label, '파싱경고', `${label} 회차 미상 ${noCyc}건 — 월중/월말 분류 안 됨, 달력 누락 가능`, '경고'))
        kakaoMsgs.push(`⚠️ ${label} 회차 미상 ${noCyc}건`);
      if (fp.isOcr) _addAlert(logSheet, label, '파싱경고', `${label} 이미지 OCR로 처리됨 — 정확도 확인 권장`, '정보');
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
      // 3) 신규 공지 (현재 회차 공시일이 직전과 다름 — 빈값→값 전환도 발송)
      if (fp.pubDate && fp.pubDate !== (prev.pubDate || '')) {
        _addAlert(logSheet, label, '신규공지', `${label} 새 분배금 공지: 공시일 ${fp.pubDate} (${fp.cycles})`, '정보');
        kakaoMsgs.push(`${label}: 공시일 ${fp.pubDate} (${fp.cycles})`);
      }
    }
    newMeta.push([source, fp.source, fp.isOcr, fp.itemCount, fp.cycles, fp.pubDate, Utilities.formatDate(new Date(),'Asia/Seoul','yyyy-MM-dd HH:mm')]);
  });

  // ⚠️ 순서가 중요하다. 예전엔 메타를 여기서 먼저 덮어쓰고 그 뒤에 알림을 보냈다. 그러면 발송이
  // 실패해도 메타는 이미 새 공시일로 갱신되므로, 다음 실행부터 fp.pubDate === prev.pubDate 가 되어
  // **그 공지는 영구히 다시 알림되지 않았다**(발송은 catch로 삼켜 console.log만 남으니 조용히 사라짐).
  // → 알림을 먼저 보내고, 한 채널이라도 접수됐을 때만 메타를 갱신한다. 둘 다 실패하면 메타를 그대로
  // 두어 다음 실행(30분 뒤)이 같은 공지를 다시 감지해 재시도한다.
  // 카톡·캘린더는 서로 독립 — 카톡이 씹혀도 구글 캘린더 알림이 백업으로 뜨도록 각각 try로 감싼다.
  let accepted = true;
  if (kakaoMsgs.length) {
    let okKakao = false, okCal = false;
    try { _notifyKakao(kakaoMsgs); okKakao = true; } catch(e) { console.log('_notifyKakao 오류', e); }
    try { _notifyCal(kakaoMsgs);   okCal   = true; } catch(e) { console.log('_notifyCal 오류', e); }
    accepted = okKakao || okCal;
    if (!accepted) console.log('알림 두 채널 모두 실패 — 메타를 갱신하지 않고 다음 실행에서 재시도');
  }

  // 메타 갱신 (전체 덮어쓰기)
  if (accepted) {
    if (metaSheet.getLastRow() > 1) metaSheet.getRange(2,1,metaSheet.getLastRow()-1,7).clearContent();
    if (newMeta.length) metaSheet.getRange(2,1,newMeta.length,7).setValues(newMeta);
  }

  return { checked: 6, notified: kakaoMsgs.length, accepted: accepted,
           time: Utilities.formatDate(new Date(),'Asia/Seoul','yyyy-MM-dd HH:mm') };
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

// ── 카카오톡 '나에게 보내기' 알림 ─────────────────────────────
// 새 분배 공지가 감지되면(checkAndLogAlerts) 나에게 카톡 memo를 보낸다.
// 야간(23~08시)엔 즉시 안 보내고 대기열(KAKAO_PENDING)에 쌓아, 아침 8시 트리거
// (flushKakaoPending)가 발송한다. 스크립트 속성 KAKAO_REST_KEY·KAKAO_REFRESH_TOKEN이
// 없으면 조용히 skip(감지·로그는 그대로 동작). 앱에서 클라이언트 시크릿을 '사용함'으로
// 켠 경우엔 KAKAO_CLIENT_SECRET 속성도 필요(끄면 불필요). 설치·설정은 맨 아래 '수동 실행' 참고.
const _KAKAO_SEND_FROM = 8;   // 발송 허용 시작 시각(포함)
const _KAKAO_SEND_TO   = 23;  // 발송 허용 끝 시각(미포함) → 23~08시는 대기

// 알림에 붙는 '앱 확인' 링크. exec URL은 '기존 배포 편집(새 버전)' 재배포로는 바뀌지 않으므로 하드코딩해도 안전.
// ⚠️ 지우지 말 것 — 2026-08-05 커밋 d9cb46b(keepWarm 자기호출 삭제)에서 이 선언이 keepWarm 코드와 함께
// 삭제됐는데 sendKakaoMemo·flushCalPending의 사용 2곳은 남아, 그날부터 8/12까지 **알림 발송이 전부 죽었다**
// (ReferenceError → 대기열에 7건 적재된 채 방치). 당시 WORKLOG에 'grep으로 확인'이라 적었지만 확인한 것은
// 사용처의 존재였고 선언의 생존이 아니었다. 참조를 지울 땐 선언이 여전히 필요한지 같이 볼 것.
const _EXEC_URL = 'https://script.google.com/macros/s/AKfycbwJS1Fd-sDCVKPLJEpEWZmPQEKAOR9pG7y-nPKZOYty65j3ArOmlDzNX2WFqiGNF_s/exec';

// 리프레시 토큰으로 액세스 토큰 발급. 회전된 리프레시 토큰이 오면 저장.
function _kakaoAccessToken() {
  const props = PropertiesService.getScriptProperties();
  const restKey = props.getProperty('KAKAO_REST_KEY');
  const refresh = props.getProperty('KAKAO_REFRESH_TOKEN');
  const secret  = props.getProperty('KAKAO_CLIENT_SECRET'); // 시크릿 '사용함'이면 필요(없으면 생략)
  if (!restKey || !refresh) throw new Error('KAKAO_REST_KEY/KAKAO_REFRESH_TOKEN 미설정');
  const payload = { grant_type: 'refresh_token', client_id: restKey, refresh_token: refresh };
  if (secret) payload.client_secret = secret;
  const res = UrlFetchApp.fetch('https://kauth.kakao.com/oauth/token', {
    method: 'post',
    payload: payload,
    muteHttpExceptions: true
  });
  const j = JSON.parse(res.getContentText());
  if (!j.access_token) throw new Error('카카오 토큰 갱신 실패: ' + res.getContentText());
  if (j.refresh_token) props.setProperty('KAKAO_REFRESH_TOKEN', j.refresh_token);
  return j.access_token;
}

// 나에게 카톡 텍스트 memo 발송 (최대 200자). 성공 시 true.
function sendKakaoMemo(text) {
  const token = _kakaoAccessToken();
  const template = {
    object_type: 'text',
    text: String(text).slice(0, 200),
    link: { web_url: _EXEC_URL, mobile_web_url: _EXEC_URL }
  };
  const res = UrlFetchApp.fetch('https://kapi.kakao.com/v2/api/talk/memo/default/send', {
    method: 'post',
    headers: { Authorization: 'Bearer ' + token },
    payload: { template_object: JSON.stringify(template) },
    muteHttpExceptions: true
  });
  const ok = res.getResponseCode() === 200;
  if (!ok) console.log('카카오 발송 실패', res.getResponseCode(), res.getContentText());
  return ok;
}

// 지금이 발송 허용 시간대(08~23시)인지
function _kakaoWithinWindow() {
  const h = parseInt(Utilities.formatDate(new Date(), 'Asia/Seoul', 'H'), 10);
  return h >= _KAKAO_SEND_FROM && h < _KAKAO_SEND_TO;
}

// 새 공지 메시지들을 대기열에 넣고, 발송 시간대면 즉시 flush(아니면 아침 8시 트리거가 발송).
function _notifyKakao(lines) {
  if (!lines || !lines.length) return;
  const props = PropertiesService.getScriptProperties();
  let queue = [];
  try { queue = JSON.parse(props.getProperty('KAKAO_PENDING') || '[]'); } catch(e) {}
  queue = queue.concat(lines);
  props.setProperty('KAKAO_PENDING', JSON.stringify(queue));
  if (_kakaoWithinWindow()) flushKakaoPending();
}

// 대기열을 한 통(최대 200자)으로 묶어 발송하고 비운다. 실패 시 대기열 유지(다음에 재시도).
// 반환값: 실제로 보냈으면 true (진단·호출부가 '갔는지'를 판별할 수 있게 한다. 대기열이 비어 보낼 게
// 없었으면 true — 실패가 아니다.)
function flushKakaoPending() {
  const props = PropertiesService.getScriptProperties();
  let queue = [];
  try { queue = JSON.parse(props.getProperty('KAKAO_PENDING') || '[]'); } catch(e) {}
  if (!queue.length) return true;
  const msg = ('📢 ETF 분배 알림\n' + queue.join('\n')).slice(0, 200);
  let ok = false;
  try { ok = sendKakaoMemo(msg); } catch(e) { console.log('flushKakaoPending', e); }
  if (ok) props.deleteProperty('KAKAO_PENDING');
  return ok;
}

// ── 구글 캘린더 알림 (카톡 백업) ─────────────────────────────
// 카톡이 씹혀도 놓치지 않게, 새 공지를 구글 기본 캘린더에 "지금 울리는" 일정으로 등록한다.
// 야간 대기 규칙은 카톡과 동일(_kakaoWithinWindow 재사용) — 밤 공지는 아침 8시 flushCalPending이 발송.
// CalendarApp은 스크립트 소유 구글 계정 안에서 도므로 카톡처럼 토큰·네트워크로 실패할 일이 거의 없다.
// 첫 실행 시 구글이 캘린더 접근 권한을 한 번 물어본다(허용 필요). 설치는 맨 아래 '수동 실행' 참고.
function _notifyCal(lines) {
  if (!lines || !lines.length) return;
  const props = PropertiesService.getScriptProperties();
  let queue = [];
  try { queue = JSON.parse(props.getProperty('CAL_PENDING') || '[]'); } catch(e) {}
  queue = queue.concat(lines);
  props.setProperty('CAL_PENDING', JSON.stringify(queue));
  if (_kakaoWithinWindow()) flushCalPending();
}

// 대기열을 구글 캘린더 일정 1건으로 만들고 비운다. 실패 시 대기열 유지(다음에 재시도).
// 반환값은 flushKakaoPending과 같은 규칙(보냈으면 true, 보낼 게 없으면 true).
// ⚠️ desc 계산을 try 안으로 옮겼다. 예전엔 try 밖이라 여기서 예외가 나면 함수 밖으로 튀어나가
// _notifyCal → checkAndLogAlerts 까지 올라갔다. 실제로 _EXEC_URL 이 미선언이던 8/5~8/12에
// 그 경로로 캘린더 알림이 통째로 죽었다.
function flushCalPending() {
  const props = PropertiesService.getScriptProperties();
  let queue = [];
  try { queue = JSON.parse(props.getProperty('CAL_PENDING') || '[]'); } catch(e) {}
  if (!queue.length) return true;
  try {
    const start = new Date();
    const end = new Date(start.getTime() + 10 * 60 * 1000); // 10분짜리
    const desc = queue.join('\n') + '\n\n앱 확인: ' + _EXEC_URL;
    const ev = CalendarApp.getDefaultCalendar()
      .createEvent('📢 새 분배 공지 — 앱 확인', start, end, { description: desc });
    ev.addPopupReminder(0); // 시작(=지금) 시각에 팝업 알림
    props.deleteProperty('CAL_PENDING');
    return true;
  } catch(e) { console.log('flushCalPending', e); return false; }
}

// ── 분배 알림 워치독 ──────────────────────────────────
// 왜: 2026-08-05~08-12에 알림이 전면 중단됐는데 아무도 몰랐다(_EXEC_URL 미선언 → 카톡·캘린더 동시 사망,
// 대기열에 7건이 8일간 갇힘). 근본 문제는 **'침묵'과 '정상'이 구별되지 않는다**는 것 —
//  ① 실패를 알리는 통로가 고장난 그 통로였고 ② _addAlert 중복억제로 경고는 한 번만 남고
//  ③ 그 기록은 앱 🔔 패널에서만 보였다. 그래서 둘을 구별하게 만든다:
//  · 이상이 있으면 **Gmail**로 보고 — 카톡·캘린더와 독립된 구글 서비스라 같이 죽지 않는다.
//  · 월요일엔 이상이 없어도 카톡으로 '정상' 신호를 보낸다 → 통로가 죽으면 사용자에게
//    침묵이 아니라 '월요일 신호가 안 왔다'로 드러난다.
// 설계: docs/superpowers/specs/2026-08-12-dist-alert-watchdog-design.md
// 트리거 설치는 맨 아래 '수동 실행'의 setupWatchdogTrigger() 참고. 이상을 고치지는 않고 보고만 한다.
const WD_QUEUE_STUCK_HOURS = 6;   // 대기열이 이 시간 넘게 안 비면 발송 사망으로 본다(발송창이 08~23시이므로 기회는 충분했다는 뜻)
const WD_RESEND_DAYS       = 3;   // 같은 이상은 이 간격으로만 재발송(매일 같은 메일이 쌓이지 않게)
const WD_WINDOW_GRACE_DAYS = 3;   // 공지창 시작 후 이 일수가 지나야 '지연'으로 판정(창 초반엔 아직 안 올라온 게 정상)

function distWatchdog() {
  const props = PropertiesService.getScriptProperties();
  const now = new Date();
  const problems = [];

  // ① 발송 전제 조건 — 이번 사고의 직접 원인부터 본다.
  let execOk = false;
  try { execOk = typeof _EXEC_URL === 'string' && /^https:/.test(_EXEC_URL); } catch(e) {}
  if (!execOk) problems.push('발송 링크 상수(_EXEC_URL)가 없거나 값이 이상하다 → 카톡·캘린더가 둘 다 죽는다');
  if (!props.getProperty('KAKAO_REST_KEY'))      problems.push('스크립트 속성 KAKAO_REST_KEY 없음');
  if (!props.getProperty('KAKAO_REFRESH_TOKEN')) problems.push('스크립트 속성 KAKAO_REFRESH_TOKEN 없음');
  try { CalendarApp.getDefaultCalendar().getName(); } catch(e) { problems.push('캘린더 접근 실패: ' + e); }

  // ② 대기열 적체 — 큐에 타임스탬프가 없으므로 '비어있지 않은 것을 처음 본 시각'을 따로 기록해 나이를 재다.
  const stuck = ['KAKAO_PENDING', 'CAL_PENDING'].filter(k => (props.getProperty(k) || '[]') !== '[]');
  if (!stuck.length) {
    props.deleteProperty('WD_QUEUE_SINCE');
  } else {
    const since = props.getProperty('WD_QUEUE_SINCE');
    if (!since) {
      props.setProperty('WD_QUEUE_SINCE', now.toISOString());
    } else {
      const hrs = (now.getTime() - new Date(since).getTime()) / 3600000;
      if (hrs >= WD_QUEUE_STUCK_HOURS)
        problems.push('대기열이 ' + Math.floor(hrs) + '시간째 안 비었다(' + stuck.join(', ') + ') → 발송 실패 중');
    }
  }

  // ③ 파서 0건·stale  ④ 공지 지연
  const day      = Number(Utilities.formatDate(now, 'Asia/Seoul', 'd'));
  const curMonth = Number(Utilities.formatDate(now, 'Asia/Seoul', 'M'));
  const winStart = day <= 20 ? 8 : 23;                    // 월중창 8일 시작 / 월말창 23일 시작
  const judgeDelay = _inNoticeWindow(day) && day >= winStart + WD_WINDOW_GRACE_DAYS;
  let all = null;
  try { all = getDistributionAll(); } catch(e) { problems.push('getDistributionAll 실패: ' + e); }
  const sources = (all && all.sources) || {};
  const lines = [];
  DIST_SOURCE_IDS.forEach(s => {
    const r = sources[s];
    if (!r) { problems.push(s + ' 응답 없음'); return; }
    if (r.stale) problems.push(s + ' 데이터가 stale(갱신 실패)');
    let fp;
    try { fp = _fingerprint(s, r); } catch(e) { problems.push(s + ' 지문 계산 실패: ' + e); return; }
    if (!fp.hasItems) problems.push(s + ' 종목 0건 — 파서 점검 필요');
    else if (judgeDelay && _pubMonth(fp.pubDate) !== curMonth)
      problems.push(s + ' 이번 달 공시일 없음(현재 ' + (fp.pubDate || '없음') + ') — 공지 지연 또는 파서 이상');
    lines.push(s + ' ' + fp.itemCount + '건 / 공시일 ' + (fp.pubDate || '-'));
  });

  // 월요일 하트비트 — 대기열을 거치지 않고 직접 보낸다(정상 신호가 분배 알림에 섞이지 않게).
  // 요일은 'u' 같은 포맷 문자에 의존하지 않고 날짜 문자열에서 직접 계산한다(0=일 … 1=월).
  const ymd = Utilities.formatDate(now, 'Asia/Seoul', 'yyyy-MM-dd');
  let heartbeat = '';
  if (new Date(ymd + 'T00:00:00Z').getUTCDay() === 1) {
    let ok = false;
    try { ok = sendKakaoMemo('✅ 분배 알림 정상 (' + ymd + ')\n' + lines.join('\n')); }
    catch(e) { console.log('하트비트 예외', e); }
    heartbeat = ok ? '성공' : '실패';
    if (!ok) problems.push('주간 정상신호 카톡 발송 실패 → 카톡 통로가 죽어 있다');
  }

  // 보고 — 이상이 있을 때만 메일. 같은 이상이면 WD_RESEND_DAYS 간격으로만.
  let mailed = false;
  if (problems.length) {
    const sig = problems.slice().sort().join('|');
    let last = {};
    try { last = JSON.parse(props.getProperty('WD_LAST') || '{}'); } catch(e) {}
    const days = last.ts ? (now.getTime() - new Date(last.ts).getTime()) / 86400000 : 999;
    if (last.sig !== sig || days >= WD_RESEND_DAYS) {
      const body = '분배 알림 파이프라인 점검에서 이상이 발견됐습니다.\n\n'
        + problems.map((p, i) => (i + 1) + ') ' + p).join('\n')
        + '\n\n[현재 파싱 상태]\n' + lines.join('\n')
        + (heartbeat ? '\n\n주간 정상신호(카톡): ' + heartbeat : '')
        + '\n\n확인 방법: Apps Script 편집기에서 _diagDistAlert() 로 상태를 보고,'
        + ' _diagSendNow() 로 실제 발송을 시험하세요.'
        + (execOk ? '\n앱: ' + _EXEC_URL : '');
      try {
        MailApp.sendEmail(Session.getEffectiveUser().getEmail(),
          '[분배알림 이상] ' + problems.length + '건 — ' + Utilities.formatDate(now, 'Asia/Seoul', 'MM-dd HH:mm'), body);
        props.setProperty('WD_LAST', JSON.stringify({ sig: sig, ts: now.toISOString() }));
        mailed = true;
      } catch(e) { console.log('워치독 메일 실패', e); }
    } else {
      console.log('같은 이상이 ' + WD_RESEND_DAYS + '일 내 이미 보고됨 — 메일 생략');
    }
  } else {
    props.deleteProperty('WD_LAST');
  }

  console.log('워치독: 이상 ' + problems.length + '건' + (mailed ? ' (메일 발송)' : '')
              + (heartbeat ? ' / 하트비트 ' + heartbeat : ''));
  problems.forEach(p => console.log('  ⚠ ' + p));
  lines.forEach(l => console.log('  · ' + l));
  return { problems: problems, mailed: mailed, heartbeat: heartbeat };
}

// 보유 종목 현재가·등락률을 네이버/야후에서 직접 받는다. { 티커: {current, prev, change} }
// 왜: 예전엔 프론트가 '주식상황' 시트(getSheetData)를 읽어 현재가를 얻었는데, 그 시트가 실시간 시세
// 수식이라 열 때마다 재계산돼 캐시 미스 시 10~50초가 걸렸다(2026-08-05 실측). 시세는 여기서 받고
// '주식상황'은 원래 용도인 동기화 전용으로 남긴다.
// 국내는 네이버 폴링 API가 콤마로 여러 종목을 한 번에 준다(ETF·일반주 모두) → 호출 1회.
// 해외는 야후 배치(v7 quote)가 401이라 종목별 호출을 UrlFetchApp.fetchAll로 한 실행에서 병렬 처리.
// 키는 holdings에 저장된 티커 원문 그대로 쓴다(프론트가 그 값으로 조회하므로 6자리 보정본을 키로 쓰면 안 맞는다).
function getLivePrices() {
  const cache = CacheService.getScriptCache();
  const hit = cache.get('liveprices_v1');
  if (hit) return JSON.parse(hit);

  const krw = {}, usd = [];   // krw: 조회코드 → 원문티커
  getHoldings().forEach(h => {
    const orig = (h.ticker || '').toString().replace(/^'/, '').trim().toUpperCase();
    if (!orig) return;
    if (String(h.currency).toUpperCase() === 'KRW') krw[padTicker(orig, 'KRW')] = orig;
    else if (usd.indexOf(orig) < 0) usd.push(orig);
  });

  const prices = {};
  const codes = Object.keys(krw);
  if (codes.length) {
    try {
      const res = UrlFetchApp.fetch('https://polling.finance.naver.com/api/realtime/domestic/stock/' + codes.join(','),
        { muteHttpExceptions: true });
      const num = v => parseFloat(String(v == null ? '' : v).replace(/,/g, '')) || 0;
      ((JSON.parse(res.getContentText()) || {}).datas || []).forEach(d => {
        const key = krw[String(d.itemCode || '').toUpperCase()];
        const cur = num(d.closePrice || d.tradePrice);
        if (!key || !cur) return;
        const diff = num(d.compareToPreviousClosePrice);
        prices[key] = { current: cur, prev: cur - diff > 0 ? cur - diff : 0, change: parseFloat(d.fluctuationsRatio) || 0 };
      });
    } catch(e) { console.log('getLivePrices KRW', e); }
  }
  if (usd.length) {
    try {
      const reqs = usd.map(t => ({
        url: 'https://query1.finance.yahoo.com/v8/finance/chart/' + encodeURIComponent(t) + '?interval=1d&range=1d',
        muteHttpExceptions: true, headers: { 'User-Agent': 'Mozilla/5.0' }
      }));
      UrlFetchApp.fetchAll(reqs).forEach((res, i) => {
        try {
          const meta = ((((JSON.parse(res.getContentText()) || {}).chart || {}).result || [])[0] || {}).meta;
          const cur = meta && parseFloat(meta.regularMarketPrice) || 0;
          if (!cur) return;
          const prev = (meta && parseFloat(meta.chartPreviousClose)) || 0;
          prices[usd[i]] = { current: cur, prev: prev, change: prev ? Math.round((cur - prev) / prev * 10000) / 100 : 0 };
        } catch(e) {}
      });
    } catch(e) { console.log('getLivePrices USD', e); }
  }

  const out = { success: true, prices: prices };
  if (Object.keys(prices).length) { try { cache.put('liveprices_v1', JSON.stringify(out), 60); } catch(e) {} }
  return out;
}

function getSheetData(force) {
  // '주식상황' 시트는 실시간 시세 수식 재계산 때문에 읽기가 매우 무겁고 편차가 크다
  // (2026-08-05 실측: 캐시 히트 1.7초 vs 미스 12·14·38·50초). 부팅·종목관리·엑셀 등 6곳에서
  // 호출하므로 캐시가 만료될 때마다 누군가 이 10~50초를 뒤집어쓴다 → TTL을 3분에서 10분으로 늘려
  // 그 확률을 1/3로 줄인다. 시세가 10분까지 지연되지만 배당 포트폴리오 추적엔 충분하고,
  // 동기화(force=1)는 캐시를 건너뛰므로 항상 최신 시트 값을 본다.
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
  try { cache.put('sheetData_v1', JSON.stringify(out), 600); } catch(e) {} // 10분. 100KB 초과 시 캐시 생략
  return out;
}

function getDivSheetData(year) {
  const ss = SpreadsheetApp.openById('19UsD0Tz6YL2eDoLdocL0ify8NLbUYSHaOOV-jtDqNLU');
  const curYear = String(new Date().getFullYear());
  const target = String(year || curYear);

  // 연도별 전용 탭(분배금2025 등)이 있으면 그걸 쓰고(1월=G열),
  // 없으면 기본 '분배금' 탭에서 해당 연도 블록을 찾는다.
  // 이 시트는 연도 블록이 좌→우로 나란히 있고(2026=G:R, 2025=X:AI …),
  // 각 블록 상단에 'YYYY년' 라벨이 그 블록의 1월 열에 붙어있다. 그 라벨 열을 1월로 잡는다.
  const tab = ss.getSheetByName('분배금' + target);
  const sheet = tab || ss.getSheetByName('분배금');
  const rows = sheet.getDataRange().getValues();

  let startCol = 6; // 기본: G열(0-based idx 6) = 1월
  if (!tab) {
    startCol = -1;
    for (let hr = 0; hr < 3 && startCol < 0; hr++) {
      const hrow = rows[hr] || [];
      for (let c = 0; c < hrow.length; c++) {
        if (String(hrow[c] || '').replace(/\s/g, '') === target + '년') { startCol = c; break; }
      }
    }
    if (startCol < 0) {
      if (target === curYear) startCol = 6;          // 당해년도인데 라벨 못 찾음 → 기본 위치
      else return { success: true, items: [] };       // 그 연도 블록이 시트에 없음
    }
  }

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
      item['m' + m] = parseFloat(r[startCol + m - 1]) || 0;   // 연도 블록의 1월부터 12칸
    }
    if (Object.keys(item).some(k => k.startsWith('m') && item[k] > 0)) items.push(item);
  }
  return { success: true, items };
}

// 분배금 탭의 월별 금액 칸에서 노란색으로 표시된 신규 입력칸을, 처음 노랗게 보인 날로부터
// YELLOW_CLEAR_DAYS일 지나면 배경을 지운다(월중/월말 색 헷갈림 방지). 올해뿐 아니라
// 2025·2024 등 모든 연도 블록을 대상으로 한다(묵은 노랑도 정리). 라벨·헤더·D열 배당구분 색은
// 월별 금액 칸 바깥이라 영향 없음. 추적 기록은 앱 DB의 '_노란셀추적' 탭.
// (트리거 설치는 맨 아래 '수동 실행'의 setupYellowClearTrigger(), 누적분 즉시정리는 clearAllYellowNow() 참고.)
const YELLOW_CLEAR_DAYS = 7;

// 분배금 탭에서 모든 연도 블록의 월별칸(각 블록 1월~12월) 컬럼 인덱스 목록.
// 연도블록 라벨('YYYY년')은 각 블록 1월 열에 붙어있음(getDivSheetData 규칙). 연도별 전용 탭이면 G:R 한 블록.
function _monthCols(isYearTab, values) {
  const cols = [];
  const push12 = s => { for (let m = 0; m < 12; m++) cols.push(s + m); };
  if (isYearTab) { push12(6); return cols; }            // 전용 탭: G(6)~R
  for (let hr = 0; hr < 3; hr++) {
    const hrow = values[hr] || [];
    for (let c = 0; c < hrow.length; c++) {
      if (/^\d{4}년$/.test(String(hrow[c] || '').replace(/\s/g, ''))) push12(c);
    }
  }
  if (!cols.length) push12(6);                          // 라벨 못 찾으면 올해 기본 위치
  return cols;
}

function clearOldYellowCells() {
  const ss = SpreadsheetApp.openById('19UsD0Tz6YL2eDoLdocL0ify8NLbUYSHaOOV-jtDqNLU');
  const tab = ss.getSheetByName('분배금' + new Date().getFullYear());
  const sheet = tab || ss.getSheetByName('분배금');
  if (!sheet) return;

  const range = sheet.getDataRange();
  const values = range.getValues();
  const bgs = range.getBackgrounds();
  const cols = _monthCols(!!tab, values);               // 모든 연도 블록의 월별칸

  // 추적 로드 (a1 → 최초발견 'yyyy-MM-dd')
  const trackSheet = _getOrCreateSheet('_노란셀추적', ['셀', '최초발견']);
  const trackRows = trackSheet.getLastRow() > 1
    ? trackSheet.getRange(2, 1, trackSheet.getLastRow() - 1, 2).getValues() : [];
  const seen = {};
  trackRows.forEach(r => { if (r[0]) seen[String(r[0])] = _ymd(r[1]); });

  const todayStr = Utilities.formatDate(new Date(), 'Asia/Seoul', 'yyyy-MM-dd');
  const nextSeen = {};   // 이번 실행 후 유지할 추적 (아직 노랗고 7일 안 지난 것)
  const toClear = [];    // 배경 지울 A1

  for (let i = 4; i < values.length; i++) {             // 5행(0-based 4)부터 = 데이터 (헤더·총계행 제외)
    const bgRow = bgs[i] || [];
    for (const c of cols) {
      if (c >= bgRow.length) continue;
      if (!(parseFloat(values[i][c]) > 0)) continue;    // 숫자 든 칸만
      if (!_isYellow(bgRow[c])) continue;               // 노랑 계열만
      const a1 = sheet.getRange(i + 1, c + 1).getA1Notation();
      const first = seen[a1] || todayStr;               // 처음 보면 오늘로 기록
      if (_daysBetween(first, todayStr) >= YELLOW_CLEAR_DAYS) toClear.push(a1);
      else nextSeen[a1] = first;
    }
  }

  toClear.forEach(a1 => sheet.getRange(a1).setBackground(null)); // 흰색(기본)으로 복원

  // 추적 시트 재작성 (현재 노랗고 안 지운 것만 유지 → 직접 지운 칸은 자동 정리)
  if (trackSheet.getLastRow() > 1) trackSheet.getRange(2, 1, trackSheet.getLastRow() - 1, 2).clearContent();
  const out = Object.keys(nextSeen).map(a1 => [a1, nextSeen[a1]]);
  if (out.length) trackSheet.getRange(2, 1, out.length, 2).setValues(out);
}

// ── 입력할 칸 노란색 표시 ─────────────────────────
// 이번에 입력해야 할 달의 빈 칸을 노랗게 칠해 어디 넣을지 바로 보이게 한다.
// 대상 달: 1~9일이면 지난 달(전월 월말분 입력 시기), 10일 이후면 이번 달.
// 대상 행: D열이 '월중'/'월말'인 월배당 행만 (반기·분기·무배당은 매달 안 들어와 제외).
// 이미 값이 있는 칸은 건드리지 않는다. 지난 달에 칠했다가 안 채운 표시는 매번 먼저 지워
// 표시가 한 달치만 남게 한다(빈 칸이라 clearOldYellowCells의 7일 규칙엔 안 걸림).
// (트리거 설치는 맨 아래 '수동 실행'의 setupInputMarkTrigger() 참고.)
const MARK_COLOR = '#ffff00';

// 분배금 탭에서 특정 연도 블록의 1월 컬럼 인덱스. 못 찾으면 -1.
function _yearBlockStart(isYearTab, values, year) {
  if (isYearTab) return 6;                               // 전용 탭: G(6)=1월
  for (let hr = 0; hr < 3; hr++) {
    const hrow = values[hr] || [];
    for (let c = 0; c < hrow.length; c++) {
      if (String(hrow[c] || '').replace(/\s/g, '') === year + '년') return c;
    }
  }
  return -1;
}

function markInputCells() {
  const now = new Date();
  const day = parseInt(Utilities.formatDate(now, 'Asia/Seoul', 'd'), 10);
  let ty = parseInt(Utilities.formatDate(now, 'Asia/Seoul', 'yyyy'), 10);
  let tm = parseInt(Utilities.formatDate(now, 'Asia/Seoul', 'M'), 10);
  if (day <= 9) { tm -= 1; if (tm === 0) { tm = 12; ty -= 1; } }   // 1~9일 = 전월 월말분 입력 시기

  const ss = SpreadsheetApp.openById('19UsD0Tz6YL2eDoLdocL0ify8NLbUYSHaOOV-jtDqNLU');
  const tab = ss.getSheetByName('분배금' + ty);
  const sheet = tab || ss.getSheetByName('분배금');
  if (!sheet) return;

  const range = sheet.getDataRange();
  const values = range.getValues();
  const bgs = range.getBackgrounds();

  const start = _yearBlockStart(!!tab, values, ty);
  if (start < 0) { console.log(ty + '년 블록 없음 — 표시 생략'); return; }
  const target = start + (tm - 1);

  // 1) 묵은 표시 제거: 모든 연도 블록 월별칸 중 '값 없는' 노랑칸 (= 안 채운 입력 표시)
  const cols = _monthCols(!!tab, values);
  let cleared = 0;
  for (let i = 4; i < values.length; i++) {
    const bgRow = bgs[i] || [];
    for (const c of cols) {
      if (c >= bgRow.length) continue;
      if (String(values[i][c] || '').trim() !== '') continue;
      if (!_isYellow(bgRow[c])) continue;
      sheet.getRange(i + 1, c + 1).setBackground(null); cleared++;
    }
  }

  // 2) 이번에 입력할 달의 빈 칸 칠하기
  let marked = 0;
  for (let i = 4; i < values.length; i++) {
    const cycle = String(values[i][3] || '').replace(/\s/g, '');   // D열 배당구분
    if (cycle !== '월중' && cycle !== '월말') continue;
    if (target >= (values[i] || []).length) continue;
    if (String(values[i][target] || '').trim() !== '') continue;   // 이미 입력됨
    sheet.getRange(i + 1, target + 1).setBackground(MARK_COLOR); marked++;
  }
  console.log(ty + '년 ' + tm + '월 칸 ' + marked + '개 표시 (묵은 표시 ' + cleared + '개 제거)');
}

// 'yyyy-MM-dd' 문자열 또는 Date → 'yyyy-MM-dd'
function _ymd(v) {
  if (v instanceof Date) return Utilities.formatDate(v, 'Asia/Seoul', 'yyyy-MM-dd');
  return String(v || '').trim().slice(0, 10);
}

// b - a (일수). 인자는 'yyyy-MM-dd'.
function _daysBetween(a, b) {
  const da = new Date(a + 'T00:00:00'), db = new Date(b + 'T00:00:00');
  return Math.round((db - da) / 86400000);
}

// 노랑 계열 배경인가 (#ffff00·#ffff66·#ffd966 등). 흰색·무색·타색 제외.
function _isYellow(hex) {
  const m = /^#?([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(String(hex || ''));
  if (!m) return false;
  const r = parseInt(m[1], 16), g = parseInt(m[2], 16), b = parseInt(m[3], 16);
  return r >= 200 && g >= 180 && b <= 160;
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
    // 헤더가 현재 스키마 버전이고 당일자면 캐시 사용 (+시세만 실시간 덮어쓰기).
    // 날짜 칸은 시트가 'yyyy-MM-dd' 문자열을 Date 값으로 자동 변환하는 일이 있어, 그대로 toString()하면
    // 'Wed Aug 05 2026 …'이 되어 today와 영원히 안 맞는다(=캐시 항상 미스, 매번 135종목 재수집 40~55초).
    // 그래서 Date/문자열 양쪽을 같은 형식으로 정규화해서 비교한다.
    const cachedDay = rows.length > 1
      ? (rows[1][0] instanceof Date ? Utilities.formatDate(rows[1][0], 'Asia/Seoul', 'yyyy-MM-dd') : String(rows[1][0] || '').trim())
      : '';
    if (rows.length > 1 && cachedDay === today &&
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

// 보유 국내 ETF 괴리율이 -1% 밑으로 이탈하면 카톡 알림(+앱 알림로그 기록).
// 크로싱 1회만 발송(-1% 밑 진입), -0.5% 이상 회복 시 재무장 → 다음 이탈 때 재알림.
// 히스테리시스(-1.0~-0.5% 유지)로 딱 -1% 근처 깜빡임 스팸 방지. 상태는 Script 속성 DEVIATION_STATE(JSON).
// 장중(평일 09~16시)에만 체크. 트리거 설치는 맨 아래 '수동 실행'의 setupDeviationTrigger() 참고.
const DEV_ALERT = -1.0;   // 이 밑으로 내려가면 알림
const DEV_REARM = -0.5;   // 이 위로 회복하면 재무장
function checkDeviationAlerts() {
  const now = new Date();
  const dow = parseInt(Utilities.formatDate(now, 'Asia/Seoul', 'u'), 10);  // 1(월)~7(일)
  const hour = parseInt(Utilities.formatDate(now, 'Asia/Seoul', 'H'), 10);
  if (dow > 5 || hour < 9 || hour >= 16) return;  // 평일 장중만

  // 보유 국내(KRW) 종목 티커 → 이름 (미국 종목은 NAV 없어 제외; 비ETF는 목록에 없어 자동 스킵)
  const held = {};
  getHoldings().forEach(h => {
    if (String(h.currency).toUpperCase() !== 'KRW') return;
    const t = (h.ticker || '').toString().replace(/^'/, '').trim().toUpperCase();
    if (t) held[t] = h.name || t;
  });
  if (!Object.keys(held).length) return;

  // etfItemList 1회 호출 → 티커별 현재가·NAV·괴리율
  let list;
  try {
    const res = UrlFetchApp.fetch('https://finance.naver.com/api/sise/etfItemList.nhn?etfType=0&targetColumn=market_sum&sortOrder=desc',
      { muteHttpExceptions: true, headers: { 'User-Agent': 'Mozilla/5.0', 'Referer': 'https://finance.naver.com/fund/etf/etfMain.naver' } });
    if (res.getResponseCode() !== 200) return;
    list = ((JSON.parse(res.getContentText('EUC-KR')).result) || {}).etfItemList || [];
  } catch(e) { console.log('checkDeviationAlerts fetch', e); return; }
  if (!list.length) return;
  const liveCnt = list.filter(r => parseFloat(r.changeRate)).length;
  if (liveCnt <= list.length * 0.05) return;  // 등락 거의 0 = 휴장 → 스테일 데이터로 알림 안 함

  const props = PropertiesService.getScriptProperties();
  let state = {};
  try { state = JSON.parse(props.getProperty('DEVIATION_STATE') || '{}'); } catch(e) {}

  const lines = [];
  list.forEach(r => {
    const t = String(r.itemcode || '').toUpperCase();
    if (!held[t]) return;
    const p = parseFloat(r.nowVal), nav = parseFloat(r.nav);
    if (!(p > 0 && nav > 0)) return;
    const dev = Math.round((p - nav) / nav * 10000) / 100;   // 괴리율 %
    const prev = state[t] || 'above';
    if (prev !== 'below' && dev < DEV_ALERT) {
      state[t] = 'below';
      lines.push(`${held[t]}(${t}): ${dev > 0 ? '+' : ''}${dev.toFixed(2)}%`);
    } else if (prev === 'below' && dev >= DEV_REARM) {
      state[t] = 'above';   // 재무장(알림 없음)
    }
  });
  props.setProperty('DEVIATION_STATE', JSON.stringify(state));
  if (!lines.length) return;

  // 장중이라 카톡 발송시간대(08~23) 안 → 즉시 발송. 실패해도 알림로그엔 남긴다.
  try { sendKakaoMemo(('📉 괴리율 -1% 이탈\n' + lines.join('\n')).slice(0, 200)); } catch(e) { console.log('deviation kakao', e); }
  try {
    const logSheet = _getOrCreateSheet('알림로그', ['시각','운용사','종류','메시지','중요도','상태']);
    lines.forEach(ln => _addAlert(logSheet, '괴리율', '괴리율', ln, '정보'));
  } catch(e) { console.log('deviation log', e); }
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

function testScreener() {
  const r = getEtfScreener();
  console.log('success:', r.success);
  console.log('items:', (r.items||[]).length);
  console.log('error:', r.error||'없음');
  if (r.items && r.items[0]) console.log('샘플:', JSON.stringify(r.items[0]));
}
// ===================================================================
// ===== 수동 실행 (편집기에서 함수 골라 ▶실행. 재배포와 별개) =====
// 새로 만드는 "손으로 한 번 돌려야 하는" 함수는 전부 여기 아래에 둘 것.
// ===================================================================

// 분배금 공지 탐지 트리거: 30분 간격.
// 실행 대상 checkDistNotices()가 공지 몰리는 날(8~12, 23~월말)·주간 09~18시만 통과시켜 쿼터를 아낀다.
// OCR은 이미지 URL 캐시로 재사용되므로 30분 폴링이어도 Vision 호출은 새 이미지에만 나간다.
function setupDistTriggers() {
  ScriptApp.getProjectTriggers()
    .filter(t => t.getHandlerFunction() === 'checkDistNotices')
    .forEach(t => ScriptApp.deleteTrigger(t));
  ScriptApp.newTrigger('checkDistNotices').timeBased().everyMinutes(30).create();
  console.log('checkDistNotices 트리거 30분 간격 재설정 완료(창 날·09~18시만 통과)');
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

// 시세로그 압축 트리거: 매주 일요일 새벽 4시. compactPriceLog가 30일 이내만 일별 유지,
// 그 이전은 다운샘플하고 원본은 시세로그_백업으로 이관 → 시세로그가 무한정 커지지 않게 유지.
function setupCompactTrigger() {
  ScriptApp.getProjectTriggers()
    .filter(t => t.getHandlerFunction() === 'compactPriceLog')
    .forEach(t => ScriptApp.deleteTrigger(t));
  ScriptApp.newTrigger('compactPriceLog').timeBased().onWeekDay(ScriptApp.WeekDay.SUNDAY).atHour(4).create();
  console.log('compactPriceLog 트리거(매주 일요일 4시) 설치 완료');
}

// 노란셀 자동삭제 트리거: 매일 06시. clearOldYellowCells가 분배금 탭 모든 연도 블록 월별칸의
// 노란색 신규표시를 처음 본 날로부터 7일(YELLOW_CLEAR_DAYS) 뒤 지운다. 편집기에서 이 함수 1회 실행(▶)으로 설치.
function setupYellowClearTrigger() {
  ScriptApp.getProjectTriggers()
    .filter(t => t.getHandlerFunction() === 'clearOldYellowCells')
    .forEach(t => ScriptApp.deleteTrigger(t));
  ScriptApp.newTrigger('clearOldYellowCells').timeBased().atHour(6).nearMinute(0).everyDays(1).create();
  console.log('clearOldYellowCells 트리거(매일 06시) 설치 완료');
}

// 입력칸 표시 트리거: 매일 06시. markInputCells가 이번에 입력할 달(1~9일=지난달, 이후=이번달)의
// 월중·월말 행 빈 칸을 노랗게 칠한다. 편집기에서 이 함수 1회 실행(▶)으로 설치.
function setupInputMarkTrigger() {
  ScriptApp.getProjectTriggers()
    .filter(t => t.getHandlerFunction() === 'markInputCells')
    .forEach(t => ScriptApp.deleteTrigger(t));
  ScriptApp.newTrigger('markInputCells').timeBased().atHour(6).nearMinute(0).everyDays(1).create();
  console.log('markInputCells 트리거(매일 06시) 설치 완료');
}

// 분배캐시 선갱신 트리거: 매일 새벽 5시 1회. 6개사를 force로 다시 긁어 캐시를 채우므로 실행이 길다
// → 사용자가 안 쓰는 시간대에만 돈다. 기존 refreshAllDistributions 트리거는 전부 지우고 다시 만든다
// (수동으로 만들다가 2개가 중복 설치돼 있었고, 시간 제한이 없어 밤낮없이 6개사 스크랩을 돌리고 있었다).
// 편집기에서 이 함수 1회 실행(▶)으로 설치.
function setupRefreshTrigger() {
  ScriptApp.getProjectTriggers()
    .filter(t => t.getHandlerFunction() === 'refreshAllDistributions')
    .forEach(t => ScriptApp.deleteTrigger(t));
  ScriptApp.newTrigger('refreshAllDistributions').timeBased().atHour(5).nearMinute(0).everyDays(1).create();
  console.log('refreshAllDistributions 트리거 재설정 완료 — 중복분 삭제 후 매일 05시 1회');
}

// 괴리율 알림 트리거: 30분마다(함수 내부에서 평일 09~16시 장중만 실제 체크). 편집기에서 이 함수 1회 실행(▶)으로 설치.
function setupDeviationTrigger() {
  ScriptApp.getProjectTriggers()
    .filter(t => t.getHandlerFunction() === 'checkDeviationAlerts')
    .forEach(t => ScriptApp.deleteTrigger(t));
  ScriptApp.newTrigger('checkDeviationAlerts').timeBased().everyMinutes(30).create();
  console.log('checkDeviationAlerts 트리거(30분마다, 장중만 체크) 설치 완료');
}

// [수동 1회] 분배금 탭의 누적 노란색(모든 연도 블록의 월별칸)을 지금 즉시 전부 지운다.
// 기능 도입 시점의 묵은 표시(2025년 등) 정리용. 이후 신규분은 clearOldYellowCells(7일)가 관리.
function clearAllYellowNow() {
  const ss = SpreadsheetApp.openById('19UsD0Tz6YL2eDoLdocL0ify8NLbUYSHaOOV-jtDqNLU');
  const tab = ss.getSheetByName('분배금' + new Date().getFullYear());
  const sheet = tab || ss.getSheetByName('분배금');
  if (!sheet) return;
  const range = sheet.getDataRange();
  const values = range.getValues();
  const bgs = range.getBackgrounds();
  const cols = _monthCols(!!tab, values);
  let n = 0;
  for (let i = 4; i < values.length; i++) {
    const bgRow = bgs[i] || [];
    for (const c of cols) {
      if (c >= bgRow.length) continue;
      if (!(parseFloat(values[i][c]) > 0)) continue;
      if (!_isYellow(bgRow[c])) continue;
      sheet.getRange(i + 1, c + 1).setBackground(null); n++;
    }
  }
  const trackSheet = _getOrCreateSheet('_노란셀추적', ['셀', '최초발견']); // 추적 초기화
  if (trackSheet.getLastRow() > 1) trackSheet.getRange(2, 1, trackSheet.getLastRow() - 1, 2).clearContent();
  console.log('노란색 ' + n + '칸 즉시 삭제 완료');
}

// ── 서버 keep-warm ─────────────────────────────
// Apps Script는 한동안 요청이 없으면 컨테이너가 잠들어, 다시 열 때 첫 호출이 20~30초(cold start) 걸린다.
// 5분마다 이 트리거가 도는 것만으로 컨테이너와 스프레드시트 핸들이 깨어 있게 유지된다.
//
// ⚠️ 예전엔 `UrlFetchApp.fetch(자기 웹앱 URL)`로 자기를 호출했는데, 바깥 실행이 안쪽 doGet을
// 기다리는 동안 실행 슬롯을 계속 물고 있어서 **한 번에 93.7초**까지 걸렸다(2026-08-05 실행 기록 실측).
// 그동안 들어온 앱 요청은 전부 큐에서 대기 → 같은 시각 서버쪽 doGet은 5.1초인데 클라이언트는 41.4초.
// "메뉴 들어갈 때마다 30초"의 정체가 이것이었다. 콜드스타트는 오래 안 쓰다 열 때 1회지만
// 이건 5분마다였으므로, 자기호출을 없애고 가벼운 워밍만 남긴다.
function keepWarm() {
  try { SpreadsheetApp.openById(SHEET_ID).getName(); } catch (e) {}
}
// 워치독 트리거: 매일 08:30. 편집기에서 이 함수를 한 번만 실행(▶)하면 설치된다.
// 08:10의 flushKakaoPending·flushCalPending 보다 뒤에 둬서, 밤새 대기분이 먼저 나갈 기회를 준 다음 점검한다.
// ⚠️ 첫 실행 때 구글이 Gmail 발송 권한을 한 번 묻는다(허용 필요).
function setupWatchdogTrigger() {
  ScriptApp.getProjectTriggers()
    .filter(t => t.getHandlerFunction() === 'distWatchdog')
    .forEach(t => ScriptApp.deleteTrigger(t));
  ScriptApp.newTrigger('distWatchdog').timeBased().atHour(8).nearMinute(30).everyDays(1).create();
  console.log('distWatchdog 트리거(매일 08:30) 설치 완료');
}

// keepWarm 트리거: 5분마다. 편집기에서 이 함수를 한 번만 실행(▶)하면 설치된다.
function setupKeepWarm() {
  ScriptApp.getProjectTriggers()
    .filter(t => t.getHandlerFunction() === 'keepWarm')
    .forEach(t => ScriptApp.deleteTrigger(t));
  ScriptApp.newTrigger('keepWarm').timeBased().everyMinutes(5).create();
  console.log('keepWarm 트리거(5분마다) 설치 완료');
}

// 공지 창 경계값 자체 점검. 통과하면 'ok' 반환, 틀리면 예외.
function _testNoticeWindow() {
  [8,10,12,23,25,27,28,29,31].forEach(d => { if (!_inNoticeWindow(d)) throw new Error('창 안인데 false: ' + d); });
  [1,7,13,22].forEach(d => { if (_inNoticeWindow(d)) throw new Error('창 밖인데 true: ' + d); });
  console.log('ok');
  return 'ok';
}

// ── 카카오 알림 설치/점검 ─────────────────────────────
// 야간(23~08시)에 대기열로 미뤄진 카톡을 아침 8시에 발송하는 트리거. 편집기에서 1회 ▶실행해 설치.
// (주간에 감지된 공지는 checkAndLogAlerts가 즉시 발송하므로, 이 트리거는 야간 대기분 보충용.)
// 야간 대기분(카톡+캘린더)을 아침 8시에 발송하는 트리거 2개를 설치. 편집기에서 1회 ▶실행.
function setupKakaoTrigger() {
  ScriptApp.getProjectTriggers()
    .filter(t => ['flushKakaoPending', 'flushCalPending'].includes(t.getHandlerFunction()))
    .forEach(t => ScriptApp.deleteTrigger(t));
  ScriptApp.newTrigger('flushKakaoPending').timeBased().atHour(8).nearMinute(10).everyDays(1).create();
  ScriptApp.newTrigger('flushCalPending').timeBased().atHour(8).nearMinute(10).everyDays(1).create();
  console.log('flushKakaoPending·flushCalPending 트리거(매일 8시) 설치 완료');
}

// 카카오 연동 점검: 스크립트 속성(KAKAO_REST_KEY·KAKAO_REFRESH_TOKEN) 설정 후 ▶실행 →
// 나에게 테스트 카톡 1건 발송. 카톡이 오면 연동 완료.
function testKakao() {
  const ok = sendKakaoMemo('✅ jjk 카카오 알림 연동 테스트 — 이 메시지가 오면 설정 완료');
  console.log(ok ? '발송 성공' : '발송 실패(실행 로그 확인)');
  return ok;
}

// 캘린더 연동 점검: ▶실행 → 구글 기본 캘린더에 지금 울리는 테스트 일정 1건 생성.
// 첫 실행 시 캘린더 접근 권한 허용 팝업이 뜬다(허용). 폰 구글 캘린더 앱 알림이 오면 완료.
function testCal() {
  _notifyCal(['✅ jjk 캘린더 알림 연동 테스트 — 이 일정 알림이 오면 설정 완료']);
  console.log('테스트 일정 생성 시도 완료(실행 로그·캘린더 확인)');
}

// 설치된 트리거 전체 목록. "앱이 가끔 20~80초씩 멈춘다"를 추적할 때 편집기에서 ▶실행.
// Apps Script는 같은 계정의 실행을 한 줄로 세우는 구간이 있어, 오래 도는 시간 트리거 하나가
// 웹앱 요청을 통째로 대기시킨다(시트를 전혀 안 건드리는 요청도 같이 느려지는 게 그 증거).
// 실행 주기는 API로 못 읽으므로, 여기서 함수 이름을 확인한 뒤 실행 기록에서 소요시간을 보면 된다.
function _diagTriggers() {
  const ts = ScriptApp.getProjectTriggers();
  console.log('설치된 트리거 ' + ts.length + '개');
  const byFn = {};
  ts.forEach(t => { const f = t.getHandlerFunction(); byFn[f] = (byFn[f] || 0) + 1; });
  Object.keys(byFn).sort().forEach(f => console.log('  ' + f + (byFn[f] > 1 ? '  ×' + byFn[f] : '')));
  // 오래 도는 것으로 알려진 후보에 표시
  ['refreshAllDistributions', 'checkDistNotices', 'snapshotPrices', 'keepWarm'].forEach(f => {
    if (byFn[f]) console.log('  ⚠ ' + f + ' — 6개사 스크랩/자기호출 등으로 길게 도는 후보');
  });
}

// 괴리율 알림 진단: "괴리율 카톡이 안 온다" 확인용. 편집기에서 ▶실행(아무것도 안 바꾸고 상태만 출력).
// checkDeviationAlerts는 조건 미달이면 조용히 return하므로, 안 오는 이유가 ①트리거 없음 ②상태가
// 'below'로 굳어 재알림 잠김 ③실제로 -1% 미만이 없음 중 무엇인지 이 함수 한 번으로 구분된다.
function _diagDeviation() {
  const trigs = ScriptApp.getProjectTriggers().filter(t => t.getHandlerFunction() === 'checkDeviationAlerts');
  console.log('트리거: ' + (trigs.length ? trigs.length + '개 설치됨' : '❌ 없음 → setupDeviationTrigger() 를 ▶실행할 것'));

  let state = {};
  try { state = JSON.parse(PropertiesService.getScriptProperties().getProperty('DEVIATION_STATE') || '{}'); } catch(e) {}
  const below = Object.keys(state).filter(k => state[k] === 'below');
  console.log('DEVIATION_STATE: 총 ' + Object.keys(state).length + '건, below(재알림 잠김) = ' + (below.join(', ') || '없음'));

  const held = {};
  getHoldings().forEach(h => {
    if (String(h.currency).toUpperCase() !== 'KRW') return;
    const t = (h.ticker || '').toString().replace(/^'/, '').trim().toUpperCase();
    if (t) held[t] = h.name || t;
  });

  const res = UrlFetchApp.fetch('https://finance.naver.com/api/sise/etfItemList.nhn?etfType=0&targetColumn=market_sum&sortOrder=desc',
    { muteHttpExceptions: true, headers: { 'User-Agent': 'Mozilla/5.0', 'Referer': 'https://finance.naver.com/fund/etf/etfMain.naver' } });
  console.log('네이버 ETF API 응답코드: ' + res.getResponseCode());
  if (res.getResponseCode() !== 200) return;
  const list = ((JSON.parse(res.getContentText('EUC-KR')).result) || {}).etfItemList || [];
  const liveCnt = list.filter(r => parseFloat(r.changeRate)).length;
  console.log('ETF ' + list.length + '건 / 등락≠0 ' + liveCnt + '건 → 휴장가드 ' + (liveCnt <= list.length * 0.05 ? '차단(알림 안 함)' : '통과'));

  const rows = [];
  list.forEach(r => {
    const t = String(r.itemcode || '').toUpperCase();
    if (!held[t]) return;
    const p = parseFloat(r.nowVal), nav = parseFloat(r.nav);
    if (p > 0 && nav > 0) rows.push({ t: t, n: held[t], dev: Math.round((p - nav) / nav * 10000) / 100 });
  });
  rows.sort((a, b) => a.dev - b.dev);
  console.log('보유 국내 ' + Object.keys(held).length + '종목 중 ETF목록 매칭 ' + rows.length + '건 (낮은 순, ' + DEV_ALERT + '% 미만이 알림 대상)');
  rows.forEach(r => console.log('  ' + r.dev.toFixed(2) + '%  ' + r.t + '  ' + r.n + (state[r.t] === 'below' ? '  [below]' : '')));
}

// 분배 공지 알림 진단: "공지 떴는데 알림이 안 왔다" 확인용. ▶실행(아무것도 안 바꾸고 상태만 출력).
// 알림이 안 오는 경로는 네 갈래뿐이라, 아래 출력이 그중 어디서 끊겼는지 한 번에 가른다.
//   ① 트리거 없음  ② 창/시간 가드에 막힘  ③ 감지됐는데 대기열에 갇힘(발송 실패)  ④ 이미 소비됨(메타가 최신)
function _diagDistAlert() {
  const now = new Date();
  const day  = Number(Utilities.formatDate(now, 'Asia/Seoul', 'd'));
  const hour = Number(Utilities.formatDate(now, 'Asia/Seoul', 'H'));

  const trigs = ScriptApp.getProjectTriggers().filter(t => t.getHandlerFunction() === 'checkDistNotices');
  console.log('① 트리거: ' + (trigs.length ? trigs.length + '개' : '❌ 없음 → setupDistTriggers() 를 ▶실행할 것'));
  console.log('② 가드: 오늘 ' + day + '일 공지창=' + _inNoticeWindow(day) + ' / ' + hour + '시 09~18=' + (hour >= 9 && hour <= 18)
              + ' / 발송창 08~23=' + _kakaoWithinWindow() + '   (셋 다 true여야 알림이 나간다)');

  // 발송 함수들이 참조하는 전역이 실제로 살아 있는지. 2026-08-05~08-12 알림 전면 중단의 원인이
  // _EXEC_URL 미선언(ReferenceError)이었으므로, 같은 사고를 즉시 드러나게 여기서 확인한다.
  try { console.log('①-2 _EXEC_URL: ' + (typeof _EXEC_URL === 'string' && /^https:/.test(_EXEC_URL) ? '정상' : '❌ 값 이상: ' + _EXEC_URL)); }
  catch(e) { console.log('①-2 _EXEC_URL: ❌ 미선언 — 카톡·캘린더 발송이 둘 다 죽는다 (' + e + ')'); }

  const props = PropertiesService.getScriptProperties();
  const kq = props.getProperty('KAKAO_PENDING') || '[]';
  const cq = props.getProperty('CAL_PENDING')   || '[]';
  console.log('③ 대기열: KAKAO_PENDING=' + kq + '   CAL_PENDING=' + cq);
  console.log('   → 비어있지 않으면 감지는 됐고 발송이 실패해 갇힌 것. 카톡키='
              + (props.getProperty('KAKAO_REST_KEY') ? '있음' : '없음')
              + ' 리프레시토큰=' + (props.getProperty('KAKAO_REFRESH_TOKEN') ? '있음' : '없음'));
  try { CalendarApp.getDefaultCalendar().getName(); console.log('   캘린더 접근: 정상'); }
  catch(e) { console.log('   캘린더 접근: ❌ ' + e + '  ← 권한 미허용이면 백업 채널도 죽는다'); }

  // ④ 저장된 메타(직전 공시일) vs 지금 파싱되는 공시일 — 같으면 '이미 소비됨'이라 알림이 안 난다.
  const ms = _getOrCreateSheet('_파서메타', ['운용사','source','isOcr','itemCount','cycles','pubDate','updated']);
  const rows = ms.getLastRow() > 1 ? ms.getRange(2,1,ms.getLastRow()-1,7).getValues() : [];
  console.log('④ 저장된 메타 ' + rows.length + '행 (운용사 / 직전 공시일 / 갱신시각)');
  rows.forEach(r => console.log('   ' + String(r[0]).padEnd(6) + ' prev=' + _normPubDate(r[5]) + '  updated=' + r[6]));
  console.log('   지금 캐시로 파싱되는 공시일(force 안 씀 — 여기서 6개사 스크랩하면 오래 걸린다):');
  ['kodex','tiger','ace','rise','plus','sol'].forEach(s => {
    let fp;
    try { fp = _fingerprint(s, getDistribution(s, false)); } catch(e) { console.log('   ' + s + ' 오류 ' + e); return; }
    const prev = rows.filter(r => r[0] === s)[0];
    const p = prev ? _normPubDate(prev[5]) : '(메타없음)';
    console.log('   ' + s.padEnd(6) + ' now=' + (fp.pubDate || '(빈값)') + '  prev=' + p
                + (fp.pubDate && fp.pubDate !== p ? '  ← 신규로 잡혀야 함' : '')
                + (fp.hasItems ? '' : '  ⚠ 종목 0건'));
  });

  const ls = _getOrCreateSheet('알림로그', ['시각','운용사','종류','메시지','중요도','상태']);
  const ln = ls.getLastRow();
  console.log('⑤ 알림로그 최근 5건 (' + Math.max(0, ln - 1) + '건 중)');
  if (ln > 1) {
    const n = Math.min(5, ln - 1);
    ls.getRange(ln - n + 1, 1, n, 6).getValues()
      .forEach(r => console.log('   ' + r[0] + ' | ' + r[1] + ' | ' + r[2] + ' | ' + r[3] + ' | ' + r[5]));
  }
  return 'ok';
}

// 알림이 실제로 '가는지' 확인. ▶실행하면 대기열을 진짜로 발송해 보고 채널별 성공/실패를 찍는다.
// 대기열이 비어 있으면 확인용 문구 1건을 넣어 보낸다(카톡 1통 + 캘린더 일정 1건이 실제로 온다).
// 진단만 하고 안 보내는 _diagDistAlert 와 달리 이건 **실제 발송**이다.
function _diagSendNow() {
  const props = PropertiesService.getScriptProperties();
  const before = { k: props.getProperty('KAKAO_PENDING') || '[]', c: props.getProperty('CAL_PENDING') || '[]' };
  console.log('발송 전 대기열: KAKAO=' + before.k + '  CAL=' + before.c);

  if (before.k === '[]' && before.c === '[]') {
    const probe = ['✅ 발송 점검 ' + Utilities.formatDate(new Date(), 'Asia/Seoul', 'MM-dd HH:mm')];
    props.setProperty('KAKAO_PENDING', JSON.stringify(probe));
    props.setProperty('CAL_PENDING',   JSON.stringify(probe));
    console.log('대기열이 비어 있어 점검용 1건을 넣었다.');
  }

  let k = false, c = false;
  try { k = flushKakaoPending(); } catch(e) { console.log('카톡 예외: ' + e); }
  try { c = flushCalPending();   } catch(e) { console.log('캘린더 예외: ' + e); }

  const after = { k: props.getProperty('KAKAO_PENDING') || '[]', c: props.getProperty('CAL_PENDING') || '[]' };
  console.log('카톡   : ' + (k && after.k === '[]' ? '✅ 발송됨(대기열 비움)' : '❌ 실패 — 대기열 유지: ' + after.k));
  console.log('캘린더 : ' + (c && after.c === '[]' ? '✅ 발송됨(대기열 비움)' : '❌ 실패 — 대기열 유지: ' + after.c));
  console.log(k || c ? '⇒ 최소 한 채널은 살아 있다. 실제로 카톡/캘린더에 왔는지 눈으로 확인할 것.'
                     : '⇒ 두 채널 다 죽었다. 위 예외 메시지가 원인이다.');
  return { kakao: k, calendar: c };
}
