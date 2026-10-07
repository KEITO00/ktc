use crate::mdct::Mdct;

pub const N: usize = 2048;
pub const M: usize = N / 2;
pub const NS: usize = 256;
pub const MS: usize = NS / 2;
pub const SHORTS: usize = M / MS;
pub const SHORT_OFF: usize = M / 2 - MS / 2;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Long,
    Start,
    Short,
    Stop,
}

impl Kind {
    pub(crate) fn index(self) -> usize {
        match self {
            Kind::Long => 0,
            Kind::Start => 1,
            Kind::Short => 2,
            Kind::Stop => 3,
        }
    }
}

struct Windows {
    long: Vec<f64>,
    start: Vec<f64>,
    stop: Vec<f64>,
    short: Vec<f64>,
}

impl Windows {
    fn new(long: &[f64], short: &[f64]) -> Self {
        let start = (0..N)
            .map(|i| {
                if i < M {
                    long[i]
                } else if i < M + SHORT_OFF {
                    1.0
                } else if i < M + SHORT_OFF + MS {
                    short[MS + i - M - SHORT_OFF]
                } else {
                    0.0
                }
            })
            .collect();
        let stop = (0..N)
            .map(|i| {
                if i < SHORT_OFF {
                    0.0
                } else if i < SHORT_OFF + MS {
                    short[i - SHORT_OFF]
                } else if i < M {
                    1.0
                } else {
                    long[i]
                }
            })
            .collect();
        Windows { long: long.to_vec(), start, stop, short: short.to_vec() }
    }

    fn long_kind(&self, k: Kind) -> &[f64] {
        match k {
            Kind::Start => &self.start,
            Kind::Stop => &self.stop,
            _ => &self.long,
        }
    }
}

pub struct Transforms {
    w: Windows,
    long: Mdct,
    short: Mdct,
}

impl Transforms {
    pub fn new() -> Self {
        let long = Mdct::new(N);
        let short = Mdct::new(NS);
        Transforms { w: Windows::new(&long.window, &short.window), long, short }
    }

    pub fn analyze(&self, x: &[f32], fi: usize, kind: Kind, out: &mut Vec<Vec<f64>>) {
        let start = (fi * M) as isize - M as isize;
        let at = |j: isize| if j >= 0 && (j as usize) < x.len() { x[j as usize] as f64 } else { 0.0 };
        if kind == Kind::Short {
            out.resize(SHORTS, vec![0.0; MS]);
            let mut buf = vec![0.0; NS];
            for (j, o) in out.iter_mut().enumerate() {
                o.resize(MS, 0.0);
                let s0 = start + (SHORT_OFF + j * MS) as isize;
                for i in 0..NS {
                    buf[i] = at(s0 + i as isize) * self.w.short[i];
                }
                self.short.forward(&buf, o);
            }
        } else {
            out.resize(1, vec![0.0; M]);
            out[0].resize(M, 0.0);
            let win = self.w.long_kind(kind);
            let buf: Vec<f64> = (0..N).map(|i| at(start + i as isize) * win[i]).collect();
            self.long.forward(&buf, &mut out[0]);
        }
    }

    pub fn synthesize(&self, coef: &[Vec<f64>], kind: Kind, out: &mut [f64]) {
        if kind == Kind::Short {
            let mut back = vec![0.0; NS];
            for (j, c) in coef.iter().enumerate() {
                self.short.inverse(c, &mut back);
                for i in 0..NS {
                    out[SHORT_OFF + j * MS + i] += back[i] * self.w.short[i];
                }
            }
        } else {
            let mut back = vec![0.0; N];
            self.long.inverse(&coef[0], &mut back);
            let win = self.w.long_kind(kind);
            for i in 0..N {
                out[i] += back[i] * win[i];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_window_sequence_reconstructs_the_signal() {
        let tf = Transforms::new();
        let mut seed = 21u32;
        let len = M * 14;
        let x: Vec<f32> = (0..len)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                (seed >> 8) as f32 / 16777216.0 - 0.5
            })
            .collect();
        use Kind::*;
        let kinds = [Long, Start, Short, Short, Stop, Long, Start, Short, Stop, Start, Short, Stop, Long, Start, Short];
        let mut out = vec![0.0; (kinds.len() + 1) * M];
        let mut coef = Vec::new();
        for (fi, &k) in kinds.iter().enumerate() {
            tf.analyze(&x, fi, k, &mut coef);
            tf.synthesize(&coef, k, &mut out[fi * M..fi * M + N]);
        }
        for i in 0..len - M {
            assert!((out[i + M] - x[i] as f64).abs() < 1e-9, "sample {i}: {} vs {}", out[i + M], x[i]);
        }
    }
}
