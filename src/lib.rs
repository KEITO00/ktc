pub mod bands;
pub mod coding;
pub mod decode;
pub mod fft;
pub mod format;
pub mod mdct;
pub mod mix;
pub mod rc;
pub mod transform;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

pub use decode::{add_chunk, decode, decode_chunk};
pub use format::{info, Info};

pub struct Audio {
    pub sr: u32,
    pub ch: Vec<Vec<f32>>,
}

impl Audio {
    pub fn len(&self) -> usize {
        self.ch.first().map_or(0, |c| c.len())
    }

    pub fn secs(&self) -> f64 {
        self.len() as f64 / self.sr as f64
    }

    pub fn mono(&self) -> Vec<f32> {
        let n = self.len();
        let k = self.ch.len().max(1) as f32;
        (0..n).map(|i| self.ch.iter().map(|c| c[i]).sum::<f32>() / k).collect()
    }
}
