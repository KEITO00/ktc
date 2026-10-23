use crate::bands::Layout;
use crate::coding::{code_frame, fill_kind, step, Coding, Frame, Group, ICC, IID_MAX, KP_MAX, KP_STEPS};
use crate::format::{chunk_seed, info, Info, CHUNK, FILL_KINDS};
use crate::rc;
use crate::tns;
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

fn env_gains(env: &[i32]) -> Vec<f64> {
    let g: Vec<f64> = env.iter().map(|&e| level_energy(e)).collect();
    let mean = g.iter().sum::<f64>() / g.len() as f64;
    g.iter().map(|v| v / mean).collect()
}

struct Joint {
    k: f64,
    el: f64,
    er: f64,
    a: f64,
    b: f64,
    sign: f64,
}

impl Joint {
    fn new(g: &Group, p: usize, energy: f64) -> Self {
        let r = level_energy(g.iid[p].clamp(-IID_MAX, IID_MAX));
        let rho = ICC[g.icc[p].clamp(0, ICC.len() as i32 - 1) as usize];
        Joint {
            k: g.kp[p].clamp(-KP_MAX, KP_MAX) as f64 / KP_STEPS as f64,
            el: 2.0 * energy * r / (1.0 + r),
            er: 2.0 * energy / (1.0 + r),
            a: ((1.0 + rho) / 2.0).sqrt(),
            b: ((1.0 - rho) / 2.0).sqrt(),
            sign: if g.flip[p] { -1.0 } else { 1.0 },
        }
    }

    fn fill(&self, l: &mut [f64], r: &mut [f64], pos: &[usize], scale: f64, rng: &mut u32, tmp: &mut [Vec<f64>; 2]) {
        if pos.is_empty() || scale * (self.el + self.er) <= 0.0 {
            return;
        }
        for t in tmp.iter_mut() {
            t.clear();
            let mut sum = 0.0;
            for _ in pos {
                *rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
                let v = (*rng >> 8) as f64 / 8_388_608.0 - 1.0;
                t.push(v);
                sum += v * v;
            }
            let g = if sum > 0.0 { (1.0 / sum).sqrt() } else { 0.0 };
            t.iter_mut().for_each(|v| *v *= g);
        }
        let (sl, sr) = ((scale * self.el).sqrt(), self.sign * (scale * self.er).sqrt());
        for (i, &k) in pos.iter().enumerate() {
            l[k] = sl * (self.a * tmp[0][i] + self.b * tmp[1][i]);
            r[k] = sr * (self.a * tmp[0][i] - self.b * tmp[1][i]);
        }
    }
}

