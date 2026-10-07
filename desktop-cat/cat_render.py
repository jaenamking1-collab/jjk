# 고양이 그림 엔진 — 관절(몸통·머리·다리 4개·꼬리 11마디)로 자세를 정의하고 PIL 로 그린다.
# 좌표 단위: 1 = 배율 1일 때 1px. 원점은 발바닥(땅에 닿는 점), y 는 아래가 +.
# 꼬리 각도는 1.5~5.0(아래→뒤→위) 안에서만 쓴다 — 자세끼리 보간할 때 꼬리가 몸을 가로지르지 않게.
import math
from PIL import Image, ImageDraw, ImageOps

CW, CH = 120, 100          # 캔버스 크기(단위)
OX, OY = 60, 88            # 캔버스 안에서 원점(발바닥) 위치
TAIL_N, TAIL_L = 11, 3.0   # 꼬리 마디 수·길이

PALETTES = {
    'cheese': dict(fur=(238, 152, 70), dark=(206, 116, 44), far=(212, 128, 56), belly=(253, 234, 206)),
    'gray':   dict(fur=(150, 156, 168), dark=(104, 110, 124), far=(128, 134, 146), belly=(232, 234, 238)),
    'black':  dict(fur=(52, 52, 60), dark=(30, 30, 36), far=(40, 40, 48), belly=(84, 84, 94)),
    'white':  dict(fur=(250, 248, 244), dark=(226, 218, 206), far=(228, 224, 218), belly=(255, 255, 255)),
}
# 그리는 비율 — hs 머리, es 눈, ey 눈 높이, mz 주둥이, bt 몸통 굵기, lw 다리 굵기, ow 외곽선, blush 볼터치
STYLES = {
    'basic': dict(hs=1.0, es=1.0, ey=-0.8, mz=1.0, bt=1.0, lw=1.0, ow=1.15, blush=0, stripe=1.0),
    'cute':  dict(hs=1.3, es=1.45, ey=-0.2, mz=0.8, bt=1.12, lw=1.18, ow=0.95, blush=1, stripe=0.0),
    'mochi': dict(hs=1.45, es=1.7, ey=0.2, mz=0.72, bt=1.22, lw=1.3, ow=0.85, blush=1, stripe=0.0),
}
COMMON = dict(line=(64, 42, 30), eye=(40, 32, 26), nose=(238, 132, 146), ear_in=(246, 172, 172),
              tongue=(238, 106, 128), mouth=(104, 44, 48), white=(255, 255, 255))


# ── 수학 도우미 ───────────────────────────────────────────────
def clamp(v, a=0.0, b=1.0):
    return a if v < a else b if v > b else v


def ease(t):
    t = clamp(t)
    return t * t * (3 - 2 * t)


def seg(t, a, b):
    """t 가 a→b 구간에서 0→1 로 (부드럽게)."""
    return ease((t - a) / (b - a)) if b > a else float(t >= b)


def lerp(a, b, t):
    if isinstance(a, dict):
        return {k: (lerp(a[k], b[k], t) if k in b else a[k]) for k in a}
    if isinstance(a, tuple):
        return tuple(lerp(x, y, t) for x, y in zip(a, b))
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        return a + (b - a) * t
    return b if t >= 0.5 else a


def add(p, q, k=1.0):
    return (p[0] + q[0] * k, p[1] + q[1] * k)


def rot(p, a):
    c, s = math.cos(a), math.sin(a)
    return (p[0] * c - p[1] * s, p[0] * s + p[1] * c)


def ik(hip, foot, la, lb, bend):
    """2마디 다리. bend=+1 이면 무릎이 뒤(-x)로, -1 이면 앞으로 꺾인다."""
    dx, dy = foot[0] - hip[0], foot[1] - hip[1]
    d = math.hypot(dx, dy) or 1e-6
    if d > la + lb - 0.01:
        f = (la + lb - 0.01) / d
        foot = (hip[0] + dx * f, hip[1] + dy * f)
        d = la + lb - 0.01
    cos_a = (la * la + d * d - lb * lb) / (2 * la * d)
    a = math.acos(clamp(cos_a, -1, 1))
    ang = math.atan2(dy, dx) + bend * a
    return (hip[0] + la * math.cos(ang), hip[1] + la * math.sin(ang)), foot


