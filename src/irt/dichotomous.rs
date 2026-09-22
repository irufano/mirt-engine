//! Model item dikotomus multidimensional — M2PL/M3PL/M4PL sebagai kasus
//! khusus dari satu parameterisasi `(a, d, g, u)`, mengikuti `P_dich()` di
//! `mirt/src/traceLinePts.cpp` dan informasi Fisher di Reckase (2009, bab 4).

use nalgebra::DMatrix;

use super::model::ItemResponseModel;
use super::probability::{linear_predictor, logistic};
use crate::error::MirtCatError;
use crate::theta::{InfoMatrix, ThetaVector};

/// Item dikotomus multidimensional: `a` (slope per dimensi), `d`
/// (intercept/difficulty), `g` (guessing/lower asymptote), `u` (upper
/// asymptote). `g=0, u=1` → M2PL; `u=1` → M3PL; keduanya bebas → M4PL.
#[derive(Debug, Clone, PartialEq)]
pub struct Dichotomous {
    a: Vec<f64>,
    d: f64,
    g: f64,
    u: f64,
}

impl Dichotomous {
    /// Konstruktor umum M4PL. Error bila `a` kosong, `g`/`u` di luar
    /// `[0, 1]`, atau `u <= g` (asymptote tidak valid).
    pub fn new(a: Vec<f64>, d: f64, g: f64, u: f64) -> Result<Self, MirtCatError> {
        if a.is_empty() {
            return Err(MirtCatError::InvalidParameter(
                "Dichotomous: `a` (slope) tidak boleh kosong".into(),
            ));
        }
        if !(0.0..=1.0).contains(&g) {
            return Err(MirtCatError::InvalidParameter(format!(
                "Dichotomous: `g` harus di [0,1], didapat {g}"
            )));
        }
        if !(0.0..=1.0).contains(&u) {
            return Err(MirtCatError::InvalidParameter(format!(
                "Dichotomous: `u` harus di [0,1], didapat {u}"
            )));
        }
        if u <= g {
            return Err(MirtCatError::InvalidParameter(format!(
                "Dichotomous: `u` harus > `g`, didapat u={u}, g={g}"
            )));
        }
        Ok(Self { a, d, g, u })
    }

    /// M2PL: `g=0, u=1`.
    pub fn m2pl(a: Vec<f64>, d: f64) -> Result<Self, MirtCatError> {
        Self::new(a, d, 0.0, 1.0)
    }

    /// M3PL: `u=1`, `g` bebas.
    pub fn m3pl(a: Vec<f64>, d: f64, g: f64) -> Result<Self, MirtCatError> {
        Self::new(a, d, g, 1.0)
    }

    /// M4PL: `g` & `u` bebas.
    pub fn m4pl(a: Vec<f64>, d: f64, g: f64, u: f64) -> Result<Self, MirtCatError> {
        Self::new(a, d, g, u)
    }

    /// Slope per dimensi.
    pub fn a(&self) -> &[f64] {
        &self.a
    }

    /// Intercept/difficulty.
    pub fn d(&self) -> f64 {
        self.d
    }

    /// Guessing/lower asymptote.
    pub fn g(&self) -> f64 {
        self.g
    }

    /// Upper asymptote.
    pub fn u(&self) -> f64 {
        self.u
    }
}

impl ItemResponseModel for Dichotomous {
    fn nfact(&self) -> usize {
        self.a.len()
    }

    fn n_categories(&self) -> usize {
        2
    }

    fn probabilities(&self, theta: &ThetaVector) -> Result<Vec<f64>, MirtCatError> {
        theta.validate_nfact(self.nfact(), "Dichotomous::probabilities")?;
        let z = linear_predictor(&self.a, self.d, theta.as_slice());
        let pstar = logistic(z);
        let p1 = self.g + (self.u - self.g) * pstar;
        Ok(vec![1.0 - p1, p1])
    }

