# 사용자가 준 그림(sprites/*.png)으로 고양이를 그린다. 행동 두뇌(cat_brain)의 동작 이름을 받아
# 알맞은 그림을 고르고, 통통 튀기·기울이기·숨쉬기·좌우 뒤집기로 움직임을 만든다.
import os
import sys
import json
import math
from PIL import Image, ImageDraw
from cat_render import HANG, clamp, ease

HERE = getattr(sys, '_MEIPASS', os.path.dirname(os.path.abspath(__file__)))   # exe 로 묶였을 땐 풀린 임시 폴더
SPR = os.path.join(HERE, 'sprites')

# 동작 → (넘기는 방식, 그림들, 초당 장수)
#   step: 걸음에 맞춰 넘김(한 걸음에 그림 목록 한 바퀴)   cycle: 일정 간격으로 넘김
#   seq: 동작 시작부터 차례로 넘기고 마지막 그림에서 멈춤   pick: 할 때마다 하나를 골라 계속 씀
#   air: 점프 전반 첫 그림, 후반 둘째 그림
ART = {
    'walk': ('step', ['walk_a', 'walk_b']), 'trot': ('step', ['walk_happy', 'walk']),
    'hop': ('step', ['walk_c', 'walk_d']), 'back': ('step', ['walk_back', 'walk_back2']),
    'stand': ('pick', ['walk_front']), 'sneak': ('pick', ['hunt', 'hunt2']),
    'run': ('step', ['run_1', 'run_2', 'run_3', 'run_4']), 'tailchase': ('cycle', ['run_1', 'run_2', 'run_3', 'run_4'], 10),
    'crouch': ('seq', ['jprep_1', 'jprep_2'], 3), 'hunt': ('pick', ['hunt', 'hunt2', 'jprep_2']),
    'air': ('air', ['jump_1', 'jump_2']), 'fall': ('pick', ['jump_2']),
    'land': ('seq', ['land_1', 'land_2'], 8), 'land_hard': ('seq', ['landfail_1', 'landfail_2'], 2.5),
    'brace': ('pick', ['jprep_1']), 'confused': ('pick', ['curious2', 'landfail_2']),
    'sit': ('pick', ['headtilt']), 'watch': ('pick', ['headtilt', 'curious']), 'sitpretty': ('pick', ['headtilt']),
    'zoneout': ('pick', ['headtilt']), 'headtilt': ('pick', ['headtilt']), 'shake': ('pick', ['headtilt']),
    'curious': ('pick', ['curious', 'curious2']), 'lookaround': ('pick', ['curious']), 'lookdown': ('pick', ['curious']),
    'happy': ('pick', ['happy', 'leaf', 'wave']), 'meow': ('pick', ['happy']), 'chatter': ('pick', ['wave']),
    'love': ('pick', ['happy', 'stretch_finish']), 'knead': ('pick', ['happy']), 'sneeze': ('pick', ['happy']),
    'yawn': ('pick', ['stretch_finish']), 'stretch_finish': ('pick', ['stretch_finish']),
    'groom': ('cycle', ['groombody_a', 'groombody_b'], 3), 'lickpaw': ('cycle', ['lick_a', 'lick_b'], 3),
    'wash': ('cycle', ['wash_a', 'wash_b'], 3), 'groom_leg': ('cycle', ['paw_a', 'paw_b'], 3),
    'groom_tail': ('cycle', ['groomtail_a', 'groomtail_b'], 3), 'scratch': ('pick', ['groomback']),
    'groom_all': ('pick', ['groomall']), 'groom_back': ('pick', ['groomback']),
    'loaf': ('pick', ['loaf']), 'sploot': ('pick', ['loaf']), 'paw_dangle': ('pick', ['loaf']),
    'tailflick': ('pick', ['loaf']), 'sleep': ('pick', ['sleep', 'cushion']), 'sleep_back': ('pick', ['sleep']),
    'sleep_side': ('pick', ['sleep']), 'sulk': ('pick', ['sulk']), 'belly': ('pick', ['belly']),
    'roll': ('cycle', ['roll_1', 'roll_2', 'roll_3', 'roll_4'], 6),
    'stretch': ('pick', ['stretch', 'stretch2']), 'stretch_back': ('pick', ['stretch2']),
    'bugcatch': ('pick', ['bugcatch', 'butterfly2']), 'swat': ('pick', ['bugcatch']), 'claw': ('pick', ['glass']),
    'hang': ('pick', ['hang_1', 'hang_wink']), 'hang_kick': ('cycle', ['hang_3', 'hang_1'], 3),
    'hang_one': ('pick', ['hang_2']), 'climbup': ('pick', ['climb_3']),
    'climb': ('step', ['climb_1', 'climb_3']), 'cling': ('pick', ['climb_2']), 'slide': ('pick', ['climb_down']),
    'hugheart': ('pick', ['hugheart']), 'stargaze': ('pick', ['stargaze']), 'fishtoy': ('pick', ['fishtoy']),
    'hat': ('pick', ['hat', 'beanie']), 'box': ('pick', ['box', 'box2']), 'backview': ('pick', ['backview', 'backview2']),
    'peek': ('pick', ['peek']), 'wave': ('pick', ['wave']), 'headphones': ('pick', ['headphones']),
    'cushion': ('pick', ['cushion']), 'teacup': ('pick', ['teacup']), 'yarn': ('pick', ['yarn']),
    'starpillow': ('pick', ['starpillow']), 'crown': ('pick', ['crown']), 'fishhug': ('pick', ['fishhug']),
    'leaf': ('pick', ['leaf']), 'shark': ('pick', ['shark']),
}
CALM_RUN = ['dash', 'scarf', 'run']
# 시트마다 고양이를 그린 크기가 달라 머리 크기가 비슷해지게 맞춘 배율(이름 앞부분으로 찾는다)
SIZE = {'run_': 1.35, 'jump_': 1.3, 'jprep_': 1.15, 'land': 1.15, 'roll_': 1.1, 'hang_': 1.3, 'climb_': 1.35,
        'lick_': 1.3, 'wash_': 1.3, 'groom': 1.3, 'paw_': 1.3, 'stretch_finish': 1.3, 'walk_a': 0.95,
        'walk_b': 0.95, 'walk_c': 1.1, 'walk_d': 1.1, 'dash': 1.1, 'scarf': 1.0, 'hunt2': 1.05}


