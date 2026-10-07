# 가짜 바탕화면(모니터 2대·아이콘·창·글자)에서 고양이를 돌려 GIF 로 저장한다. 맥·리눅스에서도 된다.
#   python preview.py --seconds 40 --out cat.gif
import argparse
import math
import random
from PIL import Image, ImageDraw
from cat_brain import CatBrain, Platform, ground_platforms
from cat_render import CatRenderer, OX, OY

W1, W2, H = 640, 640, 360
MONITORS = [dict(rect=(0, 0, W1, H), work=(0, 0, W1, H - 28)),
            dict(rect=(W1, 0, W1 + W2, H), work=(W1, 0, W1 + W2, H - 22))]
ICONS = [(20 + c * 64, 16 + r * 70) for c in range(2) for r in range(4)] + [(W1 + 560, 30), (W1 + 560, 100)]
WIN = (230, 70, 560, 290)                     # 창 하나 (글자 줄이 발판이 된다)
TEXT_LINES = [(250, 120, 470), (250, 150, 520), (250, 180, 430), (250, 240, 540)]


def platforms():
    ps = ground_platforms(MONITORS)
    for i, (x, y) in enumerate(ICONS):
        ps.append(Platform(x, x + 40, y, 'icon', ('icon', i)))
    ps.append(Platform(WIN[0], WIN[2], WIN[1], 'window', ('win', 1)))
    for x1, y, x2 in TEXT_LINES:
        ps.append(Platform(x1, x2, y, 'edge'))
    return ps


def background():
    im = Image.new('RGBA', (W1 + W2, H), (52, 92, 140, 255))
    d = ImageDraw.Draw(im)
    d.rectangle([W1, 0, W1 + W2, H], fill=(70, 110, 90, 255))
    for m in MONITORS:
        d.rectangle([m['work'][0], m['work'][3], m['work'][2], H], fill=(30, 30, 36, 255))
    d.line([W1, 0, W1, H], fill=(0, 0, 0, 255), width=3)
    for x, y in ICONS:
        d.rounded_rectangle([x, y, x + 40, y + 40], 6, fill=(240, 200, 90, 255))
        d.rectangle([x - 4, y + 46, x + 44, y + 52], fill=(220, 220, 220, 255))
    d.rectangle(WIN, fill=(250, 250, 250, 255), outline=(90, 90, 90, 255))
    d.rectangle([WIN[0], WIN[1], WIN[2], WIN[1] + 22], fill=(225, 230, 240, 255))
    for x1, y, x2 in TEXT_LINES:
        for x in range(x1, x2, 9):
            d.rectangle([x, y, x + 6, y + 9], fill=(40, 40, 40, 255))
    return im


def cursor_at(t):
    """대부분은 구석에 있다가, 가끔 고양이 쪽으로 다가온다."""
    return None if (t % 20) < 13 else 'chase'


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--seconds', type=float, default=40)
    ap.add_argument('--fps', type=int, default=12)
    ap.add_argument('--out', default='cat_preview.gif')
    ap.add_argument('--scale', type=float, default=1.0)
    ap.add_argument('--color', default='cheese')
    ap.add_argument('--seed', type=int, default=3)
    a = ap.parse_args()
    random.seed(a.seed)
    brain = CatBrain(a.scale, MONITORS)
    rend = CatRenderer(a.scale, a.color)
    bg = background()
    frames, dt, sub = [], 1 / 30, max(1, round(30 / a.fps))
    cur = [W1 + W2 - 30.0, 30.0]
    for i in range(int(a.seconds * 30)):
        t = i * dt
        if cursor_at(t) == 'chase':                 # 마우스가 고양이를 쫓아간다
            tx, ty = brain.x, brain.y - 20
            dx, dy = tx - cur[0], ty - cur[1]
            n = math.hypot(dx, dy) or 1
            cur[0] += dx / n * min(n, 9)
            cur[1] += dy / n * min(n, 9)
        world = dict(monitors=MONITORS, platforms=platforms(), cursor=tuple(cur), refs={})
        x, y, facing, pose = brain.update(dt, world)
        if i % sub == 0:
            im = bg.copy()
            cat = rend.render(pose, facing, brain.time)
            im.alpha_composite(cat, (int(x - OX * a.scale), int(y - OY * a.scale)))
            d = ImageDraw.Draw(im)
            d.polygon([(cur[0], cur[1]), (cur[0], cur[1] + 16), (cur[0] + 11, cur[1] + 11)],
                      fill=(255, 255, 255, 255), outline=(0, 0, 0, 255))
            d.text((6, H - 22), '%5.1fs  %s' % (t, brain.action), fill=(255, 255, 255, 255))
            frames.append(im.convert('RGB').convert('P', palette=Image.ADAPTIVE, colors=128))
    frames[0].save(a.out, save_all=True, append_images=frames[1:], duration=int(1000 / a.fps), loop=0)
    print('saved', a.out, len(frames), 'frames')


if __name__ == '__main__':
    main()
