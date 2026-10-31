# 인스타·유튜브 등 짧은 영상을 AI에게 보이지 않고 이 PC에서 글자로 바꾼다(토큰 절약).
# 사용: python tools/reel_text.py <주소> [주소...]  → 캡션과 음성 받아쓰기를 출력
# 화면까지 봐야 할 때만: python tools/reel_text.py --frames <주소>  → 장면이 바뀐 곳만 낱장 저장(최대 12장)
import os, subprocess, sys, tempfile
import numpy
import yt_dlp
from faster_whisper import WhisperModel

MAX_FRAMES = 12


def fetch(url, folder, frames):
    opts = {'quiet': True, 'noprogress': True, 'no_warnings': True, 'outtmpl': os.path.join(folder, '%(id)s.%(ext)s')}
    opts['format'] = 'best' if frames else 'bestaudio/best'
    with yt_dlp.YoutubeDL(opts) as ydl:
        info = ydl.extract_info(url, download=True)
        return info, ydl.prepare_filename(info)


def transcribe(model, path):
    # faster-whisper 내장 디코더(PyAV)가 버전 충돌로 죽어서 ffmpeg 로 푼 소리를 넘긴다
    pcm = subprocess.run(['ffmpeg', '-v', 'error', '-i', path, '-f', 's16le', '-ac', '1', '-ar', '16000', '-'],
                         capture_output=True, check=True).stdout
    audio = numpy.frombuffer(pcm, numpy.int16).astype(numpy.float32) / 32768
    parts, _ = model.transcribe(audio, language='ko', vad_filter=True)
    return '\n'.join(f'[{int(p.start)}초] {p.text.strip()}' for p in parts)


def scene_frames(path, out):
    os.makedirs(out, exist_ok=True)
    subprocess.run(['ffmpeg', '-v', 'error', '-y', '-i', path, '-vf',
                    "select='eq(n,0)+gt(scene,0.3)',scale=480:-2", '-vsync', 'vfr',
                    '-frames:v', str(MAX_FRAMES), os.path.join(out, 'f%02d.jpg')], check=True)
    return out


def main():
    args = sys.argv[1:]
    frames = '--frames' in args
    urls = [a for a in args if a != '--frames']
    if not urls:
        sys.exit(__doc__ or '주소를 넣어 주세요')
    sys.stdout.reconfigure(encoding='utf-8')
    model = WhisperModel('small', device='cpu', compute_type='int8')
    for url in urls:
        with tempfile.TemporaryDirectory() as folder:
            info, path = fetch(url, folder, frames)
            print(f'## {info.get("uploader")} / {url}')
            print(f'캡션:\n{info.get("description") or "(없음)"}\n')
            print(f'받아쓰기:\n{transcribe(model, path) or "(음성 없음)"}\n')
            if frames:
                print('장면:', scene_frames(path, os.path.join(tempfile.gettempdir(), 'reel_frames', info['id'])))


main()
