pub fn bark(f: f64) -> f64 {
    13.0 * (0.00076 * f).atan() + 3.5 * (f / 7500.0).powi(2).atan()
}

pub fn edges(sr: u32, n: usize) -> Vec<(usize, usize)> {
    let df = sr as f64 / n as f64;
    let mut bands = Vec::new();
    let mut cur: Option<(i64, usize)> = None;
    for k in 1..=n / 2 {
        let idx = (bark(k as f64 * df) * 2.0).floor() as i64;
        match cur {
            Some((i, _)) if i == idx => {}
            Some((_, s)) => {
                bands.push((s, k));
                cur = Some((idx, k));
            }
            None => cur = Some((idx, k)),
        }
    }
    if let Some((_, s)) = cur {
        bands.push((s, n / 2 + 1));
    }
    bands
}

pub(crate) const CLASSES: usize = 8;

const SPLIT_WIDTH: usize = 32;

pub struct Layout {
    pub m: usize,
    pub bands: Vec<(usize, usize)>,
    pub(crate) class: Vec<usize>,
    pub low_hz: Vec<f64>,
    pub parent: Vec<usize>,
    pub parent_width: Vec<usize>,
    pub parents: usize,
    pub mixed_from: usize,
}

impl Layout {
    pub fn new(sr: u32, n: usize, mixed_hz: f64) -> Self {
        let edges = edges(sr, n);
        let m = n / 2;
        let ne = edges.len();
        let hz = |a: usize| a as f64 * sr as f64 / n as f64;
        let max_w = (SPLIT_WIDTH * m / 1024).max(2);
        let (mut bands, mut parent, mut parent_width) = (Vec::new(), Vec::new(), Vec::new());
        for (i, &(a, b)) in edges.iter().enumerate() {
            let (lo, hi) = (a - 1, if i + 1 == ne { m } else { b - 1 });
            let parts = if mixed_hz > 0.0 && hz(lo) >= mixed_hz { (hi - lo).div_ceil(max_w) } else { 1 };
            for p in 0..parts {
                bands.push((lo + (hi - lo) * p / parts, lo + (hi - lo) * (p + 1) / parts));
                parent.push(i);
                parent_width.push(hi - lo);
            }
        }
        let nb = bands.len();
        let class = (0..nb).map(|b| (b * CLASSES / nb).min(CLASSES - 1)).collect();
        let low_hz: Vec<f64> = bands.iter().map(|&(a, _)| hz(a)).collect();
        let mixed_from = if mixed_hz > 0.0 { low_hz.iter().position(|&f| f >= mixed_hz).unwrap_or(nb) } else { nb };
        Layout { m, bands, class, low_hz, parent, parent_width, parents: ne, mixed_from }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands_cover_the_coefficients_in_order() {
        for sr in [29000, 44100, 48000] {
            for n in [256, 2048] {
                for mixed in [0.0, 8000.0] {
                    let l = Layout::new(sr, n, mixed);
                    assert_eq!(l.bands.first().unwrap().0, 0);
                    assert_eq!(l.bands.last().unwrap().1, n / 2);
                    for w in l.bands.windows(2) {
                        assert!(w[0].0 < w[0].1);
                        assert_eq!(w[0].1, w[1].0);
                    }
                }
            }
            let nb = Layout::new(sr, 2048, 0.0).bands.len();
            assert!(nb > 30 && nb < 60, "{nb} bands at {sr}");
            let l = Layout::new(sr, 2048, 8000.0);
            assert!(l.bands[l.mixed_from..].iter().all(|&(a, b)| b - a <= 32));
            assert!(l.low_hz[l.mixed_from] >= 8000.0 && l.low_hz[l.mixed_from - 1] < 8000.0);
        }
    }
}
