/**
 * OKX NFT — Legendary/Mystic 리스팅 알림
 *
 * puuvillasociety / puuvilla-fashionista (Kaia) 두 컬렉션을 30분마다 확인해,
 * Legendary·Mystic 매물이 새로 올라오거나 가격이 내려가면 'OKX NFT' 캘린더에
 * 일정을 만들고 팝업 알림을 띄운다.
 *
 * 설계 문서: docs/superpowers/specs/2026-08-08-okx-nft-legendary-alert-design.md
 *
 * ⚠ 이 파일은 별도 Apps Script 프로젝트의 사본이다. 여기서 고쳐도 반영되지 않는다 —
 *   Apps Script 편집기에 붙여넣어야 한다. (포트폴리오 Code.gs와 같은 규칙)
 *   포트폴리오 백엔드에 합치지 말 것. 실행 슬롯을 뺏어 앱이 멈춘다 (WORKLOG 85·86).
 */

// ── 설정 ──────────────────────────────────
var COLLECTIONS = [
  { label: 'Society',     projectId: 748906  },
  { label: 'Fashionista', projectId: 2289945 }
];
var TARGET_RARITY   = ['Legendary', 'Mystic'];
var CALENDAR_NAME   = 'OKX NFT';
var FALLBACK_USDKRW = 1450;

var MARKET_API = 'https://web3.okx.com/priapi/v1/nft/secondary/market';
var ASSET_URL  = 'https://web3.okx.com/nft/asset/kaia/';
var PAGE_SIZE  = 50;   // 50을 넘기면 OKX가 12건만 돌려준다. 올리지 말 것.
var MAX_PAGES  = 10;

// ── 진입점 (30분 트리거) ──────────────────────────────────
function checkOnce() {
  var props   = PropertiesService.getScriptProperties();
  var rows    = [];   // 알릴 것
  var pending = [];   // 알림이 다 성공한 뒤에야 저장할 상태

  COLLECTIONS.forEach(function (col) {
    try {
      var list = fetchListings_(col.projectId);
      if (!list) { Logger.log(col.label + ': 응답 실패 — 이번 회차 건너뜀 (상태 유지)'); return; }

      var cur = {}, items = [], onSale = 0;
      list.forEach(function (it) {
        if (it.sale && it.sale.price > 0) onSale++;
        var x = extract_(it);
        if (!x || TARGET_RARITY.indexOf(x.rarity) < 0) return;
        cur[x.tokenId] = { p: x.price, c: x.currency };
        items.push(x);
      });
      if (onSale > 0 && !hasAnyRarity_(list)) {
        Logger.log('⚠ ' + col.label + ': Rarity를 하나도 못 읽었다 — OKX 응답 구조 변경 의심');
      }

      var key = 'okx_state_' + col.projectId;
      var raw = props.getProperty(key);
      if (raw === null) {
        props.setProperty(key, JSON.stringify(cur));
        Logger.log(col.label + ': 첫 실행 — ' + items.length + '건 시드 (알림 없음)');
        return;
      }

      var prev = JSON.parse(raw);
      items.forEach(function (x) {
        var b = prev[x.tokenId];
        if (!b || b.c !== x.currency)  rows.push({ label: col.label, x: x, before: null });
        else if (x.price < b.p)        rows.push({ label: col.label, x: x, before: b.p });
      });
      pending.push({ key: key, json: JSON.stringify(cur) });
      Logger.log(col.label + ': 대상 ' + items.length + '건 / 판매중 ' + onSale + '건');

    } catch (e) {
      Logger.log('✖ ' + col.label + ': ' + e);
    }
  });

  // 일정을 먼저 만들고, 성공했을 때만 상태를 저장한다.
  // (여기서 실패하면 상태가 그대로라 다음 회차에 다시 알린다)
  if (rows.length) {
    var rate = fetchUsdKrw_();
    var cal  = getCalendar_();
    rows.forEach(function (r) { createEvent_(cal, r, rate, false); });
  }
  pending.forEach(function (p) { props.setProperty(p.key, p.json); });
  Logger.log('완료 — 알림 ' + rows.length + '건');
}

// ── OKX ──────────────────────────────────
/**
 * 판매 등록된 매물을 전부 가져온다. 한 건이라도 못 받으면 null (그 회차는 통째로 건너뛴다).
 *
 * makeOrderTimeDesc 정렬은 판매중 항목을 앞에 몰아서 준다(실측: 42건 연속 뒤 미판매).
 * 그래서 미판매가 섞인 페이지가 나오면 리스팅을 다 본 것이라 거기서 멈춘다.
 * 페이지를 안 넘기고 50건에서 끊으면, 리스팅이 50건을 넘는 순간 오래된 매물이
 * 창 밖으로 밀려나 상태에서 사라졌다가 다시 들어올 때 가짜 '신규' 알림이 난다.
 */
