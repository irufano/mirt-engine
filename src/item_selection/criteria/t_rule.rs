//! `Trule`/`TPrule` — maksimalkan trace (potensial berbobot) matriks
//! informasi Fisher kumulatif (Mulder & van der Linden, 2009). Trace adalah
//! jumlah informasi per-dimensi (diagonal saja) — kriteria termurah secara
//! komputasi di antara D/A/E/W-rule karena tidak butuh determinan/invers/
//! eigenvalue.

use crate::error::MirtCatError;
use crate::theta::InfoMatrix;

/// Nilai kriteria `Trule`: `sum_i weights[i] * total_info[i,i]`. Hanya
/// elemen diagonal yang dipakai (off-diagonal diabaikan) — beda dengan
/// `Wrule` yang memakai bentuk kuadratik penuh.
pub(crate) fn score(total_info: &InfoMatrix, weights: &[f64]) -> Result<f64, MirtCatError> {
    total_info.trace_weighted(weights)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use nalgebra::DMatrix;

    #[test]
    fn weighted_and_unweighted_trace_differ_per_weights() {
        let m =
            InfoMatrix::from_matrix(DMatrix::from_diagonal(&nalgebra::DVector::from_vec(vec![
                2.0, 8.0,
            ])))
            .unwrap();
        let unweighted = score(&m, &[1.0, 1.0]).unwrap();
        let weighted = score(&m, &[1.0, 0.0]).unwrap();
        assert_relative_eq!(unweighted, 10.0, epsilon = 1e-12);
        assert_relative_eq!(weighted, 2.0, epsilon = 1e-12);
        assert!(unweighted != weighted);
    }

    #[test]
    fn ignores_off_diagonal() {
        let m = InfoMatrix::from_matrix(DMatrix::from_row_slice(2, 2, &[3.0, 100.0, 100.0, 2.0]))
            .unwrap();
        assert_relative_eq!(score(&m, &[1.0, 1.0]).unwrap(), 5.0, epsilon = 1e-12);
    }
}
