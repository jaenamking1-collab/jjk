// 대기 모드의 얼굴: 어둠 속 고양이 눈. 마우스를 따라보고, 깜빡이고, 졸다가 잠들면 꼬리만 살랑거린다.
// 값은 전부 논리 픽셀 단위(눈 뜬 높이 0…9 등)라 스프링의 멈춤 기준(0.05)이 그대로 맞는다.
use crate::spring::Spring;
use std::time::{Duration, Instant};

pub const EYE_H: f32 = 9.0;
pub const TAIL_OUT: f32 = 18.0;

pub struct Face {
    pub lid: Spring,  // 눈이 드러난 높이 0…9 (웃는 눈일 땐 솟은 정도)
    pub px: Spring,   // 눈동자 위치(먼저 움직인다)
    pub py: Spring,
    pub ex: Spring,   // 눈 전체 위치(조금 늦게 따라온다)
    pub ey: Spring,
    pub ear: Spring,  // 귀 0…10
    pub tail: Spring, // 꼬리가 밖으로 나온 길이 0…18
    pub glow: Spring, // 아래 빛 세기 ×100
    pub happy_shape: bool,
    pub asleep: bool,
    pub tail_side: f32, // +1 오른쪽, -1 왼쪽
    touched: Instant,   // 마지막 직접 조작
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

impl Face {
    pub fn new() -> Self {
        let now = Instant::now();
        Face {
            lid: Spring::new(EYE_H, 320.0, 0.85),
            px: Spring::new(0.0, 300.0, 1.0),
            py: Spring::new(0.0, 300.0, 1.0),
            ex: Spring::new(0.0, 150.0, 1.0),
            ey: Spring::new(0.0, 150.0, 1.0),
            ear: Spring::new(0.0, 360.0, 0.82),
            tail: Spring::new(0.0, 28.0, 1.0),
            glow: Spring::new(100.0, 100.0, 1.0),
            happy_shape: false,
            asleep: false,
            tail_side: 1.0,
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
            rng: (std::process::id() ^ 0x9E37_79B9) | 1,
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

    /// m: 얼굴 가운데에서 본 마우스 위치. near: 노치 바로 근처. playing: 음악 재생 중(늦게 잔다).
    pub fn tick(&mut self, m: (f32, f32), near: bool, hover: bool, playing: bool) {
        let now = Instant::now();
        if (m.0 - self.mouse.0).abs() + (m.1 - self.mouse.1).abs() > 1.0 {
            self.mouse = m;
            self.mouse_moved = now;
        }
        if near || hover {
            self.touched = now;
        }
        let idle = now.duration_since(self.touched).as_secs();
        let (mut drowsy_at, mut sleep_at) = if playing { (120, 240) } else { (45, 90) };
        if let Some(n) = std::env::var("NOTCH_CAT_SLEEP").ok().and_then(|v| v.parse::<u64>().ok()) {
            (drowsy_at, sleep_at) = (n / 2, n); // 시험용: 빨리 재운다
        }
        let was_asleep = self.asleep;
        self.asleep = idle >= sleep_at;
        let drowsy = idle >= drowsy_at;
        if self.asleep && !was_asleep {
            self.sleep_at = Some(now);
            self.tail_side = if self.rand() < 0.5 { -1.0 } else { 1.0 };
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
        self.ey.target = 0.45 * dy + if self.asleep { 1.0 } else if drowsy { 0.5 } else { 0.0 };

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
            self.lid.target = 1.5;
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
            if let Some(t) = self.second_blink.filter(|t| now >= *t) {
                let _ = t;
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

        // ── 꼬리: 눈이 먼저 잠들고, 조금 뒤에 옆으로 슬그머니 ──
        let out = self.asleep && self.sleep_at.map(|t| now.duration_since(t) > Duration::from_millis(800)).unwrap_or(false);
        self.tail.target = if out { TAIL_OUT } else { 0.0 };
    }

    fn all(&mut self) -> [&mut Spring; 8] {
        [&mut self.lid, &mut self.px, &mut self.py, &mut self.ex, &mut self.ey, &mut self.ear, &mut self.tail, &mut self.glow]
    }

    pub fn step(&mut self, dt: f32) {
        for s in self.all() {
            s.step(dt);
        }
    }

    pub fn settled(&self) -> bool {
        [&self.lid, &self.px, &self.py, &self.ex, &self.ey, &self.ear, &self.tail, &self.glow].iter().all(|s| s.settled())
    }
}
