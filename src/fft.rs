pub struct Fft {
    n: usize,
    rev: Vec<usize>,
    tw: Vec<(f64, f64)>,
}

impl Fft {
    pub fn new(n: usize) -> Self {
        assert!(n.is_power_of_two() && n >= 2, "FFT の長さは 2 の累乗");
        let bits = n.trailing_zeros();
        let rev = (0..n).map(|i| i.reverse_bits() >> (usize::BITS - bits)).collect();
        let tw = (0..n / 2)
            .map(|k| {
                let a = -2.0 * std::f64::consts::PI * k as f64 / n as f64;
                (a.cos(), a.sin())
            })
            .collect();
        Fft { n, rev, tw }
    }

    pub fn len(&self) -> usize {
        self.n
    }

    pub fn forward(&self, re: &mut [f64], im: &mut [f64]) {
        let n = self.n;
        for i in 0..n {
            let j = self.rev[i];
            if j > i {
                re.swap(i, j);
                im.swap(i, j);
            }
        }
        let mut size = 2;
        while size <= n {
            let half = size / 2;
            let step = n / size;
            for start in (0..n).step_by(size) {
                for k in 0..half {
                    let (wr, wi) = self.tw[k * step];
                    let a = start + k;
                    let b = a + half;
                    let tr = re[b] * wr - im[b] * wi;
                    let ti = re[b] * wi + im[b] * wr;
                    re[b] = re[a] - tr;
                    im[b] = im[a] - ti;
                    re[a] += tr;
                    im[a] += ti;
                }
            }
            size *= 2;
        }
    }

    pub fn inverse(&self, re: &mut [f64], im: &mut [f64]) {
        for v in im.iter_mut() {
            *v = -*v;
        }
        self.forward(re, im);
        let k = 1.0 / self.n as f64;
        for i in 0..self.n {
            re[i] *= k;
            im[i] = -im[i] * k;
        }
    }

    pub fn power(&self, x: &[f64], re: &mut Vec<f64>, im: &mut Vec<f64>, out: &mut Vec<f64>) {
        re.clear();
        re.extend_from_slice(x);
        re.resize(self.n, 0.0);
        im.clear();
        im.resize(self.n, 0.0);
        self.forward(re, im);
        out.clear();
        out.extend((0..=self.n / 2).map(|k| re[k] * re[k] + im[k] * im[k]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_lands_in_its_bin_and_round_trips() {
        let n = 64;
        let f = Fft::new(n);
        let x: Vec<f64> = (0..n).map(|i| (2.0 * std::f64::consts::PI * 5.0 * i as f64 / n as f64).cos()).collect();
        let (mut re, mut im) = (x.clone(), vec![0.0; n]);
        f.forward(&mut re, &mut im);
        let mag: Vec<f64> = (0..n).map(|k| (re[k] * re[k] + im[k] * im[k]).sqrt()).collect();
        assert!((mag[5] - n as f64 / 2.0).abs() < 1e-9);
        assert!(mag[4] < 1e-9 && mag[6] < 1e-9);
        f.inverse(&mut re, &mut im);
        for i in 0..n {
            assert!((re[i] - x[i]).abs() < 1e-12);
        }
    }
}