# ── 기본 자세 ─────────────────────────────────────────────────
def leg(hip, foot, bend, la, lb):
    return {'hip': hip, 'foot': foot, 'bend': bend, 'la': la, 'lb': lb}


def base(**kw):
    p = {
        'c1': (8, -17), 'r1': 8.0, 'c2': (-9, -17.5), 'r2': 8.5,
        'head': (16, -27), 'hrot': 0.0, 'face': 0.55, 'eyes': 'open', 'look': (0.0, 0.0),
        'mouth': 'w', 'mopen': 0.0, 'ears': 0.0, 'zzz': 0.0, 'mark': 0.0,
        'fn': leg((9, -13), (10, 0), 1, 7.5, 7.0), 'ff': leg((7, -13), (7.5, 0), 1, 7.5, 7.0),
        'bn': leg((-10, -14), (-11, 0), 1, 8.0, 8.0), 'bf': leg((-12, -14), (-13.5, 0), 1, 8.0, 8.0),
        'tail': {'base': (-16, -21), 'ang': 4.03, 'curve': 0.13, 'amp': 0.22, 'spd': 1.6,
                 'puff': 1.0, 'ground': 0.0, 'front': 0.0},
    }
    p.update(kw)
    return p


def P_stand():
    return base()


def P_sit():
    return base(c1=(4, -20), r1=7.5, c2=(-6, -10), r2=9.5, head=(7, -32), face=0.35,
                fn=leg((5, -15), (6.5, 0), 1, 7.5, 7.5), ff=leg((3, -15), (3.5, 0), 1, 7.5, 7.5),
                bn=leg((-6, -8), (2, -0.5), -1, 7.0, 7.0), bf=leg((-7, -8), (-1, -0.5), -1, 7.0, 7.0),
                tail={'base': (-13, -4), 'ang': 2.3, 'curve': -0.2, 'amp': 0.08, 'spd': 1.0,
                      'puff': 1.0, 'ground': 1.0, 'front': 1.0})


def P_loaf():
    return base(c1=(7, -8.5), r1=7.5, c2=(-8, -9), r2=8.5, head=(13, -17), face=0.4,
                fn=leg((8, -4), (12, -1), 1, 4.0, 3.0), ff=leg((6, -4), (10, -1), 1, 4.0, 3.0),
                bn=leg((-8, -4), (-3, -1), 1, 4.0, 3.0), bf=leg((-9, -4), (-4, -1), 1, 4.0, 3.0),
                tail={'base': (-15, -6), 'ang': 2.4, 'curve': -0.21, 'amp': 0.05, 'spd': 0.8,
                      'puff': 1.0, 'ground': 1.0, 'front': 1.0})


def P_crouch():
    return base(c1=(9, -11), r1=7.8, c2=(-9, -12.5), r2=8.6, head=(18, -19), face=0.75,
                look=(0.6, 0.0), ears=0.3,
                fn=leg((10, -7), (12, 0), 1, 7.5, 7.0), ff=leg((8, -7), (10, 0), 1, 7.5, 7.0),
                bn=leg((-10, -8), (-9, 0), 1, 8.0, 8.0), bf=leg((-12, -8), (-11, 0), 1, 8.0, 8.0),
                tail={'base': (-16, -15), 'ang': 3.05, 'curve': 0.05, 'amp': 0.12, 'spd': 5.0,
                      'puff': 1.0, 'ground': 0.0, 'front': 0.0})


def P_air_up():
    return base(c1=(10, -18), r1=7.6, c2=(-10, -16), r2=8.0, head=(19, -25), face=0.8, ears=0.4,
                fn=leg((11, -14), (21, -11), 1, 7.5, 7.0), ff=leg((9, -14), (19, -9), 1, 7.5, 7.0),
                bn=leg((-11, -13), (-24, -8), 1, 8.0, 8.0), bf=leg((-12, -13), (-22, -6), 1, 8.0, 8.0),
                tail={'base': (-17, -17), 'ang': 3.0, 'curve': 0.02, 'amp': 0.1, 'spd': 2.0,
                      'puff': 1.0, 'ground': 0.0, 'front': 0.0})


