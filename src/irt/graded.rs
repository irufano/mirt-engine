//! Graded response model (GRM) multidimensional (politomus, respons
//! terurut) — mengikuti `P_graded()` di `mirt/src/traceLinePts.cpp` dan
//! Muraki & Carlson (1995) untuk versi multidimensional.

use nalgebra::DMatrix;

use super::model::ItemResponseModel;
use super::probability::{linear_predictor, logistic};
use crate::error::MirtCatError;
use crate::theta::{InfoMatrix, ThetaVector};

/// Batas bawah/atas probabilitas kategori setelah kliping, menyamai
/// `1e-20` di `mirt/src/traceLinePts.cpp::P_graded` — mencegah probabilitas
/// persis 0/1 yang membuat `information()` (pembagian oleh `P_k`) meledak.
const PROB_CLIP: f64 = 1e-20;

/// Item graded response multidimensional: `a` (slope per dimensi), `d`
/// (boundary/threshold per kategori, panjang `ncat - 1`, **harus** terurut
/// menurun `d[0] > d[1] > ... > d[k]`, mengikuti urutan kategori respons
/// menaik).
#[derive(Debug, Clone, PartialEq)]
pub struct GradedResponse {
    a: Vec<f64>,
    d: Vec<f64>,
}

impl GradedResponse {
    /// Error bila `a` kosong, `d` punya kurang dari 1 boundary, atau `d`
    /// tidak terurut menurun (lih. cek `notordered` di `P_graded()`, yang
    /// di R hanya membuat probabilitas nol diam-diam — di sini ditolak
    /// eksplisit saat konstruksi, sesuai prinsip desain #5).
    pub fn new(a: Vec<f64>, d: Vec<f64>) -> Result<Self, MirtCatError> {
        if a.is_empty() {
            return Err(MirtCatError::InvalidParameter(
                "GradedResponse: `a` (slope) tidak boleh kosong".into(),
            ));
        }
        if d.is_empty() {
            return Err(MirtCatError::InvalidParameter(
                "GradedResponse: `d` (boundary) harus punya minimal 1 elemen".into(),
            ));
        }
        if d.windows(2).any(|w| w[0] <= w[1]) {
            return Err(MirtCatError::InvalidParameter(format!(
                "GradedResponse: `d` harus terurut menurun (d[0] > d[1] > ...), didapat {d:?}"
            )));
        }
        Ok(Self { a, d })
    }

    pub fn a(&self) -> &[f64] {
        &self.a
    }

    pub fn d(&self) -> &[f64] {
        &self.d
    }

    /// Kurva batas kategori `Pstar_k = P(respons >= kategori k+1 | theta)`
    /// untuk `k = 0..=nint+1`, dengan `Pstar_0 = 1` dan `Pstar_{nint+1} = 0`
    /// sebagai batas implisit.
    fn boundary_curves(&self, theta: &[f64]) -> Vec<f64> {
        let mut pstar = Vec::with_capacity(self.d.len() + 2);
        pstar.push(1.0);
        for &dk in &self.d {
            let z = linear_predictor(&self.a, dk, theta);
            pstar.push(logistic(z));
        }
        pstar.push(0.0);
        pstar
    }
}

impl ItemResponseModel for GradedResponse {
    fn nfact(&self) -> usize {
        self.a.len()
    }

    fn n_categories(&self) -> usize {
        self.d.len() + 1
    }

    fn probabilities(&self, theta: &ThetaVector) -> Result<Vec<f64>, MirtCatError> {
        theta.validate_nfact(self.nfact(), "GradedResponse::probabilities")?;
        let pstar = self.boundary_curves(theta.as_slice());
        let probs = pstar
            .windows(2)
            .map(|w| (w[0] - w[1]).clamp(PROB_CLIP, 1.0 - PROB_CLIP))
            .collect();
        Ok(probs)
    }

