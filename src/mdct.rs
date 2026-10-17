use std::cell::RefCell;

use crate::fft::Fft;

pub struct Mdct {
    pub n: usize,
    pub m: usize,
    fft: Fft,
    pre: Vec<(f64, f64)>,
    post: Vec<(f64, f64)>,
    work: RefCell<(Vec<f64>, Vec<f64>, Vec<f64>)>,
    pub window: Vec<f64>,
}

impl Mdct {
    pub fn new(n: usize) -> Self {
        let m = n / 2;
        let pi = std::f64::consts::PI;
        let rot = |a: f64| (a.cos(), a.sin());
        Mdct {
            n,
            m,
            fft: Fft::new(m / 2),
            pre: (0..m / 2).map(|k| rot(-pi * k as f64 / m as f64)).collect(),
            post: (0..m / 2).map(|k| rot(-pi * (4 * k + 1) as f64 / (4 * m) as f64)).collect(),
            work: RefCell::new((vec![0.0; m], vec![0.0; m / 2], vec![0.0; m / 2])),
            window: (0..n).map(|i| (pi * (i as f64 + 0.5) / n as f64).sin()).collect(),
        }
    }

    fn dct4(&self, u: &mut [f64], re: &mut [f64], im: &mut [f64]) {
        let m = self.m;
        for k in 0..m / 2 {
            let (a, b) = (u[2 * k], u[m - 1 - 2 * k]);
            let (c, s) = self.pre[k];
            re[k] = a * c - b * s;
            im[k] = a * s + b * c;
        }
        self.fft.forward(re, im);
        for k in 0..m / 2 {
            let (c, s) = self.post[k];
            let (a, b) = (re[k] * c - im[k] * s, re[k] * s + im[k] * c);
            u[2 * k] = a;
            u[m - 1 - 2 * k] = -b;
        }
    }

    pub fn forward(&self, x: &[f64], out: &mut [f64]) {
        let m = self.m;
        let h = m / 2;
        let mut w = self.work.borrow_mut();
        let (u, re, im) = &mut *w;
        for i in 0..h {
            u[i] = -x[3 * h - 1 - i] - x[3 * h + i];
            u[h + i] = x[i] - x[m - 1 - i];
        }
        self.dct4(u, re, im);
        out[..m].copy_from_slice(u);
    }

    pub fn inverse(&self, x: &[f64], out: &mut [f64]) {
        let m = self.m;
        let h = m / 2;
        let mut w = self.work.borrow_mut();
        let (u, re, im) = &mut *w;
        u.copy_from_slice(&x[..m]);
        self.dct4(u, re, im);
        let k = 2.0 / m as f64;
        for i in 0..h {
            out[i] = u[h + i] * k;
            out[h + i] = -u[m - 1 - i] * k;
            out[m + i] = -u[h - 1 - i] * k;
            out[3 * h + i] = -u[i] * k;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_definition() {
        for n in [32, 256] {
            let t = Mdct::new(n);
            let x: Vec<f64> = (0..n).map(|i| ((i * 7 + 3) % 11) as f64 - 5.0).collect();
            let mut fast = vec![0.0; n / 2];
            t.forward(&x, &mut fast);
            for k in 0..n / 2 {
                let slow: f64 = (0..n)
                    .map(|i| x[i] * (2.0 * std::f64::consts::PI / n as f64 * (i as f64 + 0.5 + n as f64 / 4.0) * (k as f64 + 0.5)).cos())
                    .sum();
                assert!((fast[k] - slow).abs() < 1e-9, "n={n} k={k}: {} vs {slow}", fast[k]);
            }
            let mut back = vec![0.0; n];
            t.inverse(&fast, &mut back);
            for i in 0..n {
                let slow: f64 = (0..n / 2)
                    .map(|k| fast[k] * (2.0 * std::f64::consts::PI / n as f64 * (i as f64 + 0.5 + n as f64 / 4.0) * (k as f64 + 0.5)).cos())
                    .sum::<f64>()
                    * 4.0
                    / n as f64;
                assert!((back[i] - slow).abs() < 1e-9, "n={n} i={i}: {} vs {slow}", back[i]);
            }
        }
    }

    #[test]
    fn overlapping_frames_reconstruct_the_signal() {
        let n = 64;
        let t = Mdct::new(n);
        let m = n / 2;
        let len = m * 10;
        let x: Vec<f64> = (0..len).map(|i| (i as f64 * 0.37).sin() + ((i * 13) % 7) as f64 * 0.1).collect();
        let mut y = vec![0.0; len + n];
        let (mut buf, mut c, mut back) = (vec![0.0; n], vec![0.0; m], vec![0.0; n]);
        for f in 0..(len / m + 1) {
            for i in 0..n {
                let j = (f * m + i) as isize - m as isize;
                buf[i] = if j >= 0 && (j as usize) < len { x[j as usize] } else { 0.0 } * t.window[i];
            }
            t.forward(&buf, &mut c);
            t.inverse(&c, &mut back);
            for i in 0..n {
                y[f * m + i] += back[i] * t.window[i];
            }
        }
        for i in 0..len {
            assert!((y[i + m] - x[i]).abs() < 1e-9, "sample {i}: {} vs {}", y[i + m], x[i]);
        }
    }
}
