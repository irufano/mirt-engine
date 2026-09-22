//! Agregasi informasi Fisher kumulatif lintas item — dasar `info_thetas`
//! yang dipakai kriteria seleksi item `Drule`/`Trule`/`Arule`/`Erule`/
//! `Wrule` di Fase 2 (`mirtCAT/R/DesignClass.R`, matriks informasi diperbarui
//! tiap item dijawab).

use super::model::ItemResponseModel;
use crate::error::MirtCatError;
use crate::theta::{InfoMatrix, ThetaVector};

/// Jumlahkan matriks informasi Fisher `nfact × nfact` dari sekumpulan item
/// pada satu titik `theta` yang sama. Mengembalikan matriks nol bila daftar
/// item kosong (mis. sebelum item pertama dijawab).
pub fn cumulative_information<'a>(
    nfact: usize,
    items: impl IntoIterator<Item = &'a dyn ItemResponseModel>,
    theta: &ThetaVector,
) -> Result<InfoMatrix, MirtCatError> {
    theta.validate_nfact(nfact, "cumulative_information")?;
    let mut acc = InfoMatrix::zeros(nfact);
    for item in items {
        if item.nfact() != nfact {
            return Err(MirtCatError::DimensionMismatch {
                expected: nfact,
                actual: item.nfact(),
                context: "cumulative_information: item nfact mismatch",
            });
        }
        let info = item.information(theta)?;
        acc += &info;
    }
    Ok(acc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::irt::dichotomous::Dichotomous;
    use approx::assert_relative_eq;

    #[test]
    fn empty_items_yields_zero_matrix() {
        let theta = ThetaVector::zeros(2);
        let info = cumulative_information(2, std::iter::empty(), &theta).unwrap();
        assert_relative_eq!(info.trace(), 0.0);
    }

    #[test]
    fn cumulative_information_sums_per_item_matrices() {
        let item1 = Dichotomous::m2pl(vec![1.0, 0.0], 0.0).unwrap();
        let item2 = Dichotomous::m2pl(vec![0.0, 1.0], 0.0).unwrap();
        let theta = ThetaVector::zeros(2);

        let expected_single = item1.information(&theta).unwrap();
        let items: Vec<&dyn ItemResponseModel> = vec![&item1, &item2];
        let total = cumulative_information(2, items, &theta).unwrap();

        // Item1 hanya berkontribusi pada dimensi 1, item2 pada dimensi 2, tidak overlap.
        assert_relative_eq!(total.get(0, 0), expected_single.get(0, 0), epsilon = 1e-10);
        assert_relative_eq!(
            total.get(1, 1),
            item2.information(&theta).unwrap().get(1, 1),
            epsilon = 1e-10
        );
    }

    #[test]
    fn mismatched_item_nfact_errors() {
        let item = Dichotomous::m2pl(vec![1.0], 0.0).unwrap(); // nfact = 1
        let theta = ThetaVector::zeros(2);
        let items: Vec<&dyn ItemResponseModel> = vec![&item];
        assert!(cumulative_information(2, items, &theta).is_err());
    }
}
