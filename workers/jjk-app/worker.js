// 앱 페이지 중계: 허용 목록의 파일만 GitHub Pages 에서 받아 넘긴다. 나머지는 404.
const ORIGIN = 'https://jaenamking1-collab.github.io/jjk/';
const ALLOW = new Set([
  'portfolio.html', 'm.html', 'calc.js', 'script_versions.json', 'dist_notice.html',
  'ticker.html', 'ticker.webmanifest', 'ticker-192.png', 'ticker-512.png', 'dist_snapshot.txt',
  'apt.html', 'apt_dong.json',
]);
// 노치 새 판: 학교 망이 GitHub 릴리스 파일 서버를 끊어서(2026-11-26) 여기로도 받게 한다. 받는 쪽이 GitHub 지문과 대조한다.
const NOTCH = 'https://github.com/jaenamking1-collab/jjk/releases/download/notch/NotchCat.exe';

export default {
  async fetch(req) {
    const url = new URL(req.url);
    let p = url.pathname.replace(/^\/+/, '');
    if (p === '') return Response.redirect(url.origin + '/m.html', 302);
    if (req.method !== 'GET' && req.method !== 'HEAD') return new Response('Not found', { status: 404 });
    if (p === 'NotchCat.exe') {
      const r = await fetch(NOTCH, { method: req.method, cf: { cacheTtl: 0 } });
      if (!r.ok) return new Response('Upstream error', { status: 502 });
      const h = new Headers({
        'Content-Type': 'application/octet-stream',
        'Content-Disposition': 'attachment; filename="NotchCat.exe"',
        'Cache-Control': 'no-store',
      });
      const len = r.headers.get('Content-Length');
      if (len) h.set('Content-Length', len);
      return new Response(r.body, { headers: h });
    }
    if (!ALLOW.has(p)) return new Response('Not found', { status: 404 });
    const r = await fetch(ORIGIN + p + url.search, { cf: { cacheTtl: 60 } });
    const h = new Headers(r.headers);
    h.set('Cache-Control', 'no-cache');
    return new Response(r.body, { status: r.status, headers: h });
  },
};
