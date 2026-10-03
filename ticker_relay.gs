/**
 * 휴대폰 시세 화면(ticker.html)용 시세 중계.
 *
 * 왜 필요한가:
 *   휴대폰 브라우저는 빗썸·야후·네이버에 직접 물을 수 없다(CORS — 남의 사이트 응답을
 *   페이지가 읽지 못하게 막는 브라우저 규칙). 구글 서버는 그 규칙을 안 받으므로 여기서
 *   대신 받아 JSON 으로 돌려준다. PC 위젯(tools/ticker.pyw)과 **같은 출처·같은 기준**을 쓴다.
 *
 *   GET ?s=XRP-KRW,KAIA-KRW,^IXIC,069500,KOSPI&band=XRP-KRW/KAIA-KRW
 *   → { t: 응답시각(ms), q: { 심볼: [현재가, 전일대비, 등락률%, 소수자리, 이름] },
 *       band: { "A/B": [30일 최저, 최고] }, err: { 출처: 사유 } }
 *
 * 종목 구분은 위젯과 같다:
 *   KOSPI/KOSDAQ → 네이버 지수 · 6자리 코드 → 네이버 종목 · ...-KRW → 빗썸 · 나머지 → 야후
 *
 * ⚠ 이 파일은 'XRP-카이아 비율 알림' Apps Script 프로젝트(17wPsWsh…)에 Code.gs 와 함께 올라간다.
 *   같은 프로젝트에 둔 이유: 그 프로젝트는 이미 외부 접속(UrlFetchApp) 권한을 승인받았다.
 *   새 프로젝트로 만들면 사용자가 다시 승인 버튼을 눌러야 한다.
 *   배포는 clasp 로 에이전트가 한다. 사용자에게 편집기 붙여넣기를 시키지 마라.
 */

var RELAY_TTL = 15;          // 초. 여러 화면이 동시에 열려도 실제 조회는 이 간격에 한 번
var BAND_TTL  = 6 * 3600;    // 30일 구간은 하루 몇 번이면 충분하다
var BAND_DAYS = 30;

var R_NAVER   = 'https://polling.finance.naver.com/api/realtime/domestic/';
var R_BITHUMB = 'https://api.bithumb.com/public/ticker/';
var R_CANDLE  = 'https://api.bithumb.com/public/candlestick/';
var R_YAHOO   = 'https://query1.finance.yahoo.com/v7/finance/spark?range=1d&interval=1d&symbols=';
var R_INDEX   = { KOSPI: 'KOSPI', KOSDAQ: 'KOSDAQ', '^KS11': 'KOSPI', '^KQ11': 'KOSDAQ' };
var R_CODE    = /^[0-9]{4}[0-9A-Z]{2}$/;


function doGet(e) {
  var p = (e && e.parameter) || {};
  var syms  = splitList_(p.s);
  var bands = splitList_(p.band);
  var out = { t: Date.now(), q: {}, band: {}, err: {} };

  var cache = CacheService.getScriptCache();
  var need = [];
  syms.forEach(function (s) {
    var hit = cache.get('q:' + s);
    if (hit) out.q[s] = JSON.parse(hit);
    else need.push(s);
  });

  if (need.length) {
    var got = fetchQuotes_(need, out.err);
    var put = {};
    Object.keys(got).forEach(function (s) {
      out.q[s] = got[s];
      put['q:' + s] = JSON.stringify(got[s]);
    });
    if (Object.keys(put).length) cache.putAll(put, RELAY_TTL);
  }

  bands.forEach(function (k) {
    var hit = cache.get('b:' + k);
    if (hit) { out.band[k] = JSON.parse(hit); return; }
    try {
      var b = ratioBand_(k);
      if (b) { out.band[k] = b; cache.put('b:' + k, JSON.stringify(b), BAND_TTL); }
    } catch (err) { out.err['구간 ' + k] = String(err).slice(0, 120); }
  });

  return ContentService.createTextOutput(JSON.stringify(out))
                       .setMimeType(ContentService.MimeType.JSON);
}


function fetchQuotes_(syms, err) {
  var idx = [], stk = [], coin = [], yah = [];
  syms.forEach(function (s) {
    if (R_INDEX[s]) idx.push(s);
    else if (R_CODE.test(s)) stk.push(s);
    else if (/-KRW$/.test(s)) coin.push(s);
    else yah.push(s);
  });
  var out = {};
  tryInto_(out, err, '네이버 지수', function () { return naver_('index', idx, true); }, idx.length);
  tryInto_(out, err, '네이버 종목', function () { return naver_('stock', stk, false); }, stk.length);
  tryInto_(out, err, '빗썸',       function () { return bithumb_(coin, err); }, coin.length);
  tryInto_(out, err, '야후',       function () { return yahoo_(yah); }, yah.length);
  return out;
}

function tryInto_(out, err, name, fn, n) {
  if (!n) return;
  try {
    var got = fn();
    Object.keys(got).forEach(function (k) { out[k] = got[k]; });
  } catch (e) { err[name] = String(e).slice(0, 120); }
}

