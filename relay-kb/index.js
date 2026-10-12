// KB자산운용 공지(kbam.co.kr) 중계 — 구글 클라우드 서울(asia-northeast3)에서 돈다.
// 왜: 2026-10-05 이후 kbam.co.kr(AWS 서울)이 해외 접속을 연결 단계에서 끊는다. Apps Script·GitHub Actions 는
//     해외라 못 읽는다. 한국 리전의 이 함수가 대신 받아 그대로 돌려준다(Code.gs `_kbFetch`).
// 열쇠 없이 공개로 둔다 — 대신 ① KB 공지 경로만 ② 경로마다 60초 캐시(같은 주소를 KB 에 1분 1번 넘게 안 보낸다)
// ③ 배포 때 max-instances=1. 그래서 남이 두드려도 KB 에 부담을 넘기거나 요금이 쌓이지 않는다.
const functions = require('@google-cloud/functions-framework');

const ALLOW = /^\/support\/notice(\/category\/\d+|\/\d+)$/;
const cache = new Map();   // key → { at, status, type, body }

functions.http('kb', async (req, res) => {
  const p = String(req.query.p || '');
  const rsc = req.query.rsc === '1';
  if (!ALLOW.test(p)) return res.status(400).send('bad path');
  const key = p + (rsc ? '|rsc' : '');
  const hit = cache.get(key);
  if (hit && Date.now() - hit.at < 60000) return res.status(hit.status).type(hit.type).set('X-Relay-Cache', 'hit').send(hit.body);
  try {
    const ctl = new AbortController();
    const t = setTimeout(() => ctl.abort(), 20000);
    const r = await fetch('https://kbam.co.kr' + p, {
      headers: Object.assign({ 'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)', 'Accept-Language': 'ko-KR,ko;q=0.9' }, rsc ? { RSC: '1' } : {}),
      signal: ctl.signal
    });
    clearTimeout(t);
    const body = await r.text();
    const v = { at: Date.now(), status: r.status, type: r.headers.get('content-type') || 'text/plain; charset=utf-8', body };
    if (r.ok) cache.set(key, v);
    res.status(v.status).type(v.type).set('X-Relay-Cache', 'miss').send(body);
  } catch (e) {
    res.status(502).send('relay error: ' + e);
  }
});
