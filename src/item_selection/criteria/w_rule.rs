//! `Wrule`/`WPrule` — kriteria informasi berbobot kustom: maksimalkan bentuk
//! kuadratik `weights' * total_info * weights`. Berbeda dari `Trule` (yang
//! hanya menjumlahkan diagonal berbobot), `Wrule` memasukkan seluruh
//! off-diagonal matriks informasi -- setara informasi Fisher tentang skor
//! komposit `weights' * theta`, cocok saat pengambil keputusan peduli pada
//! kombinasi linear tertentu dari dimensi laten, bukan tiap dimensi secara
//! independen.

use crate::error::MirtCatError;
use crate::theta::InfoMatrix;

/// Nilai kriteria `Wrule`: `weights' * total_info * weights`.
pub(crate) fn score(total_info: &InfoMatrix, weights: &[f64]) -> Result<f64, MirtCatError> {
    total_info.quadratic_form(weights)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use nalgebra::DMatrix;

    #[test]
    fn matches_quadratic_form_and_differs_from_plain_trace() {
        let m =
            InfoMatrix::from_matrix(DMatrix::from_row_slice(2, 2, &[10.0, 1.0, 1.0, 0.1])).unwrap();
        let w = score(&m, &[0.5, 2.0]).unwrap();
        assert_relative_eq!(w, 4.9, epsilon = 1e-9);
        let plain_trace = m.trace_weighted(&[0.5, 2.0]).unwrap();
        assert!((w - plain_trace).abs() > 1e-6);
    }

    #[test]
    fn custom_weights_change_the_winning_candidate() {
        let strong_dim1 =
            InfoMatrix::from_matrix(DMatrix::from_diagonal(&nalgebra::DVector::from_vec(vec![
                10.0, 0.1,
            ])))
            .unwrap();
        let strong_dim2 =
            InfoMatrix::from_matrix(DMatrix::from_diagonal(&nalgebra::DVector::from_vec(vec![
                0.1, 10.0,
            ])))
            .unwrap();
        assert!(
            score(&strong_dim1, &[1.0, 0.0]).unwrap() > score(&strong_dim2, &[1.0, 0.0]).unwrap()
        );
        assert!(
            score(&strong_dim2, &[0.0, 1.0]).unwrap() > score(&strong_dim1, &[0.0, 1.0]).unwrap()
        );
    }
}
