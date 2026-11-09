// 메모 창 열기 + 주운 링크 보관함(pending). 저장은 순서대로 한 번에 하나씩.
// 버전을 붙여야 브라우저가 기억한 옛 창 아이콘을 쓰지 않는다
const BOARD_URL = chrome.runtime.getURL(`board.html?v=${chrome.runtime.getManifest().version}`);
const FRAME_PX = 8; // 창 아래 테두리(윈도우)

let queue = Promise.resolve();
function serial(fn) {
  const run = queue.then(fn, fn);
  queue = run.catch(() => {});
  return run;
}

async function getPending() {
  const { pending = [] } = await chrome.storage.local.get('pending');
  return pending;
}

async function setPending(items) {
  await chrome.storage.local.set({ pending: items });
  chrome.action.setBadgeBackgroundColor({ color: '#6A5BFF' });
  chrome.action.setBadgeText({ text: items.length ? String(Math.min(items.length, 999)) : '' });
}

// 게시글 제목(링크 글자)이 우선이다. 글자가 없거나 "..." 로 잘렸을 때만 연결된 페이지에서 제목을 읽어
// titles 에 둔다. 저장은 메모 창이 한다(board 를 쓰는 곳을 하나로 둔다)
const TITLE_BYTES = 300000, TITLE_JOBS = 5, TITLE_KEEP = 3000;
const CUT_TEXT = /\s*(…|\.{2,})$/;

// 제목이 없는 SNS 링크는 주소에서 아이디라도 찾는다
const SNS_FIRST_PATH = /^(www\.|m\.|mobile\.)?(instagram|x|twitter|facebook)\.com$/;
const SNS_NOT_ID = /^(p|reel|reels|tv|i|home|explore|search|hashtag|share|watch|stories|intent|status|groups|pages|photo|video)$/;

function snsHandle(url) {
  try {
    const { hostname, pathname } = new URL(url);
    const at = /^\/@([\w.-]+)/.exec(pathname);
    if (at) return '@' + at[1];
    const first = pathname.split('/')[1];
    return SNS_FIRST_PATH.test(hostname) && first && !SNS_NOT_ID.test(first) ? '@' + first : '';
  } catch (e) {
    return '';
  }
}

function decodeEntities(s) {
  const named = { amp: '&', lt: '<', gt: '>', quot: '"', apos: "'", nbsp: ' ' };
  return s.replace(/&(#x?[0-9a-f]+|[a-z]+);/gi, (all, code) => {
    if (code[0] !== '#') return named[code.toLowerCase()] || all;
    const n = /x/i.test(code[1]) ? parseInt(code.slice(2), 16) : parseInt(code.slice(1), 10);
    return n ? String.fromCodePoint(n) : all;
  });
}

function titleFromHtml(html, url) {
  // 인스타 게시물 주소에는 아이디가 없고 설명 글에 "... comments - 아이디 - 날짜" 로 들어 있다
  if (/instagram\.com\//.test(url)) {
    const insta = /comments?\s*-\s*([\w.]+)\s*-/i.exec(html);
    if (insta) return '@' + insta[1];
  }
  const meta = /<meta[^>]+property=["']og:title["'][^>]*>/i.exec(html);
  const og = meta && /content=(?:"([^"]*)"|'([^']*)')/i.exec(meta[0]);
  const tag = /<title[^>]*>([\s\S]*?)<\/title>/i.exec(html);
  const raw = (og && (og[1] || og[2])) || (tag && tag[1]) || '';
  return decodeEntities(raw).trim().replace(/\s+/g, ' ').slice(0, 200);
}

