/**
 * XRP당 카이아 교환비율 알림
 *
 * 30분마다 코인게코가 중계하는 빗썸 시세로 XRP ÷ KAIA 비율을 재고,
 * 목표에 닿으면 '시세 알림' 캘린더에 일정을 만들어 폰 팝업을 띄운다.
 *
 * 왜 PC 위젯(tools/ticker.pyw)이 아니라 여기서 하나:
 *   위젯은 PC가 켜져 있을 때만 돈다. 밤사이 목표에 닿으면 아무도 못 본다.
 *   이건 구글 클라우드에서 돌아 PC·망과 무관하다.
 *
 * 왜 빗썸을 직접 안 부르나:
 *   학교망이 api.bithumb.com 을 간헐적으로 막는다(WORKLOG 140). 코인게코는
 *   거래소별 시세를 중계하므로 빗썸 가격을 그대로 받으면서 차단을 안 탄다.
 *   여기는 구글 서버라 상관없지만, 위젯과 같은 출처를 써야 값이 어긋나지 않는다.
 *
 * ⚠ 이 파일은 별도 Apps Script 프로젝트의 사본이다. 여기서 고쳐도 반영되지 않는다 —
 *   Apps Script 편집기에 붙여넣어야 한다. (okx_nft_alert.gs 와 같은 규칙)
 *   포트폴리오 Code.gs 에 합치지 말 것. 실행 슬롯을 뺏어 앱이 멈춘다 (WORKLOG 85·86).
 */

// ── 설정 ──────────────────────────────────
var TARGET        = 50;        // 이 비율에 닿으면 알린다 (1 XRP = 50 KAIA)
var REARM_GAP     = 0.01;      // 목표보다 1% 아래로 내려가야 다시 무장한다(경계에서 떨리는 것 방지)
var BASE_COIN     = 'ripple';  // 파는 것
var QUOTE_COIN    = 'kaia';    // 사는 것
var CALENDAR_NAME = '시세 알림';
var SAMPLE_XRP    = 1000;      // 일정 본문에 "XRP 1,000개 → KAIA 몇 개"를 적는다

var GECKO = 'https://api.coingecko.com/api/v3/exchanges/bithumb/tickers?coin_ids=';
var STATE_KEY = 'ratio_armed';


// ── 진입점 (30분 트리거) ──────────────────────────────────
function checkOnce() {
  var p = fetchPair_();
  if (!p) { Logger.log('시세를 못 받았다 — 이번 회차는 건너뛴다'); return; }

  var ratio = p.base / p.quote;
  var props = PropertiesService.getScriptProperties();
  var armed = props.getProperty(STATE_KEY) !== 'no';   // 처음엔 무장 상태

  Logger.log('비율 ' + ratio.toFixed(2) + ' / 목표 ' + TARGET + ' / 무장 ' + armed);

  if (ratio >= TARGET && armed) {
    createEvent_(getCalendar_(), ratio, p, false);
    props.setProperty(STATE_KEY, 'no');                // 한 번 알리면 잠근다
  } else if (ratio < TARGET * (1 - REARM_GAP) && !armed) {
    props.setProperty(STATE_KEY, 'yes');               // 충분히 내려오면 다시 무장
    Logger.log('목표 아래로 내려와 다시 무장');
  }
}


// ── 시세 ──────────────────────────────────
/** 코인게코가 중계하는 빗썸 원화 시세. 둘 다 못 받으면 null. */
function fetchPair_() {
  var url = GECKO + BASE_COIN + ',' + QUOTE_COIN;
  var res = UrlFetchApp.fetch(url, { muteHttpExceptions: true,
                                     headers: { 'User-Agent': 'ticker-ratio-alert' } });
  if (res.getResponseCode() !== 200) {
    Logger.log('코인게코 응답 ' + res.getResponseCode());
    return null;
  }
  var ticks = JSON.parse(res.getContentText()).tickers || [];
  var out = {};
  ticks.forEach(function (t) {
    if (t.target === 'KRW' && t.last && !t.is_stale) out[t.base] = Number(t.last);
  });
  // base/quote 를 심볼로 되찾는다. 코인게코 id 와 거래소 심볼이 다르기 때문.
  var sym = { ripple: 'XRP', kaia: 'KAIA', bitcoin: 'BTC', ethereum: 'ETH' };
  var b = out[sym[BASE_COIN]], q = out[sym[QUOTE_COIN]];
  if (!b || !q) { Logger.log('빠진 시세: ' + JSON.stringify(out)); return null; }
  return { base: b, quote: q, baseSym: sym[BASE_COIN], quoteSym: sym[QUOTE_COIN] };
}