fn band_energy(g: &Group, b: usize) -> f64 {
    if g.zero[b][0] && !g.noise[b][0] { 0.0 } else { level_energy(g.level[b][0]) }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
fn reconstruct(l: &Layout, g: &Group, chans: usize, off: [f64; 2], fill: [f64; FILL_KINDS], gain: &[f64], rng: &mut u32, out: &mut [Vec<f64>], band_fill: bool) {
    let m = l.m;
    // 長い窓で TNS をかけないフレームは、混合帯域の雑音を短い窓で別に作る (stream)。
    // TNS をかけるフレームは、雑音もここで作り、TNS で時間の形を付ける
    let stream = m == M && !tns_on_mixed(l, g);
    let mut pos = Vec::new();
    let mut tmp = [Vec::new(), Vec::new()];
    for (b, &(lo, hi)) in l.bands.iter().enumerate() {
        let mixed = b >= l.mixed_from;
        if chans == 2 && mixed {
            let jt = Joint::new(g, l.parent[b], band_energy(g, b));
            let d = step(g.sf[b][0]);
            let deq = |q: i32| if q == 0 { 0.0 } else { q.signum() as f64 * (q.abs() as f64 + off[(q.abs() >= 2) as usize]) * d };
            let (left, right) = out.split_at_mut(1);
            for j in 0..g.rows {
                pos.clear();
                for k in j * m + lo..j * m + hi {
                    let q = g.q[0][k];
                    if q == 0 {
                        left[0][k] = 0.0;
                        right[0][k] = 0.0;
                        pos.push(k);
                    } else {
                        let mm = deq(q);
                        let ss = jt.k * mm + deq(g.q[1][k]);
                        left[0][k] = (mm + ss) * std::f64::consts::FRAC_1_SQRT_2;
                        right[0][k] = jt.sign * (mm - ss) * std::f64::consts::FRAC_1_SQRT_2;
                    }
                }
                if !stream {
                    jt.fill(&mut left[0], &mut right[0], &pos, pos.len() as f64 * gain[j], rng, &mut tmp);
                }
            }
            continue;
        }
        for ch in 0..chans {
            let per = if mixed && stream {
                0.0
            } else if !g.zero[b][ch] && (mixed || band_fill) {
                level_energy(g.level[b][ch])
            } else if !g.zero[b][ch] {
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
                let gj = if mixed { gain[j] } else { 1.0 };
                fill_noise(o, &pos, pos.len() as f64 * per * gj, rng);
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
    for (j, row) in g.tns.iter().enumerate() {
        for (t, &(a, z)) in row.iter().zip(&l.tns_ranges).filter(|(t, _)| !t.is_empty()) {
            let f = tns::lpc(t);
            for o in out.iter_mut().take(chans) {
                tns::synthesize(&mut o[j * m + a..j * m + z], &f);
            }
        }
    }
}

/// 長い窓のフレームで、混合帯域にかかる範囲に TNS をかけるか (かけるなら、混合帯域の雑音も長い窓の中で作る)
pub fn tns_on_mixed(l: &Layout, g: &Group) -> bool {
    let from = l.bands.get(l.mixed_from).map_or(l.m, |b| b.0);
    g.tns[0].iter().zip(&l.tns_ranges).any(|(t, &(_, z))| !t.is_empty() && z > from)
}

fn stream(l: &Layout, g: &Group, chans: usize, gain: &[f64], rng: &mut u32, out: &mut [Vec<Vec<f64>>]) {
    let mut pos = Vec::new();
    let mut tmp = [Vec::new(), Vec::new()];
    let zeros = |b: usize, ch: usize, lo: usize, hi: usize| {
        if g.zero[b][ch] {
            if g.noise[b][ch] { hi - lo } else { 0 }
        } else {
            g.q[ch][lo..hi].iter().filter(|&&v| v == 0).count()
        }
    };
    for b in l.mixed_from..l.bands.len() {
        let (lo, hi) = l.bands[b];
        let (a, e) = (lo.div_ceil(SHORTS), hi.div_ceil(SHORTS));
        if a >= e {
            continue;
        }
        pos.clear();
        pos.extend(a..e);
        if chans == 2 {
            let nz = zeros(b, 0, lo, hi);
            if nz == 0 {
                continue;
            }
            let jt = Joint::new(g, l.parent[b], band_energy(g, b));
            let (left, right) = out.split_at_mut(1);
            for j in 0..SHORTS {
                jt.fill(&mut left[0][j], &mut right[0][j], &pos, nz as f64 * gain[j] / (SHORTS * SHORTS) as f64, rng, &mut tmp);
            }
            continue;
        }
        let nz = zeros(b, 0, lo, hi);
        if nz == 0 {
            continue;
        }
        let total = nz as f64 * level_energy(g.level[b][0]) / (SHORTS * SHORTS) as f64;
        for (j, o) in out[0].iter_mut().enumerate() {
            fill_noise(o, &pos, total * gain[j], rng);
        }
    }
}

pub fn decode_chunk(data: &[u8], info: &Info, c: usize) -> Result<(isize, Vec<Vec<f32>>), String> {
    let count = info.len.div_ceil(M) + 1;
    let (a, b) = (c * CHUNK, ((c + 1) * CHUNK).min(count));
    let range = info.chunks.get(c).ok_or_else(|| format!("Chunk {c} does not exist"))?;
    let mut dec = rc::Decoder::new(&data[range.clone()]);
    let mut cd = Coding::new(&info.ll, &info.ls, info.chans, info.version >= 3);
    let mut r = Renderer::new(&info.ll, &info.ls, info.chans, b - a, info.off, info.fill, chunk_seed(c));
    let mut f = Frame { kind: Kind::Long, groups: Vec::new(), env: [0; SHORTS] };
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
    noise: Vec<Vec<Vec<f64>>>,
    tmp: Vec<Vec<f64>>,
    /// 実験用: 混合帯域より下の係数がある帯域でも、0 の所を帯域ごとの雑音のレベルで埋める (データには記録しない)
    band_fill: bool,
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
            noise: vec![vec![vec![0.0; MS]; SHORTS]; chans],
            tmp: vec![vec![0.0; M]; chans],
            band_fill: false,
        }
    }

    /// 実験用: 混合帯域より下でも、0 の所を帯域ごとの雑音のレベルで埋める
    pub fn with_band_fill(mut self, on: bool) -> Self {
        self.band_fill = on;
        self
    }

    pub fn frame(&mut self, fi: usize, f: &Frame) {
        let mut out = std::mem::take(&mut self.out);
        let mut span: Vec<&mut [f64]> = out.iter_mut().map(|c| &mut c[fi * M..fi * M + N]).collect();
        self.render(f, &mut span);
        drop(span);
        self.out = out;
    }

    fn render(&mut self, f: &Frame, out: &mut [&mut [f64]]) {
        if f.kind == Kind::Short {
            let mut j0 = 0;
            for g in &f.groups {
                let gain = env_gains(&f.env[j0..j0 + g.rows]);
                reconstruct(self.ls, g, self.chans, self.off, self.fill, &gain, &mut self.rng, &mut self.tmp, self.band_fill);
                for ch in 0..self.chans {
                    for r in 0..g.rows {
                        self.coef_s[ch][j0 + r].copy_from_slice(&self.tmp[ch][r * MS..(r + 1) * MS]);
                    }
                }
                j0 += g.rows;
            }
            for ch in 0..self.chans {
                self.tf.synthesize(&self.coef_s[ch], Kind::Short, out[ch]);
            }
        } else {
            reconstruct(self.ll, &f.groups[0], self.chans, self.off, self.fill, &[1.0], &mut self.rng, &mut self.coef_l, self.band_fill);
            for ch in 0..self.chans {
                self.tf.synthesize(std::slice::from_ref(&self.coef_l[ch]), f.kind, out[ch]);
            }
            if self.ll.mixed_from < self.ll.bands.len() && !tns_on_mixed(self.ll, &f.groups[0]) {
                for w in self.noise.iter_mut().flatten() {
                    w.fill(0.0);
                }
                stream(self.ll, &f.groups[0], self.chans, &env_gains(&f.env), &mut self.rng, &mut self.noise);
                for ch in 0..self.chans {
                    self.tf.synthesize(&self.noise[ch], Kind::Short, out[ch]);
                }
            }
        }
    }

    pub fn into_f32(self) -> Vec<Vec<f32>> {
        self.out.iter().map(|c| c.iter().map(|&v| v as f32).collect()).collect()
    }
}

pub struct Stream<'a> {
    data: &'a [u8],
    info: &'a Info,
    count: usize,
    g: usize,
    dec: Option<rc::Decoder<'a>>,
    cd: Option<Coding>,
    r: Option<Renderer<'a>>,
    f: Frame,
    ring: Vec<Vec<f64>>,
    carry: Vec<Vec<f32>>,
}

impl<'a> Stream<'a> {
    pub fn new(data: &'a [u8], info: &'a Info) -> Self {
        Self::from_chunk(data, info, 0)
    }

    pub fn from_chunk(data: &'a [u8], info: &'a Info, c: usize) -> Self {
        Stream {
            data,
            info,
            count: info.len.div_ceil(M) + 1,
            g: c * CHUNK,
            dec: None,
            cd: None,
            r: None,
            f: Frame { kind: Kind::Long, groups: Vec::new(), env: [0; SHORTS] },
            ring: vec![vec![0.0; N]; info.chans],
            carry: vec![vec![0.0; M]; info.chans],
        }
    }

    pub fn next(&mut self, out: &mut [Vec<f32>]) -> Result<Option<isize>, String> {
        if self.g >= self.count {
            return Ok(None);
        }
        let first = self.g % CHUNK == 0;
        if first {
            let c = self.g / CHUNK;
            let range = self.info.chunks.get(c).ok_or_else(|| format!("Chunk {c} does not exist"))?;
            self.dec = Some(rc::Decoder::new(&self.data[range.clone()]));
            self.cd = Some(Coding::new(&self.info.ll, &self.info.ls, self.info.chans, self.info.version >= 3));
            self.r = Some(Renderer::new(&self.info.ll, &self.info.ls, self.info.chans, 0, self.info.off, self.info.fill, chunk_seed(c)));
            self.f = Frame { kind: Kind::Long, groups: Vec::new(), env: [0; SHORTS] };
            for (cr, rg) in self.carry.iter_mut().zip(self.ring.iter_mut()) {
                for (c, v) in cr.iter_mut().zip(rg.iter_mut()) {
                    *c = *v as f32;
                    *v = 0.0;
                }
            }
        }
        let (dec, cd, r) = (self.dec.as_mut().unwrap(), self.cd.as_mut().unwrap(), self.r.as_mut().unwrap());
        code_frame(dec, cd, &self.info.ll, &self.info.ls, &mut self.f);
        let mut span: Vec<&mut [f64]> = self.ring.iter_mut().map(|c| &mut c[..]).collect();
        r.render(&self.f, &mut span);
        for ((o, rg), cr) in out.iter_mut().zip(self.ring.iter_mut()).zip(&self.carry) {
            for i in 0..M {
                o[i] = if first { cr[i] + rg[i] as f32 } else { rg[i] as f32 };
            }
            rg.copy_within(M..N, 0);
            rg[M..].fill(0.0);
        }
        let start = (self.g * M) as isize - M as isize;
        self.g += 1;
        Ok(Some(start))
    }
}
