# Changelog / 変更履歴

The `version` byte of the header describes the layout of the data.

ヘッダーの `version` は、データの並びの版を表します。

## Version 1 — 2026-10-07

- This is the first version.
  - Long windows (2048 samples) and short windows (256 samples) are switched through transition windows.
  - Bands are about 0.5 Bark wide, and each band is stored as L/R or M/S.
  - The coefficients are packed with an arithmetic coder that mixes the predictions of several contexts.
  - Coefficients quantized to 0 and noise bands are filled with noise.
  - The data is split into chunks of 1024 frames, and each chunk can be decoded on its own.

- 最初の版を定めた。
  - 長い窓（2048 サンプル）と短い窓（256 サンプル）を、つなぎの窓を挟んで切り替える。
  - 帯域はおよそ 0.5 Bark ずつで、帯域ごとに L/R か M/S で記録する。
  - 係数は、いくつかの手がかりの予測を混ぜる算術符号で詰める。
  - 0 に丸めた所と雑音の帯域は、雑音で埋める。
  - データを 1024 フレームずつの塊に分け、塊ごとに単独で復元できるようにした。
