// 메모 창 열기 + 주운 링크 보관함(pending). 저장은 순서대로 한 번에 하나씩.
const BOARD_URL = chrome.runtime.getURL('board.html');
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
        batch,
        at,
        siteUrl: tab.url || sender.url || '',
        siteName: tab.title || ''
      }));
      await setPending((await getPending()).concat(items));
      await openBoard(msg.screen, sender.tab);
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
