// 감쇠 용수철. 모든 크기·위치 변화는 이걸로 움직인다(설계 2절).

#[derive(Clone, Copy)]
pub struct Spring {
    pub pos: f32,
    pub vel: f32,
    pub target: f32,
    stiffness: f32,
    damping: f32,
}

impl Spring {
    /// ratio: 1.0 = 넘침 없음, 0.8 근처면 살짝 튕긴다.
    pub fn new(v: f32, stiffness: f32, ratio: f32) -> Self {
        Spring { pos: v, vel: 0.0, target: v, stiffness, damping: 2.0 * stiffness.sqrt() * ratio }
    }

    /// 세기·감쇠를 바꾼다(위치·속도는 그대로 이어진다).
    pub fn tune(&mut self, stiffness: f32, ratio: f32) {
        self.stiffness = stiffness;
        self.damping = 2.0 * stiffness.sqrt() * ratio;
    }

    /// dt 초만큼 진행. 고정 작은 걸음으로 나눠 프레임이 들쭉날쭉해도 같은 곡선을 탄다.
    pub fn step(&mut self, mut dt: f32) {
        dt = dt.min(0.1);
        const H: f32 = 1.0 / 480.0;
        while dt > 0.0 {
            let h = dt.min(H);
            let a = -self.stiffness * (self.pos - self.target) - self.damping * self.vel;
            self.vel += a * h;
            self.pos += self.vel * h;
            dt -= h;
        }
        if self.settled() {
            self.pos = self.target;
            self.vel = 0.0;
        }
    }

    pub fn settled(&self) -> bool {
        (self.pos - self.target).abs() < 0.05 && self.vel.abs() < 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settles_within_a_second_with_small_overshoot() {
        let mut s = Spring::new(0.0, 260.0, 0.82);
        s.target = 100.0;
        let mut max = 0.0f32;
        let mut t = 0.0;
        while t < 1.0 {
            s.step(1.0 / 60.0);
            max = max.max(s.pos);
            t += 1.0 / 60.0;
        }
        assert!(s.settled(), "1초 안에 멈춰야 한다: {}", s.pos);
        assert!(max > 100.0 && max < 106.0, "살짝만 넘쳐야 한다: {max}");
    }

    #[test]
    fn frame_rate_does_not_change_curve() {
        let mut a = Spring::new(0.0, 260.0, 0.82);
        let mut b = a;
        a.target = 50.0;
        b.target = 50.0;
        for _ in 0..12 { a.step(1.0 / 120.0); }
        for _ in 0..3 { b.step(1.0 / 30.0); }
        assert!((a.pos - b.pos).abs() < 0.5, "{} vs {}", a.pos, b.pos);
    }
}
