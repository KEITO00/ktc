# Changelog / 変更履歴

The `version` byte of the header describes the layout of the data.

ヘッダーの `version` は、データ構造の版を表します。

## Version 3 (2026-10-24)

- Added the flip flag of the mixed bands (section 7.1).
  - With two channels, each parent band of the mixed bands can store the left and the negated right.
  - When the high frequencies are recorded with the left and right in opposite phase, the sum no longer cancels out.
- Added TNS (section 7.2).
  - Each window can store a filter along the frequency axis for each of three ranges (from 100 Hz, 1000 Hz and 4000 Hz).
  - The quantization noise and the noise of the mixed bands follow the loudness changes inside the window, so that no noise is heard before an attack.
- The header is the same as in version 2.
- Readers that read version 3 also read versions 1 and 2.

- 混合帯域の反転の印を追加しました（7.1）。
  - 2 チャンネルの場合、混合帯域の元の帯域ごとに、左と反転した右を記録できます。
  - 高い周波数が左右逆位相で記録されていても、和が打ち消し合わないようにするためです。
- TNS を追加しました（7.2）。
  - 窓ごとに、3 つの範囲（100 Hz・1000 Hz・4000 Hz から）のそれぞれに、周波数の向きのフィルターを記録できます。
  - 量子化の雑音と混合帯域の雑音が窓の中の音量の変化に沿い、音の立ち上がりの前に雑音が聞こえないようにするためです。
- ヘッダーは版 2 と同じです。
- 版 3 に対応した読み込み側は、版 1 と版 2 も読み込めます。

## Looping audio (2026-10-18)

- Added section 4.2 about looping audio.
  - A writer SHOULD encode the last frames with the continuation of the loop (the audio from the loop start) placed after the end of the audio.
  - This keeps the error in the last few samples small, so that no click is heard at the loop point.
- The data layout and the `version` byte do not change.

- ループする音声についての 4.2 を追加しました。
  - 書き込み側は、音声の最後より後ろにループの続きの音（ループの開始位置からの音）を配置して、最後のフレームを符号化すべきです（SHOULD）。
  - 最後の数サンプルの誤差を小さくし、ループのつなぎ目で雑音が聞こえないようにするためです。
- データ構造と `version` は変更しません。

## Version 2 (2026-10-08)

- Added mixed bands above the frequency given by the new header field `mixedHz`.
  - Only some of the coefficients are stored, and the rest is filled with noise whose level is stored.
  - The bands are split into parts of at most 32 coefficients (4 for short windows).
  - The noise of long-window frames is made with short windows, and its envelope is stored for each frame.
  - With two channels, only the sum is stored, together with the level ratio, the similarity and the difference prediction of the left and right.
- Readers that read version 2 also read version 1.

- ヘッダーに新しい項目 `mixedHz` を追加し、その周波数より上の帯域を混合帯域としました。
  - 一部の係数だけを記録し、残りはレベルだけを記録した雑音で埋めます。
  - 帯域を 32 個以下（短い窓では 4 個以下）の係数ずつに分割します。
  - 長い窓のフレームの雑音は短い窓で生成し、その時間包絡をフレームごとに記録します。
  - 2 チャンネルの場合は和だけを記録し、左右のレベル比・相関・差の予測を併せて記録します。
- 版 2 に対応した読み込み側は、版 1 も読み込めます。

## Version 1 (2026-10-07)

- This is the first version.
  - Long windows (2048 samples) and short windows (256 samples) are switched through transition windows.
  - Bands are about 0.5 Bark wide, and each band is stored as L/R or M/S.
  - The coefficients are packed with an arithmetic coder that mixes the predictions of several contexts.
  - Coefficients quantized to 0 and noise bands are filled with noise.
  - The data is split into chunks of 1024 frames, and each chunk can be decoded on its own.

- 最初の版を定めました。
  - 長い窓（2048 サンプル）と短い窓（256 サンプル）を、遷移窓を挟んで切り替えます。
  - 帯域の幅は約 0.5 Bark で、帯域ごとに L/R または M/S で記録します。
  - 係数は、複数のコンテキストの予測を混合する算術符号で符号化します。
  - 0 に量子化された箇所と雑音の帯域は、雑音で埋めます。
  - データを 1024 フレームずつのチャンクに分割し、チャンクごとに単独で復元できるようにしました。