async function pageHtml(url) {
  // 로그인 정보를 보내지 않는다. 제목을 읽다가 수신거부·로그아웃 같은 링크가 실행되면 안 된다
  const res = await fetch(url, { credentials: 'omit', signal: AbortSignal.timeout(8000) });
  const type = res.headers.get('content-type') || '';
  if (!res.ok || !/html/i.test(type)) throw new Error('not html');
  const bytes = new Uint8Array(await res.arrayBuffer()).slice(0, TITLE_BYTES);
  const head = new TextDecoder('latin1').decode(bytes.slice(0, 4000));
  const charset = (/charset=["']?([\w-]+)/i.exec(type) || /charset=["']?([\w-]+)/i.exec(head) || [])[1];
  try {
    return new TextDecoder(charset || 'utf-8').decode(bytes);
  } catch (e) {
    return new TextDecoder().decode(bytes);
  }
}

async function pageTitle(url) {
  return titleFromHtml(await pageHtml(url), url);
}

function metaContent(html, names) {
  const tag = new RegExp(`<meta[^>]+(?:property|name)=["'](?:${names})["'][^>]*>`, 'i').exec(html);
  const value = tag && /content=(?:"([^"]*)"|'([^']*)')/i.exec(tag[0]);
  return value ? decodeEntities(value[1] || value[2] || '') : '';
}

function youtubeThumb(url) {
  try {
    const u = new URL(url);
    const id = /(^|\.)youtube\.com$/.test(u.hostname) ? u.searchParams.get('v') : u.hostname === 'youtu.be' ? u.pathname.slice(1) : '';
    return id ? `https://i.ytimg.com/vi/${id}/hqdefault.jpg` : '';
  } catch (e) {
    return '';
  }
}

// 마우스를 올렸을 때 보여 줄 그림과 설명
const previews = new Map();

async function pagePreview(url) {
  if (previews.has(url)) return previews.get(url);
  const out = { image: youtubeThumb(url), text: '' };
  try {
    const html = await pageHtml(url);
    const image = metaContent(html, 'og:image|twitter:image');
    if (image && !out.image) out.image = new URL(image, url).href;
    out.text = metaContent(html, 'og:description|description|twitter:description').slice(0, 160);
  } catch (e) { /* 그림 없이 제목만 보여 준다 */ }
  previews.set(url, out);
  return out;
}

// 사라진 링크 점검: 404·410 은 바로, 연결 실패는 두 번 연속일 때만 사라진 것으로 본다
const CHECK_BATCH = 60;

async function linkState(url) {
  const stop = new AbortController();
  const timer = setTimeout(() => stop.abort(), 10000);
  try {
    const res = await fetch(url, { credentials: 'omit', signal: stop.signal });
    return res.status === 404 || res.status === 410 ? 'gone' : 'ok';
  } catch (e) {
    return e.name === 'AbortError' ? 'ok' : 'fail'; // 느린 것은 사라진 것이 아니다
  } finally {
    clearTimeout(timer);
    stop.abort();
  }
}

async function checkLinks(limit) {
  const { board, linkHealth = {} } = await chrome.storage.local.get(['board', 'linkHealth']);
  if (!board) return 0;
  const urls = [...new Set(board.spaces.flatMap(s => s.items.map(i => i.url)))];
  const when = u => (linkHealth[u] ? linkHealth[u].at : 0);
  const queue = urls.sort((a, b) => when(a) - when(b)).slice(0, limit);
  const count = queue.length;
  const results = {};
  const worker = async () => {
    for (let url = queue.shift(); url; url = queue.shift()) {
      const state = await linkState(url);
      const fails = state === 'fail' ? ((linkHealth[url] || {}).fails || 0) + 1 : 0;
      results[url] = { at: Date.now(), fails, gone: state === 'gone' || fails >= 2 };
    }
  };
  await Promise.all(Array.from({ length: TITLE_JOBS }, worker));
  const keep = Object.fromEntries(urls.filter(u => linkHealth[u] && !results[u]).map(u => [u, linkHealth[u]]));
  await chrome.storage.local.set({ linkHealth: { ...keep, ...results }, lastCheck: Date.now() });
  return count;
}

async function fetchTitles(items) {
  const queue = items.filter(i => i.kind === 'link' && (!i.title || CUT_TEXT.test(i.title)));
  const found = {};
  const worker = async () => {
    for (let it = queue.shift(); it; it = queue.shift()) {
      const title = await pageTitle(it.url).catch(() => '');
      // 잘린 글자는 같은 글자로 시작하는 제목으로만 채운다(사이트 이름으로 덮지 않게)
      const start = it.title.replace(CUT_TEXT, '');
      if (title && title !== it.siteName && title.startsWith(start)) found[it.url] = title;
    }
  };
  await Promise.all(Array.from({ length: TITLE_JOBS }, worker));
  const seen = Object.entries(found);
  await serial(async () => {
    const { titles = {} } = await chrome.storage.local.get('titles');
    const merged = Object.entries({ ...titles, ...found }).slice(-TITLE_KEEP);
    if (seen.length) await chrome.storage.local.set({ titles: Object.fromEntries(merged) });
  });
}

async function openBoard(screen, tab) {
  const { windowId } = await chrome.storage.session.get('windowId');
  if (windowId) {
    try {
      await chrome.windows.update(windowId, { focused: true, state: 'normal' });
      return;
    } catch (e) { /* 사용자가 닫았다 — 새로 연다 */ }
  }
  const { bounds } = await chrome.storage.local.get('bounds');
  const win = await chrome.windows.create({
    url: BOARD_URL,
    type: 'popup',
    focused: true,
    ...(bounds || await defaultBounds(screen, tab))
  });
  await chrome.storage.session.set({ windowId: win.id });
}

// 화면의 1/3, 오른쪽 끝. 위쪽은 브라우저의 제목표시줄·탭·주소줄을 덮지 않고 페이지 높이부터 시작한다
async function defaultBounds(s, tab) {
  const main = await chrome.windows.getLastFocused();
  if (!s) s = { left: main.left, top: main.top, width: main.width, height: main.height };
  if (!tab) [tab] = await chrome.tabs.query({ active: true, windowId: main.id });
  const win = tab && tab.windowId !== main.id ? await chrome.windows.get(tab.windowId) : main;
  const pageTop = tab && tab.height ? win.top + win.height - tab.height - FRAME_PX : s.top;
  const top = Math.max(s.top, pageTop);
  const width = Math.max(420, Math.round(s.width / 3));
  return { left: s.left + s.width - width, top, width, height: s.top + s.height - top };
}

chrome.runtime.onMessage.addListener((msg, sender, reply) => {
  if (msg.t === 'capture') {
    serial(async () => {
      const batch = crypto.randomUUID();
      const at = Date.now();
      const tab = sender.tab || {};
      const items = msg.items.map(it => ({
        id: crypto.randomUUID(),
        url: it.url,
        kind: it.kind,
        title: (it.title || snsHandle(it.url)).slice(0, 200),
        post: it.post || '', // 사진과 함께 주운 링크
        batch,
        at,
        siteUrl: tab.url || sender.url || '',
        siteName: msg.frameTitle || tab.title || '',
        drag: !!msg.drag
      }));
      // 드래그는 새로 고른 것으로 바꾸고, 클릭은 쌓는다
      await setPending(msg.drag ? items : (await getPending()).concat(items));
      await openBoard(msg.screen, sender.tab);
      fetchTitles(items);
    }).then(() => reply({ ok: true }), e => reply({ ok: false, error: String(e) }));
    return true;
  }
  if (msg.t === 'preview') {
    pagePreview(msg.url).then(reply);
    return true;
  }
  if (msg.t === 'checkLinks') {
    checkLinks(Infinity).then(reply);
    return true;
  }
  if (msg.t === 'take') {
    serial(async () => {
      // kind 를 주면 그 종류만 꺼낸다(사진 칸을 누르면 사진만 들어가고 링크는 들고 있는다)
      const items = await getPending();
      const mine = i => !msg.kind || i.kind === msg.kind;
      await setPending(items.filter(i => !mine(i)));
      return items.filter(mine);
    }).then(reply);
    return true;
  }
});

chrome.action.onClicked.addListener(() => openBoard());

// 폴더의 파일이 새 버전으로 바뀌면 스스로 다시 켠다(사용자가 새로고침을 누르지 않게)
async function ensureAlarm(name, periodInMinutes) {
  if (!await chrome.alarms.get(name)) chrome.alarms.create(name, { periodInMinutes });
}
ensureAlarm('selfUpdate', 1);
ensureAlarm('linkCheck', 360);

chrome.alarms.onAlarm.addListener(async alarm => {
  if (alarm.name === 'linkCheck') return checkLinks(CHECK_BATCH);
  const res = await fetch(chrome.runtime.getURL('manifest.json'), { cache: 'no-store' });
  const { version } = await res.json();
  if (version !== chrome.runtime.getManifest().version) chrome.runtime.reload();
});

// 크기·위치를 바꾸면 기억한다
chrome.windows.onBoundsChanged.addListener(async win => {
  const { windowId } = await chrome.storage.session.get('windowId');
  if (win.id === windowId && win.state === 'normal') {
    chrome.storage.local.set({ bounds: { left: win.left, top: win.top, width: win.width, height: win.height } });
  }
});

chrome.windows.onRemoved.addListener(async id => {
  const { windowId } = await chrome.storage.session.get('windowId');
  if (id === windowId) chrome.storage.session.remove('windowId');
});

// 설치·업데이트 때 이미 열려 있는 탭에도 넣는다(새로고침 안 해도 되게)
chrome.runtime.onInstalled.addListener(async () => {
  const tabs = await chrome.tabs.query({ url: ['http://*/*', 'https://*/*'] });
  for (const tab of tabs) {
    chrome.scripting.executeScript({
      target: { tabId: tab.id, allFrames: true },
      files: ['content.js']
    }).catch(() => {});
  }
});

// 우클릭 '사진 다른 이름으로 저장'(2026-11-09 사용자: 따로 쓰던 사진 저장 확장을 줍줍으로 합침).
// 크롬은 확장 메뉴를 기본 메뉴 아래에만 둔다 — '이미지를 다른 이름으로 저장'보다 위로는 못 올린다.
// 옛 확장('사진을 JPG/PNG/WebP로 저장')처럼 형식을 바꿔 저장도 한다(1.9.1). 항목이 여럿이라 크롬이 '줍줍 링크 메모' 아래로 묶는다.
const SAVE_AS = { saveImage: '', jpeg: 'JPG', png: 'PNG', webp: 'WebP' };
function makeMenus() {
  chrome.contextMenus.removeAll(() => Object.entries(SAVE_AS).forEach(([id, f]) => chrome.contextMenus.create({
    id, title: f ? f + ' 로 바꿔 저장' : '사진 다른 이름으로 저장', contexts: ['image']
  })));
}
chrome.runtime.onInstalled.addListener(makeMenus);
chrome.runtime.onStartup.addListener(makeMenus);
chrome.contextMenus.onClicked.addListener(info => saveImage(info));
async function saveImage(info) {
  if (!info.srcUrl || !(info.menuItemId in SAVE_AS)) return;
  const type = info.menuItemId;
  if (type === 'saveImage') return chrome.downloads.download({ url: info.srcUrl, saveAs: true });
  try {
    const bmp = await createImageBitmap(await (await fetch(info.srcUrl)).blob());
    const c = new OffscreenCanvas(bmp.width, bmp.height), g = c.getContext('2d');
    if (type === 'jpeg') { g.fillStyle = '#fff'; g.fillRect(0, 0, c.width, c.height); }  // 투명 부분이 검게 되지 않게
    g.drawImage(bmp, 0, 0);
    const blob = await c.convertToBlob({ type: 'image/' + type, quality: 0.95 });
    const url = await new Promise(ok => { const r = new FileReader(); r.onload = () => ok(r.result); r.readAsDataURL(blob); });
    let name = 'image';
    if (/^https?:/.test(info.srcUrl)) try { name = decodeURIComponent(new URL(info.srcUrl).pathname.split('/').pop()).replace(/\.[^.]*$/, '').replace(/[\\/:*?"<>|]/g, '') || name; } catch (e) {}
    chrome.downloads.download({ url, filename: name.slice(0, 100) + '.' + (type === 'jpeg' ? 'jpg' : type), saveAs: true });
  } catch (e) {
    chrome.downloads.download({ url: info.srcUrl, saveAs: true });  // 못 바꾸면(막힌 사진) 원본이라도
  }
}
