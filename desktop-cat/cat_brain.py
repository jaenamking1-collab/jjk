# 고양이 행동·물리. OS 와 무관하다 — 발판 목록·모니터·마우스 위치만 받아서 위치와 자세를 낸다.
import math
import random
from cat_render import action_pose, lerp, clamp, ease, stride, HANG

# 아무 데서나 하는 쉬는 행동. 이름: (가중치, (최소초, 최대초))
IDLE = {
    'groom': (8, (3.5, 6.0)),        # 옆구리 그루밍
    'scratch': (6, (1.8, 3.0)),      # 뒷발로 귀 긁기
    'lickpaw': (6, (2.5, 4.5)),      # 앞발 핥기
    'wash': (6, (3.8, 5.5)),         # 세수
    'loaf': (6, (6.0, 14.0)),        # 엎드리기(식빵)
    'yawn': (5, (2.2, 2.8)),         # 하품
    'stretch': (5, (2.8, 3.4)),      # 기지개
    'lookaround': (6, (3.0, 6.0)),   # 두리번
    'sleep': (3, (12.0, 30.0)),      # 웅크려 자기
    'knead': (4, (3.0, 5.0)),        # 꾹꾹이
    'tailflick': (5, (2.5, 4.0)),    # 꼬리 탁탁
    'sit': (5, (2.0, 5.0)),          # 그냥 앉아 있기
    'headtilt': (5, (2.5, 3.5)),     # 고개 갸웃
    'curious': (4, (2.0, 3.5)),      # 궁금해하기
    'happy': (4, (2.0, 3.0)),        # 기분 좋기
    'hugheart': (3, (3.0, 5.0)),     # 하트 안기
    'stargaze': (3, (3.0, 6.0)),     # 별 보기
    'fishtoy': (3, (4.0, 7.0)),      # 생선 장난감 놀이
    'hat': (2, (4.0, 8.0)),          # 모자 쓰기
    'backview': (3, (4.0, 8.0)),     # 뒷모습으로 먼 곳 보기
    'box': (3, (5.0, 10.0)),         # 박스 안에 쏙
    'zoneout': (3, (3.0, 6.0)),      # 멍때리기
    'sneeze': (2, (1.6, 2.0)),       # 에취
    'sitpretty': (4, (3.0, 6.0)),    # 얌전히 앉기
    'groom_leg': (4, (3.5, 5.5)),    # 다리 들고 핥기
    'meow': (4, (1.5, 2.2)),         # 야옹
    'chatter': (3, (1.8, 3.0)),      # 깍깍
    'swat': (3, (1.5, 2.5)),         # 냥냥펀치
    'bugcatch': (3, (2.5, 4.0)),     # 나비 잡기
    'belly': (4, (3.0, 6.0)),        # 배 보이기
    'roll': (3, (2.0, 3.5)),         # 데굴데굴
    'sleep_back': (2, (10.0, 25.0)), # 벌러덩 자기
    'sleep_side': (2, (10.0, 25.0)), # 옆으로 누워 자기
    'sploot': (3, (4.0, 9.0)),       # 다리 쭉 엎드리기
    'sulk': (2, (3.0, 6.0)),         # 삐지기
    'shake': (3, (0.8, 1.1)),        # 몸 털기
    'sniff': (4, (2.0, 3.5)),        # 킁킁
    'stretch_back': (3, (1.8, 2.6)), # 뒷다리 쭉
    'hunt': (3, (2.0, 4.0)),         # 사냥 모드
    'tailchase': (2, (2.0, 3.5)),    # 꼬리 쫓기
}
LOOKABLE = {'sit', 'watch', 'stand', 'walk', 'trot', 'sneak', 'tailflick', 'loaf', 'knead', 'lookdown',
            'sitpretty', 'curious', 'sploot', 'belly', 'hunt', 'hang', 'cling', 'climb', 'box', 'peek'}
WALK_V = {'walk': 32, 'sneak': 16, 'trot': 70, 'hop': 60, 'back': 18, 'run': 330}   # 단위/초


class Platform:
    """발판: x1~x2 구간의 높이 y. kind = ground/window/icon/edge, owner = 함께 움직일 주인."""
    __slots__ = ('x1', 'x2', 'y', 'kind', 'owner')

    def __init__(self, x1, x2, y, kind, owner=None):
        self.x1, self.x2, self.y, self.kind, self.owner = x1, x2, y, kind, owner

    def __repr__(self):
        return 'Platform(%d-%d @%d %s)' % (self.x1, self.x2, self.y, self.kind)


def ground_platforms(monitors):
    return [Platform(m['work'][0], m['work'][2], m['work'][3], 'ground', ('ground', i))
            for i, m in enumerate(monitors)]


