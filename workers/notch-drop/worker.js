// 노치 파일 전송 중계. 방 열쇠(주소의 첫 칸)마다 Durable Object 하나가 파일을 잠깐 맡는다.
// 휴대폰 쪽 길은 노치의 와이파이 창구(notch-cat/src/xfer.rs)와 같다: /K, /K/list, /K/d/i, /K/up?n=
// PC(노치) 쪽 길: /K/open, /K/put?n=, /K/inbox, /K/take/id, /K/del/i
// 방은 노치가 /open 으로 연 것만 쓸 수 있고, 1시간 아무 일이 없으면 통째로 지워진다.

const LIMIT = 95 * 1024 * 1024;     // 파일 하나(무료 요청 크기 100MB 아래)
const ROOM_MAX = 300 * 1024 * 1024;  // 방 하나에 맡길 수 있는 합
const CHUNK = 1024 * 1024;           // 저장 칸(한 줄 2MB 제한)
const TTL = 60 * 60 * 1000;

export default {
  async fetch(req, env) {
    const u = new URL(req.url);
    const key = u.pathname.split('/')[1] || '';
    if (!/^[A-Za-z0-9_-]{10,40}$/.test(key)) return new Response('노치 파일 전송', { status: 404 });
    return env.ROOM.get(env.ROOM.idFromName(key)).fetch(req);
  },
};

const json = (v, s = 200) => new Response(JSON.stringify(v), { status: s, headers: { 'content-type': 'application/json', 'cache-control': 'no-store' } });
const disp = n => `attachment; filename="${n.replace(/[^\w.-]/g, '_')}"; filename*=UTF-8''${encodeURIComponent(n)}`;

export class Room {
  constructor(ctx) {
    this.ctx = ctx;
    this.sql = ctx.storage.sql;
    this.sql.exec(`CREATE TABLE IF NOT EXISTS meta(k TEXT PRIMARY KEY, v TEXT);
      CREATE TABLE IF NOT EXISTS files(id INTEGER PRIMARY KEY AUTOINCREMENT, dir TEXT, name TEXT, size INTEGER, done INTEGER DEFAULT 0);
      CREATE TABLE IF NOT EXISTS chunks(fid INTEGER, i INTEGER, data BLOB, PRIMARY KEY(fid, i));`);
  }

  async alarm() {
    await this.ctx.storage.deleteAll();
  }

  opened() {
    return this.sql.exec(`SELECT v FROM meta WHERE k='open'`).toArray().length > 0;
  }

  async touch() {
    await this.ctx.storage.setAlarm(Date.now() + TTL);
  }

  drop(id) {
    this.sql.exec(`DELETE FROM chunks WHERE fid=?`, id);
    this.sql.exec(`DELETE FROM files WHERE id=?`, id);
  }

  async save(req, dir, name) {
    const len = +req.headers.get('content-length') || 0;
    const used = this.sql.exec(`SELECT COALESCE(SUM(size),0) s FROM files`).one().s;
    if (!req.body || len > LIMIT || used + len > ROOM_MAX) return json({ error: '너무 큽니다' }, 413);
    const id = this.sql.exec(`INSERT INTO files(dir,name,size) VALUES(?,?,0) RETURNING id`, dir, name).one().id;
    let buf = new Uint8Array(CHUNK), at = 0, i = 0, size = 0;
    const flush = () => {
      if (at) this.sql.exec(`INSERT INTO chunks VALUES(?,?,?)`, id, i++, buf.slice(0, at));
      at = 0;
    };
    try {
      const rd = req.body.getReader();
      for (;;) {
        const { done, value } = await rd.read();
        if (done) break;
        size += value.length;
        if (size > LIMIT) throw new Error('big');
        let off = 0;
        while (off < value.length) {
          const n = Math.min(CHUNK - at, value.length - off);
          buf.set(value.subarray(off, off + n), at);
          at += n; off += n;
          if (at === CHUNK) flush();
        }
      }
      flush();
    } catch (e) {
      this.drop(id); // 끊기면 반쪽 파일을 남기지 않는다
      return json({ error: '끊김' }, 400);
    }
    this.sql.exec(`UPDATE files SET size=?, done=1 WHERE id=?`, size, id);
    return json({ ok: true, id });
  }

