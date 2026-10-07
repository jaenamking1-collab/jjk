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

  // ?settoken=… → 포트폴리오 API 열쇠를 이 프로젝트 속성에 한 번 넣는다(relay-token.yml 이 Secret 으로 부른다).
  // 이미 있으면 바꾸지 않는다 — 남이 먼저 불러도 덮어쓸 수 없게. 값은 어디에도 돌려주지 않는다.
  if (p.settoken) {
    var sp = PropertiesService.getScriptProperties();
    var had = !!sp.getProperty('APP_TOKEN');
    if (!had) sp.setProperty('APP_TOKEN', String(p.settoken));
    return ContentService.createTextOutput(JSON.stringify({ t: out.t, stored: !had, had: had }))
                         .setMimeType(ContentService.MimeType.JSON);
  }

  // ?total=코드 → 대시보드 '평가금'과 같은 식의 실시간 총평가와 오늘 등락. 금액이라 목록과 같은 코드가 있어야 준다.
  // 'selftest' 는 금액 없이 모양만 준다(공개 fetch.yml 로 통로를 확인하는 용도).
  if (p.total) {
    var okCode = (typeof PRIVATE_LISTS !== 'undefined') &&
                 Object.prototype.hasOwnProperty.call(PRIVATE_LISTS, p.total);
    if (okCode) {
      try {
        var tot = portfolioTotal_();
        out.total = (p.total === 'selftest')
          ? { ok: true, n: tot.n, missing: tot.missing, noPrev: tot.noPrev, err: tot.err, at: tot.at, positive: tot.value > 0 }
          : tot;
      } catch (err) { out.err['총자산'] = String(err).slice(0, 120); }
    }
    if (!syms.length && !p.list) return ContentService.createTextOutput(JSON.stringify(out))
                                                     .setMimeType(ContentService.MimeType.JSON);
  }

  // ?list=코드 → 휴대폰 화면의 종목 목록. 목록엔 보유 종목이 들어 있어 공개 저장소에 못 둔다.
  // 그래서 이 프로젝트에만 있는 Private.gs(저장소에 없음)의 PRIVATE_LISTS 에서 꺼낸다.
  // 코드는 claude-memory(비공개)의 ticker/phone_code.txt 에 있다. 모르는 코드면 list 를 안 준다.
  if (p.list) {
    var lists = (typeof PRIVATE_LISTS !== 'undefined') ? PRIVATE_LISTS : {};
    if (Object.prototype.hasOwnProperty.call(lists, p.list)) out.list = lists[p.list];
    if (!syms.length) return ContentService.createTextOutput(JSON.stringify(out))
                                           .setMimeType(ContentService.MimeType.JSON);
  }

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

// ── 총평가 ── 대시보드 '평가금'(acctInvestValue)과 같은 식을 **실시간**으로 낸다:
//   평가금 = Σ 수량 × 현재가(달러 종목은 × 환율), 현재가가 없으면 매수평균가(대시보드도 그렇게 한다).
//   오늘 등락 = 지금 평가금 − 전일 기준 평가금(종목별 전일대비, 달러 종목은 환율 변동까지).
// 대시보드의 '총자산'은 여기에 예수금을 더하지만, 예수금은 매매·분배금 기록을 굴려 내는 값이라
// (portfolio.html rollScorecard) 이 줄에선 뺀다 — 시세로 오르내린 만큼을 보는 게 이 줄의 목적이다.
// 보유 목록은 포트폴리오 API(getHoldings, 열쇠는 이 프로젝트 속성에만)에서 10분마다 받는다.
var PORTFOLIO_API = 'https://script.google.com/macros/s/AKfycbwJS1Fd-sDCVKPLJEpEWZmPQEKAOR9pG7y-nPKZOYty65j3ArOmlDzNX2WFqiGNF_s/exec';
function holdings_() {
  var cache = CacheService.getScriptCache(), hit = cache.get('holdings');
  if (hit) return JSON.parse(hit);
  var token = PropertiesService.getScriptProperties().getProperty('APP_TOKEN');
  if (!token) throw new Error('열쇠 없음');
  var res = UrlFetchApp.fetch(PORTFOLIO_API + '?action=getHoldings&token=' + encodeURIComponent(token),
                              { muteHttpExceptions: true });
  var arr = JSON.parse(res.getContentText());
  if (!Array.isArray(arr)) throw new Error('보유 목록 실패');
  var hs = arr.map(function (x) {
    return { t: String(x.ticker || '').trim().toUpperCase(), q: parseFloat(x.quantity) || 0,
             avg: parseFloat(x.avg_price) || 0, usd: x.currency === 'USD' };
  }).filter(function (x) { return x.t && x.q > 0; });
  cache.put('holdings', JSON.stringify(hs), 600);
  return hs;
}

function portfolioApi_(action, key, ttl) {
  var cache = CacheService.getScriptCache(), hit = cache.get(key);
  if (hit) return JSON.parse(hit);
  var token = PropertiesService.getScriptProperties().getProperty('APP_TOKEN');
  if (!token) throw new Error('열쇠 없음');
  var res = UrlFetchApp.fetch(PORTFOLIO_API + '?action=' + action + '&token=' + encodeURIComponent(token),
                              { muteHttpExceptions: true });
  var j = JSON.parse(res.getContentText());
  try { cache.put(key, JSON.stringify(j), ttl); } catch (e) {}      // 100KB 넘으면 캐시만 건너뛴다
  return j;
}

function portfolioTotal_() {
  var hs = holdings_();
  // 시세는 **대시보드가 쓰는 바로 그 함수**(getLivePrices: 네이버·야후·시트 예비시세)로 받는다.
  // 따로 받으면 종목 분류가 어긋나 11종목이 빠졌다(통화가 KRW 가 아닌 종목은 그쪽이 야후로 묻는다).
  var lp = (portfolioApi_('getLivePrices', 'livep', 60) || {}).prices || {};
  var err = {}, fxq = fetchQuotes_(['USDKRW=X'], err)['USDKRW=X'];
  var fx = fxq ? fxq[0] : 1400, fxPrev = fxq ? fxq[0] - fxq[1] : fx;
  var now = 0, prev = 0, missing = 0, noPrev = 0;
  hs.forEach(function (x) {
    var p = lp[x.t], cur = p && p.current, before = p && p.prev;
    if (!cur) missing++;
    else if (!before) noPrev++;                    // 시트 예비시세는 전일값이 없다 → 그 종목은 등락 0
    var a = cur || x.avg, b = before || a;
    now  += x.q * a * (x.usd ? fx : 1);
    prev += x.q * b * (x.usd ? fxPrev : 1);
  });
  return { value: Math.round(now), diff: Math.round(now - prev),
           pct: prev ? (now - prev) / prev * 100 : 0, n: hs.length, missing: missing, noPrev: noPrev,
           err: err, at: Utilities.formatDate(new Date(), 'Asia/Seoul', 'HH:mm') };
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
