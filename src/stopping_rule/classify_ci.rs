//! Mode klasifikasi `CI` — z-score per-dimensi vs cutoff, keputusan
//! **per-dimensi** (`AboveCutoff`/`BelowCutoff`/`NoDecision`), stop hanya
//! bila **semua** dimensi terklasifikasi (`all()`).
//!
//! Padanan `DesignClass.R:332-343`:
//! ```r
//! z <- -abs(person$thetas - .Object@classify) / diff
//! .Object@met_classify <- as.vector(z < qnorm(.Object@classify_alpha))
//! if(.Object@stage > 1L && all(.Object@met_classify)){
//!     sig <- z < qnorm(.Object@classify_alpha)
//!     direction <- ifelse((person$thetas - .Object@classify) > 0, 'above cutoff', 'below cutoff')
//!     direction[!sig] <- 'no decision'
//!     .Object@stop_now <- TRUE
//!     person$classify_decision <- as.character(direction)
//! }
//! ```
//! `diff` adalah SE terkini (`NA` -> `Inf`, lih. `sem.rs`). Beda dari R:
//! keputusan **per-dimensi** di sini dihitung independen dari gerbang
//! `all()` -- R secara efektif menghitung nilai yang sama (`sig` ==
//! `met_classify`) sebelum gerbang, kita mengeksposnya langsung supaya
//! pemanggil bisa melihat status tiap dimensi walau belum semuanya yakin
//! (mis. "satu dimensi sudah above cutoff, satu masih no decision").

use crate::stopping_rule::ClassifyDecision;
use crate::theta::{SEVector, ThetaVector};

/// Mengembalikan `(met_per_dimensi, klasifikasi_per_dimensi)`. `cutoffs`
/// harus sudah divalidasi panjangnya == `theta.nfact()` oleh pemanggil.
pub(crate) fn evaluate(
    theta: &ThetaVector,
    se: &SEVector,
    cutoffs: &[f64],
    alpha: f64,
) -> (Vec<bool>, Vec<ClassifyDecision>) {
    let z_alpha = crate::ability_estimation::quadrature::inverse_normal_cdf(alpha);
    let nfact = theta.nfact();
    let mut met = Vec::with_capacity(nfact);
    let mut decisions = Vec::with_capacity(nfact);
    for i in 0..nfact {
        let se_i = if se[i].is_nan() { f64::INFINITY } else { se[i] };
        let z = -(theta[i] - cutoffs[i]).abs() / se_i;
        let is_met = z < z_alpha;
        met.push(is_met);
        decisions.push(if !is_met {
            ClassifyDecision::NoDecision
        } else if theta[i] > cutoffs[i] {
            ClassifyDecision::AboveCutoff
        } else {
            ClassifyDecision::BelowCutoff
        });
    }
    (met, decisions)
}

/// Batas `(bawah, atas)` interval kepercayaan per-dimensi, `theta ± |z_alpha|·SE`
/// -- kriteria `evaluate` ekuivalen "cutoff berada di luar interval ini".
/// Hanya untuk jejak debug; tidak memengaruhi keputusan. `SE` `NaN` diperlakukan
/// `Inf` seperti `evaluate` (batas -> `-Inf`/`+Inf`).
pub(crate) fn interval(theta: &ThetaVector, se: &SEVector, alpha: f64) -> (Vec<f64>, Vec<f64>) {
    let z = crate::ability_estimation::quadrature::inverse_normal_cdf(alpha).abs();
    (0..theta.nfact())
        .map(|i| {
            let se_i = if se[i].is_nan() { f64::INFINITY } else { se[i] };
            (theta[i] - z * se_i, theta[i] + z * se_i)
        })
        .unzip()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_excludes_cutoff_exactly_when_dimension_is_classified() {
        let theta = ThetaVector::from_vec(vec![3.0, 0.05]);
        let se = SEVector::from_vec(vec![0.3, 1.0]);
        let cutoffs = [0.0, 0.0];
        let (met, _) = evaluate(&theta, &se, &cutoffs, 0.05);
        let (lower, upper) = interval(&theta, &se, 0.05);
        for i in 0..2 {
            assert!(lower[i] < theta[i] && theta[i] < upper[i]);
            let outside = cutoffs[i] < lower[i] || cutoffs[i] > upper[i];
            assert_eq!(met[i], outside, "dimensi {i}");
        }
    }

    #[test]
    fn interval_with_nan_se_is_unbounded() {
        let theta = ThetaVector::from_vec(vec![1.0]);
        let se = SEVector::from_vec(vec![f64::NAN]);
        let (lower, upper) = interval(&theta, &se, 0.05);
        assert_eq!(lower, vec![f64::NEG_INFINITY]);
        assert_eq!(upper, vec![f64::INFINITY]);
    }

    #[test]
    fn confident_dimension_above_cutoff_is_classified() {
        // theta jauh di atas cutoff relatif SE -> met, arah above.
        let theta = ThetaVector::from_vec(vec![3.0]);
        let se = SEVector::from_vec(vec![0.3]);
        let (met, decisions) = evaluate(&theta, &se, &[0.0], 0.05);
        assert_eq!(met, vec![true]);
        assert_eq!(decisions, vec![ClassifyDecision::AboveCutoff]);
    }

    #[test]
    fn confident_dimension_below_cutoff_is_classified() {
        let theta = ThetaVector::from_vec(vec![-3.0]);
        let se = SEVector::from_vec(vec![0.3]);
        let (met, decisions) = evaluate(&theta, &se, &[0.0], 0.05);
        assert_eq!(met, vec![true]);
        assert_eq!(decisions, vec![ClassifyDecision::BelowCutoff]);
    }

    #[test]
    fn theta_close_to_cutoff_relative_to_se_is_no_decision() {
        let theta = ThetaVector::from_vec(vec![0.05]);
        let se = SEVector::from_vec(vec![1.0]);
        let (met, decisions) = evaluate(&theta, &se, &[0.0], 0.05);
        assert_eq!(met, vec![false]);
        assert_eq!(decisions, vec![ClassifyDecision::NoDecision]);
    }

    #[test]
    fn two_dimensions_one_confident_one_not() {
        let theta = ThetaVector::from_vec(vec![3.0, 0.05]);
        let se = SEVector::from_vec(vec![0.3, 1.0]);
        let (met, decisions) = evaluate(&theta, &se, &[0.0, 0.0], 0.05);
        assert_eq!(met, vec![true, false]);
        assert_eq!(
            decisions,
            vec![ClassifyDecision::AboveCutoff, ClassifyDecision::NoDecision]
        );
        assert!(!met.iter().all(|&m| m));
    }

    #[test]
    fn nan_se_is_treated_as_infinity_never_met() {
        let theta = ThetaVector::from_vec(vec![5.0]);
        let se = SEVector::from_vec(vec![f64::NAN]);
        let (met, decisions) = evaluate(&theta, &se, &[0.0], 0.05);
        assert_eq!(met, vec![false]);
        assert_eq!(decisions, vec![ClassifyDecision::NoDecision]);
    }
}
