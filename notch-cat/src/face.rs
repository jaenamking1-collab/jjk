// 대기 모드의 얼굴: 고양이 눈 한 쌍. 깜빡이고, 두리번거리고, 마우스를 따라보고, 졸고, 반가워한다.
use crate::spring::Spring;
use std::time::{Duration, Instant};

pub struct Face {
    pub open: Spring,  // 눈 뜬 정도(1 보통, 0 감음)
    pub gx: Spring,    // 시선 -1…1
    pub gy: Spring,
    pub happy: Spring, // 0…1 웃는 눈
    pub asleep: bool,
    active: Instant,
    next_blink: Instant,
    blink_until: Instant,
    next_look: Instant,
    happy_until: Instant,
    rng: u32,
}

impl Face {
    pub fn new() -> Self {
        let now = Instant::now();
        Face {
            open: Spring::new(1.0, 600.0, 0.9),
            gx: Spring::new(0.0, 120.0, 0.75),
            gy: Spring::new(0.0, 120.0, 0.75),
            happy: Spring::new(0.0, 300.0, 0.7),
            asleep: false,
            active: now,
            next_blink: now + Duration::from_secs(2),
            blink_until: now,
            next_look: now + Duration::from_secs(3),
            happy_until: now,
            rng: now.elapsed().subsec_nanos() | 0x9E37_79B9,
        }
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng % 10_000) as f32 / 10_000.0
    }

    /// 반가운 일(곡 바뀜·재생 시작·마우스 올림)이 생겼다.
    pub fn poke(&mut self) {
        self.happy_until = Instant::now() + Duration::from_millis(1300);
        self.active = Instant::now();
    }

    /// near: 마우스가 가까이 있으면 노치 가운데에서 본 위치. wide: 마우스가 올라와 있다.
    pub fn tick(&mut self, near: Option<(f32, f32)>, wide: bool) {
        let now = Instant::now();
        match near {
            Some((dx, dy)) => {
                self.gx.target = (dx / 260.0).clamp(-1.0, 1.0);
                self.gy.target = (dy / 160.0).clamp(-0.4, 1.0);
                if dx.abs() < 220.0 && dy < 160.0 {
                    self.active = now; // 바로 근처면 깬다
                }
            }
            None if now >= self.next_look => {
                let r = self.rand();
                self.gx.target = if r < 0.45 { 0.0 } else { self.rand() * 2.0 - 1.0 };
                self.gy.target = 0.0;
                self.next_look = now + Duration::from_millis(1800 + (self.rand() * 4500.0) as u64);
            }
            None => {}
        }
        self.asleep = now.duration_since(self.active) > Duration::from_secs(40);
        self.happy.target = if now < self.happy_until { 1.0 } else { 0.0 };
        if self.asleep {
            self.open.target = 0.12;
            self.gx.target = 0.0;
            self.gy.target = 0.0;
        } else if now < self.blink_until {
            self.open.target = 0.05;
        } else {
            self.open.target = if wide { 1.25 } else { 1.0 };
            if now >= self.next_blink {
                self.blink_until = now + Duration::from_millis(110);
                self.next_blink = now + Duration::from_millis(2400 + (self.rand() * 4000.0) as u64);
            }
        }
    }

    pub fn step(&mut self, dt: f32) {
        self.open.step(dt);
        self.gx.step(dt);
        self.gy.step(dt);
        self.happy.step(dt);
    }

    pub fn settled(&self) -> bool {
        self.open.settled() && self.gx.settled() && self.gy.settled() && self.happy.settled()
    }
}
