// Local-file DLNA sender. No third-party Node packages.
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const http = require('node:http');
const dgram = require('node:dgram');
const crypto = require('node:crypto');
const {spawn} = require('node:child_process');

function xmlEscape(s) { return String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&apos;'}[c])); }
function xmlText(s) { return s.replace(/&(?:amp|lt|gt|quot|apos);/g, c => ({'&amp;':'&','&lt;':'<','&gt;':'>','&quot;':'"','&apos;':"'"}[c])); }
function tag(xml, name) { const m = xml.match(new RegExp(`<(?:[\\w-]+:)?${name}\\b[^>]*>([\\s\\S]*?)</(?:[\\w-]+:)?${name}>`, 'i')); return m ? xmlText(m[1].trim()) : ''; }
function parseDevice(xml, location) {
  const services = xml.match(/<(?:[\w-]+:)?service\b[^>]*>[\s\S]*?<\/(?:[\w-]+:)?service>/gi) || [];
  for (const service of services) {
    const type = tag(service, 'serviceType');
    if (/^urn:schemas-upnp-org:service:AVTransport:\d+$/.test(type)) {
      const control = tag(service, 'controlURL');
      if (!control) continue;
      return {name:tag(xml, 'friendlyName') || new URL(location).hostname,
        control:new URL(control, tag(xml, 'URLBase') || location).href, type};
    }
  }
  return null;
}
function byteRange(header, size) {
  if (!header) return null;
  const m = /^bytes=(\d*)-(\d*)$/.exec(header);
  if (!m || (!m[1] && !m[2]) || size <= 0) return false;
  let start = m[1] ? Number(m[1]) : Math.max(0, size - Number(m[2]));
  let end = m[1] && m[2] ? Math.min(Number(m[2]), size - 1) : size - 1;
  return Number.isSafeInteger(start) && Number.isSafeInteger(end) && start <= end && start < size ? [start,end] : false;
}
function mediaHandler(file, route, onRead) {
  return (req,res) => {
    if (req.url !== route) { res.writeHead(404); return res.end(); }
    if (!['GET','HEAD'].includes(req.method)) {res.writeHead(405); return res.end();}
    let size;
    try {size = fs.statSync(file).size;} catch {res.writeHead(404); return res.end();}
    const range = byteRange(req.headers.range, size);
    if (range === false) {res.writeHead(416, {'Content-Range':`bytes */${size}`}); return res.end();}
    const [start,end] = range || [0,size-1];
    const headers = {'Content-Type':'video/mp4','Accept-Ranges':'bytes','Content-Length':end-start+1,
      'transferMode.dlna.org':'Streaming','contentFeatures.dlna.org':'DLNA.ORG_OP=01;DLNA.ORG_CI=0'};
    if (range) headers['Content-Range'] = `bytes ${start}-${end}/${size}`;
    res.writeHead(range ? 206 : 200, headers);
    if (req.method === 'HEAD') return res.end();
    onRead();
    const stream = fs.createReadStream(file,{start,end});
    stream.on('error', () => res.destroy());
    res.on('close', () => stream.destroy());
    stream.pipe(res);
  };
}
async function soap(tv, action, values, signal) {
  const args = Object.entries(values).map(([k,v]) => `<${k}>${xmlEscape(v)}</${k}>`).join('');
  const body = `<?xml version="1.0"?><s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/"><s:Body><u:${action} xmlns:u="${tv.type}">${args}</u:${action}></s:Body></s:Envelope>`;
  const response = await fetch(tv.control, {method:'POST',headers:{'Content-Type':'text/xml; charset="utf-8"',SOAPAction:`"${tv.type}#${action}"`},body,signal:signal ? AbortSignal.any([signal,AbortSignal.timeout(10000)]) : AbortSignal.timeout(10000)});
  const result = await response.text();
  if (!response.ok || /<(?:\w+:)?Fault\b/.test(result)) throw new Error(`TV 응답 ${response.status}: ${tag(result,'errorDescription') || result.slice(0,180)}`);
  return result;
}
function addresses() {
  return Object.values(os.networkInterfaces()).flat().filter(n => n.family === 'IPv4' && !n.internal).map(n => n.address);
}
async function discover() {
  const locations = new Map();
  const errors = [];
  await Promise.all(addresses().map(address => new Promise(resolve => {
    const socket = dgram.createSocket('udp4');
    let done = false;
    const finish = () => {if (done) return; done = true; clearTimeout(timer); try{socket.close();}catch{} resolve();};
    const timer = setTimeout(finish, 4500);
    socket.on('error', e => {errors.push(e.message); finish();});
    socket.on('message', message => {
      const m = /^location:\s*(.+)$/im.exec(message.toString());
      if (m) locations.set(m[1].trim(), address);
    });
    socket.bind(0, address, () => {
      try {
        socket.setMulticastInterface(address);
        for (const st of ['urn:schemas-upnp-org:device:MediaRenderer:1','urn:schemas-upnp-org:service:AVTransport:1']) {
          const packet = Buffer.from(`M-SEARCH * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nMAN: "ssdp:discover"\r\nMX: 2\r\nST: ${st}\r\n\r\n`);
          socket.send(packet,1900,'239.255.255.250', e => {if(e) errors.push(e.message);});
        }
      } catch(e) {errors.push(e.message); finish();}
    });
  })));
  const devices = [];
  await Promise.all([...locations].map(async ([location,localAddress]) => {
    try {
      if (new URL(location).protocol !== 'http:') return;
      const r = await fetch(location,{signal:AbortSignal.timeout(3500)});
      const tv = parseDevice(await r.text(),location);
      if (tv && !devices.some(d => d.control === tv.control)) devices.push({...tv,localAddress});
    } catch(e) {errors.push(e.message);}
  }));
  return {devices,errors};
}
function findTool(name) {
  const executable = name + (process.platform === 'win32' ? '.exe' : '');
  for (const dir of (process.env.PATH || '').split(path.delimiter)) {
    const candidate = path.join(dir.replace(/^"|"$/g,''),executable);
    if (fs.existsSync(candidate)) return candidate;
  }
  if (process.platform === 'win32') {
    const base = path.join(process.env.LOCALAPPDATA || '', 'Microsoft','WinGet','Packages');
    try {
      for (const item of fs.readdirSync(base).filter(n => /ffmpeg/i.test(n))) {
        const folder = path.join(base,item);
        for (const child of fs.readdirSync(folder)) {
          const candidate = path.join(folder,child,'bin',executable);
          if (fs.existsSync(candidate)) return candidate;
        }
      }
    } catch {}
  }
  throw new Error(`${name}를 찾지 못했습니다. FFmpeg 설치 후 다시 실행해 주세요.`);
}
function run(exe, args, options = {}) {
  return new Promise((resolve,reject) => {
    const child = spawn(exe,args,{windowsHide:options.windowsHide !== false,cwd:options.cwd,signal:options.signal});
    child.stdin.end();
    let out = '', err = '', spawnError = null;
    child.stdout.on('data', b => {out += b; options.progress?.(b.toString());});
    child.stderr.on('data', b => {err = (err + b).slice(-6000);});
    child.on('error',e => {spawnError = e;});
    child.on('close', code => spawnError ? reject(spawnError) : code === 0 ? resolve(out) : reject(new Error(err || `실행 종료 코드 ${code}`)));
  });
}
function smiToSrt(text, durationMs) {
  const korean = /\.([\w-]+)\s*\{[^}]*lang\s*:\s*ko/i.exec(text)?.[1];
  const cues = [...text.matchAll(/<sync\b[^>]*start\s*=\s*["']?(\d+)[^>]*>([\s\S]*?)(?=<sync\b|$)/gi)];
  const time = ms => {const n=Math.max(0,Math.round(ms));return `${String(Math.floor(n/3600000)).padStart(2,'0')}:${String(Math.floor(n/60000)%60).padStart(2,'0')}:${String(Math.floor(n/1000)%60).padStart(2,'0')},${String(n%1000).padStart(3,'0')}`;};
  const result=[];
  for(let i=0;i<cues.length;i++) {
    const paragraphs=[...cues[i][2].matchAll(/<p\b([^>]*)>([\s\S]*?)(?=<p\b|$)/gi)];
    const pick=paragraphs.find(p => new RegExp(`class\\s*=\\s*["']?${korean || 'KRCC'}(?:["'\\s>]|$)`,'i').test(p[1])) || paragraphs[0];
    const content=(pick ? pick[2] : cues[i][2]).replace(/<br\s*\/?\s*>/gi,'\n').replace(/<[^>]*>/g,'')
      .replace(/&nbsp;/gi,' ').replace(/&#(x[0-9a-f]+|\d+);/gi,(_,n)=> {const code=n[0].toLowerCase()==='x'?parseInt(n.slice(1),16):Number(n);return code<=0x10ffff?String.fromCodePoint(code):'';});
    const line=xmlText(content).trim();
    const start=Number(cues[i][1]),end=i+1<cues.length?Number(cues[i+1][1]):durationMs;
    if(line && end>start) result.push(`${result.length+1}\n${time(start)} --> ${time(end)}\n${line}\n`);
  }
  if(!result.length) throw new Error('SMI 파일에서 자막을 찾지 못했습니다. 다른 자막 파일을 선택해 주세요.');
  return result.join('\n');
}
async function convert(video, subtitle, cache, onProgress = () => {}, signal) {
  for (const file of [video,subtitle]) if (!file || !fs.statSync(file).isFile()) throw new Error('동영상과 자막 파일을 선택해 주세요.');
  const ext = path.extname(subtitle).toLowerCase();
  if (!['.srt','.smi','.ass','.ssa','.vtt'].includes(ext)) throw new Error('SRT, SMI, ASS, SSA, VTT 자막을 선택해 주세요.');
  const ffmpeg = findTool('ffmpeg'), ffprobe = findTool('ffprobe');
  const info = fs.statSync(video);
  const raw = fs.readFileSync(subtitle);
  const key = crypto.createHash('sha256').update(JSON.stringify([video,info.size,info.mtimeMs,ext,'h264-1080-v2'])).update(raw).digest('hex').slice(0,24);
  const work = path.join(cache,key);
  fs.mkdirSync(work,{recursive:true});
  const output = path.join(work,'video.mp4');
  if (fs.existsSync(output)) {onProgress(100); return output;}
  let text;
  try {text = new TextDecoder('utf-8',{fatal:true}).decode(raw);} catch {text = new TextDecoder('euc-kr').decode(raw);}
  if (raw[0] === 255 && raw[1] === 254) text = raw.toString('utf16le');
  const probe = JSON.parse(await run(ffprobe,['-v','error','-show_entries','format=duration','-of','json',video],{signal}));
  const duration = Number(probe.format?.duration) || 0;
  const subname = 'subtitle' + (ext === '.smi' ? '.srt' : ext);
  text=text.replace(/^\uFEFF/,'');
  fs.writeFileSync(path.join(work,subname),ext === '.smi' ? smiToSrt(text,duration*1000) : text,'utf8');
  const partial = path.join(work,'partial.mp4');
  const style = 'FontName=Malgun Gothic,FontSize=22,Outline=2,Shadow=0,MarginV=24';
  const filter = `scale=w='min(1920,iw)':h='min(1080,ih)':force_original_aspect_ratio=decrease:force_divisible_by=2,subtitles=${subname}:force_style='${style}'`;
  try {
    await run(ffmpeg,['-hide_banner','-y','-nostdin','-i',video,'-map','0:v:0','-map','0:a:0?',
      '-vf',filter,'-c:v','libx264','-preset','veryfast','-crf','21','-pix_fmt','yuv420p',
      '-c:a','aac','-b:a','192k','-ac','2','-sn','-movflags','+faststart',
      '-progress','pipe:1','-nostats',partial],{cwd:work,signal,progress:s => {
        const m = /out_time_us=(\d+)/.exec(s);
        if (m && duration) onProgress(Math.min(99,Math.round(Number(m[1])/1000000/duration*100)));
      }});
    fs.renameSync(partial,output); onProgress(100); return output;
  } finally {try{fs.unlinkSync(partial);}catch{}}
}
async function startMedia(file, localAddress, onRead) {
  const route = '/' + crypto.randomBytes(18).toString('hex') + '/video.mp4';
  const server = http.createServer(mediaHandler(file,route,onRead));
  await new Promise((resolve,reject) => {server.once('error',reject); server.listen(0,'0.0.0.0',resolve);});
  return {server,url:`http://${localAddress}:${server.address().port}${route}`};
}
async function play(tv,url,signal) {
  const metadata = `<DIDL-Lite xmlns="urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:upnp="urn:schemas-upnp-org:metadata-1-0/upnp/"><item id="1" parentID="0" restricted="1"><dc:title>자막 영상</dc:title><upnp:class>object.item.videoItem</upnp:class><res protocolInfo="http-get:*:video/mp4:*">${xmlEscape(url)}</res></item></DIDL-Lite>`;
  // 다른 앱이 재생·일시정지 중이면 LG TV가 "Transition not available"(500)로 새 주소를 거절한다. 먼저 정지하고 멈출 때까지 기다린다.
  try {await soap(tv,'Stop',{InstanceID:0},signal);} catch(e) {signal?.throwIfAborted();}
  for (let i = 0; i < 8; i++) {
    let state = '';
    try {state = tag(await soap(tv,'GetTransportInfo',{InstanceID:0},signal),'CurrentTransportState');} catch(e) {signal?.throwIfAborted(); break;}
    if (!state || /^(STOPPED|NO_MEDIA_PRESENT)$/i.test(state)) break;
    await new Promise(r => setTimeout(r,500));
  }
  signal?.throwIfAborted();
  await soap(tv,'SetAVTransportURI',{InstanceID:0,CurrentURI:url,CurrentURIMetaData:metadata},signal);
  signal?.throwIfAborted();
  await soap(tv,'Play',{InstanceID:0,Speed:1},signal);
}
module.exports = {xmlEscape,parseDevice,byteRange,mediaHandler,soap,addresses,discover,findTool,run,convert,startMedia,play,smiToSrt};