def P_air_down():
    return base(c1=(9, -20), r1=7.8, c2=(-9, -22), r2=8.4, head=(17, -29), face=0.7,
                look=(0.3, 0.8), ears=0.2,
                fn=leg((10, -16), (12, 0), 1, 7.5, 8.0), ff=leg((8, -16), (9, 1), 1, 7.5, 8.0),
                bn=leg((-10, -18), (-7, -5), 1, 8.0, 8.0), bf=leg((-12, -18), (-9, -4), 1, 8.0, 8.0),
                tail={'base': (-16, -24), 'ang': 3.68, 'curve': 0.1, 'amp': 0.25, 'spd': 3.0,
                      'puff': 1.0, 'ground': 0.0, 'front': 0.0})


def P_stretch():
    return base(c1=(13, -8), r1=7.0, c2=(-8, -19), r2=8.5, head=(21, -12), face=0.65, hrot=0.15,
                fn=leg((13, -4), (26, 0), 1, 7.5, 7.5), ff=leg((11, -4), (24, 0), 1, 7.5, 7.5),
                bn=leg((-9, -15), (-10, 0), 1, 8.0, 8.0), bf=leg((-11, -15), (-12.5, 0), 1, 8.0, 8.0),
                tail={'base': (-15, -24), 'ang': 4.53, 'curve': 0.1, 'amp': 0.15, 'spd': 1.2,
                      'puff': 1.0, 'ground': 0.0, 'front': 0.0})


def P_lookdown():
    return base(c1=(9, -15), r1=7.8, head=(21, -17), hrot=0.5, face=0.85, look=(0.5, 1.0),
                fn=leg((10, -11), (13, 0), 1, 7.5, 7.0), ff=leg((8, -11), (11, 0), 1, 7.5, 7.0))


# ── 동작별 자세 (t = 그 동작을 시작한 뒤 흐른 초) ─────────────────
def gait(phase, kind):
    p = P_stand()
    if kind == 'walk':
        a, h, offs = 4.5, 3.0, {'bn': 0.0, 'fn': 0.25, 'bf': 0.5, 'ff': 0.75}
    else:
        a, h, offs = 8.5, 5.0, {'bn': 0.0, 'bf': 0.12, 'fn': 0.5, 'ff': 0.62}
    for k, o in offs.items():
        phi = 2 * math.pi * (phase + o)
        lg = dict(p[k])
        lg['foot'] = (lg['foot'][0] + a * math.sin(phi), -max(0.0, math.cos(phi)) * h)
        p[k] = lg
    bob = math.sin(4 * math.pi * phase)
    if kind == 'walk':
        p['c1'] = add(p['c1'], (0, 0.5 * bob))
        p['c2'] = add(p['c2'], (0, -0.4 * bob))
        p['head'] = add(p['head'], (0, 0.4 * bob))
    else:
        s = math.sin(2 * math.pi * phase)
        lift = -1.8 * abs(math.cos(2 * math.pi * phase))
        p['c1'] = add(p['c1'], (2.5 * s, lift))
        p['c2'] = add(p['c2'], (-2.5 * s, lift))
        p['head'] = (19 + 1.5 * s, -24 + lift)
        p['face'], p['ears'], p['look'] = 0.85, 0.9, (0.8, 0.0)
        for k in ('fn', 'ff'):
            p[k] = dict(p[k], hip=add(p[k]['hip'], (2.5 * s, lift)))
        for k in ('bn', 'bf'):
            p[k] = dict(p[k], hip=add(p[k]['hip'], (-2.5 * s, lift)))
        p['tail'] = dict(p['tail'], base=(-18, -18 + lift), ang=3.0, curve=0.02, amp=0.15, spd=6.0, puff=1.25)
    return p


