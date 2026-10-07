# KTC

[English](#english) | [日本語](#日本語)

## English

KTC is an audio compression format designed mainly for use with [KT3](https://github.com/KEITO00/kt3-format).

### Documents

- [SPEC.md](SPEC.md): the specification ([日本語](SPEC.ja.md))
- [CHANGELOG.md](CHANGELOG.md): format versions

### Reference implementation

This repository contains the reference implementation of the decoder. It is written in Rust and uses no external libraries.

#### Rust

Add the following line to the `[dependencies]` section of your `Cargo.toml`.

```toml
ktc = { git = "https://github.com/KEITO00/ktc" }
```

- `ktc::decode(data)` decodes the whole data.
- To decode on several CPUs at once, read the header with `ktc::info`, decode each chunk with `ktc::decode_chunk`, and add the chunks together with `ktc::add_chunk`.

#### WebAssembly

The following command builds `ktc.wasm`. It requires the Rust target `wasm32-unknown-unknown`.

```
cargo rustc --lib --release --target wasm32-unknown-unknown --crate-type cdylib
```

| Function | Description |
|---|---|
| `ktc_alloc(len)` | Reserves `len` bytes for the data and returns the address |
| `ktc_free(ptr, len)` | Releases memory reserved with `ktc_alloc` |
| `ktc_info(ptr, len)` | Reads the header |
| `ktc_decode(ptr, len)` | Decodes the whole data |
| `ktc_decode_chunk(ptr, len, c)` | Decodes chunk `c` |
| `ktc_free_result(ptr)` | Releases a result |

A result is an array of 4-byte values.

- [0] is the number of values in the array.
- [1] is 0 on success and 1 on failure.
- On success, the values from [2] are:
  - `ktc_info`: channels, sampling rate, samples per channel, number of chunks
  - `ktc_decode`: channels, sampling rate, samples per channel, then the samples (f32, one channel after another)
  - `ktc_decode_chunk`: channels, start position in the whole decoded audio (i32, may be negative), samples per channel, then the samples (f32, one channel after another)
    - The caller adds together the parts where adjacent chunks overlap.
- On failure, [2] is the length of the error message in bytes, and the message (UTF-8) follows from [3].

### License

The specification is licensed under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
The code of the reference implementation is licensed under the MIT License.
You can support KTC in your own software without asking for permission. See [LICENSE](LICENSE).

## 日本語

KTC は、主に [KT3](https://github.com/KEITO00/kt3-format) の利用を目的とした独自の音声圧縮方式です。

### 文書

- [SPEC.ja.md](SPEC.ja.md)：仕様（[English](SPEC.md)）
- [CHANGELOG.md](CHANGELOG.md)：形式の版ごとの変更

### 参照実装

このリポジトリには、復元の参照実装が入っています。外部のライブラリを使わず、Rust で書いています。

#### Rust

`Cargo.toml` の `[dependencies]` に、次の行を足してください。

```toml
ktc = { git = "https://github.com/KEITO00/ktc" }
```

- `ktc::decode(データ)` で、全体を復元します。
- いくつもの CPU で同時に復元する時は、`ktc::info` でヘッダーを読み、`ktc::decode_chunk` で塊ごとに復元して、`ktc::add_chunk` で足し合わせます。

#### WebAssembly

次のコマンドで `ktc.wasm` を作れます。Rust の `wasm32-unknown-unknown` が必要です。

```
cargo rustc --lib --release --target wasm32-unknown-unknown --crate-type cdylib
```

| 関数 | 説明 |
|---|---|
| `ktc_alloc(len)` | データを書き込む場所を `len` バイト取り、その位置を返します |
| `ktc_free(ptr, len)` | `ktc_alloc` で取った場所を解放します |
| `ktc_info(ptr, len)` | ヘッダーを読みます |
| `ktc_decode(ptr, len)` | 全体を復元します |
| `ktc_decode_chunk(ptr, len, c)` | 塊 `c` を復元します |
| `ktc_free_result(ptr)` | 結果の場所を解放します |

結果は、4 バイトずつの値の並びです。

- [0] は、並び全体の値の数です。
- [1] は、成功なら 0、失敗なら 1 です。
- 成功した時の [2] 以降は、次のとおりです。
  - `ktc_info`：チャンネル数、サンプリング周波数、1 チャンネルのサンプル数、塊の数
  - `ktc_decode`：チャンネル数、サンプリング周波数、1 チャンネルのサンプル数、サンプル（f32。チャンネルごとにまとめて並べます）
  - `ktc_decode_chunk`：チャンネル数、復元した音全体の中での開始位置（i32。負のこともあります）、1 チャンネルのサンプル数、サンプル（f32。チャンネルごとにまとめて並べます）
    - 隣の塊と重なる所は、呼ぶ側で足し合わせます。
- 失敗した時の [2] はエラーの文章のバイト数で、[3] 以降がその文章（UTF-8）です。

### ライセンス

仕様は [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/deed.ja) で公開しています。参照実装のコードは、MIT ライセンスで公開しています。許可なく、自分のソフトを KTC に対応させることができます。詳しくは [LICENSE](LICENSE) を参照してください。
