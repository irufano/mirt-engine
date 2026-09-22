//! Tipe data inti untuk vektor kemampuan (`theta`), standard error, dan
//! matriks informasi Fisher — newtype tipis di atas `nalgebra::DVector`/
//! `DMatrix` supaya semantik MCAT (mis. "hanya diagonal SE", "matriks
//! informasi harus persegi `nfact × nfact`") terjaga type-safe di seluruh
//! fase berikutnya.

use nalgebra::{DMatrix, DVector};
use std::ops::{Add, AddAssign, Index};

use crate::error::MirtCatError;

/// Vektor kemampuan multidimensional `theta`, panjang `nfact`.
#[derive(Debug, Clone, PartialEq)]
pub struct ThetaVector(DVector<f64>);

/// Vektor standard error per-dimensi (hanya diagonal kovarians), panjang `nfact`.
#[derive(Debug, Clone, PartialEq)]
pub struct SEVector(DVector<f64>);

/// Matriks informasi Fisher `nfact × nfact` (atau matriks kovarians populasi
/// `gcov`) — selalu persegi.
#[derive(Debug, Clone, PartialEq)]
pub struct InfoMatrix(DMatrix<f64>);

macro_rules! impl_vector_newtype {
    ($ty:ident) => {
        impl $ty {
            /// Vektor nol sepanjang `nfact`.
            pub fn zeros(nfact: usize) -> Self {
                Self(DVector::zeros(nfact))
            }

            /// Bangun dari nilai eksplisit.
            pub fn from_vec(values: Vec<f64>) -> Self {
                Self(DVector::from_vec(values))
            }

            /// Jumlah dimensi (`nfact`).
            pub fn nfact(&self) -> usize {
                self.0.len()
            }

            /// Pandangan sebagai slice `&[f64]`.
            pub fn as_slice(&self) -> &[f64] {
                self.0.as_slice()
            }

            /// Akses vektor `nalgebra` mentah (untuk dipakai lintas modul internal).
            pub fn as_dvector(&self) -> &DVector<f64> {
                &self.0
            }

            /// Validasi bahwa panjang vektor sama dengan `nfact` yang diharapkan.
            pub fn validate_nfact(
                &self,
                expected: usize,
                context: &'static str,
            ) -> Result<(), MirtCatError> {
                if self.nfact() != expected {
                    return Err(MirtCatError::DimensionMismatch {
                        expected,
                        actual: self.nfact(),
                        context,
                    });
                }
                Ok(())
            }
        }

        impl Index<usize> for $ty {
            type Output = f64;

            fn index(&self, i: usize) -> &f64 {
                &self.0[i]
            }
        }

        impl From<DVector<f64>> for $ty {
            fn from(v: DVector<f64>) -> Self {
                Self(v)
            }
        }

        impl From<&[f64]> for $ty {
            fn from(v: &[f64]) -> Self {
                Self(DVector::from_row_slice(v))
            }
        }

        // Serialisasi manual sebagai `Vec<f64>` polos (bukan derive di atas
        // `nalgebra::DVector`) -- menghindari perlu mengaktifkan fitur
        // `nalgebra/serde-serialize` (dependency tambahan) hanya untuk newtype
        // tipis ini; `from_vec`/`as_slice` yang sudah ada di atas dipakai
        // ulang langsung (Prinsip Desain #1 PLAN.md). Dipakai Fase 7
        // (`McatResult`).
        #[cfg(feature = "serde")]
        impl serde::Serialize for $ty {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                self.as_slice().serialize(serializer)
            }
        }

        #[cfg(feature = "serde")]
        impl<'de> serde::Deserialize<'de> for $ty {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                Vec::<f64>::deserialize(deserializer).map(Self::from_vec)
            }
        }
    };
}

impl_vector_newtype!(ThetaVector);
impl_vector_newtype!(SEVector);

impl SEVector {
    /// SE per-dimensi dari akar diagonal sebuah matriks kovarians/informasi —
    /// **hanya** diagonal yang diambil, bukan seluruh matriks (lih. `mirtCAT`
    /// `PersonClass.R`: `SE <- sqrt(diag(gcov))`).
    pub fn from_covariance_diagonal(cov: &InfoMatrix) -> Self {
        let diag: Vec<f64> = (0..cov.nfact()).map(|i| cov.0[(i, i)].sqrt()).collect();
        Self::from_vec(diag)
    }
}

