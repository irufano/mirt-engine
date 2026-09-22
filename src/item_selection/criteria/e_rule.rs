//! `Erule`/`EPrule` — maksimalkan eigenvalue minimum matriks informasi Fisher
//! kumulatif (Mulder & van der Linden, 2009). Mencegah satu dimensi laten
//! terus-menerus diabaikan: eigenvalue minimum kecil berarti ada arah di
//! ruang `theta` yang nyaris tidak terinformasi, terlepas seberapa besar
//! informasi di arah lain.

use crate::theta::InfoMatrix;

/// Nilai kriteria `Erule`: `min_eigenvalue(total_info)`. Tidak memakai
/// `weights` (lih. PLAN.md: `weights` hanya dipakai Trule/Arule/Wrule).
pub(crate) fn score(total_info: &InfoMatrix) -> f64 {
    total_info.min_eigenvalue()
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use nalgebra::DMatrix;

    #[test]
    fn picks_item_balancing_both_dimensions_over_lopsided_item() {
        // Kandidat 1: kuat di dim 1, nyaris nol di dim 2 -- eigenvalue minimum kecil.
        let lopsided =
            InfoMatrix::from_matrix(DMatrix::from_diagonal(&nalgebra::DVector::from_vec(vec![
                10.0, 0.01,
            ])))
            .unwrap();
        // Kandidat 2: seimbang di kedua dimensi -- eigenvalue minimum lebih besar.
        let balanced =
            InfoMatrix::from_matrix(DMatrix::from_diagonal(&nalgebra::DVector::from_vec(vec![
                3.0, 3.0,
            ])))
            .unwrap();
        assert!(score(&balanced) > score(&lopsided));
    }

    #[test]
    fn matches_known_min_eigenvalue() {
        let m =
            InfoMatrix::from_matrix(DMatrix::from_diagonal(&nalgebra::DVector::from_vec(vec![
                2.0, 8.0,
            ])))
            .unwrap();
        assert_relative_eq!(score(&m), 2.0, epsilon = 1e-12);
    }
}
