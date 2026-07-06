/**
 * 공개 분배금 프록시 GAS  (public_dist_proxy.gs)
 * ------------------------------------------------------------------
 * 목적: 분배금공지 공개 페이지가 개인 데이터 유출 없이 백엔드를 호출하도록,
 *       분배·공지 관련 "읽기" 액션만 비공개(개인) GAS로 중계한다.
 *
 * 안전 설계:
 *  - getDistribution / getEtfNotices 두 액션만 화이트리스트로 통과.
 *    getHoldings·getAccounts·getDividends 등 개인 데이터 액션은 전달되지 않는다.
 *  - PRIVATE_URL(개인 GAS exec)은 이 소스 안에만 존재. 배포된 웹앱의 소스 코드는
 *    브라우저에서 다운로드되지 않으므로 공개 페이지 방문자에게 노출되지 않는다.
 *  - force 파라미터는 무시 → 공개 사용자가 강제 재파싱으로 백엔드를 혹사시킬 수 없다.
 *  - 결과는 30분 CacheService 캐시 → 방문자가 많아도 개인 GAS 호출은 최소화.
 *
 * 배포:
 *  1) 새 Apps Script 프로젝트를 만들고 이 파일 내용을 붙여넣는다.
 *  2) 아래 PRIVATE_URL 을 "개인 GAS의 exec URL"로 교체한다.
 *     ⚠️ 이 값은 Apps Script 편집기 안에만 둔다. 공개 저장소/페이지에 커밋 금지.
 *  3) 배포 → 웹앱 → 액세스 "모든 사용자(익명 포함)" 로 배포.
 *  4) 발급된 공개 exec URL 을 공개 페이지(dist_notice.html)의 API 상수로 사용.
 */

// ⚠️ 실제 값은 Apps Script 편집기에서만 교체. 이 파일(저장소)에는 placeholder 유지.
const PRIVATE_URL = 'PASTE_PRIVATE_GAS_EXEC_URL_HERE';

// 공개 허용 액션(분배금 공지 전용 · 읽기만)
const ALLOWED_ACTIONS = ['getDistribution', 'getEtfNotices'];

function doGet(e) {
  const p = (e && e.parameter) || {};
  const action = p.action;

  // 화이트리스트 밖(개인 데이터 포함)은 즉시 거부 — 절대 중계하지 않음.
  if (ALLOWED_ACTIONS.indexOf(action) === -1) {
    return _json({ error: 'Not allowed' });
  }

  // force 제거: 공개 사용자가 강제 재파싱으로 백엔드를 혹사시키지 못하게 한다.
  const params = {};
  Object.keys(p).forEach(function (k) { if (k !== 'force') params[k] = p[k]; });

  const cacheKey = 'pub_' + Object.keys(params).sort()
    .map(function (k) { return k + '=' + params[k]; }).join('&');
  const cache = CacheService.getScriptCache();
  const hit = cache.get(cacheKey);
  if (hit) return _raw(hit);

  const qs = Object.keys(params).map(function (k) {
    return encodeURIComponent(k) + '=' + encodeURIComponent(params[k]);
  }).join('&');

  try {
    const res = UrlFetchApp.fetch(PRIVATE_URL + '?' + qs, {
      muteHttpExceptions: true, followRedirects: true
    });
    const body = res.getContentText();
    cache.put(cacheKey, body, 1800); // 30분
    return _raw(body);
  } catch (err) {
    return _json({ error: err.toString() });
  }
}

function _json(obj) {
  return ContentService.createTextOutput(JSON.stringify(obj))
    .setMimeType(ContentService.MimeType.JSON);
}
function _raw(str) {
  return ContentService.createTextOutput(str)
    .setMimeType(ContentService.MimeType.JSON);
}