impl InfoMatrix {
    /// Matriks nol persegi `nfact × nfact`.
    pub fn zeros(nfact: usize) -> Self {
        Self(DMatrix::zeros(nfact, nfact))
    }

    /// Bangun dari `nalgebra::DMatrix` persegi. Error bila bukan persegi.
    pub fn from_matrix(m: DMatrix<f64>) -> Result<Self, MirtCatError> {
        if m.nrows() != m.ncols() {
            return Err(MirtCatError::InvalidParameter(format!(
                "InfoMatrix harus persegi, didapat {}x{}",
                m.nrows(),
                m.ncols()
            )));
        }
        Ok(Self(m))
    }

    /// Matriks identitas `nfact × nfact`.
    pub fn identity(nfact: usize) -> Self {
        Self(DMatrix::identity(nfact, nfact))
    }

    /// Jumlah dimensi (`nfact`).
    pub fn nfact(&self) -> usize {
        self.0.nrows()
    }

    /// Akses matriks `nalgebra` mentah.
    pub fn as_dmatrix(&self) -> &DMatrix<f64> {
        &self.0
    }

    /// Elemen `(i, j)`.
    pub fn get(&self, i: usize, j: usize) -> f64 {
        self.0[(i, j)]
    }

    /// Determinan matriks informasi — dipakai kriteria `Drule` (Fase 2).
    pub fn determinant(&self) -> f64 {
        self.0.clone().determinant()
    }

    /// Trace (jumlah diagonal) — dipakai kriteria `Trule` (Fase 2).
    pub fn trace(&self) -> f64 {
        self.0.trace()
    }

    /// Trace berbobot: `sum_i weights[i] * info[i,i]` — dipakai `Trule`
    /// (dan `Arule`, atas pseudo-invers matriks informasi).
    pub fn trace_weighted(&self, weights: &[f64]) -> Result<f64, MirtCatError> {
        if weights.len() != self.nfact() {
            return Err(MirtCatError::DimensionMismatch {
                expected: self.nfact(),
                actual: weights.len(),
                context: "InfoMatrix::trace_weighted",
            });
        }
        Ok((0..self.nfact()).map(|i| weights[i] * self.0[(i, i)]).sum())
    }

    /// Bentuk kuadratik `weights' * info * weights` (mencakup off-diagonal,
    /// beda dengan `trace_weighted` yang hanya memakai diagonal) — dipakai
    /// kriteria `Wrule`: informasi Fisher tentang skor komposit berbobot
    /// `weights' * theta`.
    pub fn quadratic_form(&self, weights: &[f64]) -> Result<f64, MirtCatError> {
        if weights.len() != self.nfact() {
            return Err(MirtCatError::DimensionMismatch {
                expected: self.nfact(),
                actual: weights.len(),
                context: "InfoMatrix::quadratic_form",
            });
        }
        let nfact = self.nfact();
        let mut total = 0.0;
        for i in 0..nfact {
            for j in 0..nfact {
                total += weights[i] * weights[j] * self.0[(i, j)];
            }
        }
        Ok(total)
    }

    /// Invers matriks — dipakai kriteria `Arule` (negatif trace invers) dan
    /// varian `*Prule` (`inverse(gcov)` ditambahkan sebagai informasi prior).
    pub fn inverse(&self) -> Result<InfoMatrix, MirtCatError> {
        self.0.clone().try_inverse().map(InfoMatrix).ok_or_else(|| {
            MirtCatError::InvalidParameter("matriks informasi singular, tidak bisa diinvers".into())
        })
    }

    /// Eigenvalue minimum (matriks simetris) — dipakai kriteria `Erule`.
    pub fn min_eigenvalue(&self) -> f64 {
        let sym = nalgebra::SymmetricEigen::new(self.0.clone());
        sym.eigenvalues
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min)
    }

    /// Pseudo-invers Moore-Penrose (via SVD) — dipakai kriteria `Arule`/
    /// `APrule` (Fase 2), yang perlu robust terhadap matriks informasi
    /// rank-defisien: info Fisher dari satu item dikotomus tunggal di ruang
    /// `nfact > 1` selalu **rank 1** (outer product gradien), sehingga
    /// singular dan tidak punya invers biasa — kasus umum sebelum item
    /// pertama dijawab. `eps` adalah ambang singular value yang dianggap
    /// nol; nilai singular di bawah `eps` diperlakukan sebagai arah yang
    /// belum terinformasi (kontribusi nol ke pseudo-invers).
    pub fn pseudo_inverse(&self, eps: f64) -> InfoMatrix {
        let nfact = self.nfact();
        InfoMatrix(
            self.0
                .clone()
                .pseudo_inverse(eps)
                .unwrap_or_else(|_| DMatrix::zeros(nfact, nfact)),
        )
    }
}

