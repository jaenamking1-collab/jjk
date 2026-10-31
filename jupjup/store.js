// 고른 폴더에 링크를 파일로도 저장한다. 작업 저장소는 chrome.storage 이고 파일은 그 복사본이다.
const STORE_FILE = 'jupjup-links.json';
const folder = { handle: null, state: 'local', savedAt: 0 }; // state: local | ok | need | error
let writeTimer = 0;

function handleDb() {
  return new Promise((resolve, reject) => {
    const open = indexedDB.open('jupjup', 1);
    open.onupgradeneeded = () => open.result.createObjectStore('kv');
    open.onsuccess = () => resolve(open.result);
    open.onerror = () => reject(open.error);
  });
}

async function kv(mode, fn) {
  const db = await handleDb();
  return new Promise((resolve, reject) => {
    const req = fn(db.transaction('kv', mode).objectStore('kv'));
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

const rememberHandle = h => kv('readwrite', s => (h ? s.put(h, 'dir') : s.delete('dir')));

async function readFolderBoard() {
  try {
    const file = await (await folder.handle.getFileHandle(STORE_FILE)).getFile();
    return JSON.parse(await file.text());
  } catch (e) {
    if (e.name === 'NotFoundError') return null;
    throw e;
  }
}

async function writeFolderBoard(data) {
  if (folder.state !== 'ok') return;
  try {
    const out = await (await folder.handle.getFileHandle(STORE_FILE, { create: true })).createWritable();
    await out.write(JSON.stringify(data));
    await out.close();
    folder.savedAt = Date.now();
  } catch (e) {
    folder.state = 'error';
  }
  renderSettings();
}

function queueFolderWrite(data) {
  clearTimeout(writeTimer);
  writeTimer = setTimeout(() => writeFolderBoard(data), 400);
}

// 창을 열 때: 폴더 파일이 더 새것이면(다른 PC에서 고친 경우) 그것을 쓴다
async function connectFolder(local) {
  folder.handle = await kv('readonly', s => s.get('dir')) || null;
  if (!folder.handle) return local;
  if (await folder.handle.queryPermission({ mode: 'readwrite' }) !== 'granted') {
    folder.state = 'need';
    return local;
  }
  return syncFolder(local);
}

async function syncFolder(local) {
  folder.state = 'ok';
  try {
    const saved = await readFolderBoard();
    if (saved && (saved.savedAt || 0) > (local.savedAt || 0)) return saved;
    await writeFolderBoard(local);
  } catch (e) {
    folder.state = 'error';
  }
  return local;
}

// 사용자가 누른 버튼 안에서만 부를 수 있다(권한 창)
async function reconnectFolder(local) {
  if (await folder.handle.requestPermission({ mode: 'readwrite' }) !== 'granted') return local;
  return syncFolder(local);
}

async function pickFolder(local) {
  let handle;
  try {
    handle = await showDirectoryPicker({ id: 'jupjup', mode: 'readwrite' });
  } catch (e) {
    return null; // 창을 닫았다
  }
  folder.handle = handle;
  folder.state = 'ok';
  await rememberHandle(handle);
  const saved = await readFolderBoard().catch(() => null);
  if (saved && confirm('이 폴더에 저장본이 있어요.\n\n확인: 폴더의 저장본 불러오기\n취소: 지금 링크로 덮어쓰기')) return saved;
  await writeFolderBoard(local);
  return local;
}

async function forgetFolder() {
  clearTimeout(writeTimer);
  folder.handle = null;
  folder.state = 'local';
  folder.savedAt = 0;
  await rememberHandle(null);
}