class CatBrain:
    def __init__(self, scale=1.3, monitors=None):
        S = self.S = scale
        self.g = 1800 * S                 # 중력
        self.walk_v = WALK_V['walk'] * S
        self.run_v = WALK_V['run'] * S
        self.climb_v = 55 * S             # 벽 오르는 속도
        self.flee_r = 100 * S             # 마우스가 이 안으로 오면 도망
        self.max_up = 330 * S             # 위로 뛸 수 있는 높이
        self.max_dx = 700 * S             # 보통 점프 거리
        self.far_dx = 2400 * S            # 가끔 하는 먼 점프(다른 모니터)
        self.min_land = 20 * S            # 착지하려면 필요한 발판 폭
        self.margin = 9 * S               # 발판 끝에서 이만큼 안쪽에 선다
        self.gap = 9 * S                  # 이 정도 틈은 그냥 걸어서 건넌다
        self.head = 32 * S                # 머리 공간(이보다 위가 화면 밖이면 발판 무시)

        self.time = 0.0
        self.x = self.y = 0.0
        self.vx = self.vy = 0.0
        self.facing = 1
        self.mode = 'ground'              # ground / air / fall / hang(매달림) / climb(벽)
        self.plat = None
        self.ref = None                   # 탄 창의 직전 위치(창과 함께 움직이기)
        self.missing = 0.0
        self.action, self.at, self.adur = 'sit', 0.0, 2.0
        self.plan = []
        self.walk_phase = 0.0
        self.jump = None
        self.flee = False
        self.flee_t = 0.0
        self.flee_dir = 1
        self.mark_t = 0.0
        self.blink_at, self.blink_t = 2.0, 0.0
        self.cursor_prev, self.cursor_v = None, 0.0
        self.last_pose = None
        self.blend_from, self.blend_t = None, 1.0
        self.monitors = monitors or []
        self.spawned = False
        self.walls = []                   # 창 옆면: dict(x, top, bottom, side, owner, reach)
        self.hang = None                  # 매달린 발판 {'p', 'x'}
        self.wall = None                  # 타고 있는 벽
        self.climb_dir, self.climb_stop = -1, None
        self.cprog = 0.0                  # 매달림 ↔ 올라섬 진행도
        self.chain = 0                    # 남은 연속 점프 수
        self.top_y = 0.0                  # 떨어지기 시작한 높이(높은 데서 떨어지면 쿵)
        self.confused = False             # 발판이 사라져 떨어짐 → 착지 후 어리둥절
        self.still_t = 0.0                # 마우스가 가만히 있은 시간
        self.cursor = (0, 0)
        self.spin_t = 0.0
        self.cur_v = 0.0
        self.clip = -99.0
        self.ride_cool = 0.0
        self.grab = None                  # 떨어지다 붙잡을 발판
        self.stalk_cool = 0.0

    # ── 도우미 ─────────────────────────────────────────────
    def _mon_at(self, x, y):
        for m in self.monitors:
            l, t, r, b = m['rect']
            if l <= x < r and t <= y <= b:
                return m
        return None

    def _usable(self, plats):
        """화면 위쪽에 머리 공간이 없는 발판(최대화된 창의 윗변 등)은 뺀다."""
        out = []
        for p in plats:
            m = self._mon_at((p.x1 + p.x2) / 2, p.y - 1)
            if m and p.y - self.head >= m['rect'][1] and p.x2 - p.x1 >= 4:
                out.append(p)
        return out

    def _support(self, plats, x, y, tol):
        best, bd = None, 1e9
        cur = self.plat.owner if self.plat else None
        for p in plats:
            if p.x1 - self.gap <= x <= p.x2 + self.gap:
                d = abs(p.y - y)
                if d <= tol:
                    d += 0 if (cur is not None and p.owner == cur) else 2
                    if d < bd:
                        best, bd = p, d
        return best

    def _set(self, name, dur=None):
        if name != self.action:
            self.blend_from, self.blend_t = self.last_pose, 0.0
            self.action, self.at = name, 0.0
        if dur is not None:
            self.adur = dur

    def spawn(self, plats):
        grounds = [p for p in plats if p.kind == 'ground'] or plats
        p = grounds[0]
        self.x = random.uniform(p.x1 + 80, max(p.x1 + 81, p.x2 - 80))
        self.y = p.y
        self.plat, self.mode = p, 'ground'
        self.plan = []
        self._set('sit', 2.0)
        self.spawned = True

    def call_to(self, x, plats):
        """트레이 메뉴 '여기로 부르기' — 해당 모니터 바닥에 떨어뜨린다."""
        self.x, self.y = x, self._mon_y_top(x)
        self.mode, self.vx, self.vy, self.plan = 'fall', 0.0, 0.0, []
        self._set('fall')

    def pet(self):
        """Ctrl+좌클릭 = 쓰다듬기: 바닥에 있으면 하트 보내며 기분 좋아한다."""
        if self.mode == 'ground' and not self.flee:
            self.facing = 1 if self.cursor[0] >= self.x else -1
            self.plan = [('idle', 'love', 2.5), ('idle', 'happy', 1.5)]
            self.chain = 0

    def _mon_y_top(self, x):
        for m in self.monitors:
            if m['rect'][0] <= x < m['rect'][2]:
                return m['rect'][1] + 40
        return 0

    # ── 점프 ───────────────────────────────────────────────
    def _candidates(self, plats, far=False, away=None, direction=0):
        S, res = self.S, []
        maxdx = self.far_dx if far else self.max_dx
        for p in plats:
            if p is self.plat or p.x2 - p.x1 < self.min_land:
                continue
            if self.plat and abs(p.y - self.plat.y) < 5 * S and \
                    p.x1 - self.gap <= self.plat.x2 and p.x2 + self.gap >= self.plat.x1:
                continue                          # 이어진 발판 — 걸어가면 된다
            m = min(self.margin, (p.x2 - p.x1) / 2)
            lx = clamp(self.x + random.uniform(-50, 50) * S, p.x1 + m, p.x2 - m)
            dx, dy = lx - self.x, p.y - self.y
            if abs(dx) < 12 * S and abs(dy) < 12 * S:
                continue
            if -dy > self.max_up or abs(dx) > maxdx:
                continue
            if direction and dx * direction < 0 and dy < 40 * S:
                continue
            d = math.hypot(dx, dy)
            w = {'icon': 3.0, 'window': 1.8, 'edge': 2.4, 'ground': 0.5}.get(p.kind, 1.0)
            w *= (d / (600 * S)) if far else (math.exp(-d / (260 * S)) + 0.05)
            if dy > 30 * S:
                w *= 1.3
            if away:
                md = math.hypot(lx - away[0], p.y - away[1])
                if md < self.flee_r * 1.6:
                    continue
                w *= md / self.flee_r
            res.append((w, p, lx))
        return res

    @staticmethod
    def _pick(cands):
        tot = sum(c[0] for c in cands)
        r = random.uniform(0, tot)
        for c in cands:
            r -= c[0]
            if r <= 0:
                return c
        return cands[-1]

    def _launch(self, tx, ty, land=None):
        """포물선 점프. land: None(발판에 선다) / ('hang', 발판) / ('wall', 벽)."""
        S, g = self.S, self.g
        dx = tx - self.x
        apex = min(self.y, ty) - max(18 * S, 0.13 * abs(dx)) - 6 * S
        h1, h2 = self.y - apex, ty - apex
        T = math.sqrt(2 * h1 / g) + math.sqrt(2 * h2 / g)
        self.jump = dict(x0=self.x, y0=self.y, vx=dx / T, vy=-math.sqrt(2 * g * h1), T=T, t=0.0, tx=tx, ty=ty,
                         land=land)
        if abs(dx) > 1:
            self.facing = 1 if dx > 0 else -1
        self.mode, self.plat = 'air', None
        self._set('air')

    def _corner_w(self, p):
        """모니터 구석에 가까운 발판일수록 큰 값(구석 탐험용)."""
        best = 1e9
        for m in self.monitors:
            l, t, r, b = m['rect']
            for cx, cy in ((l, t), (r, t), (l, b), (r, b)):
                best = min(best, math.hypot((p.x1 + p.x2) / 2 - cx, p.y - cy))
        return 1.0 / (1.0 + best / (150 * self.S))

    def _takeoff(self, lx):
        if not self.plat:
            return self.x
        d = 1 if lx > self.x else -1
        return clamp(lx - d * 30 * self.S, self.plat.x1 + self.margin, self.plat.x2 - self.margin)

    def _plan_jump(self, plats, far=False, corner=False, quick=False):
        c = self._candidates(plats, far=far)
        if not c:
            return False
        if corner:
            c = [(w * self._corner_w(p) * 8, p, lx) for w, p, lx in c]
        _, p, lx = self._pick(c)
        land = None
        if p.y < self.y - 30 * self.S and p.kind != 'ground' and random.random() < 0.3:
            land = 'hang'                         # 위로 뛰다 아슬아슬하게 매달린다
        self.plan = [('walk', self._takeoff(lx)),
                     ('crouch', (p, lx), random.uniform(0.15, 0.3) if quick else random.uniform(0.45, 0.9), land)]
        return True

    # ── 매달리기·벽 ────────────────────────────────────────
    def _enter_hang(self, p, x, climbdown=False):
        S = self.S
        x = clamp(x, p.x1 + 4 * S, p.x2 - 4 * S)
        self.mode, self.plat, self.hang = 'hang', None, {'p': p, 'x': x}
        self.x, self.y = x, p.y + HANG * S
        self.cprog = 1.0 if climbdown else 0.0
        r = random.random()
        if r < 0.45:
            seq = [('hang', 'hang', random.uniform(1.0, 2.5)), ('hang', 'hang_kick', random.uniform(0.8, 1.8)),
                   ('climbup',)]
        elif r < 0.65:
            seq = [('hang', 'hang_kick', random.uniform(1.2, 2.2)), ('drop',)]
        elif r < 0.85:
            seq = [('hang', 'hang_one', random.uniform(1.5, 2.5)), ('hang', 'hang_kick', 0.8), ('climbup',)]
        else:
            seq = [('hang', 'hang', 1.5), ('drop',)]
        self.plan = ([('climbdown',)] if climbdown else []) + seq
        if climbdown:
            self.y = p.y
            self._set('climbup')
        else:
            self._set('hang')

    def _find_wall(self, w):
        for v in self.walls:
            if v['owner'] == w['owner'] and v['side'] == w['side']:
                return v
        return None

    def _plan_wall(self):
        S, p = self.S, self.plat
        if not p:
            return False
        cands = []
        for w in self.walls:
            if w['bottom'] - w['top'] < 70 * S:
                continue
            sx = w['x'] - w['side'] * 2 * S
            if w['top'] < self.y - 60 * S and w['bottom'] >= self.y - 4 * S and \
                    p.x1 - self.gap <= sx <= p.x2 + self.gap and abs(sx - self.x) < 600 * S:
                cands.append(('walk', w, sx))
            elif self.y - 4 * S > w['bottom'] > self.y - self.max_up and w['bottom'] - w['top'] > 90 * S:
                take = clamp(sx - w['side'] * 45 * S, p.x1 + self.margin, p.x2 - self.margin)
                if abs(take - sx) < 160 * S:
                    cands.append(('jump', w, take))
        if not cands:
            return False
        how, w, sx = random.choice(cands)
        if how == 'walk':
            self.plan = [('walk', sx), ('face', w['side']), ('climbwall', w)]
        else:
            self.plan = [('walk', sx), ('face', w['side']), ('crouchwall', w, random.uniform(0.4, 0.8))]
        return True

    def _start_climb(self, w, y=None):
        self.mode, self.wall, self.plat = 'climb', w, None
        self.x, self.facing = w['x'] - w['side'] * 1 * self.S, w['side']
        if y is not None:
            self.y = y
        self.climb_dir = -1
        self.climb_stop = None
        if random.random() < 0.3:                 # 가끔 중간에 멈칫했다 주르륵
            self.climb_stop = random.uniform(w['top'] + 60 * self.S, max(w['top'] + 61 * self.S, self.y - 30 * self.S))
        self._set('climb')

    # ── 결정 ───────────────────────────────────────────────
    def _decide(self, plats):
        S = self.S
        if self.chain > 0:                        # 연속 점프 중
            self.chain -= 1
            if self._plan_jump(plats, quick=True):
                return
        md = math.hypot(self.cursor[0] - self.x, self.cursor[1] - (self.y - 22 * S))
        if self.still_t > 3.0 and 130 * S < md < 420 * S and self.time > self.stalk_cool and random.random() < 0.3:
            self.stalk_cool = self.time + random.uniform(40, 90)   # 마우스 놀이는 가끔만
            if md < 260 * S and random.random() < 0.5:
                self.plan = [('face', 1 if self.cursor[0] > self.x else -1), ('idle', 'love', random.uniform(3, 5))]
            else:
                self.plan = [('stalk',), ('pounce', random.uniform(1.0, 1.8))]
            return
        opts = [('walk', 14), ('jump', 18), ('idle', 34), ('chain', 4), ('far', 3), ('zoom', 3),
                ('patrol', 4), ('wall', 7), ('edge', 7), ('spin', 2), ('corner', 3), ('peek', 3), ('claw', 2)]
        while opts:
            kind = random.choices([o[0] for o in opts], [o[1] for o in opts])[0]
            if self._try(kind, plats):
                return
            opts = [o for o in opts if o[0] != kind]

    def _try(self, kind, plats):
        S, p = self.S, self.plat
        width = (p.x2 - p.x1) if p else 0
        lo, hi = (p.x1 + self.margin, p.x2 - self.margin) if p else (self.x, self.x)
        if kind == 'walk':
            if width < 70 * S:
                return False
            style = random.choices(['walk', 'trot', 'sneak', 'hop', 'back'], [50, 15, 15, 12, 8])[0]
            if style == 'back':
                tx = clamp(self.x - self.facing * random.uniform(20, 45) * S, lo, hi)
            else:
                tx = random.uniform(lo, hi)
                if abs(tx - self.x) < 30 * S:
                    tx = lo if self.x - lo > hi - self.x else hi
            self.plan = [('walk', tx, style)]
            return abs(tx - self.x) > 5 * S
        if kind == 'jump':
            return self._plan_jump(plats)
        if kind == 'chain':
            self.chain = random.randint(2, 4)
            return self._plan_jump(plats, quick=True)
        if kind == 'far':
            return self._plan_jump(plats, far=True)
        if kind == 'corner':
            return self._plan_jump(plats, corner=True)
        if kind == 'zoom':                        # 우다다: 끝까지 달렸다 돌아오기
            if width < 150 * S:
                return False
            a, b = (hi, lo) if self.x - lo < hi - self.x else (lo, hi)
            self.plan = [('walk', a, 'run'), ('walk', b, 'run'), ('idle', random.choice(['shake', 'sit', 'happy']), 1.5)]
            return True
        if kind == 'patrol':                      # 순찰: 걷다 멈춰 냄새 맡고 두리번
            if width < 120 * S:
                return False
            self.plan = []
            for _ in range(3):
                self.plan += [('walk', random.uniform(lo, hi), 'walk'),
                              ('idle', random.choice(['sniff', 'lookaround', 'curious']), random.uniform(1.2, 2.2))]
            return True
        if kind == 'wall':
            return self._plan_wall()
        if kind == 'edge':                        # 높은 곳 가장자리: 내려다보기·미끄러져 매달리기·뛰어내리기·앞발 대롱
            if not p or p.kind == 'ground':
                return False
            left = self.x - p.x1 < p.x2 - self.x
            ex = p.x1 + self.margin if left else p.x2 - self.margin
            out = -1 if left else 1
            self.plan = [('walk', ex), ('face', out)]
            r = random.random()
            if r < 0.3:
                self.plan += [('idle', 'lookdown', random.uniform(2.0, 4.0)), ('slip',)]
            elif r < 0.55:
                self.plan += [('idle', 'lookdown', random.uniform(1.5, 3.0)), ('hopdown', out)]
            elif r < 0.8:
                self.plan += [('idle', 'paw_dangle', random.uniform(3.0, 6.0))]
            else:
                self.plan += [('idle', 'lookdown', random.uniform(3.0, 5.5))]
            return True
        if kind == 'spin':                        # 제자리에서 빙글 돌고 눕기
            self.plan = [('spin', random.uniform(1.0, 1.6)),
                         ('idle', random.choice(['loaf', 'sleep', 'sleep_side']), random.uniform(8, 18))]
            return True
        if kind == 'peek':                        # 창 모서리 뒤에 숨어 빼꼼
            for w in self.walls:
                x = w['x'] + w['side'] * 6 * S
                if p and w['bottom'] >= self.y - 3 * S and w['top'] <= self.y - 50 * S and lo <= x <= hi \
                        and abs(x - self.x) < 500 * S:
                    self.plan = [('walk', x, 'sneak'), ('face', -w['side']), ('peek', w, random.uniform(3, 6))]
                    return True
            return False
        if kind == 'claw':                        # 창 옆면 박박 긁기
            for w in self.walls:
                x = w['x'] - w['side'] * 9 * S
                if p and w['top'] <= self.y - 50 * S and w['bottom'] >= self.y - 4 * S and lo <= x <= hi \
                        and abs(x - self.x) < 400 * S:
                    self.plan = [('walk', x), ('face', w['side']), ('idle', 'claw', random.uniform(2.0, 3.5))]
                    return True
            return False
        names = list(IDLE)
        name = random.choices(names, [IDLE[n][0] for n in names])[0]
        self.plan = [('idle', name, random.uniform(*IDLE[name][1]))]
        return True

    # ── 도망 ───────────────────────────────────────────────
    def _start_flee(self, cx):
        self.flee, self.flee_t, self.mark_t = True, 0.0, 0.7
        self.flee_dir = 1 if self.x >= cx else -1
        self.plan = []
        self.chain = 0
        if self.mode in ('hang', 'climb'):         # 매달려 있다 놀라면 그냥 손을 놓는다
            self.mode, self.vx, self.vy = 'fall', self.flee_dir * 60 * self.S, 0.0
            self.hang = self.wall = None
            self._set('fall')
        elif self.mode == 'ground':                # 깜짝 놀라 폴짝
            self.mode, self.vx, self.vy = 'fall', self.flee_dir * 70 * self.S, -300 * self.S
            self.facing = self.flee_dir
            self.plat = None
            self._set('fall')

    def _flee_step(self, dt, plats, cursor):
        p = self.plat
        self.facing = self.flee_dir
        self._set('run')
        nx = self.x + self.flee_dir * self.run_v * dt
        edge = p.x2 if self.flee_dir > 0 else p.x1
        if (edge - nx) * self.flee_dir < 14 * self.S:
            nxt = self._support(plats, edge + self.flee_dir * (self.gap + 2), p.y, 6 * self.S)
            if nxt is None or nxt is p:
                c = self._candidates(plats, away=cursor, direction=self.flee_dir)
                if c:
                    _, tp, lx = max(c, key=lambda c: c[0] * random.uniform(0.6, 1.4))
                    self._launch(lx, tp.y)
                    return
                below = self._mon_at(edge + self.flee_dir * 20 * self.S, p.y + 5)
                if p.kind == 'ground' or below is None:
                    self.flee_dir = -self.flee_dir      # 막다른 곳 — 반대로 내뺀다
                    return
        self.x = nx

    # ── 매 프레임 ──────────────────────────────────────────
    def update(self, dt, world):
        dt = min(dt, 0.1)
        self.time += dt
        self.at += dt
        self.blend_t += dt
        self.monitors = world['monitors']
        plats = self._usable(world['platforms'])
        refs = world.get('refs', {})
        self.walls = [w for w in world.get('walls', []) if self._mon_at(w['x'], w['top'] + 1)]
        self.ride_cool -= dt
        if not self.spawned:
            self.spawn(plats)

        # 마우스
        cx, cy = world['cursor']
        if self.cursor_prev:
            v = math.hypot(cx - self.cursor_prev[0], cy - self.cursor_prev[1]) / max(dt, 1e-3)
            self.cursor_v = self.cursor_v * 0.7 + v * 0.3
        self.cursor_prev = self.cursor = (cx, cy)
        self.still_t = self.still_t + dt if self.cursor_v < 15 * self.S else 0.0
        hx, hy = self.x, self.y - 22 * self.S
        md = math.hypot(cx - hx, cy - hy)
        if not self.flee and self.mode != 'air' and md < self.flee_r and not world.get('calm') and \
                (self.cursor_v > 40 * self.S or md < self.flee_r * 0.55):
            self._start_flee(cx)
        if self.flee:
            self.flee_t += dt
            if md > self.flee_r * 2.6 and self.flee_t > 0.8:
                self.flee = False
                self.plan = [('face', 1 if cx > self.x else -1), ('idle', 'watch', random.uniform(1.5, 3.0))]
        self.mark_t -= dt

        # 창이 움직이면 같이 움직인다 (서 있든, 매달려 있든, 벽에 붙어 있든)
        owner = (self.plat.owner if self.mode == 'ground' and self.plat else
                 self.hang['p'].owner if self.mode == 'hang' and self.hang else
                 self.wall['owner'] if self.mode == 'climb' and self.wall else None)
        if owner is not None and owner in refs:
            nr = refs[owner]
            if self.ref is not None and self.ref[0] == owner:
                dx, dy = nr[0] - self.ref[1][0], nr[1] - self.ref[1][1]
                self.x += dx
                self.y += dy
                if self.hang:
                    self.hang['x'] += dx
                if math.hypot(dx, dy) > 25 and self.ride_cool <= 0 and self.mode == 'ground' and not self.flee:
                    self.plan = [('idle', 'brace', 0.7)] + self.plan     # 휙 움직이면 납작 버팀
                    self.ride_cool = 3.0
            self.ref = (owner, nr)
        else:
            self.ref = None

        if self.mode in ('ground', 'hang', 'climb'):
            self.top_y = self.y
        else:
            self.top_y = min(self.top_y, self.y)
        if self.action == 'tailchase':            # 꼬리 쫓기: 빙글빙글
            self.spin_t += dt
            if self.spin_t > 0.3:
                self.facing, self.spin_t = -self.facing, 0.0

        if self.mode == 'ground':
            self._ground(dt, plats, (cx, cy))
        elif self.mode == 'air':
            self._air(dt, plats)
        elif self.mode == 'hang':
            self._hang(dt, plats)
        elif self.mode == 'climb':
            self._climb(dt, plats)
        else:
            self._fall(dt, plats)

        return self._pose(dt, cx, cy)

    def _ground(self, dt, plats, cursor):
        kind = self.plat.kind if self.plat else 'ground'
        sup = self._support(plats, self.x, self.y, (5 if kind == 'edge' else 8) * self.S)
        if sup is not None:
            self.plat, self.y, self.missing = sup, sup.y, 0.0
        else:
            self.missing += dt
            if self.missing > (0.45 if kind == 'edge' else 0.06):
                self.mode, self.vy = 'fall', 0.0
                self.vx = self.facing * self.cur_v * 0.8
                self.confused = not self.flee and self.action not in ('walk', 'run', 'trot', 'hop', 'sneak', 'back')
                self.plat = None
                self.plan = []
                self._set('fall')
                return
        if self.plat is None:
            return
        self.cur_v = 0.0
        if self.flee:
            self._flee_step(dt, plats, cursor)
            self.cur_v = self.run_v
            self.walk_phase += self.run_v / self.S * dt / stride('run')
            return

        if not self.plan and (self.action in IDLE or self.action in ('watch', 'land', 'stand')) \
                and self.at < self.adur:
            return
        if not self.plan:
            self._decide(plats)
            if not self.plan:
                return
        step = self.plan[0]
        S = self.S
        if step[0] == 'walk':
            style = step[2] if len(step) > 2 else 'walk'
            tx = clamp(step[1], self.plat.x1 - self.gap + 1, self.plat.x2 + self.gap - 1)
            d = tx - self.x
            if abs(d) < 1.5:
                self.plan.pop(0)
                self._set('stand', 0.0)
                return
            dirn = 1 if d > 0 else -1
            self.facing = -dirn if style == 'back' else dirn
            v = WALK_V.get(style, WALK_V['walk']) * S
            mv = min(abs(d), v * dt)
            self.x += dirn * mv
            self.cur_v = v * self.facing * dirn
            self.walk_phase += (mv / S / stride(style)) * (-1 if style == 'back' else 1)
            self._set(style)
        elif step[0] == 'stalk':                  # 마우스 쪽으로 살금살금
            dirn = 1 if cursor[0] > self.x else -1
            tx = clamp(cursor[0] - dirn * 90 * S, self.plat.x1 + self.margin, self.plat.x2 - self.margin)
            if self.cursor_v > 60 * S:            # 들켰다 — 그만두고 갸웃
                self.plan = [('idle', 'curious', 1.5)]
                return
            d = tx - self.x
            if abs(d) < 2:
                self.plan.pop(0)
                return
            self.facing = 1 if d > 0 else -1
            mv = min(abs(d), WALK_V['sneak'] * S * dt)
            self.x += self.facing * mv
            self.walk_phase += mv / S / stride('sneak')
            self._set('sneak')
        elif step[0] == 'pounce':                 # 엉덩이 실룩 → 마우스 앞으로 덮치기
            self.facing = 1 if cursor[0] > self.x else -1
            if self.action != 'hunt':
                self._set('hunt', step[1])
            elif self.at >= self.adur:
                self.plan = [('idle', 'swat', 1.5)]
                tx = clamp(cursor[0] - self.facing * 40 * S, self.plat.x1 + self.margin, self.plat.x2 - self.margin)
                if abs(tx - self.x) > 10 * S:
                    self._launch(tx, self.plat.y)
        elif step[0] == 'spin':                   # 제자리 빙글
            if self.action != 'trot':
                self._set('trot', step[1])
                self.spin_t = 0.0
            self.spin_t += dt
            self.walk_phase += dt * 2.5
            if self.spin_t > 0.35:
                self.facing, self.spin_t = -self.facing, 0.0
            if self.at >= self.adur:
                self.plan.pop(0)
        elif step[0] == 'slip':                   # 가장자리에서 미끄러져 매달림
            self.plan.pop(0)
            p = self.plat
            self.mark_t = 0.5
            self._enter_hang(p, p.x2 - 3 * S if self.facing > 0 else p.x1 + 3 * S, climbdown=True)
        elif step[0] == 'hopdown':                # 아래 발판으로 뛰어내리기
            self.plan.pop(0)
            c = [x for x in self._candidates(plats, direction=step[1]) if x[1].y > self.y + 30 * S]
            if c:
                _, tp, lx = self._pick(c)
                self.plan = [('crouch', (tp, lx), random.uniform(0.25, 0.5), None)]
        elif step[0] == 'climbwall':
            self.plan.pop(0)
            w = self._find_wall(step[1])
            if w:
                self._start_climb(w)
        elif step[0] == 'crouchwall':             # 벽으로 점프해 달라붙기
            w = self._find_wall(step[1])
            if not w:
                self.plan.pop(0)
                return
            self.facing = w['side']
            if self.action != 'crouch':
                self._set('crouch', step[2])
            elif self.at >= self.adur:
                self.plan.pop(0)
                ty = clamp(w['bottom'] - 25 * S, w['top'] + 40 * S, self.y - 20 * S)
                self._launch(w['x'] - w['side'] * 1 * S, ty, ('wall', w))
        elif step[0] == 'peek':
            w = self._find_wall(step[1])
            if self.action != 'peek':
                self._set('peek', step[2])
            if w:
                self.clip = (w['x'] - self.x) * self.facing / S
            if self.at >= self.adur or not w:
                self.plan.pop(0)
                self.clip = -99.0
        elif step[0] == 'face':
            self.facing = step[1]
            self.plan.pop(0)
        elif step[0] == 'idle':
            if self.action != step[1]:
                self._set(step[1], step[2])
            elif self.at >= self.adur:
                self.plan.pop(0)
        elif step[0] == 'crouch':
            tp, lx = step[1]
            self.facing = 1 if lx >= self.x else -1
            if self.action != 'crouch':
                self._set('crouch', step[2])
            elif self.at >= self.adur:
                self.plan.pop(0)
                again = self._support(plats, lx, tp.y, 4 * self.S)
                if again is not None:
                    if len(step) > 3 and step[3] == 'hang':
                        self._launch(lx, again.y + HANG * S, ('hang', again))
                    else:
                        self._launch(lx, again.y)

    def _land(self, p):
        S = self.S
        drop = p.y - self.top_y
        self.mode, self.plat, self.y, self.missing = 'ground', p, p.y, 0.0
        self.vx = self.vy = 0.0
        self.jump = self.hang = self.wall = self.grab = None
        self._set('land', 0.25)
        if self.flee:
            return
        if drop > 260 * S:                        # 높은 데서 쿵 → 몸 털기
            self.plan = [('idle', 'land_hard', 0.5), ('idle', 'shake', 0.9)]
            self.chain = 0
        elif self.confused:                       # 발판이 사라져 떨어졌다 → 어리둥절
            self.plan = [('idle', 'land', 0.25), ('idle', 'confused', 1.6)]
        else:
            self.plan = [('idle', 'land', 0.25)] + self.plan
        self.confused = False

    def _air(self, dt, plats):
        j = self.jump
        j['t'] += dt
        t = min(j['t'], j['T'])
        self.x = j['x0'] + j['vx'] * t
        self.y = j['y0'] + j['vy'] * t + 0.5 * self.g * t * t
        if j['t'] < j['T']:
            return
        land = j['land']
        if land and land[0] == 'wall':            # 벽에 착 달라붙어 오른다
            w = self._find_wall(land[1])
            if w and w['top'] < self.y < w['bottom'] + 10 * self.S:
                self._start_climb(w, self.y)
                return
        elif land and land[0] == 'hang':
            p = self._support(plats, j['tx'], j['ty'] - HANG * self.S, 6 * self.S)
            if p is not None:
                self._enter_hang(p, j['tx'])
                return
        else:
            p = self._support(plats, j['tx'], j['ty'], 6 * self.S)
            if p is not None:
                self.x, self.y = j['tx'], p.y
                self._land(p)
                return
        self.mode, self.vx, self.vy = 'fall', j['vx'] * 0.5, j['vy'] + self.g * j['T']   # 대상이 사라졌다
        self.confused = True
        self._set('fall')

    def _hang(self, dt, plats):
        S, h = self.S, self.hang
        p = None
        for q in plats:                           # 같은 발판(창이 움직였으면 따라간 위치) 다시 찾기
            if q.x1 - 2 <= h['x'] <= q.x2 + 2 and abs(q.y - h['p'].y) <= 8 * S and q.kind != 'ground':
                if p is None or abs(q.y - h['p'].y) < abs(p.y - h['p'].y):
                    p = q
        if p is None:
            self.mode, self.vx, self.vy, self.hang, self.plan = 'fall', 0.0, 0.0, None, []
            self.confused = True
            self._set('fall')
            return
        h['p'] = p
        self.x = h['x']
        step = self.plan[0] if self.plan else ('drop',)
        if step[0] == 'climbdown':                # 가장자리에서 주르륵 → 매달림
            self.cprog = max(0.0, self.cprog - dt / 0.6)
            self._set('climbup')
            self.y = p.y + HANG * S * (1 - ease(self.cprog))
            if self.cprog <= 0:
                self.plan.pop(0)
            return
        if step[0] == 'climbup':                  # 낑낑 기어오르기
            self.cprog = min(1.0, self.cprog + dt / 0.9)
            self._set('climbup')
            self.y = p.y + HANG * S * (1 - ease(self.cprog))
            if self.cprog >= 1:
                self.plan.pop(0)
                self.mode, self.plat, self.y, self.hang, self.cprog = 'ground', p, p.y, None, 0.0
                self._set('stand', 0.0)
            return
        self.y = p.y + HANG * S
        if step[0] == 'hang':
            if self.action != step[1]:
                self._set(step[1], step[2])
            elif self.at >= self.adur:
                self.plan.pop(0)
        else:                                     # drop: 손을 놓는다
            self.plan = self.plan[1:]
            self.mode, self.vx, self.vy, self.hang = 'fall', 0.0, 0.0, None
            self._set('fall')

    def _climb(self, dt, plats):
        S = self.S
        w = self._find_wall(self.wall) if self.wall else None
        if w is None:
            self.mode, self.vx, self.vy, self.wall = 'fall', 0.0, 0.0, None
            self._set('fall')
            return
        self.wall = w
        self.x, self.facing = w['x'] - w['side'] * 1 * S, w['side']
        if self.climb_dir < 0:                    # 오르기
            self._set('climb')
            self.y -= self.climb_v * dt
            self.walk_phase += self.climb_v / S * dt / stride('walk')
            if self.climb_stop is not None and self.y <= self.climb_stop:
                self.climb_dir = 0
                self._set('cling', random.uniform(0.8, 1.8))
            elif self.y - 21 * S <= w['top']:     # 꼭대기 — 창 윗변으로 올라선다
                nx = w['x'] + w['side'] * 12 * S
                p = self._support(plats, nx, w['top'], 8 * S) if w['reach'] else None
                if p is not None:
                    self.x = nx
                    self.top_y = p.y
                    self._land(p)
                else:
                    self.mode, self.vx, self.vy, self.wall = 'fall', -w['side'] * 60 * S, -150 * S, None
                    self._set('fall')
        elif self.climb_dir == 0:                 # 멈칫
            if self.at >= self.adur:
                self.climb_dir = 1
        else:                                     # 주르륵 미끄러지기
            self._set('slide')
            prev = self.y
            self.y += 110 * S * dt
            for p in plats:
                if p.x1 - 12 * S <= self.x <= p.x2 + 12 * S and prev <= p.y <= self.y:
                    self.x = clamp(self.x - w['side'] * 8 * S, p.x1, p.x2)
                    self._land(p)
                    return
            if self.y - 21 * S > w['bottom']:
                self.mode, self.vx, self.vy, self.wall = 'fall', -w['side'] * 30 * S, 0.0, None
                self._set('fall')

    def _fall(self, dt, plats):
        prev = self.y
        self.vy = min(self.vy + self.g * dt, 2600 * self.S)
        self.vx *= 0.99
        self.x += self.vx * dt
        self.y += self.vy * dt
        g = self.grab                             # 떨어지다 발판 끝을 붙잡기
        if g is not None and prev <= g.y + HANG * self.S <= self.y and g.x1 <= self.x <= g.x2:
            self.grab = None
            self.mark_t = 0.5
            self._enter_hang(g, self.x)
            return
        if self.vy > 0:
            best = None
            for p in plats:
                if p.x1 <= self.x <= p.x2 and prev <= p.y + 0.5 <= self.y + 0.5:
                    if best is None or p.y < best.y:
                        best = p
            if best is not None:
                if best.kind != 'ground' and not self.flee and self.grab is None and \
                        self.y - self.top_y > 50 * self.S and random.random() < 0.2:
                    self.grab = best              # 바로 서지 않고 지나치며 붙잡는다
                    return
                if self.grab is None or best.y > self.grab.y:
                    self.y = best.y
                    self._land(best)
                    return
        if self._mon_at(self.x, max(self.y - 1, -1e9)) is None:      # 화면 밖 — 안으로 되돌린다
            ms = self.monitors
            if ms and not any(m['rect'][0] <= self.x < m['rect'][2] for m in ms):
                m = min(ms, key=lambda m: min(abs(self.x - m['rect'][0]), abs(self.x - m['rect'][2])))
                self.x = clamp(self.x, m['rect'][0] + 40, m['rect'][2] - 40)
                self.vx = 0.0
            if ms and self.y > max(m['rect'][3] for m in ms) + 200:
                self.spawned = False                                # 바닥까지 놓쳤다 — 다시 등장

    # ── 자세 ───────────────────────────────────────────────
    def _pose(self, dt, cx, cy):
        ctx = {'dur': self.adur, 'time': self.time, 'walk_phase': self.walk_phase, 'cprog': self.cprog,
               'clip': self.clip if self.action == 'peek' else -99.0}
        if self.mode == 'air' and self.jump:
            ctx['air_k'] = clamp(self.jump['t'] / self.jump['T'] * 1.7 - 0.45)
        pose = action_pose(self.action, self.at, ctx)
        if self.blend_from is not None and self.blend_t < 0.3:
            pose = lerp(self.blend_from, pose, ease(self.blend_t / 0.3))
        # 마우스 쳐다보기
        if self.action in LOOKABLE and pose['eyes'] == 'open':
            hx, hy = self.x + pose['head'][0] * self.S * self.facing, self.y + pose['head'][1] * self.S
            lx, ly = (cx - hx) * self.facing, cy - hy
            n = math.hypot(lx, ly)
            if n < 480 * self.S and n > 1:
                k = clamp(1.5 - n / (320 * self.S))
                pose['look'] = lerp(pose['look'], (lx / n, ly / n), k)
                if lx < 0 and self.action != 'walk':
                    pose['face'] = lerp(pose['face'], -0.2, k)
        # 눈 깜빡임
        if self.time >= self.blink_at:
            self.blink_t, self.blink_at = 0.13, self.time + random.uniform(2.0, 6.0)
        if self.blink_t > 0:
            self.blink_t -= dt
            if pose['eyes'] == 'open':
                pose['eyes'] = 'sleep'
        if self.mark_t > 0:
            pose['mark'] = 1.0
        self.last_pose = pose
        return self.x, self.y, self.facing, pose
