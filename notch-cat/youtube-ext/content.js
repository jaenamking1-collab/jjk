// 유튜브·유튜브 뮤직 화면에서 다음 곡 목록을 읽어 2초마다 노치 고양이에 보낸다.
(() => {
  const music = location.host === 'music.youtube.com';
  const txt = el => ((el && el.textContent) || '').trim().replace(/\s+/g, ' ');
  const vid = h => { const m = /[?&]v=([\w-]{11})/.exec(h || ''); return m ? m[1] : ''; };
  let els = []; // 마지막으로 보낸 목록의 화면 요소(누르기용)

  function collect() {
    const v = document.querySelector('video');
    const playing = !!(v && !v.paused && !v.ended && v.readyState > 2);
    let kind = '', items = [];
    els = [];
    if (music) {
      const all = [...document.querySelectorAll('ytmusic-player-queue #contents ytmusic-player-queue-item, ytmusic-player-queue-item')]
        .filter((e, i, a) => a.indexOf(e) === i);
      const cur = all.findIndex(e => e.hasAttribute('selected') || /playing|paused/.test(e.getAttribute('play-button-state') || ''));
      els = cur >= 0 ? all.slice(cur + 1) : all;
      kind = '유튜브 뮤직';
      items = els.map(e => {
        const t = txt(e.querySelector('.song-title'));
        const img = (e.querySelector('img') || {}).src || '';
        return { t, c: txt(e.querySelector('.byline')), k: t, img: img.startsWith('http') ? img : '' };
      });
    } else if (location.pathname === '/watch') {
      const pl = document.querySelector('#secondary ytd-playlist-panel-renderer #items') || document.querySelector('ytd-playlist-panel-renderer #items');
      const rows = pl ? [...pl.querySelectorAll('ytd-playlist-panel-video-renderer')] : [];
      if (rows.length) {
        const cur = rows.findIndex(e => e.hasAttribute('selected'));
        els = rows.slice(cur + 1);
        kind = '재생목록';
        items = els.map(e => {
          const id = vid((e.querySelector('a#wc-endpoint, a[href*="watch?v="]') || {}).href);
          return { t: txt(e.querySelector('#video-title')), c: txt(e.querySelector('#byline')), k: id || txt(e.querySelector('#video-title')), img: id ? `https://i.ytimg.com/vi/${id}/mqdefault.jpg` : '' };
        });
      } else {
        const seen = new Set();
        els = [...document.querySelectorAll('#secondary #related ytd-compact-video-renderer, #secondary #related yt-lockup-view-model, #secondary ytd-watch-next-secondary-results-renderer yt-lockup-view-model')]
          .filter(e => {
            const id = vid((e.querySelector('a[href*="watch?v="]') || {}).href);
            if (!id || seen.has(id)) return false;
            seen.add(id);
            return true;
          })
          .slice(0, 50);
        kind = '다음 동영상';
        items = els.map(e => {
          const a = e.querySelector('a[href*="watch?v="]');
          const id = vid(a.href);
          const t = txt(e.querySelector('#video-title, h3, [class*="lockup-metadata"] [class*="title"]')) || (a.title || '');
          const c = txt(e.querySelector('ytd-channel-name #text, #channel-name #text, [class*="metadata-row"]'));
          return { t, c, k: id, img: `https://i.ytimg.com/vi/${id}/mqdefault.jpg` };
        });
      }
    }
    items = items.filter(i => i.t);
    return { playing, kind, items };
  }

  function play(k) {
    const e = els.find(e => {
      const a = e.querySelector('a[href*="watch?v="], a#wc-endpoint');
      return (a && vid(a.href) === k) || txt(e.querySelector('.song-title, #video-title')) === k;
    });
    if (!e) return;
    if (music) {
      const b = e.querySelector('ytmusic-play-button-renderer');
      (b || e).click();
    } else {
      const a = e.querySelector('a#wc-endpoint, a[href*="watch?v="]');
      (a || e).click();
    }
  }

  function send() {
    let data;
    try { data = collect(); } catch (_) { return; }
    if (!data.items.length && !data.playing) return;
    try {
      chrome.runtime.sendMessage(data, r => {
        if (chrome.runtime.lastError) return;
        if (r && r.play) play(r.play);
      });
    } catch (_) {
      clearInterval(timer); // 확장을 다시 불러오면 옛 화면의 연결은 끊긴다
    }
  }
  const timer = setInterval(send, 2000);
})();
