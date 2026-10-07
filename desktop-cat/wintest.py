# GitHub Actions 윈도우 러너에서 실제로 돌려 보는 시험. 결과(보고서·스크린샷·로그)를 out 폴더에 남긴다.
#   python desktop-cat/wintest.py out
import os
import sys
import time
import ctypes
import subprocess
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else 'out')
os.makedirs(OUT, exist_ok=True)
sys.path.insert(0, HERE)
ctypes.windll.user32.SetProcessDpiAwarenessContext(ctypes.c_void_p(-4))
import win_world as W                     # noqa: E402
from PIL import ImageGrab                 # noqa: E402

rep = []


def say(*a):
    s = ' '.join(str(x) for x in a)
    print(s, flush=True)
    rep.append(s)


def last_pos():
    try:
        lines = [l for l in open(os.path.join(HERE, 'desktop_cat.log'), encoding='utf-8') if ' test ' in l]
        part = lines[-1].split()
        return int([p for p in part if p.startswith('x=')][0][2:]), int([p for p in part if p.startswith('y=')][0][2:])
    except (OSError, IndexError, ValueError):
        return None


def shot(name, around=None):
    im = ImageGrab.grab(include_layered_windows=True, all_screens=True)
    im.save(os.path.join(OUT, name + '.png'))
    if around:
        x, y = around
        im.crop((x - 160, y - 140, x + 160, y + 40)).save(os.path.join(OUT, name + '_cat.png'))


try:
    say('monitors', W.monitors())
    sample = os.path.join(OUT, 'sample.txt')
    with open(sample, 'w', encoding='utf-8') as f:
        f.write('\n'.join(['The quick brown fox jumps over the lazy dog. ' * 2] * 8))
    pad = subprocess.Popen(['notepad.exe', sample])
    time.sleep(3)
    wins = W.windows()
    say('windows', len(wins), wins[:5])
    wp, refs = W.window_platforms(wins)
    say('window platforms', [(p.x1, p.x2, p.y) for p in wp][:8])
    say('icons', W.IconReader().read()[:8])
    t = time.time()
    g = W.grab_gray(0, 0, 1000, 700)
    e = W.find_edges(g, 0, 0)
    say('edges', len(e), 'ms', round((time.time() - t) * 1000), e[:6])

    log = os.path.join(HERE, 'desktop_cat.log')
    if os.path.exists(log):
        os.remove(log)
    cat = subprocess.Popen([sys.executable, os.path.join(HERE, 'desktop_cat.py')],
                           env=dict(os.environ, DESKTOP_CAT_TEST='1'))
    for i in range(10):
        time.sleep(2.5)
        shot('shot%d' % i, last_pos())
    say('alive after 25s', cat.poll() is None)
    pos = last_pos()
    if pos:                                   # 마우스를 고양이 바로 옆으로 — 도망치는지
        ctypes.windll.user32.SetCursorPos(pos[0] + 60, pos[1] - 20)
        for k in range(12):
            time.sleep(0.05)
            ctypes.windll.user32.SetCursorPos(pos[0] + 40 - k * 3, pos[1] - 25)
        for i in range(3):
            time.sleep(0.35)
            shot('flee%d' % i, pos)
        time.sleep(2.5)
    say('alive after mouse', cat.poll() is None)

    u = ctypes.windll.user32
    pos = last_pos()                          # Ctrl 누른 채 고양이 좌클릭 = 쓰다듬기 → love
    if pos:
        u.keybd_event(0x11, 0, 0, 0)
        time.sleep(0.3)
        u.SetCursorPos(pos[0], pos[1] - 30)
        time.sleep(0.2)
        u.mouse_event(0x0002, 0, 0, 0, 0)
        u.mouse_event(0x0004, 0, 0, 0, 0)
        time.sleep(0.3)
        shot('pet', pos)
        time.sleep(2.0)
        u.keybd_event(0x11, 0, 2, 0)
        txt = open(os.path.join(HERE, 'desktop_cat.log'), encoding='utf-8').read()
        say('ctrl+click pet -> love logged', 'action=love' in txt)

    cat2 = subprocess.Popen([sys.executable, os.path.join(HERE, 'desktop_cat.py')],
                            env=dict(os.environ, DESKTOP_CAT_TEST='1'))
    time.sleep(4)
    say('second run replaced first:', cat.poll() is not None, 'second alive:', cat2.poll() is None)

    for vk, up in ((0x11, 0), (0x12, 0), (0x51, 0), (0x51, 2), (0x12, 2), (0x11, 2)):   # Ctrl+Alt+Q
        u.keybd_event(vk, 0, up, 0)
        time.sleep(0.05)
    time.sleep(2.5)
    say('ctrl+alt+q quit:', cat2.poll() is not None)
    for c in (cat, cat2):
        if c.poll() is None:
            c.terminate()
    pad.kill()
    time.sleep(1)
    if os.path.exists(log):
        txt = open(log, encoding='utf-8').read()
        open(os.path.join(OUT, 'desktop_cat.log'), 'w', encoding='utf-8').write(txt)
        say('log tail:\n' + '\n'.join(txt.splitlines()[-25:]))
except Exception:
    say('ERROR', traceback.format_exc())
finally:
    open(os.path.join(OUT, 'report.txt'), 'w', encoding='utf-8').write('\n'.join(rep))