    fn information(&self, theta: &ThetaVector) -> Result<InfoMatrix, MirtCatError> {
        theta.validate_nfact(self.nfact(), "GradedResponse::information")?;
        let theta_slice = theta.as_slice();
        let pstar = self.boundary_curves(theta_slice);
        let probs = self.probabilities(theta)?;
        let nfact = self.nfact();

        // grad(Pstar_k) = a * Pstar_k * (1 - Pstar_k); batas Pstar_0/Pstar_last tetap (grad = 0).
        let n_boundaries = pstar.len();
        let mut grad_pstar = vec![vec![0.0; nfact]; n_boundaries];
        for k in 1..(n_boundaries - 1) {
            let deriv = pstar[k] * (1.0 - pstar[k]);
            for (gj, aj) in grad_pstar[k].iter_mut().zip(&self.a) {
                *gj = aj * deriv;
            }
        }

        let mut m = DMatrix::zeros(nfact, nfact);
        for k in 0..probs.len() {
            let grad_p: Vec<f64> = (0..nfact)
                .map(|j| grad_pstar[k][j] - grad_pstar[k + 1][j])
                .collect();
            let p_k = probs[k];
            for i in 0..nfact {
                for j in 0..nfact {
                    m[(i, j)] += grad_p[i] * grad_p[j] / p_k;
                }
            }
        }
        InfoMatrix::from_matrix(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn probabilities_sum_to_one() {
        let item = GradedResponse::new(vec![1.2, -0.5], vec![1.5, 0.2, -1.0]).unwrap();
        let theta = ThetaVector::from_vec(vec![0.4, -0.2]);
        let p = item.probabilities(&theta).unwrap();
        assert_eq!(p.len(), 4);
        assert_relative_eq!(p.iter().sum::<f64>(), 1.0, epsilon = 1e-9);
    }

    #[test]
    fn probabilities_are_monotonic_with_boundary_order() {
        // theta ekstrem tinggi -> massa probabilitas terkonsentrasi di kategori tertinggi.
        let item = GradedResponse::new(vec![1.0], vec![2.0, 0.0, -2.0]).unwrap();
        let high = item
            .probabilities(&ThetaVector::from_vec(vec![10.0]))
            .unwrap();
        let low = item
            .probabilities(&ThetaVector::from_vec(vec![-10.0]))
            .unwrap();
        assert!(high[3] > high[0]); // kategori tertinggi dominan saat theta besar
        assert!(low[0] > low[3]); // kategori terendah dominan saat theta kecil
    }

    #[test]
    fn unordered_boundaries_rejected_at_construction() {
        assert!(GradedResponse::new(vec![1.0], vec![0.0, 0.5]).is_err()); // tidak menurun
        assert!(GradedResponse::new(vec![1.0], vec![0.0, 0.0]).is_err()); // sama, bukan ketat menurun
    }

    #[test]
    fn empty_a_or_d_rejected() {
        assert!(GradedResponse::new(vec![], vec![0.0]).is_err());
        assert!(GradedResponse::new(vec![1.0], vec![]).is_err());
    }

    #[test]
    fn dimension_mismatch_errors_not_panics() {
        let item = GradedResponse::new(vec![1.0, 1.0], vec![1.0, -1.0]).unwrap();
        let theta = ThetaVector::from_vec(vec![0.0]);
        assert!(item.probabilities(&theta).is_err());
        assert!(item.information(&theta).is_err());
    }

    /// Dua kategori (nint=1) graded response harus setara dengan 2PL
    /// (probabilitas & informasi identik).
    #[test]
    fn two_category_graded_matches_2pl_equivalent() {
        use super::super::dichotomous::Dichotomous;

        let a = vec![1.1, 0.6];
        let d = 0.3_f64;
        let graded = GradedResponse::new(a.clone(), vec![d]).unwrap();
        let dich = Dichotomous::m2pl(a, d).unwrap();

        let theta = ThetaVector::from_vec(vec![0.5, -0.4]);
        let p_graded = graded.probabilities(&theta).unwrap();
        let p_dich = dich.probabilities(&theta).unwrap();
        assert_relative_eq!(p_graded[0], p_dich[0], epsilon = 1e-9);
        assert_relative_eq!(p_graded[1], p_dich[1], epsilon = 1e-9);

        let info_graded = graded.information(&theta).unwrap();
        let info_dich = dich.information(&theta).unwrap();
        assert_relative_eq!(info_graded.get(0, 0), info_dich.get(0, 0), epsilon = 1e-8);
        assert_relative_eq!(info_graded.get(1, 1), info_dich.get(1, 1), epsilon = 1e-8);
    }
}
