use crate::bands::{Layout, CLASSES};
use crate::format::{CHUNK, FILL_KINDS};
use crate::mix::{Mixer, Stretch};
use crate::rc::{self, Coder, Prob};
use crate::transform::{Kind, M, MS, SHORTS};

pub const PART_SIDE: usize = 0;
pub const PART_SF: usize = 1;
pub const PART_ZERO: usize = 2;
pub const PART_MAG: usize = 3;
pub const PART_SIGN: usize = 4;

const LV: usize = 5;
const NCTX: usize = LV * LV + 2 * LV * LV * LV;
const MAG_STEPS: usize = 14;
const PAT: usize = 4 * 3 * 4 * 2;
const BLV: usize = 8;
const RUN: usize = 4 * 6;
const NM: usize = 5;

struct Model {
    ms: [[Prob; 2]; CLASSES],
    zero: [[Prob; 2]; CLASSES],
    noise: [[Prob; 2]; CLASSES],
    level: [[Prob; 40]; 3],
    sf: [[Prob; 40]; 3],
    nz: [[Prob; NCTX]; CLASSES],
    gt1: [[Prob; NCTX]; CLASSES],
    nz_band: Vec<[Prob; LV * LV]>,
    gt1_band: Vec<[Prob; LV * LV]>,
    nz_pat: [[Prob; PAT]; CLASSES],
    gt1_pat: [[Prob; PAT]; CLASSES],
    nz_blv: [[Prob; BLV * LV]; CLASSES],
    gt1_blv: [[Prob; BLV * LV]; CLASSES],
    nz_run: [[Prob; RUN]; CLASSES],
    gt1_run: [[Prob; RUN]; CLASSES],
    mx: Mixer,
    steps: [[[[Prob; MAG_STEPS]; 3 * LV - 2]; 3]; CLASSES],
    esc: [[Prob; 40]; CLASSES],
}

impl Model {
    fn new(nb: usize) -> Box<Self> {
        Box::new(Model {
            ms: [[Prob::INIT; 2]; CLASSES],
            zero: [[Prob::INIT; 2]; CLASSES],
            noise: [[Prob::INIT; 2]; CLASSES],
            level: [[Prob::INIT; 40]; 3],
            sf: [[Prob::INIT; 40]; 3],
            nz: [[Prob::INIT; NCTX]; CLASSES],
            gt1: [[Prob::INIT; NCTX]; CLASSES],
            nz_band: vec![[Prob::INIT; LV * LV]; nb],
            gt1_band: vec![[Prob::INIT; LV * LV]; nb],
            nz_pat: [[Prob::INIT; PAT]; CLASSES],
            gt1_pat: [[Prob::INIT; PAT]; CLASSES],
            nz_blv: [[Prob::INIT; BLV * LV]; CLASSES],
            gt1_blv: [[Prob::INIT; BLV * LV]; CLASSES],
            nz_run: [[Prob::INIT; RUN]; CLASSES],
            gt1_run: [[Prob::INIT; RUN]; CLASSES],
            mx: Mixer::new(2 * CLASSES, NM + 1, MIX_LR),
            steps: [[[[Prob::INIT; MAG_STEPS]; 3 * LV - 2]; 3]; CLASSES],
            esc: [[Prob::INIT; 40]; CLASSES],
        })
    }
}

const MIX_LR: i64 = 6;

fn mix_bit<C: Coder>(c: &mut C, mx: &mut Mixer, sq: &Stretch, set: usize, ps: [&mut Prob; NM], b: bool) -> bool {
    let mut x = [256; NM + 1];
    for (xi, p) in x.iter_mut().zip(&ps) {
        *xi = sq.get(4096 - p.get() as i32);
    }
    let p1 = mx.predict(set, &x);
    let bit = c.code((4096 - p1).clamp(1, 4095) as u32, b);
    for p in ps {
        p.update(bit);
    }
    mx.update(set, &x, p1, bit);
    bit
}

#[derive(Clone)]
struct ChanState {
    prev_q: Vec<i32>,
    prev_sf: Vec<i32>,
    prev_level: Vec<i32>,
    prev_zero: Vec<bool>,
    prev_noise: Vec<bool>,
}

impl ChanState {
    fn new(l: &Layout) -> Self {
        let nb = l.bands.len();
        ChanState { prev_q: vec![0; l.m], prev_sf: vec![0; nb], prev_level: vec![0; nb], prev_zero: vec![true; nb], prev_noise: vec![false; nb] }
    }
}

