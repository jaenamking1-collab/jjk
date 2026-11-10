// 대기 모드의 얼굴: 어둠 속 고양이. 마우스를 따라보고, 깜빡이고, 졸다가 잠들면 물음표 꼬리만 살랑거린다.
// 얼굴 여섯 가지(눈·코·입)와 수염 여덟 가지를 눈 감은 틈에 바꿔 끼운다.
// 값은 논리 픽셀이나 0…100 단위라 스프링의 멈춤 기준(0.05)이 그대로 맞는다.
use crate::spring::Spring;
use std::time::{Duration, Instant};

pub const EYE_H: f32 = 9.0;

/// 얼굴 하나의 눈·코·입 모양.
pub struct Style {
    pub rx: f32,    // 눈 반너비
    pub ry: f32,    // 눈 반높이
    pub point: f32, // 0 = 둥근 타원 … 1 = 양끝이 뾰족한 아몬드
    pub tilt: f32,  // 바깥 눈꼬리가 올라간 정도
    pub open: f32,  // 평소에 뜨는 정도(졸린 눈은 0.6)
    pub pw: f32,    // 눈동자 반너비
    pub ph: f32,    // 눈동자 반높이
    pub pin: f32,   // 눈동자가 안쪽으로 몰린 정도
    pub dot_nose: bool,
    pub mouth: u8, // 0 ω, 1 넓은 w, 2 한쪽 입꼬리, 3 한 줄 미소
}

const fn st(rx: f32, ry: f32, point: f32, tilt: f32, open: f32, pw: f32, ph: f32, pin: f32, dot_nose: bool, mouth: u8) -> Style {
    Style { rx, ry, point, tilt, open, pw, ph, pin, dot_nose, mouth }
}

// 고른 후보: 01 아몬드, 02 콩, 03 모찌, 05 졸린 눈, 08 초승달, 10 장난꾸러기
pub const STYLES: [Style; 6] = [
    st(7.0, 5.0, 0.55, 1.0, 1.0, 1.2, 3.6, 0.0, false, 0),
    st(4.6, 5.6, 0.0, 0.0, 1.0, 1.7, 3.3, 0.0, true, 0),
    st(7.0, 4.4, 0.15, 0.0, 1.0, 1.4, 2.8, 0.0, false, 0),
    st(7.0, 5.0, 0.3, 0.6, 0.58, 1.2, 3.4, 0.0, false, 2),
    st(7.5, 4.2, 1.0, 1.4, 1.0, 0.9, 3.0, 0.0, false, 1),
    st(5.6, 5.0, 0.0, 0.5, 1.0, 2.3, 2.6, 1.2, false, 3),
];

/// 수염 한쪽 세 가닥: [뿌리 x, 뿌리 y, 끝 x, 끝 y, 위로 휜 정도, 진하기]. 마지막 값은 라임색 정도.
pub type Whisk = ([[f32; 6]; 3], f32);
const NO: [f32; 6] = [21.0, 17.6, 27.0, 17.6, 0.0, 0.0];
pub const WHISKS: [Whisk; 8] = [
    ([[20.0, 16.2, 30.5, 14.6, 0.6, 1.0], [20.0, 18.6, 30.5, 20.0, -0.6, 1.0], NO], 1.0), // 01 부채
    ([[21.5, 16.4, 27.5, 16.2, 0.0, 1.0], [21.5, 19.0, 27.5, 19.3, 0.0, 1.0], NO], 1.0),  // 02 짧은 두 줄
    ([[20.0, 16.0, 29.0, 17.2, 1.6, 1.0], [20.5, 18.8, 28.0, 20.6, 1.4, 1.0], NO], 1.0),  // 03 늘어진 곡선
    ([[21.0, 15.2, 28.0, 13.8, 0.0, 1.0], [21.0, 20.0, 28.0, 21.4, 0.0, 1.0], [21.0, 17.6, 28.5, 17.6, 0.0, 1.0]], 1.0), // 04 세 가닥
    ([[19.0, 16.4, 32.0, 14.2, 0.4, 1.0], [19.0, 18.6, 32.0, 20.6, -0.4, 1.0], NO], 1.0), // 05 길게
    ([[20.0, 17.0, 30.0, 14.4, -1.4, 1.0], [20.0, 19.2, 30.0, 18.6, -1.4, 1.0], NO], 0.0), // 08 끝이 들린 회색
    ([[20.0, 16.6, 30.0, 15.2, 1.5, 1.0], [20.0, 18.8, 30.0, 19.8, -1.5, 1.0], NO], 0.0), // 09 벌어진 회색
    ([[21.0, 16.4, 28.5, 14.2, 0.0, 1.0], [21.0, 19.2, 28.5, 19.2, 0.0, 1.0], NO], 0.0),  // 10 비스듬 + 수평
];
const NATIVE: [usize; 6] = [0, 1, 2, 4, 5, 7]; // 얼굴마다 원래 짝인 수염