  send(f) {
    const sql = this.sql;
    let i = 0;
    const body = new ReadableStream({
      pull(c) {
        const r = sql.exec(`SELECT data FROM chunks WHERE fid=? AND i=?`, f.id, i++).toArray();
        if (r.length) c.enqueue(new Uint8Array(r[0].data)); else c.close();
      },
    });
    return new Response(body, { headers: { 'content-type': 'application/octet-stream', 'content-length': String(f.size), 'content-disposition': disp(f.name) } });
  }

  file(id, dir) {
    return this.sql.exec(`SELECT id,name,size FROM files WHERE id=? AND dir=? AND done=1`, id, dir).toArray()[0];
  }

  async fetch(req) {
    const u = new URL(req.url);
    const [, key, a = '', b = ''] = u.pathname.split('/');
    const m = req.method;
    const name = (u.searchParams.get('n') || '파일').split(/[\\/]/).pop().slice(0, 200) || '파일';
    if (m === 'POST' && a === 'open') {
      this.sql.exec(`INSERT OR REPLACE INTO meta VALUES('open','1')`);
      await this.touch();
      return json({ ok: true });
    }
    if (!this.opened()) return new Response('닫힌 방입니다', { status: 404 });
    await this.touch();
    // 휴대폰 쪽(와이파이 창구와 같은 길)
    if (m === 'GET' && a === '') return new Response(PAGE.replace('{KEY}', key), { headers: { 'content-type': 'text/html; charset=utf-8', 'cache-control': 'no-store' } });
    if (m === 'GET' && a === 'list') {
      const out = this.sql.exec(`SELECT id i,name,size FROM files WHERE dir='phone' AND done=1 ORDER BY id DESC`).toArray();
      const got = this.sql.exec(`SELECT v FROM meta WHERE k LIKE 'got:%' ORDER BY k DESC`).toArray().map(r => r.v);
      return json({ out, got });
    }
    if (m === 'GET' && a === 'd') {
      const f = this.file(+b, 'phone');
      return f ? this.send(f) : new Response('', { status: 404 });
    }
    if (m === 'POST' && a === 'up') return this.save(req, 'pc', name);
    // 노치 쪽
    if (m === 'POST' && a === 'put') return this.save(req, 'phone', name);
    if (m === 'GET' && a === 'inbox') return json(this.sql.exec(`SELECT id,name,size FROM files WHERE dir='pc' AND done=1 ORDER BY id`).toArray());
    if (m === 'GET' && a === 'take') {
      const f = this.file(+b, 'pc');
      return f ? this.send(f) : new Response('', { status: 404 });
    }
    if (m === 'POST' && a === 'took') {
      const f = this.file(+b, 'pc');
      if (f) {
        this.drop(f.id); // PC 가 다 받았으면 바로 지운다
        this.sql.exec(`INSERT INTO meta VALUES(?,?)`, `got:${String(Date.now()).padStart(15, '0')}:${f.id}`, f.name);
      }
      return json({ ok: true });
    }
    if (m === 'POST' && a === 'del') {
      this.sql.exec(`DELETE FROM chunks WHERE fid IN (SELECT id FROM files WHERE id=? AND dir='phone')`, +b);
      this.sql.exec(`DELETE FROM files WHERE id=? AND dir='phone'`, +b);
      return json({ ok: true });
    }
    return new Response('', { status: 404 });
  }
}

