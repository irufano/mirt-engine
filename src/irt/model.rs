//! Trait inti model respons item — padanan konseptual kelas item
//! `AllItemsClass` di `mirt` (`R/01-itemclass.R`), disederhanakan ke
//! permukaan yang dibutuhkan `mirtcat-engine`: probabilitas kategori dan
//! informasi Fisher pada satu titik `theta`.

use crate::error::MirtCatError;
use crate::theta::{InfoMatrix, ThetaVector};

/// Model respons item multidimensional (dikotomus atau politomus).
///
/// Implementor bertanggung jawab menghitung probabilitas tiap kategori
/// respons dan matriks informasi Fisher `nfact × nfact` pada sebuah titik
/// `theta`, mengikuti formula tertutup di Reckase (2009) & Chalmers (2012) —
/// lih. `docs/teori/00-fondasi-irt-multidimensional.md`.
pub trait ItemResponseModel {
    /// Jumlah dimensi laten (`nfact`) yang diharapkan pada `theta`.
    fn nfact(&self) -> usize;

    /// Jumlah kategori respons (2 untuk item dikotomus, `>2` untuk politomus).
    fn n_categories(&self) -> usize;

    /// Probabilitas tiap kategori respons pada `theta`, menjumlah ke 1.
    /// Error bila panjang `theta` tidak sama dengan `nfact()`.
    fn probabilities(&self, theta: &ThetaVector) -> Result<Vec<f64>, MirtCatError>;

    /// Matriks informasi Fisher `nfact × nfact` pada `theta`:
    /// `I(theta) = sum_k outer(grad P_k(theta), grad P_k(theta)) / P_k(theta)`
    /// (lih. `mirt/R/utils.R::ItemInfo2`, `MD = TRUE`).
    fn information(&self, theta: &ThetaVector) -> Result<InfoMatrix, MirtCatError>;
}
