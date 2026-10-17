pub fn squash(d: i32) -> i32 {
    const T: [i32; 33] = [
        1, 2, 3, 6, 10, 16, 27, 45, 73, 120, 194, 310, 488, 747, 1101, 1546, 2047, 2549, 2994, 3348, 3607, 3785, 3901, 3975, 4022, 4050, 4068, 4079, 4085, 4089, 4092, 4093, 4094,
    ];
    if d > 2047 {
        return 4095;
    }
    if d < -2047 {
        return 1;
    }
    let w = d & 127;
    let i = ((d >> 7) + 16) as usize;
    (T[i] * (128 - w) + T[i + 1] * w + 64) >> 7
}

pub struct Stretch {
    t: Vec<i16>,
}

impl Stretch {
    pub fn new() -> Self {
        let mut t = vec![0i16; 4096];
        let mut next = 0usize;
        for x in -2047..=2047 {
            let v = squash(x) as usize;
            for e in t.iter_mut().take(v + 1).skip(next) {
                *e = x as i16;
            }
            next = next.max(v + 1);
        }
        for e in t.iter_mut().skip(next) {
            *e = 2047;
        }
        Stretch { t }
    }

    pub fn get(&self, p1: i32) -> i32 {
        self.t[p1 as usize] as i32
    }
}

pub struct Mixer {
    n: usize,
    w: Vec<i32>,
    lr: i64,
}

impl Mixer {
    pub fn new(sets: usize, n: usize, lr: i64) -> Self {
        Mixer { n, w: vec![65536 * 3 / 10; sets * n], lr }
    }

    pub fn predict(&self, set: usize, x: &[i32]) -> i32 {
        let w = &self.w[set * self.n..(set + 1) * self.n];
        let dot: i64 = x.iter().zip(w).map(|(&a, &b)| a as i64 * b as i64).sum();
        squash((dot >> 16).clamp(-2047, 2047) as i32)
    }

    pub fn update(&mut self, set: usize, x: &[i32], p1: i32, bit: bool) {
        let err = ((bit as i64) << 12) - p1 as i64;
        let w = &mut self.w[set * self.n..(set + 1) * self.n];
        for (wi, &xi) in w.iter_mut().zip(x) {
            *wi = (*wi as i64 + ((xi as i64 * err * self.lr) >> 14)).clamp(-(1 << 22), 1 << 22) as i32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stretch_undoes_squash() {
        let s = Stretch::new();
        for d in [-1500, -700, -100, 0, 100, 700, 1500] {
            assert!((s.get(squash(d)) - d).abs() < 40, "{d} -> {} -> {}", squash(d), s.get(squash(d)));
        }
        assert!(squash(0) > 2000 && squash(0) < 2100);
    }

    #[test]
    fn mixer_learns_to_trust_the_better_input() {
        let s = Stretch::new();
        let mut m = Mixer::new(1, 3, 6);
        let mut seed = 9u32;
        let mut cost_early = 0.0;
        let mut cost_late = 0.0;
        for i in 0..20000 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let bit = (seed >> 8) % 10 < 9;
            let good = s.get(if bit { 3600 } else { 500 });
            let bad = s.get(2048);
            let x = [good, bad, 256];
            let p1 = m.predict(0, &x);
            let p = if bit { p1 } else { 4096 - p1 } as f64 / 4096.0;
            if i < 1000 {
                cost_early -= p.log2();
            } else if i >= 19000 {
                cost_late -= p.log2();
            }
            m.update(0, &x, p1, bit);
        }
        assert!(cost_late < cost_early * 0.8, "early {cost_early:.0} late {cost_late:.0}");
    }
}
