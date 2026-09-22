//! `met_SEM` — kriteria presisi standard error, semantik "ALL dimensi".
//!
//! Padanan `DesignClass.R:346-347`:
//! ```r
//! .Object@met_SEM <- diff < .Object@min_SEM
//! if(.Object@stage > 1L && !any(is.nan(diff)) && all(.Object@met_SEM))
//!     .Object@stop_now <- TRUE
//! ```
//! `diff` di sana adalah baris SE terkini (`NA` sudah ditimpa `Inf` sebelum
//! sampai di sini -- lih. `DesignClass.R:318`, direplikasi lewat pengecekan
//! `is_nan()` di [`evaluate`]).

use crate::theta::SEVector;

/// `met_i = !se[i].is_nan() && se[i] < min_sem[i]`, dievaluasi per-dimensi.
/// `min_sem` harus sudah dalam bentuk final (sudah di-broadcast ke `nfact`
/// oleh pemanggil -- lih. `stopping_rule::broadcast`).
pub(crate) fn evaluate(se: &SEVector, min_sem: &[f64]) -> Vec<bool> {
    (0..se.nfact())
        .map(|i| {
            let s = se[i];
            !s.is_nan() && s < min_sem[i]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_dimensions_below_threshold_are_met() {
        let se = SEVector::from_vec(vec![0.2, 0.25]);
        let met = evaluate(&se, &[0.3, 0.3]);
        assert_eq!(met, vec![true, true]);
    }

    #[test]
    fn one_dimension_above_threshold_is_not_met() {
        let se = SEVector::from_vec(vec![0.2, 0.35]);
        let met = evaluate(&se, &[0.3, 0.3]);
        assert_eq!(met, vec![true, false]);
    }

    #[test]
    fn nan_se_is_treated_as_infinity_never_met() {
        let se = SEVector::from_vec(vec![f64::NAN, 0.1]);
        let met = evaluate(&se, &[0.3, 0.3]);
        assert_eq!(met, vec![false, true]);
    }
}
