//! `met_delta_thetas` — kriteria presisi berbasis perubahan `theta`
//! antar-langkah terakhir, semantik "ALL dimensi", dievaluasi **independen**
//! dari mode klasifikasi (`ClassifyType::None`/`Ci`/`Sprt`).
//!
//! Padanan `DesignClass.R:352-357`:
//! ```r
//! diff2 <- abs(person$thetas_history[nrow(...),] - person$thetas_history[nrow(...)-1L,])
//! .Object@met_delta_thetas <- as.vector(diff2 < .Object@delta_thetas)
//! if(.Object@stage > 1L && all(.Object@met_delta_thetas))
//!     .Object@stop_now <- TRUE
//! ```

use crate::theta::ThetaVector;

/// `met_i = |theta[i] - previous[i]| < delta_thetas[i]`. `previous == None`
/// (belum ada estimasi ulang sebelumnya) -> seluruh dimensi `false` (belum
/// bisa dievaluasi, konsisten "belum bisa stop lewat delta").
pub(crate) fn evaluate(
    theta: &ThetaVector,
    previous: Option<&ThetaVector>,
    delta_thetas: &[f64],
) -> Vec<bool> {
    match previous {
        None => vec![false; theta.nfact()],
        Some(prev) => (0..theta.nfact())
            .map(|i| (theta[i] - prev[i]).abs() < delta_thetas[i])
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_previous_theta_means_nothing_met_yet() {
        let theta = ThetaVector::from_vec(vec![0.1, 0.2]);
        let met = evaluate(&theta, None, &[0.05, 0.05]);
        assert_eq!(met, vec![false, false]);
    }

    #[test]
    fn all_dimensions_within_delta_are_met() {
        let theta = ThetaVector::from_vec(vec![0.10, 0.20]);
        let previous = ThetaVector::from_vec(vec![0.09, 0.21]);
        let met = evaluate(&theta, Some(&previous), &[0.05, 0.05]);
        assert_eq!(met, vec![true, true]);
    }

    #[test]
    fn one_dimension_beyond_delta_is_not_met() {
        let theta = ThetaVector::from_vec(vec![0.10, 0.40]);
        let previous = ThetaVector::from_vec(vec![0.09, 0.21]);
        let met = evaluate(&theta, Some(&previous), &[0.05, 0.05]);
        assert_eq!(met, vec![true, false]);
    }
}
