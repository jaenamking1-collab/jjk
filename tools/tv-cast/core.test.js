const test = require('node:test');
const assert = require('node:assert/strict');
const http = require('node:http');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const core = require('./core');

test('SMI keeps Korean cues, HTML breaks and ending blank timestamps', () => {
  const smi='<SAMI><STYLE>.KRCC {lang: ko-KR;} .ENCC {lang: en-US;}</STYLE><BODY><SYNC Start=100><P Class=KRCC>한글<br>둘째 줄<P Class=ENCC>English<SYNC Start=1400><P Class=KRCC>&nbsp;</BODY></SAMI>';
  const srt=core.smiToSrt(smi,2000);
  assert.match(srt,/00:00:00,100 --> 00:00:01,400/);
  assert.match(srt,/한글\n둘째 줄/);
  assert.ok(!srt.includes('English'));
});

test('HTTP ranges include suffix and reject invalid ranges', () => {
  assert.deepEqual(core.byteRange('bytes=2-5', 10), [2, 5]);
  assert.deepEqual(core.byteRange('bytes=7-', 10), [7, 9]);
  assert.deepEqual(core.byteRange('bytes=-3', 10), [7, 9]);
  assert.equal(core.byteRange('bytes=10-', 10), false);
  assert.equal(core.byteRange('bytes=5-2', 10), false);
  assert.equal(core.byteRange(undefined, 10), null);
});

test('device XML resolves relative AVTransport URL and XML entities', () => {
  const xml = '<root><device><friendlyName>LG &amp; TV</friendlyName><serviceList><service><serviceType>urn:schemas-upnp-org:service:AVTransport:1</serviceType><controlURL>/upnp/control</controlURL></service></serviceList></device></root>';
  const tv = core.parseDevice(xml, 'http://192.168.1.10:123/desc.xml');
  assert.equal(tv.name, 'LG & TV');
  assert.equal(tv.control, 'http://192.168.1.10:123/upnp/control');
  assert.equal(core.parseDevice('<root/>', 'http://192.168.1.10/'), null);
});

test('media server serves only its token URL, HEAD and byte ranges', async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'cast-test-'));
  const file = path.join(dir, 'video.mp4');
  fs.writeFileSync(file, '0123456789');
  const server = http.createServer(core.mediaHandler(file, '/secret/video.mp4', () => {}));
  await new Promise(r => server.listen(0, '127.0.0.1', r));
  const url = `http://127.0.0.1:${server.address().port}`;
  try {
    const r = await fetch(url + '/secret/video.mp4', {headers:{Range:'bytes=2-5'}});
    assert.equal(r.status, 206);
    assert.equal(r.headers.get('content-range'), 'bytes 2-5/10');
    assert.equal(await r.text(), '2345');
    assert.equal((await fetch(url + '/secret/video.mp4', {method:'HEAD'})).headers.get('content-length'), '10');
    assert.equal((await fetch(url + '/secret/video.mp4', {headers:{Range:'bytes=99-'}})).status, 416);
    assert.equal((await fetch(url + '/other')).status, 404);
  } finally { await new Promise(r => server.close(r)); fs.rmSync(dir, {recursive:true}); }
});

test('SOAP escapes URI and reports TV rejection', async () => {
  let body = '';
  let reject = false;
  const server = http.createServer(async (req,res) => {
    for await (const chunk of req) body += chunk;
    res.writeHead(reject ? 500 : 200); res.end(reject ? '<errorDescription>Illegal MIME-type</errorDescription>' : '<ok/>');
  });
  await new Promise(r => server.listen(0, '127.0.0.1', r));
  const tv = {control:`http://127.0.0.1:${server.address().port}/control`, type:'urn:schemas-upnp-org:service:AVTransport:1'};
  try {
    await core.soap(tv, 'SetAVTransportURI', {InstanceID:0,CurrentURI:'http://host/?a=1&b=2'});
    assert.match(body, /a=1&amp;b=2/);
    reject = true;
    await assert.rejects(core.soap(tv, 'Play', {InstanceID:0,Speed:1}), /Illegal MIME-type/);
  } finally { await new Promise(r => server.close(r)); }
});

test('stopping during SetURI prevents any subsequent Play command', async () => {
  const controller=new AbortController(), actions=[];
  const server=http.createServer((req,res)=>{
    const action=req.headers.soapaction;
    actions.push(action);
    if(action.includes('SetAVTransportURI')) {controller.abort();setTimeout(()=>res.end('<ok/>'),50);}
    else res.end('<ok/>');
  });
  await new Promise(r=>server.listen(0,'127.0.0.1',r));
  try {
    await assert.rejects(core.play({control:`http://127.0.0.1:${server.address().port}/`,type:'urn:schemas-upnp-org:service:AVTransport:1'},'http://host/video.mp4',controller.signal));
    assert.ok(actions.some(a=>a.includes('SetAVTransportURI')));
    assert.ok(!actions.some(a=>a.includes('#Play')));
  } finally {server.closeAllConnections();await new Promise(r=>server.close(r));}
});