fn ratio_q8(d: i32) -> u32 {
    const FRAC: [u32; 4] = [256, 304, 362, 431];
    let d = d.clamp(-16, 16);
    let whole = d.div_euclid(4);
    let part = FRAC[d.rem_euclid(4) as usize];
    if whole >= 0 { part << whole } else { part >> -whole }
}

fn median(a: i32, b: i32, c: i32) -> i32 {
    a.max(b).min(a.min(b).max(c))
}

#[derive(Clone)]
pub struct Group {
    pub m: usize,
    pub rows: usize,
    pub ms: Vec<bool>,
    pub zero: Vec<[bool; 2]>,
    pub noise: Vec<[bool; 2]>,
    pub level: Vec<[i32; 2]>,
    pub sf: Vec<[i32; 2]>,
    pub q: [Vec<i32>; 2],
}

impl Group {
    pub fn new(l: &Layout, rows: usize) -> Self {
        let nb = l.bands.len();
        Group {
            m: l.m,
            rows,
            ms: vec![false; nb],
            zero: vec![[true; 2]; nb],
            noise: vec![[false; 2]; nb],
            level: vec![[0; 2]; nb],
            sf: vec![[0; 2]; nb],
            q: [vec![0; l.m * rows], vec![0; l.m * rows]],
        }
    }
}

pub struct Frame {
    pub kind: Kind,
    pub groups: Vec<Group>,
}

pub fn step(sf: i32) -> f64 {
    2f64.powf(sf as f64 / 4.0)
}

pub fn fill_kind(g: &Group, ch: usize, lo: usize, hi: usize) -> usize {
    let top = (0..g.rows).flat_map(|j| &g.q[ch][j * g.m + lo..j * g.m + hi]).map(|v| v.unsigned_abs()).max().unwrap_or(1);
    (top.clamp(1, FILL_KINDS as u32) - 1) as usize
}

