# 사용자가 준 그림(sprites/*.png)으로 고양이를 그린다. 행동 두뇌(cat_brain)의 동작 이름을 받아
# 알맞은 그림을 고르고, 통통 튀기·기울이기·숨쉬기·좌우 뒤집기로 움직임을 만든다.
import os
import json
import math
from PIL import Image, ImageDraw
from cat_render import HANG, clamp, ease

HERE = os.path.dirname(os.path.abspath(__file__))
SPR = os.path.join(HERE, 'sprites')

# 동작 → 그림. 그림이 없는 동작은 가장 비슷한 그림 + 움직임으로 대신한다.
ART = {
    'walk': 'walk', 'trot': 'walk', 'back': 'walk', 'hop': 'walk', 'stand': 'walk', 'knead': 'happy',
    'sneak': 'hunt', 'hunt': 'hunt', 'crouch': 'hunt', 'run': 'run', 'tailchase': 'run', 'air': 'run',
    'sit': 'headtilt', 'watch': 'headtilt', 'sitpretty': 'headtilt', 'zoneout': 'headtilt', 'headtilt': 'headtilt',
    'scratch': 'headtilt', 'shake': 'headtilt',
    'curious': 'curious', 'confused': 'curious', 'lookaround': 'curious', 'lookdown': 'curious', 'fall': 'curious',
    'happy': 'happy', 'meow': 'happy', 'chatter': 'happy', 'love': 'happy', 'groom': 'happy', 'lickpaw': 'happy',
    'wash': 'happy', 'groom_leg': 'happy', 'sneeze': 'happy', 'yawn': 'happy',
    'loaf': 'loaf', 'sploot': 'loaf', 'paw_dangle': 'loaf', 'tailflick': 'loaf',
    'land': 'loaf', 'land_hard': 'loaf', 'brace': 'loaf',
    'sleep': 'sleep', 'sleep_back': 'sleep', 'sleep_side': 'sleep', 'sulk': 'sulk',
    'belly': 'belly', 'roll': 'roll', 'stretch': 'stretch', 'stretch_back': 'stretch',
    'bugcatch': 'bugcatch', 'swat': 'bugcatch', 'claw': 'bugcatch',
    'hang': 'bugcatch', 'hang_kick': 'bugcatch', 'hang_one': 'bugcatch', 'climbup': 'bugcatch',
    'climb': 'bugcatch', 'cling': 'bugcatch', 'slide': 'curious',
    'hugheart': 'hugheart', 'stargaze': 'stargaze', 'fishtoy': 'fishtoy', 'hat': 'hat', 'box': 'box',
    'backview': 'backview', 'peek': 'peek',
}
WALKS = {'walk', 'trot', 'back', 'hop', 'sneak', 'run', 'tailchase'}


