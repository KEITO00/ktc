# KTC Specification (version 1)

日本語版: [SPEC.ja.md](SPEC.ja.md)

KTC is an audio compression format designed mainly for use with KT3.
It aims to make the data small while keeping the sound indistinguishable from the original to the human ear.

- The file extension is `.ktc`.
- KTC is used for the audio of KT3 and KT4 files.
  - In KT3, it is `mediaFormat` 5.
  - In KT4, it is used for the audio from version 7.
  - See §2.2 and §6.5 of the [KT3 specification](https://github.com/KEITO00/kt3-format) for details.
- All integers are little-endian.

This document defines the layout of the data and the decoding steps.
The details of how the coefficients are packed (the contexts of the arithmetic coder and how the
probabilities adapt) are defined by the reference implementation (section 6).

The key words "MUST", "MUST NOT" and "SHOULD" are used as described in RFC 2119.

## 1. Overview

```
| Header      | 25 bytes
| Chunk table | 4 + 4 × chunk count bytes
| Chunk data  | sum of the chunk sizes
```

## 2. Header

| Offset | Size | Type | Field | Description |
|---|---|---|---|---|
| 0 | 4 | — | magic | `K` `T` `C` `0x00` |
| 4 | 1 | u8 | version | Version of the data layout. This document defines version 1 |
| 5 | 1 | u8 | channels | Number of channels (1 or 2) |
| 6 | 2 | i8 × 2 | offsets | Offset of the positions where quantized values are restored, in units of 1/128. The first is for values of magnitude 1, the second for magnitude 2 or more |
| 8 | 4 | u32 | sampleRate | Sampling rate in Hz (8000 to 192000) |
| 12 | 8 | u64 | length | Number of samples per channel (at most 2^31) |
| 20 | 2 | u16 | windowLen | Length of the long window. It MUST be 2048 |
| 22 | 3 | u8 × 3 | fill | Level of the noise that fills the coefficients quantized to 0, relative to the quantization step, in units of 1/512. There are three values, in this order: for bands whose largest magnitude is 1, 2, and 3 or more |

- A reader MUST NOT read data whose `magic` or `version` is different.
  - A different version has a different layout.

## 3. Chunk table

```
u32 count           number of chunks
u32 size[count]     size of each chunk in bytes
```

- The chunk data follows the table directly, in chunk order, with no gaps.
- `count` MUST be `ceil(frames / 1024)`.
  - `frames` is defined in section 4.

## 4. Frames and chunks

- The audio is represented by windows (frames) of length 2048, shifted by 1024 samples.
  - Frame `f` covers the samples from `f × 1024 − 1024` up to, but not including, `f × 1024 + 1024`.
  - The number of frames is `frames = ceil(length / 1024) + 1`.
- Chunk `c` contains the frames from `c × 1024` up to, but not including, `min((c + 1) × 1024, frames)` (about 22 seconds at 48 kHz).

### 4.1 Independent chunks

- Each chunk can be decoded on its own.
  - At the start of a chunk, the state of the arithmetic coder and all adapted probabilities are reset to their initial values.
  - The random number generator for the fill noise starts from `0x4B54 XOR (c × 0x9E3779B9 mod 2^32)` for chunk `c`.
- The audio of a chunk is the overlap-add of its frames.
  - The audio of chunk `c` starts at sample `c × 1024 × 1024 − 1024` of the whole decoded audio (the part at negative positions is discarded).
  - Adjacent chunks overlap by 1024 samples.
  - The overlapping parts are added together.
- A reader SHOULD decode several chunks at the same time.
  - This shortens the wait when a track is opened.

## 5. Decoding steps

Each frame is decoded in this order:

1. **Window type**: one of four types: long, start transition, short and stop transition.
   - A short-window frame consists of 8 windows of 256 samples.
   - The 8 short windows are divided into one or more groups.
   - The group boundaries are also read here.
2. **Band information**: one set for a long window, and one set per group for short windows.
   - Each band is about 0.5 Bark wide.
   - For each band, it tells whether the channels are stored as left and right (L/R) or as sum and difference (M/S).
   - For each channel, it tells whether the band has coefficients.
     - A band without coefficients is either filled with noise or silent.
   - A band with coefficients stores its quantization step, and a band filled with noise stores the noise level in steps of 1.5 dB.
3. **Coefficients**: the quantized values of the bands that have coefficients.
   - They are ordered by window, then by channel, then by band.
4. **Restoring the coefficients**:
   - The quantized values are multiplied by the quantization step.
     - Nonzero values are restored at positions shifted by the header `offsets`.
   - The zeros in bands with coefficients are filled with noise at the level given by the header `fill`.
   - Noise bands are filled with noise at the stored level.
   - Sum and difference bands are converted back to left and right.
5. **Synthesis**: inverse MDCT, windowing, and overlap-add with the previous frame.
   - The windows are sine windows and the transitions between them.
   - Any sequence of windows reconstructs the original signal when the frames are overlapped.

## 6. Reference implementation

The details of how the coefficients are packed are defined by the code of the reference
implementation (`src` in this repository).

| Part | File |
|---|---|
| Header and chunk table | `src/format.rs` |
| Band boundaries (Bark) | `src/bands.rs` |
| Windows and MDCT | `src/transform.rs`, `src/mdct.rs`, `src/fft.rs` |
| Band information and coefficient packing | `src/coding.rs` |
| Arithmetic coder and probability adaptation | `src/rc.rs` |
| Probability mixing | `src/mix.rs` |
| Restoring the coefficients and synthesis | `src/decode.rs` |
| Functions for WebAssembly | `src/wasm.rs` |

- The contexts of the arithmetic coder are computed with integers only.
  - This makes every device read the same values.
- The decoded sample values may differ slightly in the last digits between devices.
  - This comes from small differences in trigonometric and power functions.
  - It does not affect how the audio sounds.

## 7. License

This specification is licensed under
[Creative Commons Attribution 4.0 International (CC BY 4.0)](https://creativecommons.org/licenses/by/4.0/).
Anyone may implement it. See [LICENSE](LICENSE) for the use of the KTC name.
