# 고양이 행동·물리. OS 와 무관하다 — 발판 목록·모니터·마우스 위치만 받아서 위치와 자세를 낸다.
import math
import random
from cat_render import action_pose, lerp, clamp, ease

# 이름: (가중치, (최소초, 최대초))
IDLE = {
    'groom': (10, (3.5, 6.0)),       # 옆구리 그루밍
    'scratch': (7, (1.8, 3.0)),      # 뒷발로 귀 긁기
    'lickpaw': (8, (2.5, 4.5)),      # 앞발 핥기
    'wash': (7, (3.8, 5.5)),         # 세수
    'loaf': (8, (6.0, 14.0)),        # 엎드리기
    'lookdown': (9, (3.0, 5.5)),     # 아래 내려다보기 (높은 곳 가장자리에서만)
    'yawn': (6, (2.2, 2.8)),         # 하품
    'stretch': (6, (2.8, 3.4)),      # 기지개
    'lookaround': (8, (3.0, 6.0)),   # 두리번
    'sleep': (4, (12.0, 30.0)),      # 자기
    'knead': (5, (3.0, 5.0)),        # 꾹꾹이
    'tailflick': (6, (2.5, 4.0)),    # 꼬리 탁탁
    'sit': (8, (2.0, 5.0)),          # 그냥 앉아 있기
}
LOOKABLE = {'sit', 'watch', 'stand', 'walk', 'tailflick', 'loaf', 'knead', 'lookdown'}


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
        self.walk_v = 42 * S
        self.run_v = 330 * S
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
        self.mode = 'ground'              # ground / air / fall
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

    def _launch(self, tx, ty):
        S, g = self.S, self.g
        dx = tx - self.x
        apex = min(self.y, ty) - max(18 * S, 0.13 * abs(dx)) - 6 * S
        h1, h2 = self.y - apex, ty - apex
        T = math.sqrt(2 * h1 / g) + math.sqrt(2 * h2 / g)
        self.jump = dict(x0=self.x, y0=self.y, vx=dx / T, vy=-math.sqrt(2 * g * h1), T=T, t=0.0, tx=tx, ty=ty)
        if abs(dx) > 1:
            self.facing = 1 if dx > 0 else -1
        self.mode, self.plat = 'air', None
        self._set('air')

    def _plan_jump(self, plats, far=False):
        c = self._candidates(plats, far=far)
        if not c:
            return False
        _, p, lx = self._pick(c)
        d = 1 if lx > self.x else -1
        if self.plat:
            room = 30 * self.S
            take = clamp(lx - d * room, self.plat.x1 + self.margin, self.plat.x2 - self.margin)
        else:
            take = self.x
        self.plan = [('walk', take), ('crouch', (p, lx), random.uniform(0.45, 0.9))]
        return True

    # ── 결정 ───────────────────────────────────────────────
    def _decide(self, plats):
        p = self.plat
        width = (p.x2 - p.x1) if p else 0
        r = random.random() * 100
        if r < 22 and width > 70 * self.S:
            lo, hi = p.x1 + self.margin, p.x2 - self.margin
            tx = random.uniform(lo, hi)
            if abs(tx - self.x) < 30 * self.S:
                tx = lo if self.x - lo > hi - self.x else hi
            self.plan = [('walk', tx)]
            return
        if r < 52 and self._plan_jump(plats, far=random.random() < 0.12):
            return
        names = list(IDLE)
        weights = [IDLE[n][0] for n in names]
        name = random.choices(names, weights)[0]
        dur = random.uniform(*IDLE[name][1])
        if name == 'lookdown':
            if not p or p.kind == 'ground':
                name = 'lookaround'
            else:                                 # 가까운 가장자리까지 걸어가서 내려다본다
                left = self.x - p.x1 < p.x2 - self.x
                ex = p.x1 + self.margin if left else p.x2 - self.margin
                self.plan = [('walk', ex), ('face', -1 if left else 1), ('idle', 'lookdown', dur)]
                return
        self.plan = [('idle', name, dur)]

    # ── 도망 ───────────────────────────────────────────────
    def _start_flee(self, cx):
        self.flee, self.flee_t, self.mark_t = True, 0.0, 0.7
        self.flee_dir = 1 if self.x >= cx else -1
        self.plan = []
        if self.mode == 'ground':                  # 깜짝 놀라 폴짝
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
        if not self.spawned:
            self.spawn(plats)

        # 마우스
        cx, cy = world['cursor']
        if self.cursor_prev:
            v = math.hypot(cx - self.cursor_prev[0], cy - self.cursor_prev[1]) / max(dt, 1e-3)
            self.cursor_v = self.cursor_v * 0.7 + v * 0.3
        self.cursor_prev = (cx, cy)
        hx, hy = self.x, self.y - 22 * self.S
        md = math.hypot(cx - hx, cy - hy)
        if not self.flee and self.mode != 'air' and md < self.flee_r and \
                (self.cursor_v > 40 * self.S or md < self.flee_r * 0.55):
            self._start_flee(cx)
        if self.flee:
            self.flee_t += dt
            if md > self.flee_r * 2.6 and self.flee_t > 0.8:
                self.flee = False
                self.plan = [('face', 1 if cx > self.x else -1), ('idle', 'watch', random.uniform(1.5, 3.0))]
        self.mark_t -= dt

        # 창이 움직이면 같이 움직인다
        if self.mode == 'ground' and self.plat is not None and self.plat.owner in refs:
            nr = refs[self.plat.owner]
            if self.ref is not None and self.ref[0] == self.plat.owner:
                self.x += nr[0] - self.ref[1][0]
                self.y += nr[1] - self.ref[1][1]
            self.ref = (self.plat.owner, nr)
        else:
            self.ref = None

        if self.mode == 'ground':
            self._ground(dt, plats, (cx, cy))
        elif self.mode == 'air':
            self._air(dt, plats)
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
                self.vx = self.facing * (self.run_v if self.action == 'run' else
                                         self.walk_v if self.action == 'walk' else 0) * 0.8
                self.plat = None
                self._set('fall')
                return
        if self.plat is None:
            return
        if self.flee:
            self._flee_step(dt, plats, cursor)
            self.walk_phase += self.run_v / self.S * dt / 60.0
            return

        if not self.plan and (self.action in IDLE or self.action in ('watch', 'land', 'stand')) \
                and self.at < self.adur:
            return
        if not self.plan:
            self._decide(plats)
            if not self.plan:
                return
        step = self.plan[0]
        if step[0] == 'walk':
            tx = clamp(step[1], self.plat.x1 - self.gap + 1, self.plat.x2 + self.gap - 1)
            d = tx - self.x
            if abs(d) < 1.5:
                self.plan.pop(0)
                self._set('stand', 0.0)
                return
            self.facing = 1 if d > 0 else -1
            mv = min(abs(d), self.walk_v * dt)
            self.x += self.facing * mv
            self.walk_phase += mv / self.S / 20.0
            self._set('walk')
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
                    self._launch(lx, again.y)

    def _land(self, p):
        self.mode, self.plat, self.y, self.missing = 'ground', p, p.y, 0.0
        self.vx = self.vy = 0.0
        self.jump = None
        self._set('land', 0.25)
        if not self.flee:
            self.plan = [('idle', 'land', 0.25)] + self.plan

    def _air(self, dt, plats):
        j = self.jump
        j['t'] += dt
        t = min(j['t'], j['T'])
        self.x = j['x0'] + j['vx'] * t
        self.y = j['y0'] + j['vy'] * t + 0.5 * self.g * t * t
        if j['t'] >= j['T']:
            p = self._support(plats, j['tx'], j['ty'], 6 * self.S)
            if p is not None:
                self.x, self.y = j['tx'], p.y
                self._land(p)
            else:                                     # 발판이 사라졌다 — 그냥 떨어진다
                self.mode, self.vx, self.vy = 'fall', j['vx'] * 0.5, j['vy'] + self.g * j['T']
                self._set('fall')

    def _fall(self, dt, plats):
        prev = self.y
        self.vy = min(self.vy + self.g * dt, 2600 * self.S)
        self.vx *= 0.99
        self.x += self.vx * dt
        self.y += self.vy * dt
        if self.vy > 0:
            best = None
            for p in plats:
                if p.x1 <= self.x <= p.x2 and prev <= p.y + 0.5 <= self.y + 0.5:
                    if best is None or p.y < best.y:
                        best = p
            if best is not None:
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
        ctx = {'dur': self.adur, 'time': self.time, 'walk_phase': self.walk_phase}
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
