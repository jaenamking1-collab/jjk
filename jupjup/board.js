// 메모 판 화면. 판은 몇 개든 저장하고, shown 은 화면 자리(최대 4)에 놓인 판, layout 은 그중 몇 자리를 보일지 정한다.
const KIND = { link: '🔗', image: '🖼️', video: '🎬' };
const ICON = {
  copy: '<svg viewBox="0 0 16 16"><rect x="5.5" y="5.5" width="8" height="8" rx="2"/><path d="M10.5 3H4a1 1 0 0 0-1 1v6.5"/></svg>',
  check: '<svg viewBox="0 0 16 16"><path d="M3 8.5 6.5 12 13 4.5"/></svg>',
  del: '<svg viewBox="0 0 16 16"><path d="M4 4l8 8M12 4l-8 8"/></svg>',
  memo: '<svg viewBox="0 0 16 16"><path d="M3 13l.8-3L10.500 3.300l2.200 2.200L6 12.200z"/></svg>',
  down: '<svg viewBox="0 0 16 16"><path d="M8 2.500v8M4.500 7.500 8 11l3.500-3.500M3 13.500h10"/></svg>'
};
const PHOTO_MAX = 12; // 사진 칸 하나에 4장씩 3줄
const isPhoto = item => item.kind === 'image';
const $ = id => document.getElementById(id);
const spacesEl = $('spaces');

let board = { layout: 1, shown: [0, 1, 2, 3], spaces: [0, 1, 2, 3].map(() => ({ title: '', items: [] })) };
let pendingCount = 0, pending = [];
let drag = null;
let undoState = null, toastTimer = 0;
let activeSlot = 0;
let health = {}, lastCheck = 0;
let previewTimer = 0, previewFor = '';

const visible = () => board.shown.slice(0, board.layout);
const spaceName = i => board.spaces[i].title || `제목 없는 판 ${i + 1}`;

// 1.0 저장본에는 shown 이 없다
function withShown(b) {
  return { shown: [0, 1, 2, 3], ...b };
}

function save() {
  board.savedAt = Date.now();
  queueFolderWrite(board);
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
  return `${d.getFullYear()}.${p(d.getMonth() + 1)}.${p(d.getDate())}`;
}

function itemRow(item, n) {
  const row = el('div', 'row');
  row.draggable = true;
  row.dataset.id = item.id;
  row.classList.toggle('dead', !!(health[item.url] && health[item.url].gone));
  row.append(el('span', 'num', n), el('span', 'kind', KIND[item.kind] || KIND.link));

  // 줄어들 때 가운데가 접혀 주소의 끝부분이 남는다
  const text = shortUrl(item.url);
  const cut = text.length > 30 ? text.length - 14 : text.length;
  const link = el('a', 'url');
  link.href = item.url;
  link.target = '_blank';
  link.rel = 'noreferrer';
  link.title = item.url;
  const address = [el('span', 'head', text.slice(0, cut)), el('span', 'tail', text.slice(cut))];
  const box = el('div', 'link-box');
  if (item.title) {
    link.classList.add('named');
    link.textContent = item.title;
    const small = el('div', 'url-sub');
    small.append(...address);
    box.append(link, small);
  } else {
    link.append(...address);
    box.append(link);
  }

  if (item.note) box.append(el('div', 'note', item.note));

  const memo = el('button', 'act memo');
  memo.innerHTML = ICON.memo;
  memo.title = '한 줄 메모';
  const copy = el('button', 'act copy');
  copy.innerHTML = ICON.copy;
  copy.title = '이 링크 복사';
  const del = el('button', 'act del');
  del.innerHTML = ICON.del;
  del.title = '삭제';
  row.append(box, memo, copy, del);
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
  box.innerHTML = '<img src="icons/128.png" alt=""><strong>아직 비어 있어요</strong>' +
    '<span><kbd>Shift+클릭</kbd> 한 개 줍기<br><kbd>Shift+드래그</kbd> 네모 안의 제목 줍기</span>';
  return box;
}

