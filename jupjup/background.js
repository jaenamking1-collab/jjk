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

async function pageTitle(url) {
  // 로그인 정보를 보내지 않는다. 제목을 읽다가 수신거부·로그아웃 같은 링크가 실행되면 안 된다
  const res = await fetch(url, { credentials: 'omit', signal: AbortSignal.timeout(8000) });
  const type = res.headers.get('content-type') || '';
  if (!res.ok || !/html/i.test(type)) return '';
  const bytes = new Uint8Array(await res.arrayBuffer()).slice(0, TITLE_BYTES);
  const head = new TextDecoder('latin1').decode(bytes.slice(0, 4000));
  const charset = (/charset=["']?([\w-]+)/i.exec(type) || /charset=["']?([\w-]+)/i.exec(head) || [])[1];
  let text;
  try {
    text = new TextDecoder(charset || 'utf-8').decode(bytes);
  } catch (e) {
    text = new TextDecoder().decode(bytes);
  }
  return titleFromHtml(text, url);
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
        batch,
        at,
        siteUrl: tab.url || sender.url || '',
        siteName: msg.frameTitle || tab.title || ''
      }));
      await setPending((await getPending()).concat(items));
      await openBoard(msg.screen, sender.tab);
      fetchTitles(items);
    }).then(() => reply({ ok: true }), e => reply({ ok: false, error: String(e) }));
    return true;
  }
  if (msg.t === 'take') {
    serial(async () => {
      const items = await getPending();
      await setPending([]);
      return items;
    }).then(reply);
    return true;
  }
});

chrome.action.onClicked.addListener(() => openBoard());

// 폴더의 파일이 새 버전으로 바뀌면 스스로 다시 켠다(사용자가 새로고침을 누르지 않게)
chrome.alarms.create('selfUpdate', { periodInMinutes: 1 });
chrome.alarms.onAlarm.addListener(async () => {
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