class SpriteRenderer:
    def __init__(self, scale=1.3, palette=None):
        self.s = scale
        meta = json.load(open(os.path.join(SPR, 'sprites.json'), encoding='utf-8'))
        f = 82 * scale / 220.0                    # 그림 속 고양이(약 220px) → 화면 약 82px × 배율
        self.img, self.face = {}, {}
        for name, m in meta.items():
            im = Image.open(os.path.join(SPR, name + '.png')).convert('RGBA')
            self.img[name] = im.resize((max(1, round(im.width * f)), max(1, round(im.height * f))), Image.LANCZOS)
            self.face[name] = m['facing']
        mw = max(i.width for i in self.img.values())
        mh = max(i.height for i in self.img.values())
        self.below = int(HANG * scale + 8)        # 매달릴 때 원점 아래로 내려가는 몫
        self.W, self.H = int(mw * 2.3), int(mh * 1.5) + self.below   # 벽에 붙을 땐 원점 한쪽으로 그림 전체가 간다
        self.ox, self.oy = self.W // 2, self.H - self.below
        self._mirror = {}

    def _get(self, name, facing):
        nf = self.face[name]
        if nf and nf != facing:
            if name not in self._mirror:
                self._mirror[name] = self.img[name].transpose(Image.FLIP_LEFT_RIGHT)
            return self._mirror[name]
        return self.img[name]

    def render_action(self, action, at, facing, time, brain=None):
        name = ART.get(action, 'headtilt')
        if action == 'roll' and int(at / 0.5) % 2:
            name = 'belly'
        im = self._get(name, facing)
        S = self.s
        dx = dy = rot = 0.0
        sx = sy = 1.0
        anchor = 'bottom'
        breathe = math.sin(time * 2.2)
        ph = brain.walk_phase if brain else time

        if action in WALKS:                       # 걸음: 한 걸음마다 통통 + 좌우로 살짝 흔들
            amp = {'run': 6, 'tailchase': 6, 'hop': 9, 'trot': 4, 'sneak': 1.5}.get(action, 3) * S
            dy = -abs(math.sin(2 * math.pi * ph)) * amp
            rot = (3 if action != 'run' else 2) * math.sin(2 * math.pi * ph)
            sy = 1 - 0.04 * math.cos(4 * math.pi * ph)
            if action == 'run':
                rot += 4
        elif action == 'air' and brain and brain.jump:
            k = clamp(brain.jump['t'] / brain.jump['T'])
            rot = -18 + 36 * k                    # 오를 땐 머리 들고, 내려갈 땐 숙인다
            sx, sy = 1.06, 0.95
        elif action == 'fall':
            rot = 10 * math.sin(time * 12)
            sy = 1.06
        elif action in ('land', 'land_hard', 'brace'):
            k = math.sin(clamp(at / 0.3) * math.pi)
            sx, sy = 1 + 0.12 * k, 1 - 0.14 * k
        elif action in ('hunt', 'crouch'):        # 엉덩이 실룩
            rot = 3 * math.sin(at * 18)
        elif action in ('shake', 'scratch'):
            rot = 8 * math.sin(at * 30)
        elif action in ('groom', 'lickpaw', 'wash', 'groom_leg', 'yawn'):
            rot = 5 * math.sin(at * 6)
            dy = -abs(math.sin(at * 6)) * 1.5 * S
        elif action in ('happy', 'meow', 'chatter', 'love', 'knead', 'sneeze'):
            dy = -abs(math.sin(at * 5)) * 3 * S
        elif action in ('curious', 'confused', 'headtilt', 'lookaround'):
            rot = 7 * math.sin(at * 1.6)
        elif action == 'lookdown':
            rot = 14 * facing
        elif action in ('bugcatch', 'swat', 'claw'):
            dy = -abs(math.sin(at * 4)) * 5 * S
        elif action in ('belly', 'roll'):
            rot = 6 * math.sin(at * 3)
        elif action in ('sleep', 'sleep_back', 'sleep_side', 'loaf', 'sploot', 'sulk', 'box'):
            sx, sy = 1 + 0.015 * breathe, 1 - 0.015 * breathe
        elif action in ('hang', 'hang_kick', 'hang_one', 'climbup'):
            anchor = 'hang'
            sp = {'hang': (5, 2.2), 'hang_kick': (7, 9), 'hang_one': (12, 2.6)}.get(action, (2, 6))
            rot = sp[0] * math.sin(time * sp[1])
        elif action in ('climb', 'cling', 'slide'):
            anchor = 'wall'
            rot = -8 * facing if action == 'climb' else 0
            dy = (-abs(math.sin(2 * math.pi * ph)) * 2 * S) if action == 'climb' else 0
        else:
            sx, sy = 1 + 0.01 * breathe, 1 - 0.01 * breathe

        if sx != 1 or sy != 1:
            im = im.resize((max(1, round(im.width * sx)), max(1, round(im.height * sy))), Image.BILINEAR)
        if abs(rot) > 0.3:
            im = im.rotate(-rot * facing, Image.BICUBIC, expand=True)
        w, h = im.size
        canvas = Image.new('RGBA', (self.W, self.H), (0, 0, 0, 0))
        if anchor == 'hang':                      # 앞발(그림 위쪽)이 발판에 걸리게
            e = ease(brain.cprog) if (brain and action == 'climbup') else 0.0
            bottom = self.oy + (h - HANG * S) * (1 - e)
            x = self.ox - w / 2
        elif anchor == 'wall':                    # 벽 쪽 가장자리를 벽에 붙인다
            bottom = self.oy
            x = self.ox - w if facing > 0 else self.ox
        else:
            bottom = self.oy
            x = self.ox - w / 2
        if action == 'peek' and brain is not None and brain.clip > -60:
            edge = self.ox + brain.clip * S * facing          # 창 모서리
            x = edge if facing > 0 else edge - w
        canvas.alpha_composite(im, (int(x + dx), int(bottom - h + dy)))
        if brain is not None and brain.mark_t > 0:            # 깜짝 놀람 !
            d = ImageDraw.Draw(canvas)
            cx, top = self.ox + 0.25 * w * facing, int(bottom - h - 4 * S)
            d.rounded_rectangle([cx - 2 * S, top - 12 * S, cx + 2 * S, top - 3 * S], 2 * S, fill=(232, 70, 80, 255))
            d.ellipse([cx - 2 * S, top - 1.5 * S, cx + 2 * S, top + 2.5 * S], fill=(232, 70, 80, 255))
        return canvas
