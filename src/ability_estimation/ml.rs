//! ML (Maximum Likelihood) — tanpa prior. Padanan `mirt::MAP.mirt(...,
//! ML=TRUE)`/`ML()` (`mirt/R/fscores.internal.R:684-703,1072-1101`):
//! maksimalkan `log L(theta)` murni.
//!
//! Pada pola respons ekstrem (semua benar/semua salah untuk seluruh item
//! terjawab), `theta` divergen ke `+-infinity` — diverifikasi empiris
//! identik dengan `mirt::fscores(method='ML')` sungguhan (R 4.6.1/mirt
//! 1.47): pola semua-benar mengembalikan `theta=Inf, SE=NA`. Deteksi lewat
//! ambang `SE > 30`, replikasi literal `mirt::ML()` (`fscores.internal.R:
//! 1094-1097`). Fallback otomatis `ML -> MAP` untuk pola ekstrem itu
//! logika `mirtCAT::Design` (level sesi CAT) — bukan tanggung jawab fungsi
//! ini.

use nalgebra::DVector;

use crate::bank::{GroupParams, ItemBank};
use crate::error::MirtCatError;
use crate::item_selection::PersonState;
use crate::theta::{SEVector, ThetaVector};

use super::optim::maximize;
use super::{log_likelihood_at, se_from_hessian, validate_answers};

/// Ambang SE (padanan literal `SEest > 30` di `mirt::ML()`,
/// `fscores.internal.R:1094-1097`) — di atasnya `theta` dilaporkan sebagai
/// `+-infinity` (tanda dari nilai mentah hasil optimisasi) & SE `NaN`,
/// bukan nilai finite yang menyesatkan (SE sebesar itu berarti interval
/// kepercayaan praktis tak berhingga).
const SE_DIVERGENCE_THRESHOLD: f64 = 30.0;

pub(crate) struct MlResult {
    pub theta: ThetaVector,
    pub se: SEVector,
    pub iterations: usize,
    pub log_likelihood_trace: Vec<f64>,
    pub converged: bool,
}

pub(crate) fn estimate(
    person: &PersonState,
    bank: &ItemBank,
    _group: &GroupParams,
) -> Result<MlResult, MirtCatError> {
    let nfact = person.nfact();
    validate_answers(person, bank)?;

    let objective = |t: &DVector<f64>| {
        let theta = ThetaVector::from_vec(t.as_slice().to_vec());
        log_likelihood_at(person, bank, &theta).expect("divalidasi sebelum optimisasi")
    };

    let initial = DVector::from_vec(person.theta().as_slice().to_vec());
    let result = maximize(objective, initial);
    let raw_se = se_from_hessian(&result.hessian_at_optimum, nfact);

    let mut theta_out = result.theta.as_slice().to_vec();
    let mut se_out = raw_se.as_slice().to_vec();
    for i in 0..nfact {
        let sei = se_out[i];
        if sei.is_nan() || sei > SE_DIVERGENCE_THRESHOLD {
            // `sei` NaN (Hessian tak invertible) ATAU > ambang -> divergen.
            theta_out[i] = if theta_out[i] >= 0.0 {
                f64::INFINITY
            } else {
                f64::NEG_INFINITY
            };
            se_out[i] = f64::NAN;
        }
    }

    Ok(MlResult {
        theta: ThetaVector::from_vec(theta_out),
        se: SEVector::from_vec(se_out),
        iterations: result.iterations,
        log_likelihood_trace: result.objective_trace,
        converged: result.converged,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bank::ItemMeta;
    use crate::irt::Dichotomous;

    fn bank_n_items(n: usize) -> ItemBank {
        let mut bank = ItemBank::new();
        for i in 0..n {
            bank.push(
                Box::new(Dichotomous::m2pl(vec![1.0 + 0.1 * i as f64], 0.0).unwrap()),
                ItemMeta::new(i),
            )
            .unwrap();
        }
        bank
    }

    #[test]
    fn ml_diverges_to_positive_infinity_on_all_correct_pattern() {
        let bank = bank_n_items(6);
        let mut person = PersonState::new(1);
        for i in 0..6 {
            person.record_answer(i, 1); // semua benar
        }
        let group = GroupParams::standard_normal(1);
        let result = estimate(&person, &bank, &group).unwrap();
        assert_eq!(result.theta[0], f64::INFINITY);
        assert!(result.se.as_slice()[0].is_nan());
    }

    #[test]
    fn ml_diverges_to_negative_infinity_on_all_incorrect_pattern() {
        let bank = bank_n_items(6);
        let mut person = PersonState::new(1);
        for i in 0..6 {
            person.record_answer(i, 0); // semua salah
        }
        let group = GroupParams::standard_normal(1);
        let result = estimate(&person, &bank, &group).unwrap();
        assert_eq!(result.theta[0], f64::NEG_INFINITY);
        assert!(result.se.as_slice()[0].is_nan());
    }

    #[test]
    fn ml_gives_finite_estimate_on_mixed_pattern() {
        let bank = bank_n_items(6);
        let mut person = PersonState::new(1);
        let responses = [1, 0, 1, 1, 0, 1];
        for (i, &r) in responses.iter().enumerate() {
            person.record_answer(i, r);
        }
        let group = GroupParams::standard_normal(1);
        let result = estimate(&person, &bank, &group).unwrap();
        assert!(result.theta[0].is_finite());
        assert!(result.se.as_slice()[0].is_finite());
        assert!(result.converged);
    }

    #[test]
    fn ml_has_no_prior_so_differs_from_map_when_prior_is_narrow() {
        // Sanity check tambahan (bukan cuma checklist MAP vs ML): dengan
        // prior sempit non-default (mean != 0), ML TIDAK tertarik ke sana
        // sama sekali, beda dari MAP.
        use super::super::map;
        let bank = bank_n_items(6);
        let mut person = PersonState::new(1);
        let responses = [1, 0, 1, 1, 0, 1];
        for (i, &r) in responses.iter().enumerate() {
            person.record_answer(i, r);
        }
        let off_center_group = GroupParams::new(
            ThetaVector::from_vec(vec![3.0]),
            crate::theta::InfoMatrix::identity(1),
        )
        .unwrap();
        let ml_result = estimate(&person, &bank, &off_center_group).unwrap();
        let map_result = map::estimate(&person, &bank, &off_center_group).unwrap();
        // MAP tertarik ke gmeans=3.0 oleh prior sempit (var=1); ML sama
        // sekali tidak (murni likelihood), jadi harus jauh lebih kecil.
        assert!(map_result.theta[0] > ml_result.theta[0]);
        assert!(ml_result.theta[0] < 1.0);
    }
}