def action_pose(name, t, ctx):
    """동작 이름 + 경과시간 → 자세. ctx: walk_phase, dur, time 등."""
    dur = ctx.get('dur', 3.0)
    breathe = math.sin(ctx.get('time', 0) * 2.2)

    if name == 'walk':
        return gait(ctx.get('walk_phase', 0), 'walk')
    if name == 'run':
        return gait(ctx.get('walk_phase', 0), 'run')
    if name == 'crouch':                                   # 점프 전 엉덩이 실룩
        p = P_crouch()
        w = math.sin(t * 22) * 1.4 * seg(t, 0.1, 0.25)
        p['c2'] = add(p['c2'], (w, 0))
        for k in ('bn', 'bf'):
            p[k] = dict(p[k], hip=add(p[k]['hip'], (w, 0)))
        p['tail'] = dict(p['tail'], ang=3.05 + 0.15 * math.sin(t * 9))
        return p
    if name == 'air':
        return lerp(P_air_up(), P_air_down(), clamp(ctx.get('air_k', 0.0)))
    if name == 'fall':
        p = P_air_down()
        p['ears'], p['mark'] = 0.6, 0.0
        p['tail'] = dict(p['tail'], ang=4.38, amp=0.5, spd=8.0)
        return p
    if name == 'land':                                     # 착지 충격 흡수
        k = math.sin(clamp(t / 0.22) * math.pi)
        p = lerp(P_stand(), P_crouch(), 0.7 * k)
        return p
    if name == 'stand':
        p = P_stand()
        p['c1'] = add(p['c1'], (0, 0.3 * breathe))
        return p
    if name == 'stretch':                                  # 기지개 + 하품
        p = lerp(P_stand(), P_stretch(), seg(t, 0.0, 0.7) * (1 - seg(t, dur - 0.6, dur)))
        k = seg(t, 0.5, 0.9) * (1 - seg(t, dur - 1.0, dur - 0.5))
        p['mouth'], p['mopen'], p['eyes'] = ('open', k, 'closed') if k > 0.05 else ('w', 0.0, 'open')
        return p
    if name == 'lookdown':                                 # 가장자리에서 아래 내려다보기
        p = lerp(P_stand(), P_lookdown(), seg(t, 0.0, 0.6))
        p['hrot'] += 0.12 * math.sin(t * 1.3)
        p['look'] = (0.5 + 0.4 * math.sin(t * 0.9), 1.0)
        p['ears'] = 0.15 + 0.15 * math.sin(t * 3.1)
        return p
    if name == 'knead':                                    # 꾹꾹이
        p = P_stand()
        p['c1'] = add(p['c1'], (0, 2))
        p['head'] = add(p['head'], (0, 1.5))
        p['eyes'] = 'closed'
        for k, o in (('fn', 0.0), ('ff', math.pi)):
            lift = max(0.0, math.sin(t * 6 + o)) * 3.2
            p[k] = dict(p[k], foot=(p[k]['foot'][0] + 1.0, -lift))
        p['tail'] = dict(p['tail'], amp=0.35, spd=2.4)
        return p

    # 이하 앉은 자세 계열
    p = P_sit()
    p['c1'] = add(p['c1'], (0, 0.35 * breathe))
    p['head'] = add(p['head'], (0, 0.25 * breathe))

    if name == 'sit' or name == 'watch':
        return p
    if name == 'groom':                                    # 옆구리 그루밍
        k = seg(t, 0.0, 0.5) * (1 - seg(t, dur - 0.4, dur))
        bob = math.sin(t * 11) * 1.1 * k
        p['head'] = lerp(p['head'], (-1.0, -24.0 + bob), k)
        p['face'] = lerp(p['face'], -0.75, k)
        p['hrot'] = 0.55 * k
        if k > 0.5:
            p['eyes'], p['mouth'] = 'closed', 'tongue'
        return p
    if name == 'scratch':                                  # 뒷발로 귀 뒤 긁기
        k = seg(t, 0.0, 0.4) * (1 - seg(t, dur - 0.35, dur))
        osc = math.sin(t * 30) * 2.0 * k
        bn = leg((-5, -12), (1 + osc * 0.4, -25 + osc), 1, 8.0, 9.0)
        p['bn'] = lerp(p['bn'], bn, k)
        p['head'] = lerp(p['head'], (5.0, -30.0), k)
        p['hrot'] = -0.4 * k
        p['c1'] = add(p['c1'], (-1.5 * k, 1.0 * k))
        p['ears'] = 0.6 * k
        if k > 0.5:
            p['eyes'] = 'closed'
        return p
    if name in ('lickpaw', 'wash'):                        # 앞발 핥기 / 세수
        k = seg(t, 0.0, 0.45) * (1 - seg(t, dur - 0.4, dur))
        lick_end = 1.8 if name == 'wash' else dur
        bob = math.sin(t * 10) * 1.0
        if t < lick_end:
            foot = (9.0, -24.0 + bob * 0.4)
            head = (7.0, -29.0 + bob)
            p['mouth'] = 'tongue' if k > 0.5 else 'w'
        else:                                              # 앞발로 얼굴 문지르기
            a = (t - lick_end) * 7.0
            foot = (9.0 + 2.5 * math.cos(a), -29.0 + 3.0 * math.sin(a))
            head = (7.0 + 0.8 * math.cos(a), -30.0 + 0.8 * math.sin(a))
        p['fn'] = lerp(p['fn'], leg((5, -15), foot, 1, 7.5, 7.5), k)
        p['head'] = lerp(p['head'], head, k)
        p['hrot'] = 0.35 * k
        p['face'] = lerp(p['face'], 0.65, k)
        if k > 0.5:
            p['eyes'] = 'closed'
        return p
    if name == 'yawn':                                     # 하품
        k = seg(t, 0.2, 0.8) * (1 - seg(t, dur - 0.7, dur - 0.1))
        p['hrot'] = -0.35 * k
        p['head'] = add(p['head'], (-0.5 * k, -1.0 * k))
        p['mouth'], p['mopen'] = ('open', k) if k > 0.05 else ('w', 0.0)
        p['ears'] = 0.5 * k
        if k > 0.3:
            p['eyes'] = 'closed'
        return p
    if name == 'lookaround':                               # 두리번
        p['face'] = 0.35 + 0.75 * math.sin(t * 1.4)
        p['hrot'] = 0.12 * math.sin(t * 2.1)
        p['look'] = (math.sin(t * 1.4), -0.2)
        p['ears'] = max(0.0, math.sin(t * 5.3)) * 0.3
        p['tail'] = dict(p['tail'], amp=0.22, spd=2.0)
        return p
    if name == 'tailflick':                                # 꼬리 탁탁
        p['tail'] = dict(p['tail'], front=0.0, ang=3.3 + 0.35 * math.sin(t * 7),
                         curve=-0.03, amp=0.4, spd=7.0)
        p['eyes'] = 'half'
        return p
    if name == 'loaf':                                     # 엎드리기(식빵)
        p = lerp(P_sit(), P_loaf(), seg(t, 0.0, 0.8))
        p['c1'] = add(p['c1'], (0, 0.35 * breathe))
        p['eyes'] = 'half' if t > 2.0 else 'open'
        tf = max(0.0, math.sin(t * 0.7)) ** 6
        p['tail'] = dict(p['tail'], amp=0.05 + 0.4 * tf, spd=6.0)
        return p
    if name == 'sleep':                                    # 웅크려 자기
        p = lerp(P_sit(), P_loaf(), seg(t, 0.0, 0.8))
        k = seg(t, 0.8, 1.8)
        p['head'] = lerp(p['head'], (12.0, -11.0), k)
        p['hrot'] = 0.35 * k
        p['eyes'] = 'sleep' if k > 0.3 else 'half'
        p['zzz'] = k
        p['c1'] = add(p['c1'], (0, 0.6 * breathe))
        p['c2'] = add(p['c2'], (0, 0.4 * breathe))
        p['r2'] += 0.3 * breathe
        p['tail'] = dict(p['tail'], amp=0.02)
        return p
    return p