pub struct Face {
    pub lid: Spring,  // 눈이 드러난 정도 0…9 (웃는 눈일 땐 솟은 정도)
    pub px: Spring,   // 눈동자 위치(먼저 움직인다)
    pub py: Spring,
    pub ex: Spring,   // 눈 전체 위치(조금 늦게 따라온다)
    pub ey: Spring,
    pub ear: Spring,  // 귀·웃음 0…10
    pub tail: Spring, // 꼬리가 나온 정도 0…100
    pub glow: Spring, // 아래 빛 세기 ×100
    pub mix: Spring,  // 모양 바뀜 0…100 (이전 → 지금)
    pub tw: [Spring; 3], // 꼬리 움직임 섞는 비율 ×100: 좌우 살랑 / 끝만 까딱 / 말았다 풀기
    pub style: usize,
    pub prev_style: usize,
    pub whisk: usize,
    pub prev_whisk: usize,
    pub happy_shape: bool,
    pub asleep: bool,
    pub tail_side: f32, // +1 오른쪽, -1 왼쪽
    swap: bool,
    next_swap: Instant,
    gesture: usize, // 0·1·2 = 위 세 가지, 3 = 쉼
    gesture_until: Instant,
    side_phase: u8, // 0 그대로, 1 숨는 중, 2 숨어서 기다림
    side_at: Instant,
    touched: Instant, // 마지막 직접 조작
    sleep_at: Option<Instant>,
    next_blink: Instant,
    blink_until: Instant,
    second_blink: Option<Instant>,
    slow_until: Instant,
    happy_until: Instant,
    next_look: Instant,
    look: (f32, f32),
    mouse: (f32, f32),
    mouse_moved: Instant,
    rng: u32,
}

fn env_secs(name: &str) -> Option<u64> {
    std::env::var(name).ok().and_then(|v| v.parse().ok())
}

