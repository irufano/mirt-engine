//! MAP (Maximum A Posteriori) — optimisasi langsung dengan prior
//! `N(gmeans, gcov)`. Padanan `mirt::MAP.mirt`/`MAP()`
//! (`mirt/R/fscores.internal.R:684-703,1047-1070`): maksimalkan
//! `log L(theta) + log dmvnorm(theta; gmeans, gcov)` lewat Newton-Raphson
//! finite-difference (`optim::maximize` — lih. catatan modul di sana soal
//! kesetiaan port terhadap `nlm()`, yang di sisi R juga tidak diberi
//! gradien analitik).

use nalgebra::DVector;

use crate::bank::{GroupParams, ItemBank};
use crate::error::MirtCatError;
use crate::item_selection::PersonState;
use crate::theta::{SEVector, ThetaVector};

use super::density::log_dmvnorm;
use super::optim::maximize;
use super::{log_likelihood_at, se_from_hessian, validate_answers};

pub(crate) struct MapResult {
    pub theta: ThetaVector,
    pub se: SEVector,
    pub iterations: usize,
    pub log_likelihood_trace: Vec<f64>,
    pub converged: bool,
}

pub(crate) fn estimate(
    person: &PersonState,
    bank: &ItemBank,
    group: &GroupParams,
) -> Result<MapResult, MirtCatError> {
    let nfact = person.nfact();
    validate_answers(person, bank)?;
    let gmeans = group.gmeans().clone();
    let gcov = group.gcov().clone();

    let objective = |t: &DVector<f64>| {
        let theta = ThetaVector::from_vec(t.as_slice().to_vec());
        let ll = log_likelihood_at(person, bank, &theta).expect("divalidasi sebelum optimisasi");
        let log_prior =
            log_dmvnorm(&theta, &gmeans, &gcov).expect("gmeans/gcov tervalidasi di GroupParams");
        ll + log_prior
    };

    let initial = DVector::from_vec(person.theta().as_slice().to_vec());
    let result = maximize(objective, initial);
    let se = se_from_hessian(&result.hessian_at_optimum, nfact);

    Ok(MapResult {
        theta: ThetaVector::from_vec(result.theta.as_slice().to_vec()),
        se,
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
    use crate::theta::InfoMatrix;
    use approx::assert_relative_eq;
    use nalgebra::DMatrix;

    fn bank_1item(a: f64, d: f64) -> ItemBank {
        let mut bank = ItemBank::new();
        bank.push(Box::new(Dichotomous::m2pl(vec![a], d).unwrap()), ItemMeta::new(0))
            .unwrap();
        bank
    }

    #[test]
    fn map_pulls_toward_prior_mean_more_than_wide_prior() {
        let bank = bank_1item(1.2, 0.0);
        let mut person = PersonState::new(1);
        person.record_answer(0, 1);

        let narrow_group =
            GroupParams::new(ThetaVector::zeros(1), InfoMatrix::identity(1)).unwrap();
        let wide_cov =
            InfoMatrix::from_matrix(DMatrix::from_row_slice(1, 1, &[100.0])).unwrap();
        let wide_group = GroupParams::new(ThetaVector::zeros(1), wide_cov).unwrap();

        let narrow_result = estimate(&person, &bank, &narrow_group).unwrap();
        let wide_result = estimate(&person, &bank, &wide_group).unwrap();

        // Prior sempit (var=1) menarik theta lebih dekat ke gmeans=0
        // dibanding prior lebar (var=100, hampir tidak ada informasi prior)
        // -- respons benar sama, jadi wide harus lebih besar dari narrow.
        assert!(wide_result.theta[0] > narrow_result.theta[0]);
        assert!(narrow_result.theta[0] > 0.0);
    }

    #[test]
    fn map_converges_and_reports_finite_se() {
        let bank = bank_1item(1.0, 0.0);
        let mut person = PersonState::new(1);
        person.record_answer(0, 1);
        let group = GroupParams::standard_normal(1);
        let result = estimate(&person, &bank, &group).unwrap();
        assert!(result.converged);
        assert!(result.se.as_slice()[0].is_finite());
        assert!(result.se.as_slice()[0] > 0.0);
    }

    #[test]
    fn map_with_no_answers_stays_near_prior_mean() {
        let bank = bank_1item(1.0, 0.0);
        let person = PersonState::new(1);
        let mean = ThetaVector::from_vec(vec![0.6]);
        let group = GroupParams::new(mean, InfoMatrix::identity(1)).unwrap();
        let result = estimate(&person, &bank, &group).unwrap();
        assert_relative_eq!(result.theta[0], 0.6, epsilon = 1e-4);
    }
}