function photoGrid(pics) {
  const grid = el('div', 'photos');
  for (const item of pics) {
    const cell = el('div', 'photo');
    cell.draggable = true;
    cell.dataset.id = item.id;
    const img = el('img');
    img.src = photoSrc(item);
    img.alt = item.title || '';
    img.referrerPolicy = 'no-referrer';
    img.loading = 'lazy';
    const down = el('button', 'act down');
    down.innerHTML = ICON.down;
    down.title = '내려받기';
    const del = el('button', 'act del');
    del.innerHTML = ICON.del;
    del.title = '삭제';
    const acts = el('span', 'photo-acts');
    acts.append(down, del);
    cell.append(img, acts);
    grid.append(cell);
  }
  return grid;
}

// 사진 칸은 사진을 격자로, 그 밖의 링크는 그 아래 줄로 보인다
function fillList(list, all, photos) {
  const pics = photos ? all.filter(isPhoto) : [];
  const items = photos ? all.filter(i => !isPhoto(i)) : all;
  const nodes = pics.length ? [photoGrid(pics)] : [];
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
    fillList(sp.querySelector('.list'), space.items, space.photos);
  });
  const all = board.spaces.reduce((n, s) => n + s.items.length, 0);
  $('total').textContent = all ? `링크 ${all}개` : '';
  $('shelfBtn').textContent = `목록 ${board.spaces.length}`;
  renderShelf();
}

// 한 줄 메모
function editNote(row, index) {
  const item = findItem(index, row.dataset.id);
  const box = row.querySelector('.link-box');
  const input = el('input', 'note-input');
  input.value = item.note || '';
  input.placeholder = '한 줄 메모';
  input.maxLength = 120;
  row.draggable = false;
  box.querySelector('.note')?.remove();
  box.append(input);
  input.focus();
  let done = false;
  const finish = keep => {
    if (done) return;
    done = true;
    if (keep) item.note = input.value.trim();
    save();
    render();
  };
  input.addEventListener('blur', () => finish(true));
  input.addEventListener('keydown', e => {
    if (e.key === 'Enter') finish(true);
    if (e.key === 'Escape') finish(false);
  });
}

// 검색
function renderSearch() {
  const words = $('searchInput').value.trim().toLowerCase().split(/\s+/).filter(Boolean);
  if (!words.length) return $('searchList').replaceChildren();
  const rows = [];
  board.spaces.forEach((space, i) => {
    for (const item of space.items) {
      const hay = [item.title, item.url, item.siteName, item.note].join(' ').toLowerCase();
      if (!words.every(w => hay.includes(w)) || rows.length >= 100) continue;
      const row = el('button', 'search-row');
      row.dataset.i = i;
      row.dataset.c = i % 4;
      row.dataset.id = item.id;
      const text = el('span', 'search-text');
      text.append(el('span', 'shelf-name', item.title || shortUrl(item.url)), el('span', 'search-sub', item.note || host(item.url)));
      row.append(el('span', 'dot'), text, el('span', 'shelf-count', spaceName(i)));
      rows.push(row);
    }
  });
  $('searchList').replaceChildren(...(rows.length ? rows : [el('p', 'store-note', '찾는 링크가 없어요')]));
}

function toggleSearch(open) {
  $('search').hidden = !open;
  if (open) {
    $('shelf').hidden = true;
    $('settings').hidden = true;
    $('searchInput').focus();
    $('searchInput').select();
    renderSearch();
  }
  renderShelf();
}

// 마우스를 올리면 나오는 미리보기
function hidePreview() {
  clearTimeout(previewTimer);
  previewFor = '';
  $('preview').hidden = true;
}

function placePreview(row) {
  const box = row.getBoundingClientRect();
  const card = $('preview').getBoundingClientRect();
  const below = box.bottom + 6 + card.height < innerHeight;
  $('preview').style.left = Math.max(8, Math.min(box.left + 30, innerWidth - card.width - 8)) + 'px';
  $('preview').style.top = (below ? box.bottom + 6 : Math.max(8, box.top - card.height - 6)) + 'px';
}

