# Changelog / 変更履歴

The `version` byte of the header describes the layout of the data.

ヘッダーの `version` は、データの並びの版を表します。

## Version 2 — 2026-10-08

- Added mixed bands above the frequency given by the new header field `mixedHz`.
  - Only some of the coefficients are stored, and the rest is filled with noise whose level is stored.
  - The bands are split into parts of at most 32 coefficients (4 for short windows).
  - The noise of long-window frames is made with short windows, and its envelope is stored for each frame.
  - With two channels, only the sum is stored, together with the level ratio, the similarity and the difference prediction of the left and right.
- Readers that read version 2 also read version 1.

- ヘッダーの新しい項目 `mixedHz` の周波数より上を、混ぜて記録する帯域にしました。
  - 一部の係数だけを記録し、残りは大きさだけを記録した雑音で埋めます。
  - 帯域を 32 個以下（短い窓では 4 個以下）の係数ずつに分けます。
  - 長い窓のフレームの雑音は短い窓で作り、その時間の形をフレームごとに記録します。
  - 2 チャンネルの時は和だけを記録し、左右の大きさの比・似ている度合い・差の予測を添えます。
- 版 2 を読む側は、版 1 も読みます。

## Version 1 — 2026-10-07

- This is the first version.
  - Long windows (2048 samples) and short windows (256 samples) are switched through transition windows.
  - Bands are about 0.5 Bark wide, and each band is stored as L/R or M/S.
  - The coefficients are packed with an arithmetic coder that mixes the predictions of several contexts.
  - Coefficients quantized to 0 and noise bands are filled with noise.
  - The data is split into chunks of 1024 frames, and each chunk can be decoded on its own.

- 最初の版を定めました。
  - 長い窓（2048 サンプル）と短い窓（256 サンプル）を、つなぎの窓を挟んで切り替えます。
  - 帯域はおよそ 0.5 Bark ずつで、帯域ごとに L/R か M/S で記録します。
  - 係数は、いくつかの手がかりの予測を混ぜる算術符号で詰めます。
  - 0 に丸めた所と雑音の帯域は、雑音で埋めます。
  - データを 1024 フレームずつの塊に分け、塊ごとに単独で復元できるようにしました。
