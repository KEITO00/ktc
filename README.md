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
- To decode from the start a little at a time, use `ktc::Stream`.
  - The samples are the same as those of `ktc::decode`.
  - `Stream::new(data, &info)` starts from the beginning.
  - `Stream::from_chunk(data, &info, c)` starts from chunk `c`. The samples are correct from the start of chunk `c`.
  - Each call of `next(&mut out)` writes the next 1,024 samples of each channel to `out` and returns the position of the first sample in the whole decoded audio.
  - Pass one `Vec<f32>` of at least 1,024 values per channel as `out`.
  - The position may be negative. Samples before 0 or after the end of the audio are not part of the audio.
  - After the last samples, `next` returns `None`.

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

KTC は、主に [KT3](https://github.com/KEITO00/kt3-format) での利用を目的とした独自の音声圧縮方式です。

### 文書

- [SPEC.ja.md](SPEC.ja.md)：仕様（[English](SPEC.md)）
- [CHANGELOG.md](CHANGELOG.md)：形式の版ごとの変更点

### 参照実装

このリポジトリには、デコーダーの参照実装を収録しています。外部のライブラリを使用せず、Rust で実装しています。

#### Rust

`Cargo.toml` の `[dependencies]` に、次の行を追加してください。

```toml
ktc = { git = "https://github.com/KEITO00/ktc" }
```

- `ktc::decode(データ)` で、全体を復元します。
- 複数の CPU で並行して復元する場合は、`ktc::info` でヘッダーを読み取り、`ktc::decode_chunk` でチャンクごとに復元して、`ktc::add_chunk` で加算してください。
- 先頭から少しずつ復元する場合は、`ktc::Stream` を使用してください。
  - 復元される値は、`ktc::decode` と同じです。
  - `Stream::new(データ, &info)` は、先頭から復元します。
  - `Stream::from_chunk(データ, &info, c)` は、チャンク `c` から復元します。正しい値になるのは、チャンク `c` の先頭以降です。
  - `next(&mut out)` を呼び出すたびに、各チャンネルの次の 1,024 サンプルを `out` に書き込み、最初のサンプルの、復元した音声全体の中での位置を返します。
  - `out` には、チャンネルごとに 1,024 個以上の値を持つ `Vec<f32>` を渡してください。
  - 位置は、負の値の場合があります。0 より前と、音声の終わりより後のサンプルは、音声に含まれません。
  - 最後まで復元すると、`next` は `None` を返します。

#### WebAssembly

次のコマンドで `ktc.wasm` を作成できます。Rust のターゲット `wasm32-unknown-unknown` が必要です。

```
cargo rustc --lib --release --target wasm32-unknown-unknown --crate-type cdylib
```

| 関数 | 説明 |
|---|---|
| `ktc_alloc(len)` | データを書き込む領域を `len` バイト確保し、そのアドレスを返します |
| `ktc_free(ptr, len)` | `ktc_alloc` で確保した領域を解放します |
| `ktc_info(ptr, len)` | ヘッダーを読み取ります |
| `ktc_decode(ptr, len)` | 全体を復元します |
| `ktc_decode_chunk(ptr, len, c)` | チャンク `c` を復元します |
| `ktc_free_result(ptr)` | 結果の領域を解放します |

結果は、4 バイトの値の配列です。

- [0] は、配列全体の値の数です。
- [1] は、成功した場合は 0、失敗した場合は 1 です。
- 成功した場合の [2] 以降は、次のとおりです。
  - `ktc_info`：チャンネル数、サンプリング周波数、1 チャンネルあたりのサンプル数、チャンク数
  - `ktc_decode`：チャンネル数、サンプリング周波数、1 チャンネルあたりのサンプル数、サンプル（f32。チャンネルごとにまとめて並べます）
  - `ktc_decode_chunk`：チャンネル数、復元した音声全体の中での開始位置（i32。負の値の場合があります）、1 チャンネルあたりのサンプル数、サンプル（f32。チャンネルごとにまとめて並べます）
    - 隣接するチャンクと重なる部分は、呼び出し側で加算してください。
- 失敗した場合の [2] はエラーメッセージのバイト数で、[3] 以降がそのメッセージ（UTF-8）です。

### ライセンス

仕様は [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/deed.ja) で公開しています。参照実装のコードは、MIT ライセンスで公開しています。許可を得ることなく、ソフトウェアに KTC を実装できます。詳しくは [LICENSE](LICENSE) を参照してください。