// ── 캘린더 ──────────────────────────────────
function getCalendar_() {
  var found = CalendarApp.getCalendarsByName(CALENDAR_NAME);
  if (found && found.length) return found[0];
  Logger.log('캘린더 생성: ' + CALENDAR_NAME);
  return CalendarApp.createCalendar(CALENDAR_NAME);
}

function createEvent_(cal, ratio, p, isTest) {
  var got = SAMPLE_XRP * ratio;

  var title = (isTest ? '🧪 테스트 · ' : '🎯 ')
            + p.baseSym + '당 ' + p.quoteSym + ' ' + ratio.toFixed(2)
            + ' · 목표 ' + TARGET + ' 도달';

  var desc = [
    '비율:   1 ' + p.baseSym + ' = ' + ratio.toFixed(2) + ' ' + p.quoteSym,
    '목표:   ' + TARGET + ' (' + ((ratio / TARGET - 1) * 100).toFixed(1) + '%)',
    '',
    p.baseSym + ':    ' + num_(p.base) + '원',
    p.quoteSym + ':   ' + p.quote.toFixed(2) + '원',
    '',
    p.baseSym + ' ' + num_(SAMPLE_XRP) + '개 → ' + p.quoteSym + ' ' + num_(got) + '개',
    '',
    isTest ? '테스트 발송 — 실제 알림이 아닙니다'
           : '한 번 알린 뒤에는 목표보다 '
             + (REARM_GAP * 100) + '% 아래로 내려가야 다시 알립니다.',
    '',
    '출처: 코인게코가 중계하는 빗썸 시세'
  ].join('\n');

  // 시작을 정확히 '지금'으로 두면 팝업이 발화하지 않을 수 있어 2분 여유를 둔다.
  var start = new Date(Date.now() + 2 * 60 * 1000);
  var end   = new Date(start.getTime() + 15 * 60 * 1000);
  cal.createEvent(title, start, end, { description: desc }).addPopupReminder(0);
  Logger.log('일정 생성 → ' + title);
}

/** 천 단위 콤마. 소수점은 버린다. */
function num_(n) {
  return String(Math.round(Number(n))).replace(/\B(?=(\d{3})+(?!\d))/g, ',');
}


// ===== 수동 실행 =====
// 아래는 Apps Script 편집기에서 손으로 돌리는 함수들이다.

/** 30분 트리거를 새로 건다. 이 파일을 처음 붙여넣은 뒤 한 번 실행할 것. */
function setupRatioTrigger() {
  ScriptApp.getProjectTriggers().forEach(function (t) {
    if (t.getHandlerFunction() === 'checkOnce') ScriptApp.deleteTrigger(t);
  });
  ScriptApp.newTrigger('checkOnce').timeBased().everyMinutes(30).create();
  Logger.log('30분 트리거 설치 완료');
}

/** 지금 비율만 찍어 본다. 일정은 만들지 않는다. */
function testOnce() {
  var p = fetchPair_();
  if (!p) { Logger.log('시세를 못 받았다'); return; }
  var ratio = p.base / p.quote;
  Logger.log(p.baseSym + ' ' + num_(p.base) + '원 / ' + p.quoteSym + ' ' + p.quote.toFixed(2) + '원');
  Logger.log('비율 ' + ratio.toFixed(2) + ' / 목표 ' + TARGET
             + ' (' + ((ratio / TARGET - 1) * 100).toFixed(1) + '%)');
}

/** 캘린더·팝업이 실제로 뜨는지 확인한다. 지금 비율로 테스트 일정을 만든다. */
function sendTestEvent() {
  var p = fetchPair_();
  if (!p) { Logger.log('시세를 못 받았다'); return; }
  createEvent_(getCalendar_(), p.base / p.quote, p, true);
}

/** 목표에 이미 닿아 잠겨 있을 때 다시 알리도록 푼다. */
function resetState() {
  PropertiesService.getScriptProperties().deleteProperty(STATE_KEY);
  Logger.log('다시 무장했다');
}
