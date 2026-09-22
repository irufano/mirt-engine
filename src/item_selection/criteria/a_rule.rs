//! `Arule`/`APrule` — minimum (potensial berbobot) trace kovarians asimtotik
//! `theta_hat` (van der Linden, 1999): meminimalkan
//! `sum_i weights[i] * inverse(total_info)[i,i]`, disajikan sudah dinegasikan
//! supaya konsisten dengan konvensi argmax dipakai seluruh kriteria lain.
//!
//! Info Fisher dari satu item dikotomus tunggal di `nfact > 1` selalu **rank
//! 1** (outer product gradien terhadap satu skalar), sehingga `total_info`
//! bisa singular -- lazim terjadi sebelum item pertama dijawab. Karena itu
//! dipakai **pseudo-invers** (`InfoMatrix::pseudo_inverse`, via SVD), bukan
//! invers biasa -- perilaku ini diverifikasi cocok numerik dengan
//! `mirtCAT::computeCriteria(criteria = "Arule")` pada eksperimen desain
//! Fase 2 (lih. `PLAN.md`).

use crate::error::MirtCatError;
use crate::theta::InfoMatrix;

/// Ambang singular value yang dianggap nol pada pseudo-invers -- cukup kecil
/// untuk tidak memotong informasi asli, cukup besar untuk meredam noise
/// floating-point pada arah yang secara matematis nol (mis. matriks rank 1).
pub(crate) const PSEUDO_INVERSE_EPS: f64 = 1e-10;

/// Nilai kriteria `Arule`: `-sum_i weights[i] * pseudo_inverse(total_info)[i,i]`.
pub(crate) fn score(total_info: &InfoMatrix, weights: &[f64]) -> Result<f64, MirtCatError> {
    let pinv = total_info.pseudo_inverse(PSEUDO_INVERSE_EPS);
    Ok(-pinv.trace_weighted(weights)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use nalgebra::DMatrix;

    #[test]
    fn matches_negative_weighted_trace_of_inverse_for_full_rank_matrix() {
        let m =
            InfoMatrix::from_matrix(DMatrix::from_diagonal(&nalgebra::DVector::from_vec(vec![
                2.0, 8.0,
            ])))
            .unwrap();
        // invers diketahui manual: diag(0.5, 0.125)
        let expected = -(1.0 * 0.5 + 0.5 * 0.125);
        assert_relative_eq!(score(&m, &[1.0, 0.5]).unwrap(), expected, epsilon = 1e-9);
    }

    #[test]
    fn handles_rank_deficient_total_info_gracefully() {
        // total_info rank 1 (khas: baru 1 kandidat, belum ada item terjawab)
        let a = [10.0_f64.sqrt(), 0.1_f64.sqrt()];
        let m = InfoMatrix::from_matrix(DMatrix::from_row_slice(
            2,
            2,
            &[a[0] * a[0], a[0] * a[1], a[0] * a[1], a[1] * a[1]],
        ))
        .unwrap();
        let val = score(&m, &[0.5, 2.0]).unwrap();
        // Nilai referensi dari eksperimen verifikasi terhadap mirtCAT (lih. PLAN.md).
        assert_relative_eq!(val, -0.05097539456915992, epsilon = 1e-8);
    }

    #[test]
    fn weights_dimension_mismatch_errors() {
        let m = InfoMatrix::identity(2);
        assert!(score(&m, &[1.0]).is_err());
    }
}