// 휴대폰 화면. notch-cat/src/xfer.rs 의 PAGE 와 같게 유지한다(여기만 크기 제한 안내가 붙는다).
const PAGE = `<!doctype html><html lang="ko"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>노치 파일 전송</title>
<style>
body{margin:0;background:#0b0b0c;color:#f3f4f6;font:16px/1.5 -apple-system,"Apple SD Gothic Neo","Malgun Gothic",sans-serif;padding:20px 18px 40px}
h1{font-size:20px;margin:0 0 18px}h2{font-size:15px;margin:26px 0 10px;color:#a3e635}
.up{display:block;background:#a3e635;color:#111;text-align:center;border-radius:16px;padding:22px 0;font-weight:700;font-size:18px}
.note{color:#6b7280;font-size:13px;margin-top:8px;text-align:center}
input{display:none}
.row{display:flex;align-items:center;gap:10px;padding:12px 14px;border-radius:12px;background:#1a1b1e;margin:6px 0;color:#f3f4f6;text-decoration:none}
.row b{flex:1;font-weight:500;word-break:break-all}.row small{color:#9ca3af}
.bar{height:6px;background:#2a2b2f;border-radius:3px;margin-top:8px;overflow:hidden}.bar i{display:block;height:100%;background:#a3e635;width:0}
.empty{color:#6b7280;font-size:14px}
</style></head><body>
<h1>🐱 노치 파일 전송</h1>
<label class="up">휴대폰 → PC 보내기<input id="f" type="file" multiple></label>
<div class="note">인터넷 중계 · 파일 하나 95MB까지 · 1시간 뒤 자동 삭제</div>
<div id="prog"></div>
<h2>PC → 휴대폰 (누르면 받기)</h2><div id="out" class="empty">PC 노치의 '파일 전송' 쪽에 파일을 끌어다 놓으면 여기 나옵니다.</div>
<h2>PC 가 받은 파일</h2><div id="got" class="empty">아직 없습니다.</div>
<script>
const K='/{KEY}',LIM=${LIMIT};
const sz=n=>n>1e9?(n/1e9).toFixed(1)+'GB':n>1e6?(n/1e6).toFixed(1)+'MB':Math.max(1,Math.round(n/1e3))+'KB';
const esc=s=>s.replace(/[&<>"]/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c]));
async function load(){try{const j=await (await fetch(K+'/list')).json();
const o=document.getElementById('out'),g=document.getElementById('got');
if(j.out.length){o.className='';o.innerHTML=j.out.map(f=>\`<a class="row" href="\${K}/d/\${f.i}" download><b>\${esc(f.name)}</b><small>\${sz(f.size)}</small></a>\`).join('')}
if(j.got.length){g.className='';g.innerHTML=j.got.map(n=>\`<div class="row"><b>\${esc(n)}</b><small>✓</small></div>\`).join('')}}catch(e){}}
function send(file){return new Promise(res=>{const d=document.createElement('div');d.className='row';d.style.display='block';
d.innerHTML=\`<b>\${esc(file.name)}</b> <small>\${sz(file.size)}</small><div class="bar"><i></i></div>\`;document.getElementById('prog').appendChild(d);
if(LIM&&file.size>LIM){d.querySelector('small').textContent='95MB 넘음 — 같은 와이파이에서 보내세요';return res()}
const x=new XMLHttpRequest();x.open('POST',K+'/up?n='+encodeURIComponent(file.name));
x.upload.onprogress=e=>{if(e.lengthComputable)d.querySelector('i').style.width=(100*e.loaded/e.total)+'%'};
x.onload=()=>{d.querySelector('small').textContent=x.status==200?'보냄 ✓':'실패';if(x.status==200)setTimeout(()=>d.remove(),2500);load();res()};
x.onerror=()=>{d.querySelector('small').textContent='실패 — 인터넷 연결 확인';res()};x.send(file)})}
document.getElementById('f').onchange=async e=>{for(const f of e.target.files)await send(f);e.target.value=''};
load();setInterval(load,3000);
</script></body></html>`;