function fetchListings_(projectId) {
  var all = [], cursor = '';
  for (var page = 1; page <= MAX_PAGES; page++) {
    var d = fetchPage_(projectId, page, cursor);
    if (!d) return null;
    all = all.concat(d.list);

    var sawUnlisted = d.list.some(function (it) { return !(it.sale && it.sale.price > 0); });
    if (sawUnlisted || d.list.length < PAGE_SIZE || !d.cursor) return all;
    cursor = d.cursor;
  }
  Logger.log('⚠ project ' + projectId + ': ' + MAX_PAGES + '페이지를 넘겼다 — 리스팅 일부를 못 봤을 수 있다');
  return all;
}

function fetchPage_(projectId, pageNum, cursor) {
  var body = {
    sources: [], stateIn: [], nameSearch: '',
    sortBy: 'makeOrderTimeDesc',
    ownerAddressIn: [], contentTypes: [],
    projectIn: [projectId],
    pageNum: pageNum, pageSize: PAGE_SIZE, cursor: cursor
  };
  var res = UrlFetchApp.fetch(MARKET_API + '?t=' + Date.now(), {
    method: 'post',
    contentType: 'application/json',
    payload: JSON.stringify(body),
    headers: { 'User-Agent': 'Mozilla/5.0' },
    muteHttpExceptions: true
  });
  if (res.getResponseCode() !== 200) return null;
  try {
    var j = JSON.parse(res.getContentText());
    if (!j || j.code !== 0 || !j.data || !j.data.list) return null;
    return { list: j.data.list, cursor: j.data.cursor };
  } catch (e) { return null; }
}

/** 판매 등록된 항목만 { tokenId, rarity, price, ... }로. 아니면 null. */
function extract_(it) {
  var s = it.sale;
  if (!s || !(s.price > 0)) return null;
  var rarity = rarityOf_(it);
  if (!rarity) return null;
  return {
    tokenId:  String(it.tokenId),
    rarity:   rarity,
    price:    s.price,
    currency: s.currency,
    usd:      s.usdPrice,
    listedAt: s.lastTime,
    project:  it.projectName,
    url:      ASSET_URL + it.contractAddress + '/' + it.tokenId
  };
}

function rarityOf_(it) {
  try {
    var props = JSON.parse(it.specialProperties || '[]');
    for (var i = 0; i < props.length; i++) if (props[i].name === 'Rarity') return props[i].key;
  } catch (e) {}
  return '';
}

function hasAnyRarity_(list) {
  for (var i = 0; i < list.length; i++) if (rarityOf_(list[i])) return true;
  return false;
}

// ── 환율 (USD → KRW) ──────────────────────────────────
// Code.gs의 fetchExchangeRate()와 같은 소스. 이 프로젝트엔 config 시트가 없어 캐시만 쓴다.
function fetchUsdKrw_() {
  var cache = CacheService.getScriptCache();
  var hit = cache.get('usdkrw');
  if (hit) return { value: parseFloat(hit), estimated: false };
  try {
    var url = 'https://query1.finance.yahoo.com/v8/finance/chart/USDKRW=X?interval=1d&range=1d';
    var res = UrlFetchApp.fetch(url, { muteHttpExceptions: true, headers: { 'User-Agent': 'Mozilla/5.0' } });
    var rate = JSON.parse(res.getContentText()).chart.result[0].meta.regularMarketPrice;
    if (!(rate > 0)) throw new Error('rate=' + rate);
    cache.put('usdkrw', String(rate), 3600);
    return { value: rate, estimated: false };
  } catch (e) {
    Logger.log('환율 조회 실패 → 폴백 ' + FALLBACK_USDKRW + ' (' + e + ')');
    return { value: FALLBACK_USDKRW, estimated: true };
  }
}

// ── 캘린더 ──────────────────────────────────
function getCalendar_() {
  var found = CalendarApp.getCalendarsByName(CALENDAR_NAME);
  if (found && found.length) return found[0];
  Logger.log('캘린더 생성: ' + CALENDAR_NAME);
  return CalendarApp.createCalendar(CALENDAR_NAME);
}

