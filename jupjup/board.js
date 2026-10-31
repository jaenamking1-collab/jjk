// 메모 판 화면. 판은 몇 개든 저장하고, shown 은 화면 자리(최대 4)에 놓인 판, layout 은 그중 몇 자리를 보일지 정한다.
const KIND = { link: '🔗', image: '🖼️', video: '🎬' };
const ICON = {
  copy: '<svg viewBox="0 0 16 16"><rect x="5.5" y="5.5" width="8" height="8" rx="2"/><path d="M10.5 3H4a1 1 0 0 0-1 1v6.5"/></svg>',
  check: '<svg viewBox="0 0 16 16"><path d="M3 8.5 6.5 12 13 4.5"/></svg>',
  del: '<svg viewBox="0 0 16 16"><path d="M4 4l8 8M12 4l-8 8"/></svg>'
};
const $ = id => document.getElementById(id);
const spacesEl = $('spaces');

let board = { layout: 1, shown: [0, 1, 2, 3], spaces: [0, 1, 2, 3].map(() => ({ title: '', items: [] })) };
let pendingCount = 0;
let drag = null;
let undoState = null, toastTimer = 0;
let activeSlot = 0;

const visible = () => board.shown.slice(0, board.layout);
const spaceName = i => board.spaces[i].title || `제목 없는 판 ${i + 1}`;

// 1.0 저장본에는 shown 이 없다
function withShown(b) {
  return { shown: [0, 1, 2, 3], ...b };
}

function save() {
  return chrome.storage.local.set({ board });
}

function el(tag, cls, text) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text != null) e.textContent = text;
  return e;
}

function shortUrl(url) {
  let s = url.replace(/^https?:\/\/(www\.)?/, '').replace(/\/$/, '');
  try { s = decodeURI(s); } catch (e) { /* 그대로 쓴다 */ }
  return s;
}

function host(url) {
  try { return new URL(url).hostname.replace(/^www\./, ''); } catch (e) { return url; }
}

