use crate::bands::Layout;
use crate::coding::{code_frame, fill_kind, step, Coding, Frame, Group};
use crate::format::{chunk_seed, info, Info, CHUNK, FILL_KINDS};
use crate::rc;
use crate::transform::{Kind, Transforms, M, MS, N, SHORTS};
use crate::Audio;

fn level_energy(l: i32) -> f64 {
    2f64.powf(l as f64 / 2.0)
}

fn fill_noise(vals: &mut [f64], pos: &[usize], energy: f64, rng: &mut u32) {
    if pos.is_empty() || energy <= 0.0 {
        return;
    }
    let mut sum = 0.0;
    for &k in pos {
        *rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
        let v = (*rng >> 8) as f64 / 8_388_608.0 - 1.0;
        vals[k] = v;
        sum += v * v;
    }
    let g = if sum > 0.0 { (energy / sum).sqrt() } else { 0.0 };
    for &k in pos {
        vals[k] *= g;
    }
}

fn reconstruct(l: &Layout, g: &Group, chans: usize, off: [f64; 2], fill: [f64; FILL_KINDS], rng: &mut u32, out: &mut [Vec<f64>]) {
    let m = l.m;
    let mut pos = Vec::new();
    for (b, &(lo, hi)) in l.bands.iter().enumerate() {
        for ch in 0..chans {
            let per = if !g.zero[b][ch] {
                let d = step(g.sf[b][ch]);
                fill[fill_kind(g, ch, lo, hi)] * d * d
            } else if g.noise[b][ch] {
                level_energy(g.level[b][ch])
            } else {
                0.0
            };
            let d = step(g.sf[b][ch]);
            for j in 0..g.rows {
                let o = &mut out[ch][j * m + lo..j * m + hi];
                pos.clear();
                for k in lo..hi {
                    let q = g.q[ch][j * m + k];
                    o[k - lo] = if q == 0 { 0.0 } else { q.signum() as f64 * (q.abs() as f64 + off[(q.abs() >= 2) as usize]) * d };
                    if q == 0 {
                        pos.push(k - lo);
                    }
                }
                fill_noise(o, &pos, pos.len() as f64 * per, rng);
            }
        }
        if chans == 2 && g.ms[b] {
            for j in 0..g.rows {
                for k in j * m + lo..j * m + hi {
                    let (mm, ss) = (out[0][k], out[1][k]);
                    out[0][k] = (mm + ss) * std::f64::consts::FRAC_1_SQRT_2;
                    out[1][k] = (mm - ss) * std::f64::consts::FRAC_1_SQRT_2;
                }
            }
        }
    }
}

pub fn decode_chunk(data: &[u8], info: &Info, c: usize) -> Result<(isize, Vec<Vec<f32>>), String> {
    let count = info.len.div_ceil(M) + 1;
    let (a, b) = (c * CHUNK, ((c + 1) * CHUNK).min(count));
    let range = info.chunks.get(c).ok_or("その塊はありません")?;
    let mut dec = rc::Decoder::new(&data[range.clone()]);
    let mut cd = Coding::new(&info.ll, &info.ls, info.chans);
    let mut r = Renderer::new(&info.ll, &info.ls, info.chans, b - a, info.off, info.fill, chunk_seed(c));
    let mut f = Frame { kind: Kind::Long, groups: Vec::new() };
    for fi in 0..b - a {
        code_frame(&mut dec, &mut cd, &info.ll, &info.ls, &mut f);
        r.frame(fi, &f);
    }
    Ok(((a * M) as isize - M as isize, r.into_f32()))
}

pub fn add_chunk(out: &mut [Vec<f32>], start: isize, pcm: &[Vec<f32>]) {
    for (o, p) in out.iter_mut().zip(pcm) {
        let len = o.len() as isize;
        for (i, &v) in p.iter().enumerate() {
            let at = start + i as isize;
            if (0..len).contains(&at) {
                o[at as usize] += v;
            }
        }
    }
}

pub fn decode(data: &[u8]) -> Result<Audio, String> {
    let info = info(data)?;
    let mut out = vec![vec![0.0f32; info.len]; info.chans];
    for c in 0..info.chunks() {
        let (start, pcm) = decode_chunk(data, &info, c)?;
        add_chunk(&mut out, start, &pcm);
    }
    Ok(Audio { sr: info.sr, ch: out })
}

pub struct Renderer<'a> {
    ll: &'a Layout,
    ls: &'a Layout,
    tf: Transforms,
    chans: usize,
    off: [f64; 2],
    fill: [f64; FILL_KINDS],
    rng: u32,
    out: Vec<Vec<f64>>,
    coef_l: Vec<Vec<f64>>,
    coef_s: Vec<Vec<Vec<f64>>>,
    tmp: Vec<Vec<f64>>,
}

impl<'a> Renderer<'a> {
    pub fn new(ll: &'a Layout, ls: &'a Layout, chans: usize, count: usize, off: [f64; 2], fill: [f64; FILL_KINDS], seed: u32) -> Self {
        Renderer {
            ll,
            ls,
            tf: Transforms::new(),
            chans,
            off,
            fill,
            rng: seed,
            out: vec![vec![0.0; (count + 1) * M]; chans],
            coef_l: vec![vec![0.0; M]; chans],
            coef_s: vec![vec![vec![0.0; MS]; SHORTS]; chans],
            tmp: vec![vec![0.0; M]; chans],
        }
    }

    pub fn frame(&mut self, fi: usize, f: &Frame) {
        let span = fi * M..fi * M + N;
        if f.kind == Kind::Short {
            let mut j0 = 0;
            for g in &f.groups {
                reconstruct(self.ls, g, self.chans, self.off, self.fill, &mut self.rng, &mut self.tmp);
                for ch in 0..self.chans {
                    for r in 0..g.rows {
                        self.coef_s[ch][j0 + r].copy_from_slice(&self.tmp[ch][r * MS..(r + 1) * MS]);
                    }
                }
                j0 += g.rows;
            }
            for ch in 0..self.chans {
                self.tf.synthesize(&self.coef_s[ch], Kind::Short, &mut self.out[ch][span.clone()]);
            }
        } else {
            reconstruct(self.ll, &f.groups[0], self.chans, self.off, self.fill, &mut self.rng, &mut self.coef_l);
            for ch in 0..self.chans {
                self.tf.synthesize(std::slice::from_ref(&self.coef_l[ch]), f.kind, &mut self.out[ch][span.clone()]);
            }
        }
    }

    pub fn into_f32(self) -> Vec<Vec<f32>> {
        self.out.iter().map(|c| c.iter().map(|&v| v as f32).collect()).collect()
    }
}