test('play stops a TV paused by another app before SetURI (LG 500 Transition not available)', async () => {
  let state='PAUSED_PLAYBACK'; const actions=[];
  const server=http.createServer((req,res)=>{
    const action=req.headers.soapaction.replace(/.*#|"/g,''); actions.push(action);
    if(action==='Stop') {state='STOPPED'; return res.end('<ok/>');}
    if(action==='GetTransportInfo') return res.end(`<CurrentTransportState>${state}</CurrentTransportState>`);
    if(action==='SetAVTransportURI' && state!=='STOPPED') {res.statusCode=500; return res.end('<errorDescription>Transition not available</errorDescription>');}
    res.end('<ok/>');
  });
  await new Promise(r=>server.listen(0,'127.0.0.1',r));
  try {
    await core.play({control:`http://127.0.0.1:${server.address().port}/`,type:'urn:schemas-upnp-org:service:AVTransport:1'},'http://host/video.mp4');
    assert.deepEqual(actions,['Stop','GetTransportInfo','SetAVTransportURI','Play']);
  } finally {server.closeAllConnections();await new Promise(r=>server.close(r));}
});

test('startLive streams MPEG-TS with burned subtitles immediately and stops ffmpeg on disconnect', async () => {
  const os=require('node:os'), fs=require('node:fs'), path=require('node:path'), {execFileSync}=require('node:child_process');
  const dir=fs.mkdtempSync(path.join(os.tmpdir(),'live-')), v=path.join(dir,'a.mp4'), s=path.join(dir,'a.srt');
  execFileSync(core.findTool('ffmpeg'),['-y','-loglevel','error','-f','lavfi','-i','testsrc=size=640x360:rate=25:duration=3','-f','lavfi','-i','sine=duration=3','-c:v','libx264','-pix_fmt','yuv420p','-c:a','aac','-shortest',v]);
  fs.writeFileSync(s,'1\n00:00:00,000 --> 00:00:02,000\n안녕하세요\n');
  const live=await core.startLive(v,s,path.join(dir,'cache'),'127.0.0.1');
  try {
    const r=await fetch(live.url); assert.equal(r.status,200); assert.equal(r.headers.get('content-type'),'video/mpeg');
    const reader=r.body.getReader(); const {value}=await reader.read();
    assert.equal(value[0],0x47);
    await reader.cancel();
    assert.equal((await fetch(live.url.replace('video.ts','x.ts'))).status,404);
  } finally {live.server.closeAllConnections();await new Promise(r=>live.server.close(r));}
});

test('remux puts Korean subtitle into mkv without re-encoding video, null for unsupported codec', async () => {
  const os=require('node:os'), fs=require('node:fs'), path=require('node:path'), {execFileSync}=require('node:child_process');
  const ff=core.findTool('ffmpeg'), dir=fs.mkdtempSync(path.join(os.tmpdir(),'remux-')), v=path.join(dir,'a.mp4'), m=path.join(dir,'b.avi'), s=path.join(dir,'a.smi');
  execFileSync(ff,['-y','-loglevel','error','-f','lavfi','-i','testsrc=size=640x360:rate=25:duration=2','-f','lavfi','-i','sine=duration=2','-c:v','libx264','-pix_fmt','yuv420p','-c:a','aac','-shortest',v]);
  execFileSync(ff,['-y','-loglevel','error','-f','lavfi','-i','testsrc=size=320x240:rate=25:duration=1','-c:v','mpeg4',m]);
  fs.writeFileSync(s,'<SAMI><BODY><SYNC Start=0><P Class=KRCC>안녕하세요<SYNC Start=1500><P Class=KRCC>&nbsp;</BODY></SAMI>');
  const r=await core.remux(v,s,path.join(dir,'cache'));
  const info=JSON.parse(execFileSync(core.findTool('ffprobe'),['-v','error','-show_entries','stream=codec_name,codec_type:stream_tags=language','-of','json',r.output]));
  assert.deepEqual(info.streams.map(x=>x.codec_name),['h264','aac','subrip']);
  assert.equal(info.streams[2].tags.language,'kor');
  assert.equal(await core.remux(m,s,path.join(dir,'cache')),null);
});