fn code_group<C: Coder>(c: &mut C, md: &mut Model, sq: &Stretch, l: &Layout, st: &mut [ChanState], prev_ms: &mut [bool], g: &mut Group) {
    let chans = st.len();
    let nb = l.bands.len();
    let m = l.m;
    let mut last: [Option<(usize, i32)>; 2] = [None; 2];
    let mut last_level: [Option<(usize, i32)>; 2] = [None; 2];
    for b in 0..nb {
        let (a, e) = l.bands[b];
        let cls = l.class[b];
        let grp = (b * 3 / nb).min(2);
        c.part(PART_SIDE);
        if chans == 2 {
            g.ms[b] = c.bit(&mut md.ms[cls][prev_ms[b] as usize], g.ms[b]);
            prev_ms[b] = g.ms[b];
        }
        for ch in 0..chans {
            c.part(PART_SIDE);
            let s = &mut st[ch];
            let z = c.bit(&mut md.zero[cls][s.prev_zero[b] as usize], g.zero[b][ch]);
            g.zero[b][ch] = z;
            s.prev_zero[b] = z;
            if z {
                for j in 0..g.rows {
                    g.q[ch][j * m + a..j * m + e].fill(0);
                }
                let n = c.bit(&mut md.noise[cls][s.prev_noise[b] as usize], g.noise[b][ch]);
                g.noise[b][ch] = n;
                s.prev_noise[b] = n;
                if n {
                    let pred = match last_level[ch] {
                        Some((lb, lv)) => median(lv, s.prev_level[b], lv + s.prev_level[b] - s.prev_level[lb]),
                        None => s.prev_level[b],
                    };
                    let d = rc::sint(c, &mut md.level[grp], g.level[b][ch] - pred);
                    g.level[b][ch] = pred + d;
                    last_level[ch] = Some((b, g.level[b][ch]));
                }
                continue;
            }
            g.noise[b][ch] = false;
            s.prev_noise[b] = false;
            let pred = match last[ch] {
                Some((lb, lv)) => median(lv, s.prev_sf[b], lv + s.prev_sf[b] - s.prev_sf[lb]),
                None => s.prev_sf[b],
            };
            c.part(PART_SF);
            let d = rc::sint(c, &mut md.sf[grp], g.sf[b][ch] - pred);
            g.sf[b][ch] = pred + d;
            last[ch] = Some((b, g.sf[b][ch]));
        }
    }
    for j in 0..g.rows {
        let row = j * m;
        for ch in 0..chans {
            for b in (0..nb).filter(|&b| !g.zero[b][ch]) {
                let (a, e) = l.bands[b];
                let cls = l.class[b];
                let r = if j > 0 { 256 } else { ratio_q8(st[ch].prev_sf[b] - g.sf[b][ch]) as u64 };
                let set = if ch == 0 { 0 } else { 1 + g.ms[b] as usize };
                let rx = if ch == 1 && !g.zero[b][0] { ratio_q8(g.sf[b][0] - g.sf[b][1]) as u64 } else { 0 };
                let blv = {
                    let p: &[i32] = if j > 0 { &g.q[ch][row - m..row] } else { &st[ch].prev_q };
                    let sum: u64 = p[a..e].iter().map(|v| v.unsigned_abs() as u64).sum();
                    level_band(sum * r / (e - a) as u64)
                };
                let mut seen = 0usize;
                for k in a..e {
                    let (lv_l, lv_u, lv_x, pat) = {
                        let q = &g.q[ch];
                        let q1 = if k > 0 { q[row + k - 1].unsigned_abs() } else { 0 };
                        let q2 = if k > 1 { q[row + k - 2].unsigned_abs() } else { 0 };
                        let p: &[i32] = if j > 0 { &q[row - m..row] } else { &st[ch].prev_q };
                        let around = |p: &[i32]| 2 * p[k].unsigned_abs() + if k > 0 { p[k - 1].unsigned_abs() } else { 0 } + p.get(k + 1).map_or(0, |v| v.unsigned_abs());
                        let x = if set > 0 { level_up(around(&g.q[0][row..row + m]) as u64 * rx) } else { 0 };
                        let pk = ((p[k].unsigned_abs() as u64 * r + 128) >> 8).min(3) as usize;
                        let pat = ((q1.min(3) as usize * 3 + q2.min(2) as usize) * 4 + pk) * 2 + (k == a) as usize;
                        (level_left(2 * q1 + q2), level_up(around(p) as u64 * r), x, pat)
                    };
                    let ctx = if set == 0 { lv_l * LV + lv_u } else { LV * LV + ((set - 1) * LV + lv_x) * LV * LV + lv_l * LV + lv_u };
                    let bctx = lv_l * LV + lv_u;
                    let lctx = blv * LV + lv_l;
                    let pos = k - a;
                    let run = seen.min(3) * 6 + if pos < 4 { pos } else if pos < 8 { 4 } else { 5 };
                    let v = g.q[ch][row + k];
                    c.part(PART_ZERO);
                    let nz = mix_bit(
                        c,
                        &mut md.mx,
                        sq,
                        cls * 2,
                        [&mut md.nz[cls][ctx], &mut md.nz_band[b][bctx], &mut md.nz_pat[cls][pat], &mut md.nz_blv[cls][lctx], &mut md.nz_run[cls][run]],
                        v != 0,
                    );
                    seen += nz as usize;
                    let out = if nz {
                        c.part(PART_MAG);
                        let mut mag = 1;
                        let gt1 = [&mut md.gt1[cls][ctx], &mut md.gt1_band[b][bctx], &mut md.gt1_pat[cls][pat], &mut md.gt1_blv[cls][lctx], &mut md.gt1_run[cls][run]];
                        if mix_bit(c, &mut md.mx, sq, cls * 2 + 1, gt1, v.abs() > 1) {
                            mag = 2;
                            let steps = &mut md.steps[cls][set][lv_l + lv_u + lv_x];
                            while mag < 2 + MAG_STEPS as i32 && c.bit(&mut steps[(mag - 2) as usize], v.abs() > mag) {
                                mag += 1;
                            }
                            if mag == 2 + MAG_STEPS as i32 {
                                mag += rc::uint(c, &mut md.esc[cls], (v.abs() - mag).max(0) as u32) as i32;
                            }
                        }
                        c.part(PART_SIGN);
                        if c.raw(v < 0) { -mag } else { mag }
                    } else {
                        0
                    };
                    g.q[ch][row + k] = out;
                }
            }
        }
    }
    let last_row = (g.rows - 1) * m;
    for ch in 0..chans {
        st[ch].prev_q.copy_from_slice(&g.q[ch][last_row..last_row + m]);
        for b in 0..nb {
            if !g.zero[b][ch] {
                st[ch].prev_sf[b] = g.sf[b][ch];
            } else if g.noise[b][ch] {
                st[ch].prev_level[b] = g.level[b][ch];
            }
        }
    }
}

