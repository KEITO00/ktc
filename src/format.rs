use crate::bands::Layout;
use crate::coding::{self, Frame};
use crate::rc;
use crate::transform::{M, N, NS};

pub const MAGIC: &[u8; 4] = b"KTC\0";
pub const VERSION: u8 = 1;
pub const FILL_KINDS: usize = 3;
const HEADER: usize = 4 + 1 + 1 + 2 + 4 + 8 + 2 + FILL_KINDS;
pub const CHUNK: usize = 1024;

pub fn chunk_seed(c: usize) -> u32 {
    0x4b54 ^ (c as u32).wrapping_mul(0x9e37_79b9)
}

pub struct Header {
    pub chans: usize,
    pub sr: u32,
    pub len: usize,
    pub offsets: [i8; 2],
    pub fill: [u8; FILL_KINDS],
}

pub fn write(h: &Header, frames: &mut [Frame]) -> Vec<u8> {
    let ll = Layout::new(h.sr, N);
    let ls = Layout::new(h.sr, NS);
    let mut out = Vec::with_capacity(HEADER);
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.push(h.chans as u8);
    out.extend(h.offsets.map(|v| v as u8));
    out.extend_from_slice(&h.sr.to_le_bytes());
    out.extend_from_slice(&(h.len as u64).to_le_bytes());
    out.extend_from_slice(&(N as u16).to_le_bytes());
    out.extend_from_slice(&h.fill);
    let mut chunks: Vec<Vec<u8>> = Vec::new();
    let mut enc = rc::Encoder::new();
    let mut first = true;
    coding::pack(&mut enc, &ll, &ls, h.chans, frames, |e| {
        if !std::mem::take(&mut first) {
            chunks.push(std::mem::replace(e, rc::Encoder::new()).finish());
        }
    });
    chunks.push(enc.finish());
    out.extend_from_slice(&(chunks.len() as u32).to_le_bytes());
    for c in &chunks {
        out.extend_from_slice(&(c.len() as u32).to_le_bytes());
    }
    for c in &chunks {
        out.extend_from_slice(c);
    }
    out
}

pub struct Info {
    pub chans: usize,
    pub sr: u32,
    pub len: usize,
    pub(crate) off: [f64; 2],
    pub(crate) fill: [f64; FILL_KINDS],
    pub(crate) chunks: Vec<std::ops::Range<usize>>,
    pub(crate) ll: Layout,
    pub(crate) ls: Layout,
}

impl Info {
    pub fn chunks(&self) -> usize {
        self.chunks.len()
    }
}

pub fn info(data: &[u8]) -> Result<Info, String> {
    if data.len() < HEADER || &data[0..4] != MAGIC {
        return Err("KTC のデータではありません".into());
    }
    if data[4] != VERSION {
        return Err(format!("この版 ({}) の KTC には対応していません", data[4]));
    }
    let bad = || "KTC のヘッダーが正しくありません".to_string();
    let chans = data[5] as usize;
    let off = [data[6], data[7]].map(|v| v as i8 as f64 / 128.0);
    let sr = u32::from_le_bytes(data[8..12].try_into().unwrap());
    let len64 = u64::from_le_bytes(data[12..20].try_into().unwrap());
    let n = u16::from_le_bytes(data[20..22].try_into().unwrap()) as usize;
    let fill: [f64; FILL_KINDS] = std::array::from_fn(|i| (data[22 + i] as f64 / 512.0).powi(2));
    if n != N || !(1..=2).contains(&chans) || !(8000..=192_000).contains(&sr) || len64 > 1 << 31 {
        return Err(bad());
    }
    let len = len64 as usize;
    let u32_at = |o: usize| data.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize).ok_or_else(bad);
    let count = u32_at(HEADER)?;
    if count != (len.div_ceil(M) + 1).div_ceil(CHUNK) {
        return Err(bad());
    }
    let mut pos = HEADER + 4 + count * 4;
    let mut chunks = Vec::with_capacity(count);
    for c in 0..count {
        let size = u32_at(HEADER + 4 + c * 4)?;
        if pos + size > data.len() {
            return Err("KTC のデータが途中で切れています".into());
        }
        chunks.push(pos..pos + size);
        pos += size;
    }
    Ok(Info { chans, sr, len, off, fill, chunks, ll: Layout::new(sr, N), ls: Layout::new(sr, NS) })
}
