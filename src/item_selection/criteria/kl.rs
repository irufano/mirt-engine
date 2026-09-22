//! `KL`/`KLn` — Kullback-Leibler pointwise (Chang & Ying, 1996). Berbeda
//! dari D/T/A/E/W-rule, KL tidak memakai matriks informasi kumulatif sama
//! sekali -- hanya distribusi probabilitas kategori **item kandidat sendiri**
//! pada dua titik `theta - delta` dan `theta + delta` (delta skalar,
//! di-broadcast rata ke semua dimensi). `KLn` memakai interval yang melebar
//! seiring tes berjalan (`delta * sqrt(n_answered)`), mendekati pendekatan
//! informasi global tanpa perlu integrasi kuadratur eksplisit.

use crate::error::MirtCatError;
use crate::irt::ItemResponseModel;
use crate::theta::ThetaVector;

/// Nilai kriteria `KL` untuk satu item kandidat pada titik `theta`, dengan
/// jarak `delta` (skalar, sama di semua dimensi):
/// `sum_k p1_k * (ln(p1_k) - ln(p0_k))`, `p0 = probs(theta - delta)`,
/// `p1 = probs(theta + delta)`.
pub(crate) fn score(
    item: &dyn ItemResponseModel,
    theta: &ThetaVector,
    delta: f64,
) -> Result<f64, MirtCatError> {
    let lower = ThetaVector::from_vec(theta.as_slice().iter().map(|t| t - delta).collect());
    let upper = ThetaVector::from_vec(theta.as_slice().iter().map(|t| t + delta).collect());
    let p0 = item.probabilities(&lower)?;
    let p1 = item.probabilities(&upper)?;
    Ok(p1
        .iter()
        .zip(p0.iter())
        .map(|(p1_k, p0_k)| p1_k * (p1_k.ln() - p0_k.ln()))
        .sum())
}

/// Delta efektif untuk `KLn`: `delta * sqrt(n_answered)` — interval melebar
/// seiring jumlah item terjawab bertambah.
pub(crate) fn scaled_delta(delta: f64, n_answered: usize) -> f64 {
    delta * (n_answered as f64).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::irt::Dichotomous;
    use approx::assert_relative_eq;

    #[test]
    fn score_is_zero_at_zero_delta_and_nonnegative_otherwise() {
        // KL(P||P) = 0 di delta=0; untuk delta != 0 nilainya adalah
        // divergensi KL sungguhan (KL(p1 || p0)), yang selalu >= 0
        // (Gibbs' inequality) -- catatan: KL TIDAK simetris terhadap tanda
        // delta secara umum (membalik delta menukar arah KL(p1||p0) jadi
        // KL(p0||p1), yang berbeda kecuali pada titik simetri item).
        let item = Dichotomous::m2pl(vec![1.2, 0.8], 0.1).unwrap();
        let theta = ThetaVector::from_vec(vec![0.3, -0.2]);
        assert_relative_eq!(score(&item, &theta, 0.0).unwrap(), 0.0, epsilon = 1e-12);
        assert!(score(&item, &theta, 0.15).unwrap() >= 0.0);
        assert!(score(&item, &theta, -0.15).unwrap() >= 0.0);
    }

    #[test]
    fn scaled_delta_widens_with_n_answered() {
        let base = 0.1;
        assert_relative_eq!(scaled_delta(base, 0), 0.0);
        assert_relative_eq!(scaled_delta(base, 1), 0.1);
        assert_relative_eq!(scaled_delta(base, 4), 0.2);
        assert_relative_eq!(scaled_delta(base, 9), 0.3);
        assert!(scaled_delta(base, 9) > scaled_delta(base, 4));
    }

    #[test]
    fn larger_delta_yields_larger_kl_for_discriminating_item() {
        let item = Dichotomous::m2pl(vec![1.5, 0.0], 0.0).unwrap();
        let theta = ThetaVector::from_vec(vec![0.0, 0.0]);
        let small = score(&item, &theta, 0.05).unwrap();
        let large = score(&item, &theta, 0.3).unwrap();
        assert!(large > small);
    }
}
