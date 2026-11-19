const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const {spawn} = require('node:child_process');
const core = require('./core');

test('real FFmpeg burns Korean subtitles, preserves original and reuses completed output', {timeout:60000}, async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(),"tv-한글 ' 경로-"));
  const video = path.join(dir,'원본 영상.mp4'), subtitle = path.join(dir,'자막.srt');
  try {
    await core.run(core.findTool('ffmpeg'),['-hide_banner','-y','-f','lavfi','-i','color=c=black:s=640x360:d=2','-f','lavfi','-i','sine=frequency=440:duration=2','-c:v','libx264','-c:a','aac','-shortest',video]);
    const original = fs.readFileSync(video);
    fs.writeFileSync(subtitle,'1\n00:00:00,100 --> 00:00:01,400\n한글 자막 시험\n');
    const output = await core.convert(video,subtitle,path.join(dir,'cache'));
    assert.deepEqual(fs.readFileSync(video),original);
    const stats = fs.statSync(output);
    assert.equal(await core.convert(video,subtitle,path.join(dir,'cache')),output);
    assert.equal(fs.statSync(output).mtimeMs,stats.mtimeMs);
    // White subtitle pixels must exist in the lower half of the burned frame.
    const raw = path.join(dir,'frame.gray');
    await core.run(core.findTool('ffmpeg'),['-v','error','-y','-ss','0.7','-i',output,'-frames:v','1','-vf','crop=640:180:0:180','-pix_fmt','gray','-f','rawvideo',raw]);
    const pixels = fs.readFileSync(raw);
    assert.ok(pixels.filter(b => b > 150).length > 100,'subtitle pixels must be burned into frame');
    const probe = JSON.parse(await core.run(core.findTool('ffprobe'),['-v','error','-show_streams','-of','json',output]));
    assert.equal(probe.streams[0].codec_name,'h264');
    assert.equal(probe.streams[1].codec_name,'aac');
    // CP949 SMI: 한글, and a second sync marker to end its cue.
    const smi = path.join(dir,'legacy.smi');
    fs.writeFileSync(smi,Buffer.concat([Buffer.from('<SAMI><HEAD><STYLE>p {font-family:Malgun Gothic;} .KRCC {Name:Korean; lang:ko-KR; SAMIType:CC;}</STYLE></HEAD><BODY><SYNC Start=100><P Class=KRCC>'),Buffer.from([0xc7,0xd1,0xb1,0xdb]),Buffer.from('<SYNC Start=1400><P Class=KRCC>&nbsp;</BODY></SAMI>')]));
    const legacy = await core.convert(video,smi,path.join(dir,'cache'));
    assert.ok(fs.readFileSync(path.join(path.dirname(legacy),'subtitle.srt'),'utf8').includes('한글'));
  } finally {fs.rmSync(dir,{recursive:true,force:true});}
});

test('failed conversion leaves no reusable video cache', {timeout:30000}, async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(),'tv-fail-'));
  try {
    const video=path.join(dir,'bad.mp4'), sub=path.join(dir,'s.srt');
    fs.writeFileSync(video,'invalid'); fs.writeFileSync(sub,'1\n00:00:00,000 --> 00:00:01,000\nhello');
    await assert.rejects(core.convert(video,sub,path.join(dir,'cache')));
    for(const folder of fs.readdirSync(path.join(dir,'cache'))) assert.ok(!fs.existsSync(path.join(dir,'cache',folder,'video.mp4')));
  } finally {fs.rmSync(dir,{recursive:true,force:true});}
});

test('control UI requires session header and exposes no file listing', {timeout:15000}, async () => {
  const child=spawn(process.execPath,[path.join(__dirname,'app.js'),'--no-open'],{windowsHide:true});
  try {
    const url=await new Promise((resolve,reject) => {
      const timer=setTimeout(()=>reject(Error('startup timeout')),5000);
      child.stdout.on('data',b=>{const m=/http:\/\/127\.0\.0\.1:\d+\//.exec(b.toString());if(m){clearTimeout(timer);resolve(m[0]);}});
      child.once('error',reject);
    });
    assert.equal((await fetch(url+'state')).status,403);
    const html=await (await fetch(url)).text();
    const token=/const token='([a-f0-9]+)'/.exec(html)[1];
    const headers={'X-Cast-Token':token};
    assert.equal((await fetch(url+'state',{headers})).status,200);
    assert.equal((await fetch(url+'unknown',{headers})).status,404);
    assert.equal((await fetch(url+'quit',{method:'POST',headers,body:'{}'})).status,200);
  } finally {child.kill();}
});
