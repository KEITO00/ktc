//! 雑音の時間の形を整える (TNS、版 3)。
//! 窓の係数に周波数の向きの予測のフィルターをかけた値を記録し、復元で逆のフィルターをかける。
//! 丸めの雑音や、高い音を作り直す雑音が、窓の中の音の大きさの移り変わりに沿うので、
//! 音の立ち上がりの前の静かな所に雑音がにじまない。
//! 周波数を RANGES_HZ で 3 つの範囲に分け、窓ごと・範囲ごとにフィルターを 1 つ持てる (左右のチャンネルで共通)。
//! 範囲を分けるのは、ほかの音が鳴り続けている中で高い音だけが立ち上がる時に、
//! 全体で 1 つのフィルターでは鳴り続ける音に埋もれて、立ち上がりの形を捉えられないため

use crate::transform::M;

/// フィルターの次数 (長い窓、短い窓)
pub const ORDER_LONG: usize = 8;
pub const ORDER_SHORT: usize = 4;
/// 記録する反射係数の値の範囲 (-STEPS から STEPS)。値 i は sin(i π / 16) を表す
pub const STEPS: i32 = 7;
/// 範囲の数と、範囲ごとの始まりの周波数 (Hz)。範囲は次の範囲の始まり (最後は一番上) まで
pub const RANGES: usize = 3;
pub const RANGES_HZ: [f64; RANGES] = [100.0, 1000.0, 4000.0];

/// sin(i π / 16) (i = -7 から 7)。どの環境でも同じ値になるよう、計算せずに表で持つ
const K: [f64; 15] = [
    -0.980_785_280_403_230_4,
    -0.923_879_532_511_286_7,
    -0.831_469_612_302_545_2,
    -0.707_106_781_186_547_5,
    -0.555_570_233_019_602_2,
    -0.382_683_432_365_089_8,
    -0.195_090_322_016_128_3,
    0.0,
    0.195_090_322_016_128_3,
    0.382_683_432_365_089_8,
    0.555_570_233_019_602_2,
    0.707_106_781_186_547_5,
    0.831_469_612_302_545_2,
    0.923_879_532_511_286_7,
    0.980_785_280_403_230_4,
];

/// 窓の長さ (係数の数 m) に合わせたフィルターの次数
pub fn order(m: usize) -> usize {
    if m == M { ORDER_LONG } else { ORDER_SHORT }
}

/// 反射係数の値に対応する反射係数
pub fn reflection(i: i32) -> f64 {
    K[(i.clamp(-STEPS, STEPS) + STEPS) as usize]
}

/// 記録した反射係数の値から、予測のフィルター (a[j] が j+1 個前の係数に掛かる) を作る
pub fn lpc(idx: &[i32]) -> Vec<f64> {
    let mut a: Vec<f64> = Vec::with_capacity(idx.len());
    for (i, &q) in idx.iter().enumerate() {
        let k = reflection(q);
        let prev = a.clone();
        for t in 0..i {
            a[t] = prev[t] + k * prev[i - 1 - t];
        }
        a.push(k);
    }
    a
}

/// 復元側: 記録した値 e から係数 x を戻す (x[k] = e[k] - Σ a[j] x[k-1-j])
pub fn synthesize(x: &mut [f64], a: &[f64]) {
    for k in 0..x.len() {
        let mut v = x[k];
        for (j, &aj) in a.iter().enumerate().take(k) {
            v -= aj * x[k - 1 - j];
        }
        x[k] = v;
    }
}

/// 書き込み側: 係数 x にフィルターをかけて、記録する値にする (e[k] = x[k] + Σ a[j] x[k-1-j])
pub fn analyze(x: &mut [f64], a: &[f64]) {
    for k in (0..x.len()).rev() {
        let mut v = x[k];
        for (j, &aj) in a.iter().enumerate().take(k) {
            v += aj * x[k - 1 - j];
        }
        x[k] = v;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_sine_of_sixteenths() {
        for i in -STEPS..=STEPS {
            assert!((reflection(i) - (i as f64 * std::f64::consts::PI / 16.0).sin()).abs() < 1e-15);
        }
    }

    #[test]
    fn synthesis_undoes_analysis() {
        let x: Vec<f64> = (0..200).map(|k| ((k * 37 % 101) as f64 - 50.0) / 7.0).collect();
        let a = lpc(&[5, -3, 2, 0, -1, 4, 1, -2]);
        let mut y = x.clone();
        analyze(&mut y, &a);
        synthesize(&mut y, &a);
        for (p, q) in x.iter().zip(&y) {
            assert!((p - q).abs() < 1e-9);
        }
    }
}