function createEvent_(cal, r, rate, isTest) {
  var x    = r.x;
  var krw  = x.usd * rate.value;
  var isNew = (r.before === null);

  var title = (isTest ? '🧪 테스트 · ' : (isNew ? '🆕 ' : '🔻 '))
            + '[' + r.label + '] ' + x.rarity + ' #' + x.tokenId
            + ' · ' + num_(x.price) + ' ' + x.currency + (isNew ? '' : ' ↓' + num_(r.before))
            + ' · $' + num_(x.usd)
            + ' · ' + krwShort_(krw);

  var desc = [
    '컬렉션: ' + x.project,
    '등급:   ' + x.rarity,
    '가격:   ' + num_(x.price) + ' ' + x.currency,
    '달러:   $' + num_(x.usd),
    '원화:   ' + num_(krw) + '원  (환율 ' + rate.value.toFixed(2) + (rate.estimated ? ', 추정치' : '') + ')',
    '구분:   ' + (isTest ? '테스트 발송 — 실제 알림이 아닙니다'
                : isNew  ? '신규 리스팅'
                         : '가격 인하 ' + num_(r.before) + ' → ' + num_(x.price) + ' ' + x.currency),
    '리스팅: ' + Utilities.formatDate(new Date(x.listedAt), 'Asia/Seoul', 'yyyy-MM-dd HH:mm'),
    '',
    x.url
  ].join('\n');

  // 시작을 정확히 '지금'으로 두면 팝업이 발화하지 않을 수 있어 2분 여유를 둔다.
  var start = new Date(Date.now() + 2 * 60 * 1000);
  var end   = new Date(start.getTime() + 15 * 60 * 1000);
  cal.createEvent(title, start, end, { description: desc }).addPopupReminder(0);
  Logger.log('일정 생성 → ' + title);
}

/** 천 단위 콤마. 소수점은 버린다 (KAIA·USD·원화 모두 정수로 충분). */
function num_(n) {
  return String(Math.round(Number(n))).replace(/\B(?=(\d{3})+(?!\d))/g, ',');
}

/** 제목은 폰 알림에서 잘리므로 원화를 만원 단위로 줄인다. 본문에는 원 단위 그대로 적는다. */
function krwShort_(krw) {
  return krw >= 10000 ? num_(krw / 10000) + '만원' : num_(krw) + '원';
}


// ===== 수동 실행 =====
// 아래는 Apps Script 편집기에서 손으로 돌리는 함수들이다.

/** 30분 트리거를 하나만 남긴다. */
function setupOkxTriggers() {
  ScriptApp.getProjectTriggers().forEach(function (t) {
    if (t.getHandlerFunction() === 'checkOnce') ScriptApp.deleteTrigger(t);
  });
  ScriptApp.newTrigger('checkOnce').timeBased().everyMinutes(30).create();
  Logger.log('checkOnce 30분 트리거 1개 생성');
}

/** 일정을 만들지 않고, 지금 걸릴 대상 매물만 로그로 본다. */
function testOnce() {
  COLLECTIONS.forEach(function (col) {
    var list = fetchListings_(col.projectId);
    if (!list) { Logger.log(col.label + ': 응답 실패'); return; }
    Logger.log('── ' + col.label + ' (' + list.length + '건 조회) ──');
    list.forEach(function (it) {
      var x = extract_(it);
      if (!x || TARGET_RARITY.indexOf(x.rarity) < 0) return;
      Logger.log('  ' + x.rarity + ' #' + x.tokenId + ' · ' + num_(x.price) + ' ' + x.currency
               + ' · $' + num_(x.usd) + ' · ' + x.url);
    });
  });
}

/** 실제 매물 1건으로 테스트 일정을 만들어 알림이 오는지 확인한다. */
function sendTestEvent() {
  var best = null, bestLabel = '';
  COLLECTIONS.forEach(function (col) {
    var list = fetchListings_(col.projectId) || [];
    list.forEach(function (it) {
      var x = extract_(it);
      if (!x) return;
      var target = TARGET_RARITY.indexOf(x.rarity) >= 0;
      // 대상 등급을 우선하고, 그중 가장 최근에 올라온 것을 고른다.
      var score = (target ? 1e15 : 0) + x.listedAt;
      if (!best || score > best._score) { x._score = score; best = x; bestLabel = col.label; }
    });
  });
  if (!best) { Logger.log('✖ 매물을 하나도 못 읽었다. OKX 응답을 확인할 것'); return; }

  var rate = fetchUsdKrw_();
  createEvent_(getCalendar_(), { label: bestLabel, x: best, before: null }, rate, true);
  Logger.log('테스트 일정을 2분 뒤로 만들었다. 캘린더 앱에서 "' + CALENDAR_NAME + '" 동기화가 켜져 있어야 알림이 온다.');
}

/** 저장된 상태를 지운다. 다음 checkOnce()는 알림 없이 다시 시드한다. */
function resetState() {
  var props = PropertiesService.getScriptProperties();
  COLLECTIONS.forEach(function (col) { props.deleteProperty('okx_state_' + col.projectId); });
  Logger.log('상태 삭제 완료');
}