def size_of(name):
    for k, v in SIZE.items():
        if name.startswith(k):
            return v
    return 1.0                  # 도망이 아닌 우다다는 신나게 달리는 그림
WALKS = {'walk', 'trot', 'back', 'hop', 'sneak', 'run', 'tailchase'}


class SpriteRenderer:
    def __init__(self, scale=1.3, palette=None):
        self.s = scale
        meta = json.load(open(os.path.join(SPR, 'sprites.json'), encoding='utf-8'))
        f = 82 * scale / 220.0                    # 그림 속 고양이(약 220px) → 화면 약 82px × 배율
        self.img, self.face = {}, {}
        for name, m in meta.items():
            im = Image.open(os.path.join(SPR, name + '.png')).convert('RGBA')
            k = f * size_of(name)
            self.img[name] = im.resize((max(1, round(im.width * k)), max(1, round(im.height * k))), Image.LANCZOS)
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

    def _pick(self, action, at, time, brain):
        spec = ART.get(action, ('pick', ['headtilt']))
        mode, frames = spec[0], [f for f in spec[1] if f in self.img] or ['headtilt']
        ph = brain.walk_phase if brain else time
        if action == 'run' and brain is not None and not brain.flee:
            mode, frames = 'pick', CALM_RUN
        if mode == 'step':
            return frames[int((ph % 1.0) * len(frames)) % len(frames)]
        if mode == 'cycle':
            return frames[int(at * spec[2]) % len(frames)]
        if mode == 'seq':
            return frames[min(len(frames) - 1, int(at * spec[2]))]
        if mode == 'air':
            k = brain.jump['t'] / brain.jump['T'] if (brain and brain.jump) else 0.0
            return frames[0 if k < 0.5 else 1]
        seed = int(((time - at) * 7.31) * 1000) if brain else 0   # 이번 동작 동안은 같은 그림
        return frames[seed % len(frames)]

    def render_action(self, action, at, facing, time, brain=None):
        name = self._pick(action, at, time, brain)
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
            rot = 1.5 * math.sin(2 * math.pi * ph)   # 그림이 걸음마다 바뀌니 기울기는 살짝만
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
        elif action == 'belly':
            rot = 6 * math.sin(at * 3)
        elif action in ('sleep', 'sleep_back', 'sleep_side', 'loaf', 'sploot', 'sulk', 'box'):
            sx, sy = 1 + 0.015 * breathe, 1 - 0.015 * breathe
        elif action in ('hang', 'hang_kick', 'hang_one', 'climbup'):
            anchor = 'hang'
            sp = {'hang': (5, 2.2), 'hang_kick': (7, 9), 'hang_one': (12, 2.6)}.get(action, (2, 6))
            rot = sp[0] * math.sin(time * sp[1])
        elif action in ('climb', 'cling', 'slide'):
            anchor = 'wall'
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
