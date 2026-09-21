/**
 * XRP당 카이아 교환비율 알림
 *
 * 30분마다 코인게코가 중계하는 빗썸 시세로 XRP ÷ KAIA 비율을 재고,
 * 목표에 닿으면 기본 캘린더에 일정을 만들어 폰 팝업을 띄운다.
 *
 * 왜 PC 위젯(tools/ticker.pyw)이 아니라 여기서 하나:
 *   위젯은 PC가 켜져 있을 때만 돈다. 밤사이 목표에 닿으면 아무도 못 본다.
 *   이건 구글 클라우드에서 돌아 PC·망과 무관하다.
 *
 * 시세는 빗썸을 직접 부른다:
 *   위젯이 코인게코를 거치는 건 **학교망이 빗썸을 막기 때문**이다. 여기는 구글
 *   서버라 그 제약이 없다. 오히려 구글 IP 는 여러 사람이 같이 쓰므로 코인게코가
 *   429 로 막는다(2026-09-21 실제로 첫 실행이 그렇게 실패했다).
 *   빗썸이 안 되면 코인게코로 넘어간다.
 *
 * 어느 달력에 넣나 — **기본 달력**이다. 처음엔 '시세 알림' 달력을 새로 만들어 넣었는데
 *   일정은 멀쩡히 들어갔는데도 폰에 알림이 안 왔다(2026-09-21). 새로 만든 보조 달력은
 *   휴대폰 구글 캘린더 앱에서 **기본이 꺼짐**이라 동기화 자체가 안 되고, 동기화가 안 되면
 *   알림도 영영 안 온다. 같은 내용을 기본 달력에 넣어 보니 바로 왔다. 폰에서 토글 하나
 *   켜면 되는 일이지만, 알림이 도착하느냐가 이 기능의 전부라 토글에 기대지 않는다.
 *
 * ⚠ 이 파일은 별도 Apps Script 프로젝트의 사본이다. 여기서 고치면 **clasp 로 올려야** 한다.
 *   프로젝트: 17wPsWshNsGLGSeel9rqHvfCWyiAAWAfL0MUUrJAB5kkDv0PqMLKCcUds (XRP-카이아 비율 알림)
 *   사용자에게 편집기 붙여넣기를 시키지 마라 (CLAUDE.md).
 *   포트폴리오 Code.gs 에 합치지 말 것. 실행 슬롯을 뺏어 앱이 멈춘다 (WORKLOG 85·86).
 */

// ── 설정 ──────────────────────────────────
var TARGET        = 50;        // 이 비율에 닿으면 알린다 (1 XRP = 50 KAIA)
var REARM_GAP     = 0.01;      // 목표보다 1% 아래로 내려가야 다시 무장한다(경계에서 떨리는 것 방지)
var BASE_COIN     = 'ripple';  // 파는 것
var QUOTE_COIN    = 'kaia';    // 사는 것
var CALENDAR_NAME = '';      // 빈 칸이면 기본 달력. 폰에서 반드시 켜져 있는 유일한 달력이다
var SAMPLE_XRP    = 1000;      // 일정 본문에 "XRP 1,000개 → KAIA 몇 개"를 적는다

var BITHUMB = 'https://api.bithumb.com/public/ticker/';   // 종목별. ALL_KRW 는 캐시를 타 값이 멎는다
var GECKO   = 'https://api.coingecko.com/api/v3/exchanges/bithumb/tickers?coin_ids=';
var SYM     = { ripple: 'XRP', kaia: 'KAIA', bitcoin: 'BTC', ethereum: 'ETH' };

var STATE_KEY = 'ratio_armed';
var FAIL_KEY  = 'ratio_fail';
var FAIL_ALERT = 8;            // 연속 이만큼(=4시간) 못 받으면 "알림이 죽었다"고 알린다


