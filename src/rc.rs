pub const PROB_BITS: u32 = 12;
const TOP: u32 = 1 << 24;
const FAST: u32 = 4;
const SLOW: u32 = 7;

#[derive(Clone, Copy)]
pub struct Prob {
    fast: u16,
    slow: u16,
}

impl Prob {
    pub const INIT: Prob = Prob { fast: 1 << 15, slow: 1 << 15 };

    pub fn get(&self) -> u32 {
        ((self.fast as u32 + self.slow as u32) >> 5).clamp(1, (1 << PROB_BITS) - 1)
    }

    pub fn update(&mut self, b: bool) {
        if b {
            self.fast -= self.fast >> FAST;
            self.slow -= self.slow >> SLOW;
        } else {
            self.fast += (65535 - self.fast) >> FAST;
            self.slow += (65535 - self.slow) >> SLOW;
        }
    }
}

pub trait Coder {
    fn code(&mut self, p0: u32, b: bool) -> bool;
    fn raw(&mut self, b: bool) -> bool;
    fn part(&mut self, _p: usize) {}

    fn bit(&mut self, p: &mut Prob, b: bool) -> bool {
        let r = self.code(p.get(), b);
        p.update(r);
        r
    }
}

pub struct Encoder {
    low: u64,
    range: u32,
    cache: u8,
    cache_size: u64,
    pub out: Vec<u8>,
}

impl Encoder {
    pub fn new() -> Self {
        Encoder { low: 0, range: 0xFFFF_FFFF, cache: 0, cache_size: 1, out: Vec::new() }
    }

    fn shift_low(&mut self) {
        if (self.low as u32) < 0xFF00_0000 || (self.low >> 32) != 0 {
            let carry = (self.low >> 32) as u8;
            let mut temp = self.cache;
            loop {
                self.out.push(temp.wrapping_add(carry));
                temp = 0xFF;
                self.cache_size -= 1;
                if self.cache_size == 0 {
                    break;
                }
            }
            self.cache = ((self.low >> 24) & 0xFF) as u8;
        }
        self.cache_size += 1;
        self.low = (self.low & 0x00FF_FFFF) << 8;
    }

    fn normalize(&mut self) {
        while self.range < TOP {
            self.range <<= 8;
            self.shift_low();
        }
    }

    pub fn finish(mut self) -> Vec<u8> {
        for _ in 0..5 {
            self.shift_low();
        }
        self.out
    }
}

impl Coder for Encoder {
    fn code(&mut self, p0: u32, b: bool) -> bool {
        let bound = (self.range >> PROB_BITS) * p0;
        if b {
            self.low += bound as u64;
            self.range -= bound;
        } else {
            self.range = bound;
        }
        self.normalize();
        b
    }

    fn raw(&mut self, b: bool) -> bool {
        self.range >>= 1;
        if b {
            self.low += self.range as u64;
        }
        self.normalize();
        b
    }
}

pub struct Decoder<'a> {
    code: u32,
    range: u32,
    data: &'a [u8],
    pos: usize,
}

impl<'a> Decoder<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        let mut d = Decoder { code: 0, range: 0xFFFF_FFFF, data, pos: 0 };
        for _ in 0..5 {
            d.code = (d.code << 8) | d.next() as u32;
        }
        d
    }

    fn next(&mut self) -> u8 {
        let b = self.data.get(self.pos).copied().unwrap_or(0);
        self.pos += 1;
        b
    }

    fn normalize(&mut self) {
        while self.range < TOP {
            self.range <<= 8;
            self.code = (self.code << 8) | self.next() as u32;
        }
    }
}

impl Coder for Decoder<'_> {
    fn code(&mut self, p0: u32, _: bool) -> bool {
        let bound = (self.range >> PROB_BITS) * p0;
        let b = self.code >= bound;
        if b {
            self.code -= bound;
            self.range -= bound;
        } else {
            self.range = bound;
        }
        self.normalize();
        b
    }

    fn raw(&mut self, _: bool) -> bool {
        self.range >>= 1;
        let b = self.code >= self.range;
        if b {
            self.code -= self.range;
        }
        self.normalize();
        b
    }
}

pub fn uint<C: Coder>(c: &mut C, ctx: &mut [Prob], v: u32) -> u32 {
    let n = v as u64 + 1;
    let k = 63 - n.leading_zeros() as usize;
    let mut len = 0usize;
    while len < ctx.len() - 1 && c.bit(&mut ctx[len], len < k) {
        len += 1;
    }
    let mut r: u64 = 1;
    for i in (0..len).rev() {
        r = (r << 1) | c.raw((n >> i) & 1 == 1) as u64;
    }
    (r - 1) as u32
}

pub fn sint<C: Coder>(c: &mut C, ctx: &mut [Prob], v: i32) -> i32 {
    let (zero, rest) = ctx.split_at_mut(1);
    if !c.bit(&mut zero[0], v != 0) {
        return 0;
    }
    let (sign, mag) = rest.split_at_mut(1);
    let neg = c.bit(&mut sign[0], v < 0);
    let m = uint(c, mag, v.unsigned_abs().saturating_sub(1)) as i32 + 1;
    if neg { -m } else { m }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_bits_and_numbers() {
        let mut seed = 5u32;
        let mut rnd = || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            seed >> 8
        };
        let bits: Vec<bool> = (0..20000).map(|_| rnd() % 10 == 0).collect();
        let nums: Vec<i32> = (0..5000).map(|_| (rnd() % 2000) as i32 - 1000).collect();
        let big = [0u32, 1, 2, 1000, 70000, u32::MAX - 1];
        let mut e = Encoder::new();
        let (mut p, mut ctx, mut ctx2) = (Prob::INIT, vec![Prob::INIT; 40], vec![Prob::INIT; 40]);
        for &b in &bits {
            e.bit(&mut p, b);
        }
        for &v in &nums {
            sint(&mut e, &mut ctx, v);
            e.raw(v & 1 == 1);
        }
        for &v in &big {
            uint(&mut e, &mut ctx2, v);
        }
        let data = e.finish();
        assert!(data.len() < 20000 / 8 * 6 / 10 + 5000 * 3, "skewed bits should compress: {} bytes", data.len());
        let mut d = Decoder::new(&data);
        let (mut p, mut ctx, mut ctx2) = (Prob::INIT, vec![Prob::INIT; 40], vec![Prob::INIT; 40]);
        for &b in &bits {
            assert_eq!(d.bit(&mut p, false), b);
        }
        for &v in &nums {
            assert_eq!(sint(&mut d, &mut ctx, 0), v);
            assert_eq!(d.raw(false), v & 1 == 1);
        }
        for &v in &big {
            assert_eq!(uint(&mut d, &mut ctx2, 0), v);
        }
    }
}
