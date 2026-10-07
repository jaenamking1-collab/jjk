# 고양이 그림 엔진 — 관절(몸통·머리·다리 4개·꼬리 11마디)로 자세를 정의하고 PIL 로 그린다.
# 좌표 단위: 1 = 배율 1일 때 1px. 원점은 발바닥(땅에 닿는 점), y 는 아래가 +.
# 꼬리 각도는 1.5~5.0(아래→뒤→위) 안에서만 쓴다 — 자세끼리 보간할 때 꼬리가 몸을 가로지르지 않게.
import math
from PIL import Image, ImageDraw, ImageOps

CW, CH = 120, 100          # 캔버스 크기(단위)
OX, OY = 60, 88            # 캔버스 안에서 원점(발바닥) 위치
TAIL_N, TAIL_L = 11, 3.0   # 꼬리 마디 수·길이

PALETTES = {
    'cheese': dict(fur=(238, 152, 70), dark=(206, 116, 44), far=(192, 108, 42), belly=(253, 234, 206)),
    'gray':   dict(fur=(150, 156, 168), dark=(104, 110, 124), far=(112, 118, 132), belly=(232, 234, 238)),
    'black':  dict(fur=(52, 52, 60), dark=(30, 30, 36), far=(30, 30, 36), belly=(84, 84, 94)),
    'white':  dict(fur=(250, 248, 244), dark=(226, 218, 206), far=(208, 200, 190), belly=(255, 255, 255)),
    # 사용자가 준 참고 그림(2026-10-07): 하얀 털, 큰 파란 눈, 분홍 귀, 연한 선
    'snow':   dict(fur=(253, 250, 246), dark=(238, 230, 222), far=(238, 231, 224), belly=(255, 255, 255),
                   line=(222, 200, 190), ear_in=(248, 174, 180), nose=(242, 150, 162), eye=(30, 42, 78),
                   iris=(82, 136, 196), iris_hi=(150, 196, 236)),
}
# 그리는 비율 — hs 머리, es 눈, ey 눈 높이, mz 주둥이, bt 몸통 굵기, lw 다리 굵기, ow 외곽선, blush 볼터치
STYLES = {
    'basic': dict(hs=1.0, es=1.0, ey=-0.8, mz=1.0, bt=1.0, lw=1.0, ow=1.15, blush=0, stripe=1.0),
    'cute':  dict(hs=1.3, es=1.45, ey=-0.2, mz=0.8, bt=1.12, lw=1.18, ow=0.95, blush=1, stripe=0.0),
    'mochi': dict(hs=1.45, es=1.7, ey=0.2, mz=0.72, bt=1.22, lw=1.3, ow=0.85, blush=1, stripe=0.0),
    # 참고 그림체: 머리 더 크게, 눈 아주 크게, 줄무늬·수염·주둥이 무늬 없음
    'snow':  dict(hs=1.6, es=2.05, ey=0.7, mz=0.0, bt=1.2, lw=1.35, ow=0.75, blush=1, stripe=0.0,
                  whisker=0, ring=0, bodystripe=0),
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
        'mouth': 'w', 'mopen': 0.0, 'ears': 0.0, 'zzz': 0.0, 'mark': 0.0, 'emote': '', 'prop': '', 'back': 0.0, 'clip': -99.0, 'armsfront': 0.0,
        'fn': leg((10, -13), (11, 0), 1, 7.5, 7.0), 'ff': leg((8, -13), (9.5, 0), 1, 7.5, 7.0),
        'bn': leg((-11, -14), (-12, 0), 1, 8.0, 8.0), 'bf': leg((-13, -14), (-14.5, 0), 1, 8.0, 8.0),
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
HANG = 47.0   # 매달린 자세에서 앞발 끝 ~ 원점 거리(단위). 고양이 y = 발판 y + HANG*배율


def P_hang():
    """발판 끝에 앞발로 매달린 자세. 앞발 끝이 y=-HANG, 몸은 그 아래로 늘어진다."""
    return base(c1=(1, -20), r1=7.0, c2=(0, -10), r2=7.5, head=(3, -31), face=0.2, look=(0.0, -0.3),
                fn=leg((15, -22), (13, -45), 1, 12.0, 12.0), ff=leg((-9, -22), (-7, -45), 1, 12.0, 12.0),
                bn=leg((2, -6), (3, 0), 1, 4.0, 4.0), bf=leg((-2, -6), (-3, 0), 1, 4.0, 4.0),
                tail={'base': (-4, -6), 'ang': 2.3, 'curve': -0.2, 'amp': 0.35, 'spd': 2.5,
                      'puff': 1.0, 'ground': 0.0, 'front': 0.0}, armsfront=1.0)


def P_belly():
    """배 보이고 벌러덩."""
    return base(c1=(6, -7), r1=7.5, c2=(-8, -7.5), r2=8.5, head=(17, -9), hrot=1.9, face=0.0,
                fn=leg((6, -11), (9, -21), 1, 6.0, 6.0), ff=leg((4, -11), (1, -20), 1, 6.0, 6.0),
                bn=leg((-8, -11), (-5, -22), -1, 6.5, 6.5), bf=leg((-10, -11), (-14, -20), -1, 6.5, 6.5),
                tail={'base': (-15, -4), 'ang': 2.9, 'curve': -0.05, 'amp': 0.2, 'spd': 1.5,
                      'puff': 1.0, 'ground': 1.0, 'front': 0.0})


def P_side():
    """옆으로 누워 다리 쭉."""
    return base(c1=(7, -7), r1=7.5, c2=(-8, -7.5), r2=8.5, head=(17, -10), hrot=0.45, face=0.4,
                fn=leg((8, -6), (19, -3), 1, 6.5, 6.5), ff=leg((6, -6), (17, -2), 1, 6.5, 6.5),
                bn=leg((-8, -6), (-21, -3), 1, 7.0, 7.0), bf=leg((-10, -6), (-20, -2), 1, 7.0, 7.0),
                tail={'base': (-15, -6), 'ang': 3.1, 'curve': 0.02, 'amp': 0.08, 'spd': 1.0,
                      'puff': 1.0, 'ground': 1.0, 'front': 0.0})


def P_rearup():
    """뒷발로 일어선 자세(벽 긁기·벌레 잡기)."""
    return base(c1=(4, -28), r1=7.0, c2=(-2, -13), r2=8.5, head=(6, -42), face=0.7, look=(0.5, -0.6),
                fn=leg((7, -32), (15, -44), 1, 7.5, 7.0), ff=leg((5, -32), (13, -40), 1, 7.5, 7.0),
                bn=leg((-2, -9), (-1, 0), 1, 5.5, 5.5), bf=leg((-4, -9), (-4, 0), 1, 5.5, 5.5),
                tail={'base': (-8, -7), 'ang': 3.0, 'curve': -0.05, 'amp': 0.25, 'spd': 2.0,
                      'puff': 1.0, 'ground': 1.0, 'front': 0.0})


def P_sploot():
    """뒷다리를 뒤로 쭉 뻗고 엎드리기."""
    p = P_loaf()
    p['bn'] = leg((-8, -4), (-24, -1.5), 1, 8.0, 8.0)
    p['bf'] = leg((-9, -4), (-23, -1), 1, 8.0, 8.0)
    p['tail'] = dict(p['tail'], ground=1.0, front=0.0, ang=3.15, curve=0.0)
    return p


def _swing(p, th):
    """매달린 자세를 앞발 끝(0, -HANG)을 축으로 흔든다."""
    return translate_pose(rotate_pose(translate_pose(p, (0, HANG)), th), (0, -HANG))


def _lying(name, t, dur, breathe):
    if name == 'belly':                                    # 배 보이고 꼼지락
        p = lerp(P_loaf(), P_belly(), seg(t, 0, 0.6))
        for k, o in (('fn', 0.0), ('ff', 1.3), ('bn', 2.1), ('bf', 0.7)):
            p[k] = dict(p[k], foot=add(p[k]['foot'], (1.5 * math.sin(t * 3 + o), 1.2 * math.cos(t * 3 + o))))
        p['eyes'], p['look'] = 'open', (0.4, 0.6)
        p['tail'] = dict(p['tail'], amp=0.3, spd=3.0)
        return p
    if name == 'roll':                                     # 데굴데굴
        p = lerp(P_side(), P_belly(), 0.5 + 0.5 * math.sin(t * 3.2))
        p['eyes'] = 'open' if math.sin(t * 3.2) > 0 else 'closed'
        return p
    if name == 'sleep_back':                               # 벌러덩 자기
        p = lerp(P_loaf(), P_belly(), seg(t, 0, 0.8))
        p['eyes'], p['zzz'] = 'sleep', seg(t, 1.0, 2.0)
        p['c1'] = add(p['c1'], (0, -0.5 * breathe))
        for k in ('fn', 'ff'):
            p[k] = dict(p[k], foot=add(p[k]['foot'], (1.5, 4.0)))
        return p
    if name == 'sleep_side':                               # 옆으로 누워 자기
        p = lerp(P_loaf(), P_side(), seg(t, 0, 0.8))
        p['eyes'], p['zzz'] = 'sleep', seg(t, 1.0, 2.0)
        p['c1'] = add(p['c1'], (0, 0.5 * breathe))
        return p
    if name == 'sploot':                                   # 뒷다리 쭉 뻗고 엎드리기
        p = lerp(P_loaf(), P_sploot(), seg(t, 0, 0.7))
        p['eyes'] = 'half' if t > 1.5 else 'open'
        return p
    if name == 'paw_dangle':                               # 가장자리에 엎드려 앞발 대롱
        p = P_loaf()
        p['fn'] = leg((8, -4), (14 + 1.5 * math.sin(t * 1.8), 9), 1, 6.5, 6.5)
        p['head'], p['hrot'], p['look'] = (14, -14), 0.35, (0.4, 1.0)
        return lerp(P_loaf(), p, seg(t, 0, 0.6))
    if name == 'sulk':                                     # 삐지기: 납작 엎드려 째려보기
        p = P_loaf()
        p['head'], p['eyes'], p['ears'], p['look'] = (13, -14), 'half', 0.35, (1.0, 0.2)
        p['tail'] = dict(p['tail'], amp=0.35, spd=5.0)
        p['emote'] = '...' if t > 1.5 else ''
        return p
    # box: 박스 안에 쏙 — 얼굴만 빼꼼, 가끔 두리번
    p = P_sit()
    p['head'] = (2, -22 + 1.5 * max(0.0, math.sin(t * 0.9)))
    p['face'], p['look'], p['prop'] = 0.0, (math.sin(t * 0.6), 0.2), 'box'
    p['ears'] = 0.1 if math.sin(t * 2.3) > 0.9 else 0.0
    return p


def translate_pose(p, d):
    """자세 전체를 d 만큼 옮긴다(매달리기·벽타기처럼 원점이 발바닥이 아닐 때)."""
    q = dict(p, c1=add(p['c1'], d), c2=add(p['c2'], d), head=add(p['head'], d),
             tail=dict(p['tail'], base=add(p['tail']['base'], d)))
    for k in ('fn', 'ff', 'bn', 'bf'):
        q[k] = dict(p[k], hip=add(p[k]['hip'], d), foot=add(p[k]['foot'], d))
    return q


def rotate_pose(p, th):
    """원점(발바닥)을 중심으로 자세 전체를 돌린다. -pi/2 = 오른쪽 벽에 발을 붙이고 위를 보는 자세."""
    q = dict(p, c1=rot(p['c1'], th), c2=rot(p['c2'], th), head=rot(p['head'], th), hrot=p['hrot'] + th,
             tail=dict(p['tail'], base=rot(p['tail']['base'], th), ang=p['tail']['ang'] + th, ground=0.0))
    for k in ('fn', 'ff', 'bn', 'bf'):
        q[k] = dict(p[k], hip=rot(p[k]['hip'], th), foot=rot(p[k]['foot'], th))
    return q


# 걸음새: (반보폭 A, 발 드는 높이 h, 발이 땅에 닿아 있는 비율 D, 다리별 위상)
# 실제 고양이 걸음은 왼뒤→왼앞→오른뒤→오른앞 순서이고, 발은 주기의 60% 넘게 땅을 딛고 있다.
GAITS = {
    'walk':  (6.0, 3.2, 0.62, {'bn': 0.0, 'fn': 0.25, 'bf': 0.5, 'ff': 0.75}),
    'sneak': (4.0, 2.0, 0.72, {'bn': 0.0, 'fn': 0.25, 'bf': 0.5, 'ff': 0.75}),
    'trot':  (7.0, 4.2, 0.50, {'bn': 0.0, 'ff': 0.0, 'bf': 0.5, 'fn': 0.5}),
    'run':   (10.0, 5.5, 0.38, {'bn': 0.0, 'bf': 0.1, 'fn': 0.5, 'ff': 0.6}),
}
GAITS['back'] = GAITS['walk']


def stride(kind):
    """한 주기에 몸이 나아가는 거리(단위). 발이 미끄러지지 않게 걸음 속도를 여기에 맞춘다."""
    if kind == 'hop':
        return 26.0
    a, _, d, _ = GAITS.get(kind, GAITS['walk'])
    return 2 * a / d


def _foot_cycle(u, a, h, d):
    """딛는 동안은 일정한 속도로 뒤로(땅에 붙어 있으니), 드는 동안은 둥근 호로 빠르게 앞으로."""
    u %= 1.0
    if u < d:
        return a - 2 * a * (u / d), 0.0
    s = (u - d) / (1 - d)
    return -a + 2 * a * ease(s), -h * math.sin(math.pi * s)


def _hop(phase):
    u = phase % 1.0
    if u < 0.55:                                           # 공중
        s = math.sin(math.pi * u / 0.55)
        return translate_pose(lerp(P_stand(), P_air_up(), 0.6 * s), (0, -8 * s))
    s = math.sin(math.pi * (u - 0.55) / 0.45)             # 착지해서 움츠렸다 다시 튐
    return lerp(P_stand(), P_crouch(), 0.6 * s)


def gait(phase, kind):
    if kind == 'hop':
        return _hop(phase)
    a, h, d, offs = GAITS.get(kind, GAITS['walk'])
    p = P_stand()
    for k, o in offs.items():
        x, y = _foot_cycle(phase + o, a, h, d)
        lg = dict(p[k])
        lg['foot'] = (lg['foot'][0] + x, y)
        p[k] = lg
    if kind != 'run':
        # 딛는 발 쪽 어깨·엉덩이가 살짝 오르내린다. 머리는 거의 수평(고양이는 시선을 고정한다)
        bob = 0.9 if kind == 'trot' else 0.5
        sh = (0, -bob * math.sin(4 * math.pi * (phase + 0.125)))
        hp = (0, -bob * math.sin(4 * math.pi * phase))
        p['c1'], p['c2'] = add(p['c1'], sh), add(p['c2'], hp)
        p['head'] = add(p['head'], (0, sh[1] * 0.3))
        for k in ('fn', 'ff'):
            p[k] = dict(p[k], hip=add(p[k]['hip'], sh))
        for k in ('bn', 'bf'):
            p[k] = dict(p[k], hip=add(p[k]['hip'], hp))
        p['tail'] = dict(p['tail'], amp=0.16, spd=2.2)
        if kind == 'sneak':                               # 살금살금: 몸을 낮추고 꼬리는 뒤로
            low = (0, 5.0)
            p['c1'], p['c2'], p['head'] = add(p['c1'], low), add(p['c2'], (0, 4.5)), (18, -21)
            for k in ('fn', 'ff'):
                p[k] = dict(p[k], hip=add(p[k]['hip'], low))
            for k in ('bn', 'bf'):
                p[k] = dict(p[k], hip=add(p[k]['hip'], (0, 4.5)))
            p['face'], p['ears'], p['look'] = 0.8, 0.35, (1.0, 0.0)
            p['tail'] = dict(p['tail'], base=(-16, -15), ang=3.1, curve=0.02, amp=0.08, spd=3.0)
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

    if name in GAITS or name == 'hop':                    # walk / sneak / trot / run / back / hop
        return gait(ctx.get('walk_phase', 0), name)
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

    # ── 매달리기 (원점 = 앞발 끝에서 HANG 아래) ──
    if name == 'hang':                                     # 대롱대롱
        return _swing(P_hang(), 0.10 * math.sin(t * 2.2))
    if name == 'hang_kick':                                # 버둥버둥
        p = P_hang()
        for k, o in (('bn', 0.0), ('bf', math.pi)):
            p[k] = dict(p[k], foot=(p[k]['foot'][0] + 3 * math.sin(t * 14 + o), -3 - 2 * math.cos(t * 14 + o)))
        p['ears'], p['emote'] = 0.6, 'sweat'
        p['tail'] = dict(p['tail'], amp=0.6, spd=9.0)
        return _swing(p, 0.06 * math.sin(t * 7))
    if name == 'hang_one':                                 # 한 발로 매달려 흔들
        p = P_hang()
        p['ff'] = leg((-9, -22), (-14 + 3 * math.sin(t * 6), -30 + 2 * math.cos(t * 6)), 1, 12.0, 12.0)
        p['emote'] = '!' if t < 0.8 else ''
        return _swing(p, 0.22 * math.sin(t * 2.6))
    if name == 'climbup':                                  # 매달렸다가 낑낑 기어오르기 (ctx cprog 0→1)
        e = ease(ctx.get('cprog', 0.0))
        p = lerp(P_hang(), P_crouch(), e)
        for k, o in (('bn', 0.0), ('bf', math.pi)):
            p[k] = dict(p[k], foot=add(p[k]['foot'], (2.5 * math.sin(t * 16 + o) * (1 - e), 0)))
        return p
    # ── 벽 (오른쪽 벽에 발을 붙인 자세. 왼쪽 벽은 좌우반전) ──
    if name == 'climb':
        p = translate_pose(rotate_pose(gait(ctx.get('walk_phase', 0), 'walk'), -math.pi / 2), (0, -10))
        p['hrot'], p['face'], p['look'] = -0.3, 0.3, (0.2, -0.8)   # 머리는 세우고 위를 본다
        return p
    if name in ('slide', 'cling'):
        p = translate_pose(rotate_pose(P_crouch(), -math.pi / 2), (0, -10))
        p['ears'] = 0.7 if name == 'slide' else 0.2
        p['hrot'], p['face'] = -0.2, 0.25
        p['look'] = (0.2, 0.9) if name == 'slide' else (0.6, 0.0)
        p['emote'] = 'sweat' if name == 'slide' else ''
        return p
    if name == 'claw':                                     # 벽 박박 긁기
        p = P_rearup()
        for k, o, x in (('fn', 0.0, 15), ('ff', 0.5, 13)):
            u = (t * 2.5 + o) % 1.0
            p[k] = dict(p[k], foot=(x, -46 + 10 * u))
        p['eyes'] = 'closed'
        return lerp(P_stand(), p, seg(t, 0, 0.4) * (1 - seg(t, dur - 0.4, dur)))
    # ── 서서 ──
    if name == 'shake':                                    # 몸 털기
        p = P_stand()
        w = math.sin(t * 28) * (1 - seg(t, dur - 0.3, dur))
        p['c1'], p['c2'] = add(p['c1'], (0, 1.2 * w)), add(p['c2'], (0, -1.2 * w))
        p['head'] = add(p['head'], (1.0 * w, 0))
        p['hrot'], p['ears'], p['eyes'] = 0.3 * w, 0.5 + 0.4 * w, 'closed'
        p['tail'] = dict(p['tail'], amp=0.5, spd=20.0)
        return p
    if name == 'sniff':                                    # 킁킁 냄새 맡기
        p = P_stand()
        p['head'] = (21 + 2 * math.sin(t * 1.5), -11 + 0.6 * math.sin(t * 9))
        p['hrot'], p['face'], p['look'] = 0.55, 0.85, (0.3, 1.0)
        p['c1'] = add(p['c1'], (0, 2))
        for k in ('fn', 'ff'):
            p[k] = dict(p[k], hip=add(p[k]['hip'], (0, 2)))
        return p
    if name == 'stretch_back':                             # 뒷다리 쭉
        p = P_stand()
        k = seg(t, 0, 0.5) * (1 - seg(t, dur - 0.5, dur))
        p['bn'] = lerp(p['bn'], leg((-11, -14), (-26, -5), 1, 8.0, 8.0), k)
        p['c2'] = add(p['c2'], (0, 1.5 * k))
        if k > 0.5:
            p['eyes'], p['mouth'], p['mopen'] = 'closed', 'open', 0.4
        return p
    if name == 'hunt':                                     # 사냥 모드: 납작 엎드려 엉덩이 실룩
        p = P_crouch()
        p['c1'], p['head'] = add(p['c1'], (0, 2.5)), (19, -15)
        p['look'], p['ears'], p['face'] = (1.0, 0.1), 0.15, 0.8
        w = math.sin(t * 9) * 1.2 * (0.5 + 0.5 * math.sin(t * 0.8))
        p['c2'] = add(p['c2'], (w, -1.5))
        p['tail'] = dict(p['tail'], ang=3.25 + 0.2 * math.sin(t * 6), amp=0.2, spd=8.0)
        return p
    if name == 'tailchase':                                # 꼬리 쫓기 (방향은 두뇌가 뒤집는다)
        p = gait(t * 3.0, 'trot')
        p['face'], p['look'] = -0.8, (-1.0, 0.3)
        p['tail'] = dict(p['tail'], ang=2.2, curve=-0.25, amp=0.5, spd=8.0)
        return p
    if name == 'land_hard':                                # 높은 데서 쿵
        p = P_crouch()
        p['c1'], p['eyes'] = add(p['c1'], (0, 2)), 'closed'
        p['emote'] = '!' if t < 0.35 else ''
        return p
    if name == 'brace':                                    # 발판이 움직여 납작 버팀
        p = P_crouch()
        p['ears'], p['emote'] = 1.0, '!'
        return p
    if name == 'bugcatch':                                 # 나비 잡기: 뒷발로 서서 앞발 휘적
        p = P_rearup()
        c = abs(math.sin(t * 5))
        p['fn'] = dict(p['fn'], foot=(13 + 4 * c, -46 + 3 * c))
        p['ff'] = dict(p['ff'], foot=(13 - 3 * c, -44 + 2 * c))
        p['look'], p['prop'] = (0.7, -1.0), 'butterfly'
        return p
    # ── 누워서 ──
    if name in ('belly', 'roll', 'sleep_back', 'sleep_side', 'sploot', 'paw_dangle', 'sulk', 'box'):
        return _lying(name, t, dur, breathe)

    # 이하 앉은 자세 계열
    p = P_sit()
    p['c1'] = add(p['c1'], (0, 0.35 * breathe))
    p['head'] = add(p['head'], (0, 0.25 * breathe))

    if name == 'sit' or name == 'watch':
        return p
    if name == 'headtilt':                                 # 고개 갸웃
        side = 1 if int(t / 1.6) % 2 == 0 else -1
        p['hrot'] = 0.45 * side * math.sin(min(1.0, (t % 1.6) / 0.4) * math.pi / 2)
        p['emote'] = '?' if t < 2.0 else ''
        return p
    if name == 'curious':                                  # 궁금해하기: 몸을 앞으로 내밀고 ?
        p['c1'], p['head'] = add(p['c1'], (2, 1)), add(p['head'], (4, 2))
        p['hrot'], p['look'], p['emote'] = 0.2, (0.6, 0.3), '?'
        return p
    if name == 'happy':                                    # 기분 좋기: 눈웃음 + 입 벌려 웃기 + 반짝
        p['eyes'], p['mouth'], p['prop'] = 'closed', 'smile', 'sparkle'
        p['head'] = add(p['head'], (0, -1.0 * abs(math.sin(t * 3))))
        p['tail'] = dict(p['tail'], ground=0.0, front=0.0, ang=4.2, curve=0.15, amp=0.3, spd=3.0)
        return p
    if name == 'hugheart':                                 # 하트 안기
        p['eyes'], p['prop'], p['emote'] = 'closed', 'heart', 'heart'
        for k in ('fn', 'ff'):
            p[k] = dict(p[k], foot=(9.0, -15.0))
        p['hrot'] = 0.12 * math.sin(t * 1.5)
        return p
    if name == 'stargaze':                                 # 별 보기
        p['hrot'], p['look'], p['prop'] = -0.35, (0.5, -1.0), 'star'
        p['head'] = add(p['head'], (-1, -1))
        return p
    if name == 'fishtoy':                                  # 생선 장난감 놀이
        p['prop'], p['look'] = 'fish', (0.6, 0.9)
        p['fn'] = leg((5, -15), (13 + 2 * math.sin(t * 5), -6 + 2 * abs(math.sin(t * 5))), 1, 7.5, 7.5)
        p['head'], p['hrot'] = add(p['head'], (3, 3)), 0.3
        p['emote'] = 'heart' if (t % 3.0) > 2.0 else ''
        return p
    if name == 'hat':                                      # 모자 쓰기
        p['prop'], p['face'] = 'hat', 0.15
        p['eyes'] = 'closed' if (t % 3.2) > 2.8 else 'open'
        return p
    if name == 'backview':                                 # 뒷모습 (먼 곳 바라보기)
        p['back'], p['face'], p['head'] = 1.0, 0.0, (0, -32)
        p['c1'], p['c2'] = (1, -19), (-1, -10)
        p['ears'] = 0.15 * max(0.0, math.sin(t * 2.0))
        p['tail'] = dict(p['tail'], base=(-4, -4), ang=3.0, curve=-0.25, amp=0.25, spd=1.5, front=1.0)
        return p
    if name == 'peek':                                     # 몰래 보기 (창 모서리 뒤 — clip 은 두뇌가 정한다)
        p['head'], p['face'] = add(p['head'], (3, 0)), 0.5
        p['hrot'] = 0.25 + 0.1 * math.sin(t * 1.2)
        p['clip'] = ctx.get('clip', -99.0)
        p['eyes'] = 'closed' if (t % 2.6) > 2.45 else 'open'
        return p
    if name == 'zoneout':                                  # 멍때리기
        p['look'], p['ears'] = (0.0, 0.0), 0.15
        p['emote'] = '...' if t > 1.0 else ''
        p['tail'] = dict(p['tail'], amp=0.02)
        return p
    if name == 'sneeze':                                   # 에취
        if t < 0.7:
            k = seg(t, 0, 0.7)
            p['hrot'], p['mouth'], p['mopen'] = -0.3 * k, 'open', 0.3 * k
            p['eyes'] = 'closed' if k > 0.5 else 'open'
        elif t < 0.95:
            p['hrot'], p['head'], p['eyes'], p['emote'] = 0.35, add(p['head'], (1.5, 1.5)), 'closed', 'sweat'
        else:
            p['eyes'], p['ears'] = ('closed' if t < 1.4 else 'open'), 0.4
        return p
    if name == 'sitpretty':                                # 얌전히 앉아 천천히 눈 깜빡
        p['c1'], p['head'] = add(p['c1'], (0, -1.5)), add(p['head'], (0, -2))
        p['eyes'] = 'closed' if (t % 3.0) > 2.3 else 'open'
        return p
    if name == 'groom_leg':                                # 다리 번쩍 들고 핥기(첼로 자세)
        k = seg(t, 0, 0.6) * (1 - seg(t, dur - 0.5, dur))
        bob = math.sin(t * 10)
        p['bn'] = lerp(p['bn'], leg((-5, -10), (9, -31), 1, 12.0, 13.0), k)
        p['head'] = lerp(p['head'], (-1.0, -20.0 + bob), k)
        p['face'], p['hrot'] = lerp(p['face'], -0.3, k), 0.6 * k
        p['c1'] = add(p['c1'], (-2 * k, 0))
        if k > 0.5:
            p['eyes'], p['mouth'] = 'closed', 'tongue'
        return p
    if name == 'meow':                                     # 야옹
        k = seg(t, 0.3, 0.5) * (1 - seg(t, 0.9, 1.1))
        if k > 0.05:
            p['mouth'], p['mopen'] = 'open', 0.45 * k
        p['hrot'] = -0.15 * k
        p['emote'] = 'note' if 0.3 < t < 2.0 else ''
        return p
    if name == 'chatter':                                  # 창밖 새 보고 깍깍
        p['hrot'], p['look'], p['ears'] = -0.35, (0.3, -1.0), 0.2
        p['mouth'], p['mopen'] = 'open', 0.1 + abs(math.sin(t * 38)) * 0.35
        p['tail'] = dict(p['tail'], ground=0.0, front=0.0, ang=3.3, amp=0.4, spd=9.0)
        return p
    if name == 'swat':                                     # 허공에 냥냥펀치
        a = t * 8
        p['fn'] = leg((5, -15), (12 + 4 * math.sin(a), -26 + 6 * math.cos(a)), 1, 7.5, 7.5)
        p['look'], p['ears'] = (0.7, -0.6), 0.2
        return p
    if name == 'love':                                     # 마우스 보고 천천히 눈 깜빡 + 하트
        p['eyes'] = 'closed' if (t % 2.4) > 1.2 else 'open'
        p['emote'] = 'heart'
        p['tail'] = dict(p['tail'], amp=0.25, spd=1.5)
        return p
    if name == 'confused':                                 # 어리둥절
        p['hrot'] = 0.4 * math.sin(min(t, 0.6) / 0.6 * math.pi / 2)
        p['emote'] = '?'
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
    def __init__(self, scale=1.3, palette='snow', ss=3, style='snow'):
        self.st = STYLES.get(style, STYLES['cute'])
        self.s, self.ss = scale, ss
        self.k = scale * ss
        self.W, self.H = int(CW * scale), int(CH * scale)
        self.pal = dict(COMMON, **PALETTES.get(palette, PALETTES['cheese']))   # 털색 쪽 값이 우선
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
            ring = self.st.get('ring', 1) and (i % 3 == 2 or i >= TAIL_N - 2)
            col = self.pal['dark'] if ring else self.pal['fur']
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
        if p['back'] < 0.5:
            self._ell(d, add(c1, (0.8, r1 * 0.45)), r1 * 0.6, r1 * 0.42, self.pal['belly'])
        for t in ((0.22, 0.42, 0.62) if self.st.get('bodystripe', 1) else ()):   # 등 줄무늬
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
        if p['back'] >= 0.5:                      # 뒷모습: 얼굴 없이 머리·귀만
            return
        if p['prop'] == 'hat':                    # 파란 고양이 모자: 머리를 덮고 얼굴만 남긴다
            blue = (120, 158, 214)
            self._group(d, [('poly', [H(q) for q in ears[0]], blue), ('poly', [H(q) for q in ears[1]], blue),
                            ('e', H((0, -0.5)), 10.4 * hs, 9.4 * hs, blue)])
            self._ell(d, H((0.8 * f, 2.2)), 7.6 * hs, 6.0 * hs, fur)
            self._cap(d, H((-3, 8.4)), H((3, 8.4)), 0.7, blue)
            ears = []
        for e in ears:                            # 귀 안쪽
            cx = sum(q[0] for q in e) / 3
            cy = sum(q[1] for q in e) / 3
            inner = [H((cx + (q[0] - cx) * 0.55, cy + (q[1] - cy) * 0.55 + 0.4)) for q in e]
            d.polygon([self.P(q) for q in inner], fill=self.pal['ear_in'])
        for dx in ((-2.2, 0.0, 2.2) if st['stripe'] else ()):   # 이마 줄무늬
            q = H((dx + 1.2 * f, -7.2))
            self._cap(d, q, H((dx * 0.8 + 1.2 * f, -5.4 + 0.8 * st['stripe'])), 0.6, self.pal['dark'])
        if st['mz']:
            self._ell(d, H((2.2 * f, 3.5)), 4.3 * hs * st['mz'], 3.0 * hs * st['mz'], self.pal['belly'])
        lw = max(1, int(0.45 * self.k))
        for sgn in ((-1, 1) if st.get('whisker', 1) else ()):   # 수염
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
        if m == 'smile':                          # 기분 좋아 입 벌리고 웃기
            c = H((2.2 * f, 4.3))
            x, y = self.P(c)
            r = 1.9 * k
            d.chord([x - r, y - r, x + r, y + r], 0, 180, fill=self.pal['mouth'])
            self._ell(d, add(c, (0, 1.1)), 1.1, 0.7, self.pal['tongue'])
        elif m == 'open' and p['mopen'] > 0.05:
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
        if state == 'open' and 'iris' in self.pal:   # 파란 눈: 진한 테두리 → 홍채 → 동공 → 반짝이
            d.ellipse([x - r * k, y - r * 1.15 * k, x + r * k, y + r * 1.15 * k], fill=col)
            ri = 0.8 * r
            d.ellipse([x - ri * k, y - ri * 1.1 * k, x + ri * k, y + ri * 1.15 * k], fill=self.pal['iris'])
            d.ellipse([x - 0.55 * ri * k, y + 0.2 * ri * k, x + 0.55 * ri * k, y + 0.95 * ri * k],
                      fill=self.pal['iris_hi'])
            rp = 0.5 * r
            d.ellipse([x - rp * k, y - rp * 1.1 * k, x + rp * k, y + rp * 1.0 * k], fill=col)
            hr = 0.33 * r
            hx, hy = x + 0.3 * r * k, y - 0.45 * r * k
            d.ellipse([hx - hr * k, hy - hr * k, hx + hr * k, hy + hr * k], fill=self.pal['white'])
            sx, sy, sr = x - 0.35 * r * k, y + 0.4 * r * k, 0.14 * r
            d.ellipse([sx - sr * k, sy - sr * k, sx + sr * k, sy + sr * k], fill=self.pal['white'])
        elif state == 'open':
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
        """머리 위 말풍선 기호: ! ? 음표 하트 땀 …"""
        e = p['emote'] or ('!' if p['mark'] >= 0.5 else '')
        if not e:
            return
        top = add(p['head'], (3, -20 - 8 * self.st['hs']))
        if e == '!':
            self._cap(d, top, add(top, (0, 5.5)), 0.95, (230, 60, 60))
            self._ell(d, add(top, (0, 8.6)), 1.0, 1.0, (230, 60, 60))
        elif e == '?':
            x, y = self.P(add(top, (0, 2)))
            k, c = self.k, (80, 90, 210)
            d.arc([x - 2.4 * k, y - 2.4 * k, x + 2.4 * k, y + 2.4 * k], 180, 90, fill=c, width=int(1.2 * k))
            self._cap(d, add(top, (0, 4.4)), add(top, (0, 5.6)), 0.6, c)
            self._ell(d, add(top, (0, 8.4)), 0.9, 0.9, c)
        elif e == 'note':
            c = (70, 70, 90)
            self._ell(d, add(top, (-1.5, 7)), 1.8, 1.3, c)
            self._cap(d, add(top, (0, 7)), add(top, (0, 0)), 0.4, c)
            self._cap(d, add(top, (0, 0)), add(top, (3, 2)), 0.5, c)
        elif e == 'heart':
            c = (240, 90, 120)
            self._ell(d, add(top, (-1.5, 3)), 1.9, 1.9, c)
            self._ell(d, add(top, (1.5, 3)), 1.9, 1.9, c)
            d.polygon([self.P(add(top, (-3.3, 3.6))), self.P(add(top, (3.3, 3.6))), self.P(add(top, (0, 8)))], fill=c)
        elif e == 'sweat':
            c = (110, 170, 240)
            s = add(p['head'], (11 * self.st['hs'], -8))
            d.polygon([self.P(add(s, (0, -3))), self.P(add(s, (-1.5, 0.5))), self.P(add(s, (1.5, 0.5)))], fill=c)
            self._ell(d, add(s, (0, 0.8)), 1.6, 1.6, c)
        elif e == '...':
            for i in range(3):
                self._ell(d, add(top, (-3 + 3 * i, 6)), 0.8, 0.8, (90, 90, 90))

    def render(self, p, facing=1, time=0.0):
        big = Image.new('RGBA', (self.W * self.ss, self.H * self.ss), (0, 0, 0, 0))
        d = ImageDraw.Draw(big)
        far = self.pal['far']
        prop = p['prop']
        if prop == 'box':                         # 박스 뒷면
            d.polygon([self.P(q) for q in ((-20, -15), (21, -15), (25, -19), (-16, -19))], fill=(176, 128, 84))
        arms_front = p['armsfront'] >= 0.5           # 매달리기: 앞발이 큰 머리 앞으로 보여야 한다
        for k in (('bf',) if arms_front else ('ff', 'bf')):
            self._group(d, self._leg_shapes(p[k], far, k == 'bf'))
        if p['tail']['front'] < 0.5:
            self._tail(d, p['tail'], time)
        self._body(d, p)
        if p['tail']['front'] >= 0.5:
            self._tail(d, p['tail'], time)
        for k in (('bn',) if arms_front else ('bn', 'fn')):
            self._group(d, self._leg_shapes(p[k], self.pal['fur'], k == 'bn'))
        if prop == 'fish':                        # 생선 장난감 (앞발 아래)
            blue = (110, 150, 205)
            self._group(d, [('e', (14, -5), 6.0, 3.2, blue),
                            ('poly', [(7, -5), (3, -9), (3, -1)], blue)])
            self._ell(d, (17.5, -5.8), 0.9, 0.9, (40, 50, 80))
        self._head(d, p)
        if arms_front:
            self._group(d, self._leg_shapes(p['ff'], far, False))
            self._group(d, self._leg_shapes(p['fn'], self.pal['fur'], False))
        if prop == 'heart':                       # 하트 쿠션 안기
            pink = (244, 140, 160)
            hc = add(p['head'], (2, 14))
            self._group(d, [('e', add(hc, (-3.4, -1.5)), 4.6, 4.6, pink), ('e', add(hc, (3.4, -1.5)), 4.6, 4.6, pink),
                            ('poly', [add(hc, (-7.6, 0)), add(hc, (7.6, 0)), add(hc, (0, 8.5))], pink)])
            for dx in (-6.5, 7.0):
                self._group(d, [('e', add(hc, (dx, -2.5)), 2.8, 2.3, self.pal['fur'])])
        if prop == 'box':                         # 박스 앞면 — 몸은 가리고 얼굴만 빼꼼
            front = [self.P(q) for q in ((-20, -15), (21, -15), (21, 2), (-20, 2))]
            d.polygon(front, fill=(214, 166, 114), outline=(160, 112, 72))
            d.polygon([self.P(q) for q in ((21, -15), (25, -19), (25, -2), (21, 2))], fill=(192, 144, 96))
            for dx, dy, r in ((0, -4, 1.7), (-2.4, -7, 0.8), (0, -7.8, 0.8), (2.4, -7, 0.8)):
                self._ell(d, (dx, dy), r, r, (170, 122, 82))
        if prop == 'butterfly':                   # 나비
            bx, by = 20 + 4 * math.sin(time * 1.3), -50 + 3 * math.sin(time * 2.1)
            fl = abs(math.sin(time * 14))
            yel = (246, 196, 80)
            for sgn in (-1, 1):
                self._ell(d, (bx + sgn * 2.4 * fl, by - 1), 2.6 * fl + 0.4, 2.2, yel)
                self._ell(d, (bx + sgn * 1.8 * fl, by + 1.8), 1.6 * fl + 0.3, 1.4, yel)
            self._cap(d, (bx, by - 2), (bx, by + 2.5), 0.4, (120, 90, 60))
        if prop in ('star', 'sparkle'):           # 반짝이는 별 / 기분 좋은 반짝이
            yel = (246, 200, 90)
            pts = [(20, -52)] if prop == 'star' else [add(p['head'], q) for q in ((-19, -6), (19, -6), (-17, 6), (17, 6))]
            s = 1.6 + 0.8 * math.sin(time * 5)
            for (sx, sy) in pts:
                d.polygon([self.P(q) for q in ((sx, sy - 2 * s), (sx + 0.5 * s, sy - 0.5 * s), (sx + 2 * s, sy),
                                                (sx + 0.5 * s, sy + 0.5 * s), (sx, sy + 2 * s), (sx - 0.5 * s, sy + 0.5 * s),
                                                (sx - 2 * s, sy), (sx - 0.5 * s, sy - 0.5 * s))], fill=yel)
        img = big.resize((self.W, self.H), Image.LANCZOS)
        if p['clip'] > -60:                       # 몰래 보기: 창 모서리 뒤에 숨은 부분은 지운다
            cx = int((OX + p['clip']) * self.s)
            if cx > 0:
                img.paste((0, 0, 0, 0), (0, 0, min(cx, self.W), self.H))
        if facing < 0:
            img = ImageOps.mirror(img)
        # 글자(Zzz, !)는 뒤집히면 안 되므로 뒤집은 뒤에 따로 그린다
        if p['zzz'] >= 0.3 or p['mark'] >= 0.5 or p['emote']:
            over = Image.new('RGBA', big.size, (0, 0, 0, 0))
            od = ImageDraw.Draw(over)
            q = dict(p, head=(p['head'][0] * facing, p['head'][1]))
            self._zzz(od, q, time)
            self._mark(od, q)
            img.alpha_composite(over.resize((self.W, self.H), Image.LANCZOS))
        return img
