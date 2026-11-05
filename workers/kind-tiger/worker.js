// KIND(거래소 공시) 에서 TIGER 분배금 신고를 읽어 Apps Script 에 넘기는 중계.
// 왜: 미래에셋 사이트·KIND 가 구글 서버를 403 으로 막는다(2026-10-27~, KIND 는 10-06 실측).
// Cloudflare 에서는 KIND 가 열린다(2026-11-05 실측: 기본 배치 SEA 200).
// 날짜: 7~10월 공지 10건 모두 미래에셋 홈페이지 게시일 = KIND 공시일. 금액: 10월말 88/88 일치.
// 공개 공시만 읽는다. 정체를 꾸미는 머리글은 넣지 않는다.
// 응답 모양은 PC 중계 파일(tiger_notices.json)과 같다: { savedAt, items:[{title,date,url,...}] }.

const KIND = 'https://kind.krx.co.kr';
const LIST_URL = KIND + '/disclosure/disclosurebystocktype.do';
const REPORT = 'ETF이익금분배신고(분배금안내)(일괄공시)';

const kst = (d = new Date()) => new Date(d.getTime() + 9 * 3600e3).toISOString();
const ymd = d => kst(d).slice(0, 10);

async function text(url, init) {
  const r = await fetch(url, init);
  if (r.status !== 200) throw new Error(url.split('?')[0] + ' ' + r.status);
  return r.text();
}

// 목록: 분배 신고 + 종목명 TIGER. 한 회차가 종목 수만큼(10/28 은 88행) 나온다.
async function listRows(from, to) {
  const rows = [];
  for (let p = 1; p <= 5; p++) {
    const body = new URLSearchParams({
      method: 'searchDisclosureByStockTypeEtfSub', forward: 'disclosurebystocktype_etf_sub',
      currentPageSize: '100', pageIndex: String(p), orderMode: '1', orderStat: 'D',
      reportNm: REPORT, etfIsuSrtNm: 'TIGER', fromDate: from, toDate: to
    });
    const h = await text(LIST_URL, { method: 'POST', body,
      headers: { 'Content-Type': 'application/x-www-form-urlencoded; charset=UTF-8' } });
    const re = /<td class="txc">([\d\- :]+)<\/td>\s*<td>[\s\S]*?title='([^']*)'[\s\S]*?openDisclsViewer\('(\d+)','[^']*'\)" title='([^']*)'/g;
    let m, n = 0;
    while ((m = re.exec(h))) {
      n++;
      if (m[4] === REPORT && /^TIGER/.test(m[2].trim())) rows.push({ time: m[1].trim(), name: m[2].trim(), acptNo: m[3] });
    }
    if (n < 100) break;
  }
  return rows;
}

// 접수번호 → 원문 주소. 주소를 짐작해 만들지 않고 뷰어 응답에서 읽는다(정정공시 대비).
async function docUrl(acptNo) {
  const v = await text(KIND + '/common/disclsviewer.do?method=search&acptno=' + acptNo);
  const doc = (v.match(/<option value='(\d+)\|[YN]'\s*selected/) || v.match(/<option value='(\d+)\|/) || [])[1];
  if (!doc) throw new Error('docNo 없음 ' + acptNo);
  const c = await text(KIND + '/common/disclsviewer.do?method=searchContents&docNo=' + doc);
  const u = (c.match(/setPath\('[^']*','(https:\/\/kind\.krx\.co\.kr\/external\/[^']+\.htm)'/) || [])[1];
  if (!u) throw new Error('원문 주소 없음 ' + doc);
  return u;
}

// 원문 표: ISIN | 종목명 | 지급기준일 | 지급예정일 | 분배금(원). 다른 운용사 행도 섞여 있어 TIGER 만 고른다.
async function docRows(url) {
  const h = await text(url);
  const t = h.replace(/<[^>]+>/g, '|').replace(/&amp;|&#38;/g, '&').replace(/\s+/g, ' ').replace(/(\| ?)+/g, '|');
  const out = [], re = /KR7(\w{6})\d{3}\|([^|]+)\|(\d{4}-\d\d-\d\d)\|(\d{4}-\d\d-\d\d)\|([\d,]+)\|/g;
  let m;
  while ((m = re.exec(t))) {
    const name = m[2].trim();
    if (/^TIGER/.test(name)) out.push({ ticker: m[1], name, record: m[3], pay: m[4], amount: Number(m[5].replace(/,/g, '')) });
  }
  return out;
}

async function build(days) {
  const now = new Date(), from = ymd(new Date(now.getTime() - days * 86400e3)), to = ymd(now);
  const rows = await listRows(from, to);
  const by = {};
  rows.forEach(r => { (by[r.acptNo] = by[r.acptNo] || { acptNo: r.acptNo, time: r.time, count: 0 }).count++; });
  const groups = Object.values(by).sort((a, b) => b.time.localeCompare(a.time)).slice(0, 6);
  const items = [];
  for (const g of groups) {
    const url = await docUrl(g.acptNo);
    const dr = await docRows(url);
    const rec = dr.length ? dr[0].record : '';
    const [y, mo, d] = rec.split('-').map(Number);
    const cyc = d <= 20 ? ' 중순' : '말';   // 앱과 같은 규칙: 기준일 20일 이하 = 월중
    items.push({
      title: rec ? `TIGER ETF ${y}년 ${mo}월${cyc} 분배금 지급 안내` : 'TIGER ETF 분배금 지급 안내',
      date: g.time.slice(0, 10).replace(/-/g, '.'),
      time: g.time, url, acptNo: g.acptNo, listed: g.count, record: rec,
      pay: dr.length ? dr[0].pay : '', rows: dr
    });
  }
  return { savedAt: kst().slice(0, 16).replace('T', ' '), source: 'kind', from, to, items };
}

export default {
  async fetch(req, env, ctx) {
    const u = new URL(req.url);
    if (u.pathname !== '/tiger') return new Response('not found', { status: 404 });
    const days = Math.min(Math.max(Number(u.searchParams.get('days')) || 45, 1), 120);
    const key = new Request('https://cache.local/tiger?days=' + days);
    const fresh = u.searchParams.has('fresh');
    if (!fresh) {
      const hit = await caches.default.match(key);
      if (hit) return hit;
    }
    try {
      const body = JSON.stringify(await build(days));
      const res = new Response(body, { headers: { 'Content-Type': 'application/json; charset=utf-8', 'Cache-Control': 'max-age=1800' } });
      ctx.waitUntil(caches.default.put(key, res.clone()));
      return res;
    } catch (e) {
      return new Response(JSON.stringify({ error: String(e && e.message || e) }), { status: 502,
        headers: { 'Content-Type': 'application/json; charset=utf-8' } });
    }
  }
};