impl Face {
    pub fn new() -> Self {
        let now = Instant::now();
        let rng = (std::process::id() ^ 0x9E37_79B9) | 1;
        let style = rng as usize % STYLES.len();
        Face {
            lid: Spring::new(EYE_H, 320.0, 0.85),
            px: Spring::new(0.0, 300.0, 1.0),
            py: Spring::new(0.0, 300.0, 1.0),
            ex: Spring::new(0.0, 150.0, 1.0),
            ey: Spring::new(0.0, 150.0, 1.0),
            ear: Spring::new(0.0, 360.0, 0.82),
            tail: Spring::new(0.0, 28.0, 1.0),
            glow: Spring::new(100.0, 100.0, 1.0),
            mix: Spring::new(100.0, 60.0, 1.0),
            tw: [Spring::new(100.0, 16.0, 1.0), Spring::new(0.0, 16.0, 1.0), Spring::new(0.0, 16.0, 1.0)],
            style,
            prev_style: style,
            whisk: NATIVE[style],
            prev_whisk: NATIVE[style],
            happy_shape: false,
            asleep: false,
            tail_side: 1.0,
            swap: false,
            next_swap: now + Duration::from_secs(env_secs("NOTCH_CAT_FACE").unwrap_or(60)),
            gesture: 0,
            gesture_until: now + Duration::from_secs(7),
            side_phase: 0,
            side_at: now,
            touched: now,
            sleep_at: None,
            next_blink: now + Duration::from_secs(3),
            blink_until: now,
            second_blink: None,
            slow_until: now,
            happy_until: now,
            next_look: now + Duration::from_secs(12),
            look: (0.0, 0.0),
            mouse: (0.0, 0.0),
            mouse_moved: now,
            rng,
        }
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng % 10_000) as f32 / 10_000.0
    }

    /// 직접 조작(마우스 올림·클릭·가까이 옴): 잠을 깨운다.
    pub fn touch(&mut self) {
        self.touched = Instant::now();
    }

    /// 반가움: 웃는 눈 + 귀.
    pub fn happy(&mut self, ms: u64) {
        self.happy_until = Instant::now() + Duration::from_millis(ms);
    }

    /// 한 번 깜빡(곡이 바뀜).
    pub fn blink(&mut self) {
        self.blink_until = Instant::now() + Duration::from_millis(90);
        self.glow.pos += 45.0;
    }

    /// 느린 눈인사(재생 시작·멈춤).
    pub fn slow_blink(&mut self, ms: u64) {
        self.slow_until = Instant::now() + Duration::from_millis(ms);
    }

    /// 다음 얼굴로(눈을 감은 틈에 바뀐다).
    pub fn next_face(&mut self) {
        self.swap = true;
    }

    /// m: 얼굴 가운데에서 본 마우스 위치. near: 노치 바로 근처. playing: 음악 재생 중(늦게 잔다).
    pub fn tick(&mut self, m: (f32, f32), near: bool, hover: bool, playing: bool) {
        let now = Instant::now();
        if (m.0 - self.mouse.0).abs() + (m.1 - self.mouse.1).abs() > 1.0 {
            self.mouse = m;
            self.mouse_moved = now;
            self.touched = now; // 마우스가 움직이면 어디에 있든 눈을 뜬다
        }
        if near || hover {
            self.touched = now;
        }
        let mut idle = now.duration_since(self.touched).as_secs();
        if std::env::var("NOTCH_CAT_SLEEP").as_deref() == Ok("0") {
            idle = 999; // 시험용: 무조건 재운다
        }
        let (mut drowsy_at, mut sleep_at) = if playing { (20, 40) } else { (8, 15) };
        if let Some(n) = env_secs("NOTCH_CAT_SLEEP") {
            (drowsy_at, sleep_at) = (n / 2, n); // 시험용: 빨리 재운다
        }
        let was_asleep = self.asleep;
        self.asleep = idle >= sleep_at;
        let drowsy = idle >= drowsy_at;
        if self.asleep && !was_asleep {
            self.sleep_at = Some(now);
            self.tail_side = if self.rand() < 0.5 { -1.0 } else { 1.0 };
            self.side_phase = 0;
            self.side_at = now + Duration::from_secs(40 + (self.rand() * 30.0) as u64);
        }
        if was_asleep && !self.asleep {
            self.swap = true; // 자고 일어나면 다른 얼굴
        }

        // ── 얼굴 바꾸기: 1~2분마다, 또는 깨어날 때. 눈을 감은 틈에 바꾼다 ──
        if now >= self.next_swap && !self.asleep {
            self.swap = true;
        }
        if self.swap {
            if !self.happy_shape && !self.asleep && now >= self.blink_until {
                self.blink_until = now + Duration::from_millis(140);
            }
            if self.lid.pos < 2.0 || self.happy_shape {
                self.swap = false;
                self.next_swap = now + Duration::from_secs(env_secs("NOTCH_CAT_FACE").unwrap_or(60 + (self.rand() * 60.0) as u64));
                self.prev_style = self.style;
                self.prev_whisk = self.whisk;
                self.style = (self.style + 1) % STYLES.len();
                self.whisk = (self.whisk + 1) % WHISKS.len(); // 수염은 따로 돌아서 조합이 매번 달라진다
                self.mix.pos = 0.0;
                self.mix.target = 100.0;
            }
        }

        // ── 시선: 마우스가 움직이는 동안은 마우스, 한참 멈춰 있으면 가끔 혼자 두리번 ──
        let (u, v) = if now.duration_since(self.mouse_moved) < Duration::from_secs(4) || near {
            (m.0 / 180.0, m.1 / 120.0)
        } else {
            if now >= self.next_look {
                self.look = if self.look != (0.0, 0.0) { (0.0, 0.0) } else { ((self.rand() - 0.5) * 3.0, 0.0) };
                let wait = if self.look == (0.0, 0.0) { 12_000 + (self.rand() * 10_000.0) as u64 } else { 450 + (self.rand() * 400.0) as u64 };
                self.next_look = now + Duration::from_millis(wait);
            }
            self.look
        };
        let s = (1.0 + u * u + v * v).sqrt(); // 가장자리에서 부드럽게 멈춘다
        let (dx, dy) = if self.asleep { (0.0, 0.0) } else { (u / s, v / s) };
        self.px.target = 2.2 * dx;
        self.py.target = 1.2 * dy;
        self.ex.target = 0.8 * dx;
        self.ey.target = 0.45 * dy + if self.asleep { 0.8 } else if drowsy { 0.4 } else { 0.0 };

        // ── 눈꺼풀 ──
        let want_happy = now < self.happy_until && !self.asleep;
        if want_happy != self.happy_shape {
            // 열린 눈 ↔ 웃는 눈은 겹쳐 바꾸지 않고, 잠깐 감은 틈에 바꾼다
            self.lid.tune(1600.0, 1.0);
            self.lid.target = 0.0;
            if self.lid.pos < 1.0 {
                self.happy_shape = want_happy;
            }
        } else if self.happy_shape {
            self.lid.tune(360.0, 0.82);
            self.lid.target = EYE_H;
        } else if self.asleep {
            self.lid.tune(30.0, 1.0);
            self.lid.target = 0.0;
        } else if now < self.blink_until {
            self.lid.tune(2400.0, 1.0);
            self.lid.target = 0.0;
        } else if now < self.slow_until {
            self.lid.tune(180.0, 1.0);
            self.lid.target = 2.0;
        } else if drowsy {
            self.lid.tune(45.0, 1.0);
            self.lid.target = 4.5;
        } else {
            let just_blinked = now.duration_since(self.blink_until) < Duration::from_millis(250);
            if just_blinked { self.lid.tune(1600.0, 1.0) } else { self.lid.tune(320.0, 0.85) }
            self.lid.target = if hover { EYE_H + 1.0 } else { EYE_H };
            if self.second_blink.is_some_and(|t| now >= t) {
                self.second_blink = None;
                self.blink_until = now + Duration::from_millis(90);
            } else if now >= self.next_blink {
                self.blink_until = now + Duration::from_millis(90);
                self.next_blink = now + Duration::from_millis(4500 + (self.rand() * 4000.0) as u64);
                if self.rand() < 0.08 {
                    self.second_blink = Some(now + Duration::from_millis(220));
                }
            }
        }
        self.ear.target = if want_happy { 10.0 } else { 0.0 };
        self.glow.target = if self.asleep { 40.0 } else if want_happy { 140.0 } else { 100.0 };

        // ── 꼬리: 눈이 먼저 잠들고, 조금 뒤에 옆으로 슬그머니. 가끔 숨었다가 반대편에서 다시 나온다 ──
        let slept = self.asleep && self.sleep_at.is_some_and(|t| now.duration_since(t) > Duration::from_millis(850));
        if slept && self.side_phase == 0 && now >= self.side_at {
            self.side_phase = 1;
        }
        if self.side_phase == 1 && self.tail.pos < 0.5 {
            self.side_phase = 2;
            self.side_at = now + Duration::from_secs(2);
        }
        if self.side_phase == 2 && now >= self.side_at {
            self.side_phase = 0;
            self.tail_side = -self.tail_side;
            self.side_at = now + Duration::from_secs(40 + (self.rand() * 30.0) as u64);
        }
        let out = slept && self.side_phase == 0;
        self.tail.tune(if out { 28.0 } else { 42.0 }, 1.0);
        self.tail.target = if out { 100.0 } else { 0.0 };
        // 움직임을 6~10초씩 바꿔 가며 섞고, 사이사이 2~4초 쉰다
        if now >= self.gesture_until {
            let mut g = (self.rand() * 3.0) as usize;
            if g >= self.gesture {
                g += 1; // 같은 동작을 연달아 고르지 않는다
            }
            self.gesture = g.min(3);
            let ms = if self.gesture == 3 { 2000.0 + self.rand() * 2000.0 } else { 6000.0 + self.rand() * 4000.0 };
            self.gesture_until = now + Duration::from_millis(ms as u64);
        }
        for (i, w) in self.tw.iter_mut().enumerate() {
            w.target = if i == self.gesture { 100.0 } else { 0.0 };
        }
    }

    fn all(&mut self) -> [&mut Spring; 12] {
        let [a, b, c] = &mut self.tw;
        [&mut self.lid, &mut self.px, &mut self.py, &mut self.ex, &mut self.ey, &mut self.ear, &mut self.tail, &mut self.glow, &mut self.mix, a, b, c]
    }

    pub fn step(&mut self, dt: f32) {
        for s in self.all() {
            s.step(dt);
        }
    }

    pub fn settled(&self) -> bool {
        [&self.lid, &self.px, &self.py, &self.ex, &self.ey, &self.ear, &self.tail, &self.glow, &self.mix, &self.tw[0], &self.tw[1], &self.tw[2]].iter().all(|s| s.settled())
    }
}