async function showPreview(row, item) {
  previewFor = item.id;
  let image = item.kind === 'image' ? photoSrc(item) : '';
  let text = '';
  if (item.kind !== 'image') {
    const info = await chrome.runtime.sendMessage({ t: 'preview', url: item.url });
    if (previewFor !== item.id || !info) return;
    image = info.image;
    text = info.text;
  }
  const video = item.kind === 'video' && !image && /\.(mp4|webm)(\?|$)/i.test(item.url);
  $('previewImg').hidden = !image;
  if (image) $('previewImg').src = image;
  $('previewVideo').hidden = !video;
  if (video) $('previewVideo').src = item.url;
  $('previewTitle').textContent = item.title || shortUrl(item.url);
  $('previewText').textContent = text;
  $('preview').classList.toggle('big', row.classList.contains('photo'));
  $('preview').hidden = false;
  placePreview(row);
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
  if (open) {
    $('settings').hidden = true;
    $('search').hidden = true;
  }
  renderShelf();
}

// 저장 위치 설정
const pad = n => String(n).padStart(2, '0');

function renderSettings() {
  const { state, handle, savedAt } = folder;
  const name = handle ? handle.name : '';
  const time = savedAt ? ` · ${pad(new Date(savedAt).getHours())}:${pad(new Date(savedAt).getMinutes())} 저장` : '';
  $('storePlace').textContent = {
    local: '이 브라우저 안',
    ok: `폴더: ${name}${time}`,
    need: `폴더: ${name} · 연결이 끊겼어요`,
    error: `폴더: ${name} · 저장하지 못했어요`
  }[state];
  $('storeNote').textContent = state === 'local'
    ? '확장을 지우면 링크도 사라져요. 폴더를 고르면 파일로도 저장돼요.'
    : '링크는 이 폴더의 jupjup-links.json 에 저장돼요.';
  $('storePick').textContent = state === 'local' ? '폴더 고르기' : '다른 폴더로 옮기기';
  $('storeReconnect').hidden = state !== 'need' && state !== 'error';
  $('storeLocal').hidden = state === 'local';
  $('settingsBtn').classList.toggle('warn', state === 'need' || state === 'error');
  const dead = Object.values(health).filter(h => h.gone).length;
  const d = new Date(lastCheck);
  $('checkInfo').textContent = lastCheck
    ? `마지막 점검 ${d.getMonth() + 1}/${d.getDate()} ${pad(d.getHours())}:${pad(d.getMinutes())} · 사라진 링크 ${dead}개`
    : '아직 점검 전이에요. 6시간마다 자동으로 확인해요.';
}

function toggleSettings(open) {
  $('settings').hidden = !open;
  if (open) {
    $('shelf').hidden = true;
    $('search').hidden = true;
  }
  renderSettings();
  renderShelf();
}