# ── 그리기 ────────────────────────────────────────────────────
class CatRenderer:
    def __init__(self, scale=1.3, palette='cheese', ss=3, style='mochi'):
        self.st = STYLES.get(style, STYLES['cute'])
        self.s, self.ss = scale, ss
        self.k = scale * ss
        self.W, self.H = int(CW * scale), int(CH * scale)
        self.pal = dict(PALETTES.get(palette, PALETTES['cheese']), **COMMON)
        self.ow = self.st['ow']  # 외곽선 두께(단위)

    def P(self, p):
        return ((OX + p[0]) * self.k, (OY + p[1]) * self.k)

    def _ell(self, d, c, rx, ry, col):
        x, y = self.P(c)
        k = self.k
        d.ellipse([x - rx * k, y - ry * k, x + rx * k, y + ry * k], fill=col)

    def _cap(self, d, p, q, r, col):
        d.line([self.P(p), self.P(q)], fill=col, width=max(1, int(2 * r * self.k)))
        self._ell(d, p, r, r, col)
        self._ell(d, q, r, r, col)

    def _group(self, d, shapes):
        """도형 묶음을 외곽선 → 채우기 순으로. 묶음 안끼리는 선이 안 생긴다."""
        for g in (self.ow, 0.0):
            for sh in shapes:
                col = self.pal['line'] if g else sh[-1]
                if sh[0] == 'e':
                    self._ell(d, sh[1], sh[2] + g, sh[3] + g, col)
                elif sh[0] == 'cap':
                    self._cap(d, sh[1], sh[2], sh[3] / 2 + g, col)
                elif sh[0] == 'poly':
                    pts = [self.P(p) for p in sh[1]]
                    d.polygon(pts, fill=col)
                    if g:
                        d.line(pts + [pts[0]], fill=col, width=int(2 * g * self.k), joint='curve')
                        for p in sh[1]:
                            self._ell(d, p, g, g, col)

    # ── 부위 ──
    def _leg_shapes(self, lg, col, back):
        knee, foot = ik(lg['hip'], lg['foot'], lg['la'], lg['lb'], 1 if lg['bend'] >= 0 else -1)
        lw = self.st['lw']
        return [('cap', lg['hip'], knee, (6.0 if back else 5.0) * lw, col),
                ('cap', knee, foot, 3.8 * lw, col),
                ('e', add(foot, (1.2, -0.7)), 2.6 * lw, 1.9 * lw, col)]

    def _tail_pts(self, tl, time):
        pts = [tl['base']]
        a = tl['ang']
        for i in range(1, TAIL_N + 1):
            w = tl['amp'] * math.sin(time * tl['spd'] - i * 0.55) * (i / TAIL_N) ** 0.8
            ang = a + tl['curve'] * i + w
            x, y = pts[-1]
            nx, ny = x + TAIL_L * math.cos(ang), y + TAIL_L * math.sin(ang)
            if tl['ground'] > 0.5:
                ny = min(ny, -1.4)
            pts.append((nx, ny))
        return pts

    def _tail(self, d, tl, time):
        pts = self._tail_pts(tl, time)
        shapes = []
        for i in range(TAIL_N):
            w = (3.7 - 1.4 * i / TAIL_N) * tl['puff'] * self.st['lw']
            col = self.pal['dark'] if (i % 3 == 2 or i >= TAIL_N - 2) else self.pal['fur']
            shapes.append(('cap', pts[i], pts[i + 1], w, col))
        self._group(d, shapes)

    def _body(self, d, p):
        c1, c2 = p['c1'], p['c2']
        r1, r2 = p['r1'] * self.st['bt'], p['r2'] * self.st['bt']
        vx, vy = c1[0] - c2[0], c1[1] - c2[1]
        n = math.hypot(vx, vy) or 1
        nx, ny = -vy / n, vx / n                  # 아래쪽 법선
        fur = self.pal['fur']
        self._group(d, [('e', c1, r1 * 1.05, r1, fur), ('e', c2, r2, r2, fur),
                        ('poly', [add(c1, (nx, ny), r1), add(c2, (nx, ny), r2),
                                  add(c2, (nx, ny), -r2), add(c1, (nx, ny), -r1)], fur)])
        self._ell(d, add(c1, (0.8, r1 * 0.45)), r1 * 0.6, r1 * 0.42, self.pal['belly'])
        for t in (0.22, 0.42, 0.62):              # 등 줄무늬
            q = lerp(c2, c1, t)
            r = lerp(r2, r1, t)
            top = add(q, (nx, ny), -r + 0.4)
            self._cap(d, top, add(top, (nx, ny), 3.6), 0.75, self.pal['dark'])

    def _head(self, d, p):
        h, a, f, fl = p['head'], p['hrot'], p['face'], p['ears']
        st = self.st
        hs = st['hs']
        H = lambda q: add(h, rot((q[0] * hs, q[1] * hs), a))
        fur = self.pal['fur']
        ears = []
        for sgn, b1, b2, tip in ((-1, (-8.0, -3.5), (-2.5, -7.8), (-7.6, -12.5)),
                                 (1, (2.0, -8.0), (7.6, -3.8), (7.2, -12.6))):
            sh = 1.5 * f
            tip = (tip[0] + sh + sgn * 3.2 * fl, tip[1] + 4.0 * fl)
            ears.append([(b1[0] + sh, b1[1]), (b2[0] + sh, b2[1]), tip])
        self._group(d, [('poly', [H(q) for q in ears[0]], fur), ('poly', [H(q) for q in ears[1]], fur),
                        ('e', h, 9.4 * hs, 8.3 * hs, fur), ('e', H((0.5 * f, 2.6)), 9.6 * hs, 6.2 * hs, fur)])
        for e in ears:                            # 귀 안쪽
            cx = sum(q[0] for q in e) / 3
            cy = sum(q[1] for q in e) / 3
            inner = [H((cx + (q[0] - cx) * 0.55, cy + (q[1] - cy) * 0.55 + 0.4)) for q in e]
            d.polygon([self.P(q) for q in inner], fill=self.pal['ear_in'])
        for dx in ((-2.2, 0.0, 2.2) if st['stripe'] else ()):   # 이마 줄무늬
            q = H((dx + 1.2 * f, -7.2))
            self._cap(d, q, H((dx * 0.8 + 1.2 * f, -5.4 + 0.8 * st['stripe'])), 0.6, self.pal['dark'])
        mz = H((2.2 * f, 3.5))
        self._ell(d, mz, 4.3 * hs * st['mz'], 3.0 * hs * st['mz'], self.pal['belly'])
        lw = max(1, int(0.45 * self.k))
        for sgn in (-1, 1):                       # 수염
            for dy in (-0.6, 0.6):
                s0 = H((2.2 * f + sgn * 3.4, 3.3 + dy))
                s1 = H((2.2 * f + sgn * 9.5, 2.6 + dy * 2.4))
                d.line([self.P(s0), self.P(s1)], fill=self.pal['line'], width=lw)
        # 눈
        lk = p['look']
        for ex, er in ((-3.6 + 3.0 * f, 1.85), (3.6 + 2.2 * f, 2.0)):
            c = H((ex + lk[0] * 0.5, st['ey'] + lk[1] * 0.45))
            self._eye(d, c, er * st['es'], p['eyes'], a)
            if st['blush']:                       # 볼터치 (털색과 분홍을 섞은 불투명 색)
                bc = tuple(int(u + (v - u) * 0.55) for u, v in zip(fur, (255, 120, 140)))
                self._ell(d, H((ex + (-0.6 if ex < 0 else 0.6), 3.0)), 1.7 * hs, 0.95 * hs, bc)
        # 코·입
        nz = H((2.2 * f, 1.7))
        x, y = self.P(nz)
        k = self.k
        d.polygon([(x - 1.3 * k, y - 0.8 * k), (x + 1.3 * k, y - 0.8 * k), (x, y + 0.7 * k)],
                  fill=self.pal['nose'])
        m = p['mouth']
        if m == 'open' and p['mopen'] > 0.05:
            mo = p['mopen']
            c = H((2.2 * f, 4.4 + 1.4 * mo))
            self._ell(d, c, 2.4 * mo + 0.6, 3.0 * mo + 0.4, self.pal['mouth'])
            self._ell(d, add(c, (0, 1.4 * mo)), 1.6 * mo, 1.3 * mo, self.pal['tongue'])
        else:
            for sgn in (-1, 1):
                c = H((2.2 * f + sgn * 0.95, 2.6))
                x, y = self.P(c)
                r = 1.0 * k
                d.arc([x - r, y - r, x + r, y + r], 10, 170, fill=self.pal['line'], width=lw)
            if m == 'tongue':
                self._ell(d, H((2.2 * f + 0.3, 4.6)), 1.15, 1.5, self.pal['tongue'])

    def _eye(self, d, c, r, state, a):
        x, y = self.P(c)
        k = self.k
        col = self.pal['eye']
        lw = max(1, int(0.65 * k))
        if state == 'open':
            d.ellipse([x - r * k, y - r * 1.2 * k, x + r * k, y + r * 1.2 * k], fill=col)
            hr = 0.35 * r                         # 눈 반짝이 (큰 것 + 작은 것)
            hx, hy = x + 0.3 * r * k, y - 0.45 * r * k
            d.ellipse([hx - hr * k, hy - hr * k, hx + hr * k, hy + hr * k], fill=self.pal['white'])
            if r > 2.4:
                sx, sy, sr = x - 0.35 * r * k, y + 0.45 * r * k, 0.16 * r
                d.ellipse([sx - sr * k, sy - sr * k, sx + sr * k, sy + sr * k], fill=self.pal['white'])
        elif state == 'half' and self.st['blush']:  # 귀여운 그림체: 반쯤 감은 눈은 화나 보여서 졸린 ‿ 로
            d.arc([x - r * k, y - r * k - 0.6 * k, x + r * k, y + r * k - 0.6 * k], 20, 160, fill=col, width=lw)
        elif state == 'half':
            d.chord([x - r * k, y - r * 1.2 * k, x + r * k, y + r * 1.2 * k], 0, 180, fill=col)
        elif state == 'closed':                  # ^ 모양(기분 좋음)
            d.arc([x - r * k, y - r * k + 0.8 * k, x + r * k, y + r * k + 0.8 * k], 200, 340, fill=col, width=lw)
        else:                                    # sleep: ‿
            d.arc([x - r * k, y - r * k - 0.6 * k, x + r * k, y + r * k - 0.6 * k], 20, 160, fill=col, width=lw)

    def _zzz(self, d, p, time):
        if p['zzz'] < 0.3:
            return
        lw = max(1, int(0.7 * self.k))
        for i in range(3):
            ph = (time * 0.35 + i / 3) % 1.0
            sz = 2.0 + 2.5 * ph
            c = add(p['head'], (7 + 6 * ph + math.sin(ph * 6) * 1.5, -12 - 22 * ph))
            pts = [(c[0] - sz, c[1] - sz), (c[0] + sz, c[1] - sz), (c[0] - sz, c[1] + sz), (c[0] + sz, c[1] + sz)]
            alpha = int(255 * (1 - ph) * min(1.0, ph * 6))
            d.line([self.P(q) for q in pts], fill=self.pal['line'] + (alpha,), width=lw, joint='curve')

    def _mark(self, d, p):
        if p['mark'] < 0.5:
            return
        top = add(p['head'], (2, -24))
        self._cap(d, top, add(top, (0, 5.5)), 0.95, (230, 60, 60))
        self._ell(d, add(top, (0, 8.6)), 1.0, 1.0, (230, 60, 60))

    def render(self, p, facing=1, time=0.0):
        big = Image.new('RGBA', (self.W * self.ss, self.H * self.ss), (0, 0, 0, 0))
        d = ImageDraw.Draw(big)
        far = self.pal['far']
        for k in ('ff', 'bf'):
            self._group(d, self._leg_shapes(p[k], far, k == 'bf'))
        if p['tail']['front'] < 0.5:
            self._tail(d, p['tail'], time)
        self._body(d, p)
        if p['tail']['front'] >= 0.5:
            self._tail(d, p['tail'], time)
        for k in ('bn', 'fn'):
            self._group(d, self._leg_shapes(p[k], self.pal['fur'], k == 'bn'))
        self._head(d, p)
        img = big.resize((self.W, self.H), Image.LANCZOS)
        if facing < 0:
            img = ImageOps.mirror(img)
        # 글자(Zzz, !)는 뒤집히면 안 되므로 뒤집은 뒤에 따로 그린다
        if p['zzz'] >= 0.3 or p['mark'] >= 0.5:
            over = Image.new('RGBA', big.size, (0, 0, 0, 0))
            od = ImageDraw.Draw(over)
            q = dict(p, head=(p['head'][0] * facing, p['head'][1]))
            self._zzz(od, q, time)
            self._mark(od, q)
            img.alpha_composite(over.resize((self.W, self.H), Image.LANCZOS))
        return img
