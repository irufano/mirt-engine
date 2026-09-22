//! Densitas normal multivariat (log-space) — dipakai bobot prior EAP
//! grid-based, `EAPsum`, dan suku prior `Map`. Padanan `mirt::mirt_dmvnorm`
//! (`mirt/R/utils.R:2351-2377`), versi log tanpa stabilisasi clamp
//! `-690.7755` (irrelevan untuk `nfact` kecil di domain ini).

use crate::error::MirtCatError;
use crate::theta::{InfoMatrix, ThetaVector};

/// `log N(x; mean, cov)` — densitas normal multivariat dalam ruang log,
/// dihitung lewat `InfoMatrix::inverse()`/`determinant()`/`quadratic_form()`
/// yang sudah ada (Fase 0/2), bukan implementasi baru dari nol. Error bila
/// dimensi `x`/`mean`/`cov` tidak cocok, atau `cov` singular.
pub(crate) fn log_dmvnorm(
    x: &ThetaVector,
    mean: &ThetaVector,
    cov: &InfoMatrix,
) -> Result<f64, MirtCatError> {
    let nfact = cov.nfact();
    x.validate_nfact(nfact, "ability_estimation::density::log_dmvnorm: x")?;
    mean.validate_nfact(nfact, "ability_estimation::density::log_dmvnorm: mean")?;
    let diff: Vec<f64> = (0..nfact).map(|i| x[i] - mean[i]).collect();
    let inv_cov = cov.inverse()?;
    let mahalanobis = inv_cov.quadratic_form(&diff)?;
    let log_det = cov.determinant().ln();
    Ok(-0.5 * (nfact as f64 * (2.0 * std::f64::consts::PI).ln() + log_det + mahalanobis))
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn standard_normal_at_zero_matches_closed_form() {
        let x = ThetaVector::zeros(2);
        let mean = ThetaVector::zeros(2);
        let cov = InfoMatrix::identity(2);
        let log_p = log_dmvnorm(&x, &mean, &cov).unwrap();
        // N(0; 0, I_2) di titik nol = 1/(2*pi) -> log = -log(2*pi)
        assert_relative_eq!(log_p, -(2.0 * std::f64::consts::PI).ln(), epsilon = 1e-12);
    }

    #[test]
    fn univariate_matches_closed_form_normal_density() {
        let x = ThetaVector::from_vec(vec![1.5]);
        let mean = ThetaVector::from_vec(vec![0.5]);
        let cov = InfoMatrix::from_matrix(nalgebra::DMatrix::from_row_slice(1, 1, &[4.0])).unwrap();
        let log_p = log_dmvnorm(&x, &mean, &cov).unwrap();
        let var = 4.0_f64;
        let expected =
            -0.5 * (2.0 * std::f64::consts::PI * var).ln() - (1.5_f64 - 0.5).powi(2) / (2.0 * var);
        assert_relative_eq!(log_p, expected, epsilon = 1e-12);
    }

    #[test]
    fn farther_from_mean_has_lower_density() {
        let mean = ThetaVector::zeros(2);
        let cov = InfoMatrix::identity(2);
        let near = log_dmvnorm(&ThetaVector::from_vec(vec![0.1, 0.1]), &mean, &cov).unwrap();
        let far = log_dmvnorm(&ThetaVector::from_vec(vec![3.0, 3.0]), &mean, &cov).unwrap();
        assert!(near > far);
    }

    #[test]
    fn dimension_mismatch_errors_not_panics() {
        let x = ThetaVector::from_vec(vec![0.0, 0.0, 0.0]);
        let mean = ThetaVector::zeros(2);
        let cov = InfoMatrix::identity(2);
        assert!(log_dmvnorm(&x, &mean, &cov).is_err());
    }
}
