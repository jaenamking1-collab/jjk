const http = require('node:http');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const crypto = require('node:crypto');
const {spawn} = require('node:child_process');
const {pipeline} = require('node:stream/promises');
const core = require('./core');

const token = crypto.randomBytes(24).toString('hex');
const cache = path.join(os.tmpdir(),'jjk-tv-cast');
const state = {phase:'대기',progress:0,message:'동영상과 자막을 선택해 주세요.',video:'',subtitle:'',devices:[],lastCheck:null,lastRead:null,offset:0,position:0,duration:0,error:'',busy:false};
let lastIndex = 0, media = null, activeTV = null, controller = null, stopping = false, castJob = null, readTimer = null, stopJob = null;
function hms(t) {t = Math.max(0,Math.floor(t)); return [Math.floor(t/3600),Math.floor(t/60)%60,t%60].map((n,i) => String(n).padStart(i ? 2 : 1,'0')).join(':');}
function update(values) {Object.assign(state,values,{lastCheck:new Date().toISOString()});}
function json(res, data, status=200) {res.writeHead(status,{'Content-Type':'application/json; charset=utf-8','Cache-Control':'no-store'}); res.end(JSON.stringify(data));}
// TV로 보내는 동안 컴퓨터가 절전에 들어가면 영상 전송이 끊긴다. 재생 중에만 절전을 막는다(화면 꺼짐은 그대로 둔다).
let awake = null, stoppedPolls = 0;
function keepAwake(on) {
  if (!on) {awake?.kill(); awake = null; return;}
  if (awake || process.platform !== 'win32') return;
  const ps = "Add-Type -Name P -Namespace W -MemberDefinition '[DllImport(\"kernel32.dll\")] public static extern uint SetThreadExecutionState(uint f);'; [void][W.P]::SetThreadExecutionState([uint32]2147483649); while ($true) {Start-Sleep 60}";
  awake = spawn('powershell.exe',['-NoProfile','-NonInteractive','-Command',ps],{windowsHide:true,stdio:'ignore'});
  awake.on('exit',() => {awake = null;}).on('error',() => {awake = null;});
}
process.on('exit',() => awake?.kill());
function closeMedia() {
  keepAwake(false); stoppedPolls = 0;
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
async function cast(index,start=0) {
  if (state.busy || stopping) throw new Error('진행 중인 작업을 먼저 중지해 주세요.');
  const tv = state.devices[index];
  if (!tv) throw new Error('TV를 선택해 주세요.');
  if (!state.video) throw new Error('동영상을 선택해 주세요.');
  lastIndex = index;
  stopping = false; controller = new AbortController();
  update({busy:true,phase:'자막 준비 중',progress:0,error:'',lastRead:null,offset:start,position:start,message:'자막을 입히면서 바로 TV로 보냅니다.'});
  try {
    if (activeTV) await core.soap(activeTV,'Stop',{InstanceID:0},controller.signal);
    activeTV = null; closeMedia();
    update({message:'자막을 영상 파일에 넣고 있습니다.'});
    const onRead = (method,from) => update({lastRead:new Date().toISOString(),phase:'TV가 영상 받는 중',message:'TV의 영상 요청을 확인했습니다. 리모컨으로 넘길 수 있습니다.',...(from === undefined ? {} : {offset:from,position:from})});
    const native = state.subtitle ? await core.remux(state.video,state.subtitle,cache,controller.signal) : await core.plain(state.video,controller.signal);
    if (!native && !state.subtitle) throw new Error('자막 파일이 없고, TV가 이 영상을 그대로 열 수 없습니다. 자막을 골라 주세요.');
    if (native) {
      media = await core.startMedia(native.output,tv.localAddress,onRead,native.mime || 'video/x-mkv');
      Object.assign(media,{duration:native.duration,native:true,mime:native.mime || 'video/x-mkv'});
      update({offset:0});
    } else {
      update({message:'TV가 직접 못 여는 영상이라 자막을 입히면서 보냅니다(넘기기 시 다시 시작).'});
      media = await core.startLive(state.video,state.subtitle,cache,tv.localAddress,onRead,controller.signal,start);
    }
    update({duration:media.duration});
    if (stopping) {closeMedia(); return;}
    update({phase:'TV로 보내는 중',message:'TV의 재생 명령 응답을 기다리고 있습니다.'});
    controller.signal.throwIfAborted();
    activeTV = tv;
    await core.play(tv,media.url,controller.signal,media.native ? media.mime : 'video/mpeg',media.duration,media.native ? core.FILE_FEATURES : '*');
    if (media.native && start > 0) await core.soap(tv,'Seek',{InstanceID:0,Unit:'REL_TIME',Target:hms(start)},controller.signal).catch(() => {});
    if (stopping) return;
    keepAwake(true);
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
    if (req.url === '/upload' && req.method === 'POST') {
      if (state.busy || stopping) throw new Error('진행 중인 작업이 있습니다.');
      const kind = req.headers['x-file-kind'];
      if (!['video','subtitle'].includes(kind)) throw new Error('잘못된 파일 종류');
      const name = path.basename(decodeURIComponent(req.headers['x-file-name'] || 'file'));
      const folder = path.join(cache,'inputs',crypto.randomBytes(12).toString('hex'));
      fs.mkdirSync(folder,{recursive:true});
      const file = path.join(folder,name);
      update({busy:true,phase:'파일 준비 중',error:'',message:'선택한 파일을 이 컴퓨터의 임시 폴더에 준비하고 있습니다.'});
      try {
        await pipeline(req,fs.createWriteStream(file,{flags:'wx'}));
        update({[kind]:file,phase:'파일 선택 완료',message:'파일이 준비되었습니다. 자막과 TV를 확인한 뒤 재생해 주세요.'});
        return json(res,state);
      } catch(e) {try{fs.unlinkSync(file);}catch{} throw e;}
      finally {update({busy:false});}
    }
    if (req.url === '/state' && req.method === 'GET') return json(res,state);
    if (req.method !== 'POST') return json(res,{error:'없는 요청'},404);
    const input = await body(req);
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
    if (req.url === '/seek') {
      if (!activeTV || !media || state.busy || stopping) throw new Error('재생 중일 때만 이동할 수 있습니다.');
      const to = input.to !== undefined ? Number(input.to) : state.position + Number(input.delta);
      if (!Number.isFinite(to)) throw new Error('이동할 위치가 올바르지 않습니다.');
      const target = Math.max(0,Math.min(to,Math.max(0,state.duration - 5)));
      if (media.native) {await core.soap(activeTV,'Seek',{InstanceID:0,Unit:'REL_TIME',Target:hms(target)}); update({position:target}); return json(res,{ok:true,target});}
      castJob = cast(lastIndex,target); void castJob.catch(e => update({phase:'오류',error:e.message}));
      return json(res,{ok:true,target});
    }
    if (req.url === '/stop') {await stop(); return json(res,state);}
    if (req.url === '/quit') {await stop(); json(res,{ok:true}); server.close(); setTimeout(() => process.exit(0),300).unref(); return;}
    return json(res,{error:'없는 요청'},404);
  } catch(e) {json(res,{error:e.message},400);}
});
// 영화 파일을 인자로 받으면(탐색기 '보내기') 같은 폴더의 자막을 고르고 TV를 찾아, TV가 한 대면 바로 재생한다.
async function autoStart(arg) {
  try {
    const file = path.resolve(arg);
    if (!fs.statSync(file).isFile()) throw new Error('동영상 파일이 아닙니다.');
    const dir = path.dirname(file), base = path.basename(file,path.extname(file)).toLowerCase();
    const subs = fs.readdirSync(dir).filter(n => n.toLowerCase().startsWith(base) && /\.(srt|smi|ass|ssa|vtt)$/i.test(n))
      .map(n => ({file:path.join(dir,n),size:fs.statSync(path.join(dir,n)).size})).sort((a,b) => b.size-a.size);
    update({video:file,subtitle:subs[0]?.file || ''});
    update({busy:true,phase:'TV 찾는 중',error:''});
    let result;
    try {result = await core.discover();} finally {update({busy:false});}
    update({devices:result.devices,phase:'검색 완료',message:result.devices.length ? `TV ${result.devices.length}대를 찾았습니다.` : 'TV를 찾지 못했습니다. TV 전원과 같은 네트워크인지 확인해 주세요.',error:result.devices.length ? '' : result.errors.join('; ').slice(0,500)});
    if (result.devices.length === 1) {castJob = cast(0); void castJob.catch(e => update({phase:'오류',error:e.message}));}
  } catch(e) {update({busy:false,phase:'오류',error:e.message});}
}
server.listen(0,'127.0.0.1',() => {
  const url = `http://127.0.0.1:${server.address().port}/`;
  console.log(`TV 자막 전송: ${url}`);
  console.log('종료하려면 화면의 프로그램 종료 버튼을 누르세요.');
  const arg = process.argv.slice(2).find(a => !a.startsWith('--'));
  if (arg) void autoStart(arg);
  if (!process.argv.includes('--no-open')) spawn('rundll32.exe',['url.dll,FileProtocolHandler',url],{windowsHide:true}).on('error',e => console.error(e.message));
});
setInterval(async () => {
  if (!activeTV || !media || state.busy || stopping) return;
  try {
    const m = /<RelTime>(\d+):(\d+):(\d+)/.exec(await core.soap(activeTV,'GetPositionInfo',{InstanceID:0}));
    if (m) update({position:state.offset + Number(m[1])*3600 + Number(m[2])*60 + Number(m[3])});
    // TV에서 끝났거나 꺼서 5분 넘게 멈춰 있으면 절전 막기를 푼다(다 보고 자리를 떠도 컴퓨터가 계속 켜져 있지 않게).
    const t = /<CurrentTransportState>(\w+)/.exec(await core.soap(activeTV,'GetTransportInfo',{InstanceID:0}))?.[1];
    stoppedPolls = ['STOPPED','NO_MEDIA_PRESENT'].includes(t) ? stoppedPolls + 1 : 0;
    if (stoppedPolls === 150) {keepAwake(false); update({message:'TV 재생이 멈춰 있어 컴퓨터 절전 막기를 풀었습니다.'});}
    else if (stoppedPolls === 0 && t === 'PLAYING') keepAwake(true);
  } catch {}
},2000).unref();
process.on('SIGINT',async () => {await stop(); process.exit(0);});