fn level_band(v: u64) -> usize {
    match v {
        0 => 0,
        1..=31 => 1,
        32..=63 => 2,
        64..=127 => 3,
        128..=255 => 4,
        256..=511 => 5,
        512..=1023 => 6,
        _ => 7,
    }
}

fn level_left(v: u32) -> usize {
    match v {
        0 => 0,
        1..=2 => 1,
        3..=4 => 2,
        5..=8 => 3,
        _ => 4,
    }
}

fn level_up(v: u64) -> usize {
    match v {
        0..=127 => 0,
        128..=639 => 1,
        640..=1535 => 2,
        1536..=3071 => 3,
        _ => 4,
    }
}

pub(crate) struct Coding {
    long: Box<Model>,
    short: Box<Model>,
    sq: Stretch,
    kind_p: [[Prob; 3]; 4],
    group_p: [Prob; 2],
    st_long: Vec<ChanState>,
    st_short: Vec<ChanState>,
    ms_long: Vec<bool>,
    ms_short: Vec<bool>,
    prev_kind: Kind,
}

impl Coding {
    pub(crate) fn new(ll: &Layout, ls: &Layout, chans: usize) -> Self {
        Coding {
            long: Model::new(ll.bands.len()),
            short: Model::new(ls.bands.len()),
            sq: Stretch::new(),
            kind_p: [[Prob::INIT; 3]; 4],
            group_p: [Prob::INIT; 2],
            st_long: vec![ChanState::new(ll); chans],
            st_short: vec![ChanState::new(ls); chans],
            ms_long: vec![false; ll.bands.len()],
            ms_short: vec![false; ls.bands.len()],
            prev_kind: Kind::Long,
        }
    }
}

pub(crate) fn code_frame<C: Coder>(c: &mut C, cd: &mut Coding, ll: &Layout, ls: &Layout, f: &mut Frame) {
    c.part(PART_SIDE);
    let pk = cd.prev_kind.index();
    let short = c.bit(&mut cd.kind_p[pk][0], f.kind == Kind::Short);
    f.kind = if short {
        Kind::Short
    } else if !c.bit(&mut cd.kind_p[pk][1], f.kind != Kind::Long) {
        Kind::Long
    } else if c.bit(&mut cd.kind_p[pk][2], f.kind == Kind::Stop) {
        Kind::Stop
    } else {
        Kind::Start
    };
    let was_short = cd.prev_kind == Kind::Short;
    cd.prev_kind = f.kind;
    if short {
        if !was_short {
            for s in cd.st_short.iter_mut() {
                s.prev_q.fill(0);
            }
        }
        let mut new = [false; SHORTS];
        let mut j = 0;
        for g in f.groups.iter().filter(|g| g.m == MS) {
            if j < SHORTS {
                new[j] = true;
            }
            j += g.rows;
        }
        new[0] = true;
        for j in 1..SHORTS {
            new[j] = c.bit(&mut cd.group_p[new[j - 1] as usize], new[j]);
        }
        let mut rows: Vec<usize> = Vec::new();
        for &n in &new {
            if n {
                rows.push(0);
            }
            *rows.last_mut().unwrap() += 1;
        }
        if f.groups.len() != rows.len() || f.groups.iter().zip(&rows).any(|(g, &r)| g.m != MS || g.rows != r) {
            f.groups = rows.iter().map(|&r| Group::new(ls, r)).collect();
        }
        for g in f.groups.iter_mut() {
            code_group(c, &mut cd.short, &cd.sq, ls, &mut cd.st_short, &mut cd.ms_short, g);
        }
    } else {
        if was_short {
            for s in cd.st_long.iter_mut() {
                s.prev_q.fill(0);
            }
        }
        if f.groups.len() != 1 || f.groups[0].m != M {
            f.groups = vec![Group::new(ll, 1)];
        }
        code_group(c, &mut cd.long, &cd.sq, ll, &mut cd.st_long, &mut cd.ms_long, &mut f.groups[0]);
    }
}

pub fn pack<C: Coder>(c: &mut C, ll: &Layout, ls: &Layout, chans: usize, frames: &mut [Frame], mut each: impl FnMut(&mut C)) {
    for chunk in frames.chunks_mut(CHUNK) {
        each(c);
        let mut cd = Coding::new(ll, ls, chans);
        for f in chunk.iter_mut() {
            code_frame(c, &mut cd, ll, ls, f);
        }
    }
}
