# KTC Specification (version 2)

日本語版: [SPEC.ja.md](SPEC.ja.md)

KTC is an audio compression format designed mainly for use with KT3.
It aims to make the data small while keeping the sound indistinguishable from the original to the human ear.

- The file extension is `.ktc`.
- KTC is used for the audio of KT3 and KT4 files.
  - In KT3, it is `mediaFormat` 5.
  - In KT4, it is used for the audio from version 7.
  - See §2.2 and §6.5 of the [KT3 specification](https://github.com/KEITO00/kt3-format) for details.
- All integers are little-endian.

This document defines the layout of the data and the decoding steps of versions 1 and 2.
The details of how the coefficients are packed (the contexts of the arithmetic coder and how the
probabilities adapt) are defined by the reference implementation (section 7).

The key words "MUST", "MUST NOT" and "SHOULD" are used as described in RFC 2119.

## 1. Overview

```
| Header      | 25 bytes (version 1), 27 bytes (version 2)
| Chunk table | 4 + 4 × chunk count bytes
| Chunk data  | sum of the chunk sizes
```

## 2. Header

| Offset | Size | Type | Field | Description |
|---|---|---|---|---|
| 0 | 4 | — | magic | `K` `T` `C` `0x00` |
| 4 | 1 | u8 | version | Version of the data layout (1 or 2) |
| 5 | 1 | u8 | channels | Number of channels (1 or 2) |
| 6 | 2 | i8 × 2 | offsets | Offset of the positions where quantized values are restored, in units of 1/128 |
| 8 | 4 | u32 | sampleRate | Sampling rate in Hz (8000 to 192000) |
| 12 | 8 | u64 | length | Number of samples per channel (at most 2^31) |
| 20 | 2 | u16 | windowLen | Length of the long window (2048) |
| 22 | 3 | u8 × 3 | fill | Level of the noise that fills the coefficients quantized to 0, relative to the quantization step, in units of 1/512 |
| 25 | 2 | u16 | mixedHz | Lowest frequency of the mixed bands (section 6) in Hz (version 2 only) |

- This document defines versions 1 and 2.
- The first value of `offsets` is used for values of magnitude 1, and the second for magnitude 2 or more.
- `windowLen` MUST be 2048.
- `fill` has three values, in this order: for bands whose largest magnitude is 1, 2, and 3 or more.
- `mixedHz` is present only in version 2.
- A reader MUST NOT read data whose `magic` is different or whose `version` it does not know.
  - A different version has a different layout.
- A reader that reads version 2 MUST also read version 1.
  - Files written in version 1 stay in use.
- A writer SHOULD write version 1 when it does not use mixed bands.
  - Readers that do not support version 2 can then read the data.

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
  - The random number generator for the noise starts from `0x4B54 XOR (c × 0x9E3779B9 mod 2^32)` for chunk `c`.
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
   - With two channels and mixed bands, the stereo values follow the coefficients of each group (6.3).
4. **Noise envelope**: read at the end of the frame when there are mixed bands (6.4).
5. **Restoring the coefficients**:
   - The quantized values are multiplied by the quantization step.
     - Nonzero values are restored at positions shifted by the header `offsets`.
   - The zeros in bands with coefficients are filled with noise at the level given by the header `fill`.
   - Noise bands are filled with noise at the stored level.
   - Sum and difference bands are converted back to left and right.
   - Mixed bands are restored as described in 6.5 and 6.6.
6. **Synthesis**: inverse MDCT, windowing, and overlap-add with the previous frame.
   - The windows are sine windows and the transitions between them.
   - Any sequence of windows reconstructs the original signal when the frames are overlapped.

## 6. Mixed bands (version 2)

- When `mixedHz` is greater than 0, the bands above that frequency are "mixed bands".
- In mixed bands, only some of the coefficients are stored, and the rest is filled with noise whose level is stored.
  - In high frequencies, much of the data is spent on noise-like parts whose fine structure is hard to hear.
  - The writer decides which coefficients to store.

### 6.1 Band layout

- A band (a parent band) whose start frequency is `mixedHz` or higher is split further.
  - The parts have at most 32 coefficients for long windows and at most 4 for short windows.
  - With `w` coefficients in the parent band and an upper limit of `u`, the band is split into `p = ceil(w / u)` parts.
  - Part `i` (counted from 0) covers the coefficients from `floor(w × i / p)` up to, but not including, `floor(w × (i + 1) / p)`, counted from the start of the parent band.
- The start frequency of a band is the index of its first coefficient × the sampling rate ÷ the window length.
- The mixed bands run from the first band whose start frequency is `mixedHz` or higher to the last band.
- The stereo values (6.3) are stored per parent band.

### 6.2 Band information and coefficients

- A mixed band with coefficients also stores a noise level (in steps of 1.5 dB) after its quantization step.
  - Its zeros are filled with noise at this level instead of the header `fill`.
- With two channels, the following changes apply.
  - Whether the band is L/R or M/S is not stored.
  - Channel 0 stores the sum `(L + R) / √2`.
  - Channel 1 stores only whether it has coefficients.
    - In a band where channel 0 has no coefficients, nothing is stored for channel 1.
    - The quantization step of channel 1 is the same as that of channel 0.
  - Channel 1 has coefficients only where the coefficient of channel 0 is not 0.
    - They are the difference from the difference predicted from the sum (6.5).
  - The noise level is stored only for channel 0.
    - It is the average of the left and right noise energies per coefficient.

### 6.3 Stereo values (two channels)

After the coefficients of a group, the following values are stored for each parent band.

| Value | Range | Description |
|---|---|---|
| iid | −30 to 30 | Level ratio of the left and right noise, in steps of 1.5 dB |
| icc | 0 to 7 | Index of the similarity ρ of the left and right noise |
| kp | −16 to 16 | Coefficient of the difference prediction, in units of 1/8 |

- A positive `iid` means that the left is louder.
- The indices of `icc` correspond to ρ as shown in the following table.

| icc | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---|---|---|---|---|---|---|---|---|
| ρ | 1 | 0.94 | 0.84 | 0.6 | 0.37 | 0 | −0.59 | −1 |

- `iid` and `icc` are stored only when the parent band contains a part with coefficients or a part filled with noise.
- `kp` is stored only when the parent band contains a part with coefficients.

### 6.4 Noise envelope

- When there are mixed bands, eight values `e[0]` to `e[7]` (in steps of 1.5 dB), one for each short window, are stored at the end of the frame.
  - `e[0]` is 0.
  - When all of them are 0, only that fact is stored.
  - Otherwise, `e[j] − e[j − 1]` (`j` from 1 to 7) is stored in order.
- The noise energy of window `j` is multiplied by `g[j]`, which is `2^(e[j] / 2)` divided so that the average becomes 1.
  - In long-window frames, it is divided by the average of the eight values.
  - In short-window frames, it is divided by the average of the values of the windows in the group.
- `g[j]` is the same for all channels.

### 6.5 Restoring the coefficients and the noise

- With two channels, the coefficients are restored as follows.
  - Let `M` be the restored value of channel 0, `D` the restored value of channel 1, and `k = kp / 8` the coefficient of the difference prediction.
  - The difference is `S = k × M + D`.
  - The left is `(M + S) / √2`, and the right is `(M − S) / √2`.
- The noise energy per coefficient is `E = 2^(level / 2)`, where `level` is the noise level.
- With two channels, the noise is made as follows.
  - With `r = 2^(iid / 2)`, the left noise energy is `2 × E × r / (1 + r)`, and the right is `2 × E / (1 + r)`.
  - From two unrelated noises `n1` and `n2`, the left is `√(left energy) × (a × n1 + b × n2)`, and the right is `√(right energy) × (a × n1 − b × n2)`.
    - `a = √((1 + ρ) / 2)` and `b = √((1 − ρ) / 2)`.
- In short-window frames, the zeros of each window are filled with this noise.
  - The noise energy of a window is the number of zeros in that window × `E` × `g[j]`.

### 6.6 Noise in long-window frames

- In long-window frames, the noise of the mixed bands is not put into the long-window coefficients; it is made with eight short windows and added.
  - This lets the noise level change at the resolution of a short window (about 3 ms).
  - The short windows are placed in the same positions as in short-window frames.
- For a part that covers the coefficients `[lo, hi)`, its noise goes into the short-window coefficients `[ceil(lo / 8), ceil(hi / 8))`.
- The noise energy of window `j` is the number of zeros × `E` × `g[j]` / 64.
  - For a band filled with noise, the number of zeros is the number of coefficients in the band.
  - The division by 64 comes from the long-window coefficients having eight times the energy of the short-window ones, and from splitting the energy into eight windows.
- How the noise values are made (the use of the random number generator and how the energy is matched) is defined by the reference implementation.

## 7. Reference implementation

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

## 8. License

This specification is licensed under
[Creative Commons Attribution 4.0 International (CC BY 4.0)](https://creativecommons.org/licenses/by/4.0/).
Anyone may implement it. See [LICENSE](LICENSE) for the use of the KTC name.
