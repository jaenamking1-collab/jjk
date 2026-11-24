// 앱 페이지 중계: 허용 목록의 파일만 GitHub Pages 에서 받아 넘긴다. 나머지는 404.
const ORIGIN = 'https://jaenamking1-collab.github.io/jjk/';
const ALLOW = new Set([
  'portfolio.html', 'm.html', 'calc.js', 'script_versions.json', 'dist_notice.html',
  'ticker.html', 'ticker.webmanifest', 'ticker-192.png', 'ticker-512.png', 'dist_snapshot.txt',
  'apt.html', 'apt_dong.json',
]);

export default {
  async fetch(req) {
    const url = new URL(req.url);
    let p = url.pathname.replace(/^\/+/, '');
    if (p === '') return Response.redirect(url.origin + '/m.html', 302);
    if (!ALLOW.has(p) || (req.method !== 'GET' && req.method !== 'HEAD')) return new Response('Not found', { status: 404 });
    const r = await fetch(ORIGIN + p + url.search, { cf: { cacheTtl: 60 } });
    const h = new Headers(r.headers);
    h.set('Cache-Control', 'no-cache');
    return new Response(r.body, { status: r.status, headers: h });
  },
};
