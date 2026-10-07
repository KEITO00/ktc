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

pub struct Layout {
    pub m: usize,
    pub bands: Vec<(usize, usize)>,
    pub(crate) class: Vec<usize>,
    pub low_hz: Vec<f64>,
}

impl Layout {
    pub fn new(sr: u32, n: usize) -> Self {
        let edges = edges(sr, n);
        let m = n / 2;
        let nb = edges.len();
        let bands: Vec<(usize, usize)> = edges.iter().enumerate().map(|(i, &(a, b))| (a - 1, if i + 1 == nb { m } else { b - 1 })).collect();
        let class = (0..nb).map(|b| (b * CLASSES / nb).min(CLASSES - 1)).collect();
        let low_hz = bands.iter().map(|&(a, _)| a as f64 * sr as f64 / n as f64).collect();
        Layout { m, bands, class, low_hz }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands_cover_the_coefficients_in_order() {
        for sr in [29000, 44100, 48000] {
            for n in [256, 2048] {
                let l = Layout::new(sr, n);
                assert_eq!(l.bands.first().unwrap().0, 0);
                assert_eq!(l.bands.last().unwrap().1, n / 2);
                for w in l.bands.windows(2) {
                    assert!(w[0].0 < w[0].1);
                    assert_eq!(w[0].1, w[1].0);
                }
            }
            let nb = Layout::new(sr, 2048).bands.len();
            assert!(nb > 30 && nb < 60, "{nb} bands at {sr}");
        }
    }
}
