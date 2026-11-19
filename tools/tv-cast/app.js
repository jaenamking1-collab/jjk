const http = require('node:http');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const crypto = require('node:crypto');
const {spawn} = require('node:child_process');
const core = require('./core');

const token = crypto.randomBytes(24).toString('hex');
const cache = path.join(os.tmpdir(),'jjk-tv-cast');
const state = {phase:'대기',progress:0,message:'동영상과 자막을 선택해 주세요.',video:'',subtitle:'',devices:[],lastCheck:null,lastRead:null,error:'',busy:false};
let media = null, activeTV = null, controller = null, stopping = false, castJob = null, readTimer = null, stopJob = null;
function update(values) {Object.assign(state,values,{lastCheck:new Date().toISOString()});}
function json(res, data, status=200) {res.writeHead(status,{'Content-Type':'application/json; charset=utf-8','Cache-Control':'no-store'}); res.end(JSON.stringify(data));}
function choose(kind) {
  const filter = kind === 'video' ? '동영상|*.mp4;*.mkv;*.avi;*.mov;*.ts;*.m4v;*.wmv|모든 파일|*.*' : '자막|*.srt;*.smi;*.ass;*.ssa;*.vtt|모든 파일|*.*';
  const script = `Add-Type -AssemblyName System.Windows.Forms; $d=New-Object System.Windows.Forms.OpenFileDialog; $d.Filter='${filter}'; $d.Title='${kind === 'video' ? '동영상' : '자막'} 선택'; $owner=New-Object System.Windows.Forms.Form; $owner.TopMost=$true; if($d.ShowDialog($owner) -eq 'OK'){[Console]::OutputEncoding=[Text.UTF8Encoding]::new(); [Console]::Write($d.FileName)}; $d.Dispose(); $owner.Dispose()`;
  return core.run('powershell.exe',['-NoProfile','-STA','-EncodedCommand',Buffer.from(script,'utf16le').toString('base64')]);
}
function autoSubtitle(video) {
  const folder = path.dirname(video), base = path.basename(video,path.extname(video)).toLowerCase();
  const files = fs.readdirSync(folder);
  return files.filter(n => ['.srt','.smi','.ass','.ssa','.vtt'].includes(path.extname(n).toLowerCase()))
    .sort((a,b) => a.localeCompare(b)).find(n => path.basename(n,path.extname(n)).toLowerCase() === base);
}
function closeMedia() {
  clearTimeout(readTimer); readTimer = null;
  if (media) {media.server.closeAllConnections(); media.server.close(); media = null;}
}
function stop() {
  if (stopJob) return stopJob;
  stopJob = performStop().finally(() => {stopping = false; stopJob = null; update({busy:false});});
  return stopJob;
}
async function performStop() {
  stopping = true;
  update({busy:true,phase:'중지 중',message:'진행 중인 변환과 TV 전송을 정리하고 있습니다.'});
  controller?.abort();
  await castJob?.catch(() => {});
  let stopError = '';
  if (activeTV) {
    try {await core.soap(activeTV,'Stop',{InstanceID:0});} catch(e) {stopError = e.message;}
  }
  activeTV = null; closeMedia();
  update({phase:'중지',message:stopError ? '전송을 중지했습니다. TV 중지 명령은 확인하지 못했습니다.' : '전송을 중지했습니다.',error:stopError});
}
async function cast(index) {
  if (state.busy || stopping) throw new Error('진행 중인 작업을 먼저 중지해 주세요.');
  const tv = state.devices[index];
  if (!tv) throw new Error('TV를 선택해 주세요.');
  if (!state.video || !state.subtitle) throw new Error('동영상과 자막을 선택해 주세요.');
  stopping = false; controller = new AbortController();
  update({busy:true,phase:'자막 합치는 중',progress:0,error:'',lastRead:null,message:'변환이 끝나면 TV로 보냅니다. 처음에는 시간이 걸릴 수 있습니다.'});
  try {
    if (activeTV) await core.soap(activeTV,'Stop',{InstanceID:0},controller.signal);
    activeTV = null; closeMedia();
    const output = await core.convert(state.video,state.subtitle,cache,progress => update({progress}),controller.signal);
    if (stopping) return;
    update({phase:'TV로 보내는 중',message:'TV의 재생 명령 응답을 기다리고 있습니다.'});
    media = await core.startMedia(output,tv.localAddress,() => update({lastRead:new Date().toISOString(),phase:'TV가 영상 받는 중',message:'TV의 영상 요청을 확인했습니다. 실제 화면의 자막과 소리는 TV에서 확인해 주세요.'}));
    controller.signal.throwIfAborted();
    activeTV = tv;
    await core.play(tv,media.url,controller.signal);
    if (stopping) return;
    if (!state.lastRead) update({phase:'TV 응답 확인',message:'TV가 재생 명령을 수락했습니다. 영상 요청을 기다리고 있습니다.'});
    readTimer = setTimeout(() => {
      if (!stopping && media && !state.lastRead) update({phase:'영상 요청 미확인',message:'TV의 영상 요청이 없습니다. Windows 방화벽의 개인 네트워크 허용 여부와 TV 연결 허용 화면을 확인해 주세요.'});
    },20000).unref();
  } catch(e) {
    closeMedia();
    if (!stopping) update({phase:'오류',error:e.message,message:'아래 오류를 확인해 주세요. 다시 재생할 수 있습니다.'});
  } finally {controller = null; update({busy:stopping});}
}
async function body(req) {
  let s = '';
  for await (const chunk of req) {s += chunk; if(s.length > 8192) throw new Error('요청이 너무 큽니다.');}
  return s ? JSON.parse(s) : {};
}
const server = http.createServer(async (req,res) => {
  try {
    // Only the local page may operate native file dialogs or control the TV.
    if (req.headers.host !== `127.0.0.1:${server.address().port}`) return json(res,{error:'잘못된 호스트'},403);
    if (req.url === '/' && req.method === 'GET') {
      res.writeHead(200,{'Content-Type':'text/html; charset=utf-8','Cache-Control':'no-store','X-Frame-Options':'DENY',
        'Content-Security-Policy':"default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'"});
      return res.end(fs.readFileSync(path.join(__dirname,'index.html'),'utf8').replace('__TOKEN__',token));
    }
    if (req.headers['x-cast-token'] !== token) return json(res,{error:'잘못된 세션'},403);
    if (req.url === '/state' && req.method === 'GET') return json(res,state);
    if (req.method !== 'POST') return json(res,{error:'없는 요청'},404);
    const input = await body(req);
    if (req.url === '/choose') {
      if (state.busy || stopping) throw new Error('작업 중에는 파일을 바꿀 수 없습니다.');
      if (!['video','subtitle'].includes(input.kind)) throw new Error('잘못된 파일 종류');
      const file = await choose(input.kind);
      if (file) {
        update({[input.kind]:file,error:''});
        if (input.kind === 'video') {
          const sub = autoSubtitle(file);
          update({subtitle:sub ? path.join(path.dirname(file),sub) : ''});
        }
      }
      return json(res,state);
    }
    if (req.url === '/discover') {
      if (state.busy || stopping) throw new Error('진행 중인 작업이 있습니다.');
      update({busy:true,phase:'TV 찾는 중',error:''});
      try {
        const result = await core.discover();
        update({devices:result.devices,phase:'검색 완료',message:result.devices.length ? `TV ${result.devices.length}대를 찾았습니다.` : 'TV를 찾지 못했습니다. TV 전원과 같은 네트워크인지 확인해 주세요.',error:result.devices.length ? '' : result.errors.join('; ').slice(0,500)});
      } finally {update({busy:false});}
      return json(res,state);
    }
    if (req.url === '/cast') {
      if (state.busy || stopping) throw new Error('진행 중인 작업이 있습니다.');
      castJob = cast(Number(input.index));
      void castJob.catch(e => update({phase:'오류',error:e.message}));
      return json(res,{ok:true});
    }
    if (req.url === '/stop') {await stop(); return json(res,state);}
    if (req.url === '/quit') {await stop(); json(res,{ok:true}); server.close(); setTimeout(() => process.exit(0),300).unref(); return;}
    return json(res,{error:'없는 요청'},404);
  } catch(e) {json(res,{error:e.message},400);}
});
server.listen(0,'127.0.0.1',() => {
  const url = `http://127.0.0.1:${server.address().port}/`;
  console.log(`TV 자막 전송: ${url}`);
  console.log('종료하려면 화면의 프로그램 종료 버튼을 누르세요.');
  if (!process.argv.includes('--no-open')) spawn('rundll32.exe',['url.dll,FileProtocolHandler',url],{windowsHide:true}).on('error',e => console.error(e.message));
});
process.on('SIGINT',async () => {await stop(); process.exit(0);});