function getJson_(url) {
  var res = UrlFetchApp.fetch(url, { muteHttpExceptions: true,
                                     headers: { 'User-Agent': 'Mozilla/5.0' } });
  if (res.getResponseCode() !== 200) throw new Error('HTTP ' + res.getResponseCode());
  return JSON.parse(res.getContentText());
}

function num_r_(s) { return Number(String(s).replace(/,/g, '')); }


/** 네이버 실시간. 위젯 naver() 와 같다. 지수는 소수 둘째 자리까지 보여준다. */
function naver_(kind, syms, isIndex) {
  var codes = syms.map(function (s) { return R_INDEX[s] || s; });
  var d = getJson_(R_NAVER + kind + '/' + codes.join(','));
  var byCode = {};
  (d.datas || []).forEach(function (x) {
    byCode[x.itemCode] = [num_r_(x.closePrice), num_r_(x.compareToPreviousClosePrice),
                          num_r_(x.fluctuationsRatio), isIndex ? 2 : 0, x.stockName || null];
  });
  var out = {};
  syms.forEach(function (s) { var v = byCode[R_INDEX[s] || s]; if (v) out[s] = v; });
  return out;
}

/** 빗썸 종목별. 기준은 전일 종가 — 빗썸 화면의 '변동(당일)'과 같다(위젯 bithumb() 주석 참고). */
function bithumb_(syms, err) {
  var reqs = syms.map(function (s) {
    return { url: R_BITHUMB + s.split('-')[0] + '_KRW', muteHttpExceptions: true };
  });
  var res = UrlFetchApp.fetchAll(reqs);
  var out = {};
  res.forEach(function (r, i) {
    try {
      var j = JSON.parse(r.getContentText());
      if (j.status !== '0000') throw new Error('status ' + j.status);
      var price = Number(j.data.closing_price), prev = Number(j.data.prev_closing_price);
      var diff = price - prev;
      out[syms[i]] = [price, diff, prev ? diff / prev * 100 : 0, price >= 100 ? 0 : 2, null];
    } catch (e) { err['빗썸 ' + syms[i]] = String(e).slice(0, 80); }
  });
  return out;
}

/** 야후. 장 전후 포함 값(fullday…)이 있으면 그걸 쓴다 — 위젯 yahoo() 와 같은 이유(PLTR 9/4). */
function yahoo_(syms) {
  var d = getJson_(R_YAHOO + encodeURIComponent(syms.join(',')));
  var out = {};
  ((d.spark && d.spark.result) || []).forEach(function (r) {
    var m = r.response && r.response[0] && r.response[0].meta;
    if (!m) return;
    var price, diff, pct;
    if (m.fulldayPrice && m.fulldayChangePercent != null) {
      price = m.fulldayPrice; diff = m.fulldayChange || 0; pct = m.fulldayChangePercent;
    } else {
      price = m.regularMarketPrice;
      var prev = m.chartPreviousClose || price;
      diff = price - prev; pct = prev ? diff / prev * 100 : 0;
    }
    var dec = (/=X$/.test(r.symbol) || m.currency !== 'KRW') ? 2 : 0;
    out[r.symbol] = [price, diff, pct, dec, null];
  });
  return out;
}

/** 교환비율 "A-KRW/B-KRW" 의 최근 30일 최저~최고. 빗썸 일봉 종가끼리 같은 날짜로 짝지어 나눈다. */
function ratioBand_(k) {
  var ab = k.split('/');
  if (ab.length !== 2 || !/-KRW$/.test(ab[0]) || !/-KRW$/.test(ab[1])) return null;
  var a = candles_(ab[0]), b = candles_(ab[1]);
  var rs = [];
  Object.keys(a).forEach(function (day) { if (b[day]) rs.push(a[day] / b[day]); });
  rs = rs.slice(-BAND_DAYS);
  if (rs.length < 5) return null;
  return [Math.min.apply(null, rs), Math.max.apply(null, rs)];
}

function candles_(sym) {
  var j = getJson_(R_CANDLE + sym.split('-')[0] + '_KRW/24h');
  if (j.status !== '0000') throw new Error('candle ' + j.status);
  var out = {}, rows = j.data.slice(-BAND_DAYS - 2);
  rows.forEach(function (c) {           // [시각ms, 시가, 종가, 고가, 저가, 거래량]
    out[Utilities.formatDate(new Date(Number(c[0])), 'Asia/Seoul', 'yyyyMMdd')] = Number(c[2]);
  });
  return out;
}

function splitList_(s) {
  return String(s || '').split(',').map(function (x) { return x.trim(); })
                        .filter(function (x) { return x; });
}

/** 편집기에서 ▶ 로 중계가 도는지 본다(배포 없이). */
function testRelay() {
  var r = doGet({ parameter: { s: 'KOSPI,069500,USDKRW=X,BTC-KRW,XRP-KRW,KAIA-KRW,^IXIC,^GSPC',
                               band: 'XRP-KRW/KAIA-KRW' } });
  Logger.log(r.getContent());
}