    fn information(&self, theta: &ThetaVector) -> Result<InfoMatrix, MirtCatError> {
        theta.validate_nfact(self.nfact(), "Dichotomous::information")?;
        let probs = self.probabilities(theta)?;
        let z = linear_predictor(&self.a, self.d, theta.as_slice());
        let pstar = logistic(z);
        // dP1/dtheta_j = (u - g) * pstar * (1 - pstar) * a_j
        let factor = (self.u - self.g) * pstar * (1.0 - pstar);
        let grad1: Vec<f64> = self.a.iter().map(|aj| factor * aj).collect();
        // info = outer(grad1, grad1)/P1 + outer(-grad1, -grad1)/P0
        let scale = 1.0 / probs[1] + 1.0 / probs[0];
        let nfact = self.nfact();
        let mut m = DMatrix::zeros(nfact, nfact);
        for i in 0..nfact {
            for j in 0..nfact {
                m[(i, j)] = grad1[i] * grad1[j] * scale;
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
    fn probabilities_sum_to_one_2pl() {
        let item = Dichotomous::m2pl(vec![1.0, 0.5], 0.2).unwrap();
        let theta = ThetaVector::from_vec(vec![0.3, -0.7]);
        let p = item.probabilities(&theta).unwrap();
        assert_relative_eq!(p[0] + p[1], 1.0, epsilon = 1e-12);
    }

    #[test]
    fn probabilities_sum_to_one_3pl_and_4pl() {
        let m3pl = Dichotomous::m3pl(vec![1.2], -0.5, 0.2).unwrap();
        let theta = ThetaVector::from_vec(vec![1.0]);
        let p3 = m3pl.probabilities(&theta).unwrap();
        assert_relative_eq!(p3[0] + p3[1], 1.0, epsilon = 1e-12);

        let m4pl = Dichotomous::m4pl(vec![1.2], -0.5, 0.15, 0.95).unwrap();
        let p4 = m4pl.probabilities(&theta).unwrap();
        assert_relative_eq!(p4[0] + p4[1], 1.0, epsilon = 1e-12);
        // guessing floor & upper asymptote menyempitkan rentang P1
        assert!(p4[1] >= 0.15 && p4[1] <= 0.95);
    }

    #[test]
    fn invalid_asymptotes_rejected_at_construction() {
        assert!(Dichotomous::new(vec![1.0], 0.0, 0.5, 0.5).is_err()); // u <= g
        assert!(Dichotomous::new(vec![1.0], 0.0, -0.1, 1.0).is_err()); // g < 0
        assert!(Dichotomous::new(vec![], 0.0, 0.0, 1.0).is_err()); // a kosong
    }

    #[test]
    fn dimension_mismatch_errors_not_panics() {
        let item = Dichotomous::m2pl(vec![1.0, 1.0], 0.0).unwrap();
        let theta = ThetaVector::from_vec(vec![0.0]);
        assert!(item.probabilities(&theta).is_err());
        assert!(item.information(&theta).is_err());
    }

    /// Sanity check 1 dimensi terhadap turunan analitik 2PL:
    /// `I(theta) = a^2 * P * (1 - P)`.
    #[test]
    fn information_matches_closed_form_2pl_1d() {
        let a = 1.3_f64;
        let d = 0.4_f64;
        let item = Dichotomous::m2pl(vec![a], d).unwrap();
        let theta = ThetaVector::from_vec(vec![0.6]);
        let p = item.probabilities(&theta).unwrap();
        let info = item.information(&theta).unwrap();
        let expected = a * a * p[1] * p[0];
        assert_relative_eq!(info.get(0, 0), expected, epsilon = 1e-10);
    }

    /// Sanity check 3PL 1 dimensi terhadap turunan analitik:
    /// `I(theta) = a^2 * ((u-g) * Pstar * (1-Pstar))^2 / (P1 * P0)`.
    #[test]
    fn information_matches_closed_form_3pl_1d() {
        let a = 0.9_f64;
        let d = -0.2_f64;
        let g = 0.2_f64;
        let item = Dichotomous::m3pl(vec![a], d, g).unwrap();
        let theta = ThetaVector::from_vec(vec![-0.4]);
        let z = a * theta[0] + d;
        let pstar = 1.0 / (1.0 + (-z).exp());
        let p1 = g + (1.0 - g) * pstar;
        let p0 = 1.0 - p1;
        let expected = (a * (1.0 - g) * pstar * (1.0 - pstar)).powi(2) / (p1 * p0);
        let info = item.information(&theta).unwrap();
        assert_relative_eq!(info.get(0, 0), expected, epsilon = 1e-10);
    }

    #[test]
    fn information_is_symmetric_2d() {
        let item = Dichotomous::m2pl(vec![0.8, 1.4], 0.1).unwrap();
        let theta = ThetaVector::from_vec(vec![0.2, -0.3]);
        let info = item.information(&theta).unwrap();
        assert_relative_eq!(info.get(0, 1), info.get(1, 0), epsilon = 1e-12);
    }
}