function when(at) {
  const d = new Date(at);
  const p = n => String(n).padStart(2, '0');
  return `${d.getFullYear()}.${p(d.getMonth() + 1)}.${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
}

function itemRow(item, n) {
  const row = el('div', 'row');
  row.draggable = true;
  row.dataset.id = item.id;
  row.append(el('span', 'num', n), el('span', 'kind', KIND[item.kind] || KIND.link));

  // 줄어들 때 가운데가 접혀 주소의 끝부분이 남는다
  const text = shortUrl(item.url);
  const cut = text.length > 30 ? text.length - 14 : text.length;
  const link = el('a', 'url');
  link.href = item.url;
  link.target = '_blank';
  link.rel = 'noreferrer';
  link.title = item.url;
  link.append(el('span', 'head', text.slice(0, cut)), el('span', 'tail', text.slice(cut)));

  const copy = el('button', 'act copy');
  copy.innerHTML = ICON.copy;
  copy.title = '이 링크 복사';
  const del = el('button', 'act del');
  del.innerHTML = ICON.del;
  del.title = '삭제';
  row.append(link, copy, del);
  return row;
}

function metaRow(item, size) {
  const meta = el('div', 'meta');
  meta.draggable = true;
  meta.dataset.batch = item.batch;
  meta.title = '끌면 이 묶음이 통째로 옮겨져요';
  if (item.siteName) {
    const name = el('span', 'name', item.siteName);
    name.title = item.siteName;
    meta.append(name);
  }
  const site = el('a', '', host(item.siteUrl));
  site.href = item.siteUrl;
  site.target = '_blank';
  site.rel = 'noreferrer';
  site.title = item.siteUrl;
  meta.append(site, el('time', '', when(item.at)));
  if (size > 1) {
    const bulk = el('span', 'bulk');
    bulk.append(el('button', 'bulk-copy', `${size}개 복사`), el('button', 'bulk-del', '모두 삭제'));
    meta.append(bulk);
  }
  return meta;
}

function emptyView() {
  const box = el('div', 'empty');
  box.innerHTML = '<svg><use href="#joobi"/></svg><strong>아직 비어 있어요</strong>' +
    '<span><kbd>Shift+클릭</kbd> 한 개 줍기<br><kbd>Shift+드래그</kbd> 페이지 전체 줍기</span>';
  return box;
}

function fillList(list, items) {
  const nodes = [];
  items.forEach((item, i) => {
    if (i === 0 || items[i - 1].batch !== item.batch) {
      let size = 1;
      while (items[i + size] && items[i + size].batch === item.batch) size++;
      nodes.push(metaRow(item, size));
    }
    nodes.push(itemRow(item, i + 1));
  });
  list.replaceChildren(...(nodes.length ? nodes : [emptyView()]));
}

function buildSpaces() {
  const frag = [];
  for (const i of visible()) {
    const sp = el('section', 'space');
    sp.dataset.i = i;
    sp.dataset.c = i % 4;
    const head = el('div', 'space-head');
    const title = el('input', 'title');
    title.placeholder = '큰제목 쓰기';
    title.value = board.spaces[i].title;
    title.setAttribute('aria-label', `${i + 1}번 판 제목`);
    title.maxLength = 60;
    head.append(title, el('span', 'count'));
    sp.append(head, el('div', 'list'));
    frag.push(sp);
  }
  spacesEl.replaceChildren(...frag);
  spacesEl.dataset.key = viewKey();
  spacesEl.dataset.layout = board.layout;
  spacesEl.classList.toggle('compact', board.layout === 4);
  document.querySelectorAll('.layouts button').forEach(b => {
    b.setAttribute('aria-pressed', String(+b.dataset.layout === board.layout));
  });
}

function viewKey() {
  return board.layout + ':' + visible().join();
}

function render() {
  if (spacesEl.dataset.key !== viewKey()) buildSpaces();
  [...spacesEl.children].forEach(sp => {
    const space = board.spaces[+sp.dataset.i];
    const title = sp.querySelector('.title');
    if (document.activeElement !== title) title.value = space.title;
    sp.querySelector('.count').textContent = space.items.length ? `${space.items.length}개` : '';
    fillList(sp.querySelector('.list'), space.items);
  });
  const all = board.spaces.reduce((n, s) => n + s.items.length, 0);
  $('total').textContent = all ? `링크 ${all}개` : '';
  $('shelfBtn').textContent = `목록 ${board.spaces.length}`;
  renderShelf();
}

// 판 목록
function renderShelf() {
  const open = !$('shelf').hidden;
  $('shelfBtn').setAttribute('aria-expanded', String(open));
  if (!open) return;
  const on = visible();
  const rows = board.spaces.map((space, i) => {
    const row = el('div', 'shelf-row');
    row.dataset.i = i;
    row.dataset.c = i % 4;
    row.classList.toggle('on', on.includes(i));
    const pick = el('button', 'shelf-pick');
    pick.append(el('span', 'dot'), el('span', 'shelf-name', spaceName(i)), el('span', 'shelf-count', `${space.items.length}개`));
    row.append(pick);
    if (board.spaces.length > 4) {
      const del = el('button', 'act del');
      del.innerHTML = ICON.del;
      del.title = '이 판 삭제';
      row.append(del);
    }
    return row;
  });
  $('shelfList').replaceChildren(...rows);
}

function toggleShelf(open) {
  $('shelf').hidden = !open;
  renderShelf();
}

function showSpace(i) {
  if (visible().includes(i)) return;
  const slot = Math.min(activeSlot, board.layout - 1);
  const at = board.shown.indexOf(i);
  if (at >= 0) board.shown[at] = board.shown[slot];
  board.shown[slot] = i;
  save();
  render();
}

function addSpace() {
  board.spaces.push({ title: '', items: [] });
  const i = board.spaces.length - 1;
  showSpace(i);
  toggleShelf(false);
  spacesEl.querySelector(`[data-i="${i}"] .title`).focus();
}

function deleteSpace(i) {
  const before = JSON.stringify(board);
  const n = board.spaces[i].items.length;
  board.spaces.splice(i, 1);
  const shown = board.shown.filter(s => s !== i).map(s => (s > i ? s - 1 : s));
  for (let s = 0; shown.length < 4; s++) if (!shown.includes(s)) shown.push(s);
  board.shown = shown;
  save();
  render();
  toast(`판을 삭제했어요${n ? ` (링크 ${n}개)` : ''}`, before);
}

function renderDock() {
  document.body.classList.toggle('holding', pendingCount > 0);
  $('dock').hidden = !pendingCount;
  $('dockText').textContent = `링크 ${pendingCount}개 · 넣을 판을 눌러요`;
}

async function paste(index) {
  const items = await chrome.runtime.sendMessage({ t: 'take' });
  if (!items || !items.length) return;
  board.spaces[index].items.push(...items);
  await save();
  render();
  const list = spacesEl.querySelector(`[data-i="${index}"] .list`);
  if (list) list.scrollTop = list.scrollHeight;
  else toast(`${spaceName(index)}에 ${items.length}개 넣었어요`);
}

function toast(text, undo) {
  undoState = undo || null;
  $('toastText').textContent = text;
  $('toastUndo').hidden = !undo;
  $('toast').hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { $('toast').hidden = true; undoState = null; }, undo ? 6000 : 1500);
}

function removeItems(index, keep, label) {
  undoState = JSON.stringify(board);
  const before = undoState;
  board.spaces[index].items = board.spaces[index].items.filter(keep);
  save();
  render();
  toast(label, before);
}

function findItem(index, id) {
  return board.spaces[index].items.find(i => i.id === id);
}

async function copyText(text) {
  await navigator.clipboard.writeText(text);
}

function flashDone(btn) {
  btn.innerHTML = ICON.check;
  btn.classList.add('done');
  setTimeout(() => {
    btn.innerHTML = ICON.copy;
    btn.classList.remove('done');
  }, 1000);
}

// 들고 있는 링크가 있으면 칸을 누르는 것이 곧 붙여넣기다
spacesEl.addEventListener('click', e => {
  const sp = e.target.closest('.space');
  if (!sp) return;
  const index = +sp.dataset.i;
  activeSlot = [...spacesEl.children].indexOf(sp);
  const btn = e.target.closest('button');

  if (btn && btn.classList.contains('copy')) {
    const row = btn.closest('.row');
    copyText(findItem(index, row.dataset.id).url).then(() => flashDone(btn));
    return;
  }
  if (btn && btn.classList.contains('del')) {
    const id = btn.closest('.row').dataset.id;
    removeItems(index, i => i.id !== id, '삭제했어요');
    return;
  }
  if (btn && btn.classList.contains('bulk-copy')) {
    const batch = btn.closest('.meta').dataset.batch;
    const urls = board.spaces[index].items.filter(i => i.batch === batch).map(i => i.url);
    copyText(urls.join('\n')).then(() => toast(`${urls.length}개 복사했어요`));
    return;
  }
  if (btn && btn.classList.contains('bulk-del')) {
    const batch = btn.closest('.meta').dataset.batch;
    const n = board.spaces[index].items.filter(i => i.batch === batch).length;
    removeItems(index, i => i.batch !== batch, `${n}개 삭제했어요`);
    return;
  }
  if (pendingCount && !e.target.closest('.title')) {
    e.preventDefault();
    paste(index);
  }
});

let titleTimer = 0;
spacesEl.addEventListener('input', e => {
  if (!e.target.classList.contains('title')) return;
  board.spaces[+e.target.closest('.space').dataset.i].title = e.target.value;
  clearTimeout(titleTimer);
  titleTimer = setTimeout(save, 300);
});

// 끌어 옮기기
function dropLine() {
  const line = document.querySelector('.drop-line') || el('div', 'drop-line');
  return line;
}

function clearDrag() {
  document.querySelector('.drop-line')?.remove();
  document.querySelectorAll('.dragging').forEach(n => n.classList.remove('dragging'));
  drag = null;
}

spacesEl.addEventListener('dragstart', e => {
  const row = e.target.closest?.('.row');
  const meta = e.target.closest?.('.meta');
  if (!row && !meta) return;
  const from = +e.target.closest('.space').dataset.i;
  const ids = row
    ? [row.dataset.id]
    : board.spaces[from].items.filter(i => i.batch === meta.dataset.batch).map(i => i.id);
  drag = { from, ids };
  e.dataTransfer.effectAllowed = 'move';
  const urls = ids.map(id => findItem(from, id).url);
  e.dataTransfer.setData('text/plain', urls.join('\n'));
  (row || meta).classList.add('dragging');
});

spacesEl.addEventListener('dragover', e => {
  if (!drag) return;
  const list = e.target.closest('.space')?.querySelector('.list');
  if (!list) return;
  e.preventDefault();
  e.dataTransfer.dropEffect = 'move';
  if (list.querySelector('.empty')) return; // 안내를 지우면 끌던 대상이 사라져 놓기가 끊긴다
  const rows = [...list.querySelectorAll('.row')];
  const next = rows.find(r => {
    const box = r.getBoundingClientRect();
    return e.clientY < box.top + box.height / 2;
  });
  const line = dropLine();
  if (next) list.insertBefore(line, next);
  else list.append(line);
});

spacesEl.addEventListener('drop', e => {
  if (!drag) return;
  const sp = e.target.closest('.space');
  if (!sp) return;
  e.preventDefault();
  const line = sp.querySelector('.drop-line');
  moveDragged(+sp.dataset.i, line?.nextElementSibling?.dataset.id);
});

function moveDragged(to, beforeId) {
  const moving = drag.ids.map(id => findItem(drag.from, id));
  const ids = new Set(drag.ids);
  board.spaces[drag.from].items = board.spaces[drag.from].items.filter(i => !ids.has(i.id));
  const target = board.spaces[to].items;
  const at = beforeId && !ids.has(beforeId) ? target.findIndex(i => i.id === beforeId) : -1;
  target.splice(at < 0 ? target.length : at, 0, ...moving);
  clearDrag();
  save();
  render();
}

spacesEl.addEventListener('dragend', () => {
  if (drag) { clearDrag(); render(); }
});

document.querySelector('.layouts').addEventListener('click', e => {
  const btn = e.target.closest('button');
  if (!btn) return;
  board.layout = +btn.dataset.layout;
  save();
  render();
});

$('shelfBtn').addEventListener('click', e => {
  e.stopPropagation();
  toggleShelf($('shelf').hidden);
});

$('shelfAdd').addEventListener('click', addSpace);

$('shelfList').addEventListener('click', e => {
  const row = e.target.closest('.shelf-row');
  if (!row) return;
  const i = +row.dataset.i;
  if (e.target.closest('.del')) return deleteSpace(i);
  if (pendingCount) paste(i);
  else showSpace(i);
  toggleShelf(false);
});

// 목록의 판 위에 놓으면 그 판 맨 뒤로 옮긴다(화면에 없는 판으로도 옮길 수 있게)
$('shelfList').addEventListener('dragover', e => {
  const row = e.target.closest('.shelf-row');
  if (!drag || !row) return;
  e.preventDefault();
  document.querySelector('.shelf-row.over')?.classList.remove('over');
  row.classList.add('over');
});

$('shelfList').addEventListener('drop', e => {
  const row = e.target.closest('.shelf-row');
  if (!drag || !row) return;
  e.preventDefault();
  moveDragged(+row.dataset.i);
});

document.addEventListener('click', e => {
  if (!$('shelf').hidden && !e.target.closest('#shelf')) toggleShelf(false);
});

$('dockCancel').addEventListener('click', async () => {
  await chrome.runtime.sendMessage({ t: 'take' });
});

$('toastUndo').addEventListener('click', () => {
  if (!undoState) return;
  board = JSON.parse(undoState);
  undoState = null;
  save();
  render();
  $('toast').hidden = true;
});

chrome.storage.onChanged.addListener((changes, area) => {
  if (area !== 'local') return;
  if (changes.pending) {
    pendingCount = (changes.pending.newValue || []).length;
    renderDock();
  }
  if (changes.board && !drag && JSON.stringify(changes.board.newValue) !== JSON.stringify(board)) {
    board = withShown(changes.board.newValue);
    render();
  }
});

(async () => {
  const data = await chrome.storage.local.get(['board', 'pending']);
  if (data.board) board = withShown(data.board);
  pendingCount = (data.pending || []).length;
  render();
  renderDock();
})();