// Serialisasi manual sebagai `Vec<Vec<f64>>` row-major (bukan derive di atas
// `nalgebra::DMatrix`) -- pola sama persis seperti `ThetaVector`/`SEVector`
// di atas (menghindari dependency `nalgebra/serde-serialize`). Dipakai Fase 8
// (`ItemSelectionDebug::info_before`/`info_after`, bagian `McatSession::debug_log`).
#[cfg(feature = "serde")]
impl serde::Serialize for InfoMatrix {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let rows: Vec<Vec<f64>> = (0..self.nfact())
            .map(|i| (0..self.nfact()).map(|j| self.0[(i, j)]).collect())
            .collect();
        rows.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for InfoMatrix {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let rows = Vec::<Vec<f64>>::deserialize(deserializer)?;
        let nfact = rows.len();
        let mut m = DMatrix::zeros(nfact, nfact);
        for (i, row) in rows.into_iter().enumerate() {
            if row.len() != nfact {
                return Err(serde::de::Error::custom(format!(
                    "InfoMatrix::deserialize: baris {i} panjang {} (diharapkan {nfact}, matriks \
                     harus persegi)",
                    row.len()
                )));
            }
            for (j, v) in row.into_iter().enumerate() {
                m[(i, j)] = v;
            }
        }
        Ok(InfoMatrix(m))
    }
}

impl Add for InfoMatrix {
    type Output = InfoMatrix;

    fn add(self, rhs: InfoMatrix) -> InfoMatrix {
        InfoMatrix(self.0 + rhs.0)
    }
}

impl AddAssign<&InfoMatrix> for InfoMatrix {
    fn add_assign(&mut self, rhs: &InfoMatrix) {
        self.0 += &rhs.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    #[cfg(feature = "serde")]
    fn theta_vector_and_se_vector_serde_roundtrip_as_plain_json_array() {
        let theta = ThetaVector::from_vec(vec![0.5, -1.2, 0.0]);
        let json = serde_json::to_string(&theta).unwrap();
        assert_eq!(json, "[0.5,-1.2,0.0]");
        let back: ThetaVector = serde_json::from_str(&json).unwrap();
        assert_eq!(back, theta);

        let se = SEVector::from_vec(vec![0.3, 0.4]);
        let json = serde_json::to_string(&se).unwrap();
        let back: SEVector = serde_json::from_str(&json).unwrap();
        assert_eq!(back, se);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn info_matrix_serde_roundtrip_as_row_major_nested_array() {
        let m =
            InfoMatrix::from_matrix(DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0])).unwrap();
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, "[[1.0,2.0],[3.0,4.0]]");
        let back: InfoMatrix = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn info_matrix_deserialize_rejects_non_square_rows() {
        let err = serde_json::from_str::<InfoMatrix>("[[1.0,2.0],[3.0]]").unwrap_err();
        assert!(err.to_string().contains("persegi"));
    }

    #[test]
    fn theta_vector_zeros_has_correct_nfact() {
        let t = ThetaVector::zeros(3);
        assert_eq!(t.nfact(), 3);
        assert_eq!(t.as_slice(), &[0.0, 0.0, 0.0]);
    }

    #[test]
    fn theta_vector_validate_nfact_mismatch_errors() {
        let t = ThetaVector::from_vec(vec![0.1, 0.2]);
        let err = t.validate_nfact(3, "test").unwrap_err();
        assert_eq!(
            err,
            MirtCatError::DimensionMismatch {
                expected: 3,
                actual: 2,
                context: "test",
            }
        );
    }

    #[test]
    fn se_vector_from_covariance_diagonal_ignores_off_diagonal() {
        let cov = InfoMatrix::from_matrix(DMatrix::from_row_slice(2, 2, &[4.0, 100.0, 100.0, 9.0]))
            .unwrap();
        let se = SEVector::from_covariance_diagonal(&cov);
        assert_relative_eq!(se[0], 2.0);
        assert_relative_eq!(se[1], 3.0);
    }