// ── 진입점 (30분 트리거) ──────────────────────────────────
function checkOnce() {
  var props = PropertiesService.getScriptProperties();
  var p = fetchPair_();
  if (!p) {
    // 조용히 죽으면 "알림이 안 온다 = 목표에 안 닿았다"로 오해한다. 오래 못 받으면 알린다.
    var n = Number(props.getProperty(FAIL_KEY) || 0) + 1;
    props.setProperty(FAIL_KEY, String(n));
    Logger.log('시세를 못 받았다 (' + n + '회 연속)');
    if (n === FAIL_ALERT) createDeadEvent_(getCalendar_(), n);
    return;
  }
  props.deleteProperty(FAIL_KEY);

  var ratio = p.base / p.quote;
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
/** 빗썸 원화 시세. 빗썸이 안 되면 코인게코로 넘어간다. 둘 다 안 되면 null. */
function fetchPair_() {
  return pack_(fromBithumb_(), '빗썸') || pack_(fromGecko_(), '코인게코');
}

function pack_(out, src) {
  if (!out) return null;
  var b = out[SYM[BASE_COIN]], q = out[SYM[QUOTE_COIN]];
  if (!b || !q) { Logger.log(src + ' 에 빠진 시세: ' + JSON.stringify(out)); return null; }
  Logger.log('시세 출처: ' + src);
  return { base: b, quote: q, baseSym: SYM[BASE_COIN], quoteSym: SYM[QUOTE_COIN], src: src };
}

/** 빗썸 공개 시세. 종목마다 한 번씩 부른다(30분에 두 번이라 부담이 없다). */
function fromBithumb_() {
  var out = {}, syms = [SYM[BASE_COIN], SYM[QUOTE_COIN]];
  for (var i = 0; i < syms.length; i++) {
    var res = UrlFetchApp.fetch(BITHUMB + syms[i] + '_KRW', { muteHttpExceptions: true });
    if (res.getResponseCode() !== 200) {
      Logger.log('빗썸 응답 ' + res.getResponseCode());
      return null;
    }
    var j = JSON.parse(res.getContentText());
    if (j.status !== '0000' || !j.data || !j.data.closing_price) {
      Logger.log('빗썸 status ' + (j && j.status));
      return null;
    }
    out[syms[i]] = Number(j.data.closing_price);
  }
  return out;
}

/** 대체 경로. 구글 IP 는 공용이라 429 가 잦다 — 어디까지나 빗썸이 안 될 때만. */
function fromGecko_() {
  var res = UrlFetchApp.fetch(GECKO + BASE_COIN + ',' + QUOTE_COIN,
                              { muteHttpExceptions: true,
                                headers: { 'User-Agent': 'ticker-ratio-alert' } });
  if (res.getResponseCode() !== 200) {
    Logger.log('코인게코 응답 ' + res.getResponseCode());
    return null;
  }
  var out = {};
  (JSON.parse(res.getContentText()).tickers || []).forEach(function (t) {
    if (t.target === 'KRW' && t.last && !t.is_stale) out[t.base] = Number(t.last);
  });
  return out;
}


// ── 캘린더 ──────────────────────────────────
function getCalendar_() {
  if (!CALENDAR_NAME) return CalendarApp.getDefaultCalendar();
  var found = CalendarApp.getCalendarsByName(CALENDAR_NAME);
  if (found && found.length) return found[0];
  // 없는 달력 이름을 적어 놨으면 만들지 마라. 새로 만든 달력은 폰에서 꺼져 있어
  // 알림이 안 온다(맨 위 설명). 조용히 안 오느니 기본 달력에 넣는 게 낫다.
  Logger.log('달력 "' + CALENDAR_NAME + '" 이 없다. 기본 달력을 쓴다');
  return CalendarApp.getDefaultCalendar();
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
    '출처: ' + (p.src || '빗썸')
  ].join('\n');

  // 시작을 정확히 '지금'으로 두면 팝업이 발화하지 않을 수 있어 2분 여유를 둔다.
  var start = new Date(Date.now() + 2 * 60 * 1000);
  var end   = new Date(start.getTime() + 15 * 60 * 1000);
  cal.createEvent(title, start, end, { description: desc }).addPopupReminder(0);
  Logger.log('일정 생성 → ' + title);
}

/** 시세를 오래 못 받으면 알린다. 조용한 실패가 제일 위험하다. */
function createDeadEvent_(cal, n) {
  var start = new Date(Date.now() + 2 * 60 * 1000);
  cal.createEvent('\u26a0\ufe0f 시세 알림이 값을 못 받고 있습니다',
                  start, new Date(start.getTime() + 15 * 60 * 1000),
                  { description: n + '회 연속 실패(약 ' + (n / 2) + '시간).\n'
                                 + 'Apps Script 실행 기록에서 사유를 확인하세요.' })
     .addPopupReminder(0);
  Logger.log('죽음 알림 발송');
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