function adoptBoard(next) {
  board = withShown(next);
  chrome.storage.local.set({ board });
  render();
  renderSettings();
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

// 페이지에서 읽어 온 제목이 링크 글자보다 우선이다
function applyTitles(titles) {
  let changed = false;
  for (const space of board.spaces) {
    for (const item of space.items) {
      if (titles[item.url] && item.title !== titles[item.url]) {
        item.title = titles[item.url];
        changed = true;
      }
    }
  }
  if (changed) { save(); render(); }
}

function renderDock() {
  document.body.classList.toggle('holding', pendingCount > 0);
  $('dock').hidden = !pendingCount;
  const photos = pending.filter(isPhoto).length, links = pending.length - photos;
  const what = [links && `링크 ${links}개`, photos && `사진 ${photos}장`].filter(Boolean).join(' · ');
  $('dockText').textContent = `${what} · ${links ? '넣을 판을 눌러요' : '아무 판이나 누르면 사진 칸으로 가요'}`;
}

async function paste(index) {
  const all = await chrome.runtime.sendMessage({ t: 'take' });
  if (!all || !all.length) return;
  const items = all.filter(i => !isPhoto(i));
  if (items.length) await pasteLinks(index, items);
  if (items.length < all.length) await placePhotos(all.filter(isPhoto));
}

async function pasteLinks(index, items) {
  const before = JSON.stringify(board);
  const owner = url => board.spaces.findIndex(s => s.items.some(i => i.url === url));
  const fresh = items.filter(i => !i.drag || owner(i.url) < 0);
  const dropped = items.length - fresh.length;
  const again = fresh.find(i => owner(i.url) >= 0);
  const againIn = again ? owner(again.url) : -1;
  board.spaces[index].items.push(...fresh);
  const { titles = {} } = await chrome.storage.local.get('titles');
  applyTitles(titles);
  await save();
  render();
  const list = spacesEl.querySelector(`[data-i="${index}"] .list`);
  if (list) list.scrollTop = list.scrollHeight;
  if (dropped) toast(`이미 있는 ${dropped}개를 빼고 ${fresh.length}개 넣었어요`);
  else if (again) toast(`이미 ${spaceName(againIn)}에 있어요`, before);
  else if (!list) toast(`${spaceName(index)}에 ${items.length}개 넣었어요`);
}

// 사진이 들어갈 칸: 자리가 남은 사진 칸 → 빈 칸(첫 칸은 링크 자리라 둘째부터) → 새 판(꽉 찬 사진 칸 자리에 보인다)
function photoSpace() {
  const vis = visible();
  const pics = i => board.spaces[i].items.filter(isPhoto).length;
  const room = vis.find(i => board.spaces[i].photos && pics(i) < PHOTO_MAX);
  if (room != null) return room;
  const empty = vis.slice(1).find(i => !board.spaces[i].items.length);
  if (empty != null) {
    board.spaces[empty].photos = true;
    return empty;
  }
  board.spaces.push({ title: '', items: [], photos: true });
  const full = vis.filter(i => board.spaces[i].photos).pop();
  board.shown[full != null ? board.shown.indexOf(full) : board.layout - 1] = board.spaces.length - 1;
  return board.spaces.length - 1;
}

// SNS 사진 주소는 며칠 뒤 만료되므로 줄인 사본(긴 변 1280px)을 같이 저장한다
async function makeThumb(url) {
  const res = await fetch(url, { credentials: 'omit' });
  if (!res.ok) throw new Error(res.status);
  const bmp = await createImageBitmap(await res.blob());
  const k = Math.min(1, 1280 / Math.max(bmp.width, bmp.height));
  const c = el('canvas');
  c.width = Math.round(bmp.width * k);
  c.height = Math.round(bmp.height * k);
  c.getContext('2d').drawImage(bmp, 0, 0, c.width, c.height);
  return c.toDataURL('image/jpeg', 0.85);
}

const photoSrc = item => item.thumb || item.url;

// 칸을 눌러 붙여넣을 때 사진은 누른 칸이 아니라 사진 칸으로 간다
async function placePhotos(items) {
  await Promise.all(items.map(async i => { i.thumb = await makeThumb(i.url).catch(() => ''); }));
  const before = JSON.stringify(board);
  const have = new Set(board.spaces.flatMap(s => s.items.map(i => i.url)));
  const fresh = items.filter(i => !have.has(i.url) && have.add(i.url));
  if (!fresh.length) return toast('이미 있는 사진이에요');
  if (board.layout === 1) board.layout = 2;
  let at = -1;
  for (const item of fresh) {
    at = photoSpace();
    board.spaces[at].items.push(item);
  }
  await save();
  render();
  const skipped = items.length - fresh.length;
  toast(`사진 ${fresh.length}장을 ${spaceName(at)}에 넣었어요${skipped ? ` (이미 있는 ${skipped}장 뺌)` : ''}`, before);
}

async function download(item) {
  try {
    const res = await fetch(item.url, { credentials: 'omit' });
    if (!res.ok) throw new Error(res.status);
    const blob = await res.blob();
    let name = decodeURIComponent(new URL(item.url).pathname.split('/').pop() || '') || 'jupjup';
    if (!/\.\w{2,5}$/.test(name)) name += '.' + (blob.type.split('/')[1] || 'jpg').replace('jpeg', 'jpg').replace(/\+.*/, '');
    const a = el('a');
    a.href = URL.createObjectURL(blob);
    a.download = name;
    a.click();
    setTimeout(() => URL.revokeObjectURL(a.href), 10000);
  } catch (e) {
    if (item.thumb) { // 원본 주소가 만료됐으면 저장해 둔 사본을 받는다
      const a = el('a');
      a.href = item.thumb;
      a.download = 'jupjup.jpg';
      a.click();
      return toast('원본을 받을 수 없어 저장해 둔 사본을 받았어요');
    }
    window.open(item.url, '_blank', 'noreferrer');
    toast('바로 받을 수 없어 새 탭에 열었어요');
  }
}

function zoom(item) {
  hidePreview();
  $('zoomImg').src = photoSrc(item);
  $('zoom').hidden = false;
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

  if (btn && btn.classList.contains('memo')) {
    editNote(btn.closest('.row'), index);
    return;
  }
  if (e.target.closest('.note-input')) return;
  if (btn && btn.classList.contains('copy')) {
    const row = btn.closest('.row');
    copyText(findItem(index, row.dataset.id).url).then(() => flashDone(btn));
    return;
  }
  if (btn && btn.classList.contains('del')) {
    const id = btn.closest('[data-id]').dataset.id;
    removeItems(index, i => i.id !== id, '삭제했어요');
    return;
  }
  if (btn && btn.classList.contains('down')) {
    download(findItem(index, btn.closest('.photo').dataset.id));
    return;
  }
  const photo = e.target.closest('.photo');
  if (photo && !pendingCount) {
    zoom(findItem(index, photo.dataset.id));
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

// 사진 끌기: 끄는 동안 자리를 바로 바꾸고, 밀려난 사진은 살짝 들렸다 내려앉는다
let liftUntil = 0;

function movePhoto(e) {
  const grid = spacesEl.querySelector(`.space[data-i="${drag.from}"] .photos`);
  const over = e.target.closest?.('.photo');
  const mine = grid && grid.querySelector(`.photo[data-id="${drag.ids[0]}"]`);
  if (!grid || !over || !mine || over === mine || over.parentNode !== grid || performance.now() < liftUntil) return;
  const cells = [...grid.children];
  const from = new Map(cells.map(c => [c, c.getBoundingClientRect()]));
  grid.insertBefore(mine, cells.indexOf(mine) < cells.indexOf(over) ? over.nextSibling : over);
  liftUntil = performance.now() + 120;
  for (const c of cells) {
    if (c === mine) continue;
    const a = from.get(c), b = c.getBoundingClientRect();
    if (a.left === b.left && a.top === b.top) continue;
    const dx = a.left - b.left, dy = a.top - b.top;
    c.animate([
      { transform: `translate(${dx}px, ${dy}px)` },
      { transform: `translate(${dx / 2}px, ${dy / 2 - 7}px) scale(1.05)`, boxShadow: '0 8px 16px rgba(43, 42, 76, .25)', offset: 0.45 },
      { transform: 'none' }
    ], { duration: 280, easing: 'cubic-bezier(.2, .8, .2, 1)' });
  }
}

spacesEl.addEventListener('dragstart', e => {
  const photo = e.target.closest?.('.photo');
  if (photo) {
    drag = { from: +photo.closest('.space').dataset.i, ids: [photo.dataset.id], photo: true };
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('text/plain', findItem(drag.from, photo.dataset.id).url);
    photo.classList.add('dragging');
    return;
  }
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
  if (drag.photo) {
    if (+e.target.closest?.('.space')?.dataset.i !== drag.from) return;
    e.preventDefault();
    return movePhoto(e);
  }
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
  if (drag.photo) {
    const space = board.spaces[drag.from];
    const order = [...sp.querySelectorAll('.photo')].map(c => findItem(drag.from, c.dataset.id));
    space.items = order.concat(space.items.filter(i => !isPhoto(i)));
    clearDrag();
    save();
    return render();
  }
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

$('searchBtn').addEventListener('click', e => {
  e.stopPropagation();
  toggleSearch($('search').hidden);
});

$('searchInput').addEventListener('input', renderSearch);
$('searchInput').addEventListener('keydown', e => {
  if (e.key === 'Escape') toggleSearch(false);
});

$('searchList').addEventListener('click', e => {
  const row = e.target.closest('.search-row');
  if (!row) return;
  showSpace(+row.dataset.i);
  toggleSearch(false);
  const found = spacesEl.querySelector(`.row[data-id="${row.dataset.id}"]`);
  if (!found) return;
  found.scrollIntoView({ block: 'center' });
  found.classList.add('found');
  setTimeout(() => found.classList.remove('found'), 1600);
});

$('checkNow').addEventListener('click', async () => {
  $('checkNow').textContent = '확인하는 중…';
  await chrome.runtime.sendMessage({ t: 'checkLinks' });
  $('checkNow').textContent = '사라진 링크 지금 확인';
});

spacesEl.addEventListener('mouseover', e => {
  const row = e.target.closest('.row, .photo');
  if (!row || row.dataset.id === previewFor || drag) return;
  if (e.target.closest('button,input')) return hidePreview();
  hidePreview();
  const item = findItem(+row.closest('.space').dataset.i, row.dataset.id);
  previewTimer = setTimeout(() => showPreview(row, item), 350);
});
spacesEl.addEventListener('mouseleave', hidePreview);
spacesEl.addEventListener('dragstart', hidePreview);
spacesEl.addEventListener('scroll', hidePreview, true);
$('previewImg').addEventListener('error', () => { $('previewImg').hidden = true; });
$('zoom').addEventListener('click', () => { $('zoom').hidden = true; });
addEventListener('keydown', e => {
  if (e.key === 'Escape') $('zoom').hidden = true;
});

$('settingsBtn').addEventListener('click', e => {
  e.stopPropagation();
  toggleSettings($('settings').hidden);
});

$('storePick').addEventListener('click', async () => {
  const next = await pickFolder(board);
  if (next) adoptBoard(next);
});

$('storeReconnect').addEventListener('click', async () => {
  adoptBoard(await reconnectFolder(board));
});

$('storeLocal').addEventListener('click', async () => {
  await forgetFolder();
  renderSettings();
  toast('이 브라우저 안에 저장해요. 폴더의 파일은 그대로 있어요');
});

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
  if (!$('settings').hidden && !e.target.closest('#settings')) toggleSettings(false);
  if (!$('search').hidden && !e.target.closest('#search')) toggleSearch(false);
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
  if (changes.titles) applyTitles(changes.titles.newValue || {});
  if (changes.linkHealth || changes.lastCheck) {
    if (changes.linkHealth) health = changes.linkHealth.newValue || {};
    if (changes.lastCheck) lastCheck = changes.lastCheck.newValue || 0;
    render();
    renderSettings();
  }
  if (changes.pending) {
    pending = changes.pending.newValue || [];
    pendingCount = pending.length;
    renderDock();
  }
  if (changes.board && !drag && JSON.stringify(changes.board.newValue) !== JSON.stringify(board)) {
    board = withShown(changes.board.newValue);
    render();
  }
});

(async () => {
  const data = await chrome.storage.local.get(['board', 'pending', 'titles', 'linkHealth', 'lastCheck']);
  health = data.linkHealth || {};
  lastCheck = data.lastCheck || 0;
  if (data.board) board = withShown(data.board);
  // 1.6.2 까지 구글 이미지의 데이터 사진이 검색 페이지 주소로 저장돼 엑박이었다
  for (const space of board.spaces) space.items = space.items.filter(i => !(isPhoto(i) && i.url === i.siteUrl));
  const fresh = await connectFolder(board);
  if (fresh !== board) adoptBoard(fresh);
  renderSettings();
  pending = data.pending || [];
  pendingCount = pending.length;
  render();
  renderDock();
  applyTitles(data.titles || {});
})();