    #[test]
    fn info_matrix_identity_ops() {
        let id = InfoMatrix::identity(3);
        assert_relative_eq!(id.determinant(), 1.0);
        assert_relative_eq!(id.trace(), 3.0);
        assert_relative_eq!(id.min_eigenvalue(), 1.0);
        let inv = id.inverse().unwrap();
        assert_relative_eq!(inv.get(0, 0), 1.0);
    }

    #[test]
    fn info_matrix_trace_weighted() {
        let m = InfoMatrix::from_matrix(DMatrix::from_diagonal(&DVector::from_vec(vec![2.0, 4.0])))
            .unwrap();
        let w = m.trace_weighted(&[1.0, 0.5]).unwrap();
        assert_relative_eq!(w, 2.0 * 1.0 + 4.0 * 0.5);
        assert!(m.trace_weighted(&[1.0]).is_err());
    }

    #[test]
    fn info_matrix_diagonal_min_eigenvalue_and_inverse() {
        let m = InfoMatrix::from_matrix(DMatrix::from_diagonal(&DVector::from_vec(vec![2.0, 8.0])))
            .unwrap();
        assert_relative_eq!(m.min_eigenvalue(), 2.0);
        let inv = m.inverse().unwrap();
        assert_relative_eq!(inv.get(0, 0), 0.5);
        assert_relative_eq!(inv.get(1, 1), 0.125);
    }

    #[test]
    fn info_matrix_quadratic_form_includes_off_diagonal() {
        // Nilai referensi diverifikasi numerik terhadap mirtCAT::computeCriteria
        // (Wrule) pada eksperimen desain Fase 2 (lih. PLAN.md).
        let m =
            InfoMatrix::from_matrix(DMatrix::from_row_slice(2, 2, &[10.0, 1.0, 1.0, 0.1])).unwrap();
        let q = m.quadratic_form(&[0.5, 2.0]).unwrap();
        assert_relative_eq!(q, 4.9, epsilon = 1e-9);
        // Beda dari trace_weighted (yang mengabaikan off-diagonal).
        let t = m.trace_weighted(&[0.5, 2.0]).unwrap();
        assert!((q - t).abs() > 1e-6);
        assert!(m.quadratic_form(&[1.0]).is_err());
    }

    #[test]
    fn info_matrix_add_assign_accumulates() {
        let mut acc = InfoMatrix::zeros(2);
        let a = InfoMatrix::from_matrix(DMatrix::from_diagonal(&DVector::from_vec(vec![1.0, 2.0])))
            .unwrap();
        acc += &a;
        acc += &a;
        assert_relative_eq!(acc.get(0, 0), 2.0);
        assert_relative_eq!(acc.get(1, 1), 4.0);
    }

    #[test]
    fn non_square_matrix_is_rejected() {
        let m = DMatrix::from_row_slice(1, 2, &[1.0, 2.0]);
        assert!(InfoMatrix::from_matrix(m).is_err());
    }

    #[test]
    fn pseudo_inverse_matches_regular_inverse_for_full_rank_matrix() {
        let m = InfoMatrix::from_matrix(DMatrix::from_diagonal(&DVector::from_vec(vec![2.0, 8.0])))
            .unwrap();
        let inv = m.inverse().unwrap();
        let pinv = m.pseudo_inverse(1e-10);
        assert_relative_eq!(pinv.get(0, 0), inv.get(0, 0), epsilon = 1e-10);
        assert_relative_eq!(pinv.get(1, 1), inv.get(1, 1), epsilon = 1e-10);
    }

    #[test]
    fn pseudo_inverse_handles_rank_deficient_matrix() {
        // Matriks rank-1 (outer product [sqrt(10), sqrt(0.1)]) -- khas info
        // Fisher satu item dikotomus tunggal di nfact=2, singular terhadap
        // invers biasa.
        let a = [10.0_f64.sqrt(), 0.1_f64.sqrt()];
        let m = InfoMatrix::from_matrix(DMatrix::from_row_slice(
            2,
            2,
            &[a[0] * a[0], a[0] * a[1], a[0] * a[1], a[1] * a[1]],
        ))
        .unwrap();
        let pinv = m.pseudo_inverse(1e-10);
        // Nilai referensi diverifikasi numerik terhadap mirtCAT::computeCriteria
        // (Arule) pada eksperimen desain Fase 2.
        assert_relative_eq!(pinv.get(0, 0), 0.09802960494069211, epsilon = 1e-9);
        assert_relative_eq!(pinv.get(1, 1), 0.0009802960494069213, epsilon = 1e-9);
    }
}
