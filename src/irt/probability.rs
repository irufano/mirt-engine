//! Fungsi probabilitas dasar dipakai lintas tipe item — padanan
//! `probtrace()`/`itemTrace()` di `mirt` (`mirt/src/traceLinePts.cpp`).

/// Batas absolut nilai logit `z` sebelum dilewatkan ke fungsi logistik,
/// menyamai konstanta `ABS_MAX_Z` di `mirt/src/Misc.h` — mencegah overflow
/// numerik `exp()` untuk `theta` ekstrem tanpa mengubah hasil praktis
/// (`logistic(±35)` sudah menyaturkan probabilitas ke `~0`/`~1`).
pub(crate) const ABS_MAX_Z: f64 = 35.0;

/// Fungsi logistik standar `1 / (1 + exp(-z))`, dengan `z` diklem ke
/// `[-ABS_MAX_Z, ABS_MAX_Z]` lebih dulu.
pub(crate) fn logistic(z: f64) -> f64 {
    let z = z.clamp(-ABS_MAX_Z, ABS_MAX_Z);
    1.0 / (1.0 + (-z).exp())
}

/// Logit linear multidimensional `z = d + a · theta` (+ intercept opsional
/// dari `theta` terobservasi lain bila diperlukan fase selanjutnya).
pub(crate) fn linear_predictor(a: &[f64], d: f64, theta: &[f64]) -> f64 {
    debug_assert_eq!(a.len(), theta.len());
    d + a.iter().zip(theta).map(|(aj, tj)| aj * tj).sum::<f64>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn logistic_at_zero_is_half() {
        assert_relative_eq!(logistic(0.0), 0.5);
    }

    #[test]
    fn logistic_saturates_for_extreme_z() {
        assert!(logistic(1000.0) > 1.0 - 1e-12);
        assert!(logistic(-1000.0) < 1e-12);
    }

    #[test]
    fn linear_predictor_matches_dot_product_plus_intercept() {
        let a = [1.0, -2.0, 0.5];
        let theta = [0.5, 1.0, -1.0];
        let z = linear_predictor(&a, 0.25, &theta);
        assert_relative_eq!(z, 0.25 + (1.0 * 0.5 + -2.0 * 1.0 - 0.5));
    }
}
