//! `Drule`/`DPrule` — D-optimality: maksimalkan determinan matriks informasi
//! Fisher kumulatif (Segall, 1996). Determinan matriks informasi berbanding
//! terbalik dengan volume elipsoid kepercayaan asimtotik `theta`; memaksimalkan
//! determinan meminimalkan volume itu.

use crate::theta::InfoMatrix;

/// Nilai kriteria `Drule`: `det(total_info)`. `total_info` sudah mencakup
/// info kumulatif dari item terjawab + info kandidat (+ prior bila `*Prule`)
/// — dibentuk oleh caller (`select_next_item`).
pub(crate) fn score(total_info: &InfoMatrix) -> f64 {
    total_info.determinant()
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use nalgebra::{DMatrix, DVector};

    #[test]
    fn determinant_increases_as_high_information_item_added() {
        let low =
            InfoMatrix::from_matrix(DMatrix::from_diagonal(&DVector::from_vec(vec![0.1, 0.1])))
                .unwrap();
        let high =
            InfoMatrix::from_matrix(DMatrix::from_diagonal(&DVector::from_vec(vec![5.0, 5.0])))
                .unwrap();
        assert!(score(&high) > score(&low));
    }

    #[test]
    fn matches_known_determinant() {
        let m =
            InfoMatrix::from_matrix(DMatrix::from_row_slice(2, 2, &[3.0, 1.0, 1.0, 2.0])).unwrap();
        assert_relative_eq!(score(&m), 3.0 * 2.0 - 1.0 * 1.0, epsilon = 1e-12);
    }
}
