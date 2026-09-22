//! WLE (Weighted Likelihood Estimation) — koreksi bias Warm (1989),
//! digeneralisasi multidimensional persis `mirt::WLE.mirt()`
//! (`mirt/R/fscores.internal.R:705-738`): maksimalkan
//! `log L(theta) + 0.5*log(det(info_kumulatif(theta)))`. Suku kedua
//! (bukan formula tertutup Warm 1D) adalah cara `mirt` menggeneralisasi
//! koreksi bias ke multidimensional — determinan info Fisher kumulatif
//! menggantikan skalar informasi 1D.

use nalgebra::DVector;

use crate::bank::{GroupParams, ItemBank};
use crate::error::MirtCatError;
use crate::irt::{cumulative_information, ItemResponseModel};
use crate::item_selection::PersonState;
use crate::theta::{SEVector, ThetaVector};

use super::optim::maximize;
use super::{log_likelihood_at, se_from_hessian, validate_answers};

pub(crate) struct WleResult {
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
) -> Result<WleResult, MirtCatError> {
    let nfact = person.nfact();
    validate_answers(person, bank)?;

    let items: Vec<&dyn ItemResponseModel> = person
        .answered_items()
        .iter()
        .map(|&idx| bank.item(idx).expect("divalidasi sebelum optimisasi"))
        .collect();

    let objective = |t: &DVector<f64>| {
        let theta = ThetaVector::from_vec(t.as_slice().to_vec());
        let ll = log_likelihood_at(person, bank, &theta).expect("divalidasi sebelum optimisasi");
        let info = cumulative_information(nfact, items.iter().copied(), &theta)
            .expect("nfact/theta konsisten -- divalidasi di atas");
        let det = info.determinant();
        // Determinan bisa <=0 (info kumulatif belum penuh rank -- umum di
        // awal tes sebelum cukup item terjawab untuk menginformasi semua
        // dimensi). Suku log(det) dilewati (kontribusi nol) alih-alih NaN
        // meledakkan seluruh objective; likelihood murni tetap berjalan.
        let bias_term = if det > 0.0 { 0.5 * det.ln() } else { 0.0 };
        ll + bias_term
    };

    let initial = DVector::from_vec(person.theta().as_slice().to_vec());
    let result = maximize(objective, initial);
    let se = se_from_hessian(&result.hessian_at_optimum, nfact);

    Ok(WleResult {
        theta: ThetaVector::from_vec(result.theta.as_slice().to_vec()),
        se,
        iterations: result.iterations,
        log_likelihood_trace: result.objective_trace,
        converged: result.converged,
    })
}

#[cfg(test)]
mod tests {
    use super::super::ml;
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

    /// Checklist Fase 3: WLE harus punya bias lebih kecil dibanding ML pada
    /// simulasi respons ekstrem (semua benar), di mana ML divergen ke
    /// +infinity sementara WLE tetap terbatas (koreksi bias Warm menahan
    /// estimasi supaya tidak meledak untuk pola respons "sempurna").
    #[test]
    fn wle_stays_finite_and_less_extreme_than_ml_on_all_correct_pattern() {
        let bank = bank_n_items(6);
        let mut person = PersonState::new(1);
        for i in 0..6 {
            person.record_answer(i, 1);
        }
        let group = GroupParams::standard_normal(1);

        let wle_result = estimate(&person, &bank, &group).unwrap();
        let ml_result = ml::estimate(&person, &bank, &group).unwrap();

        assert!(wle_result.theta[0].is_finite(), "WLE harus tetap terbatas");
        assert_eq!(ml_result.theta[0], f64::INFINITY, "ML harus divergen (pembanding)");
        assert!(wle_result.theta[0] > 0.0, "arah tetap ke atas (semua benar)");
    }

    #[test]
    fn wle_gives_finite_estimate_on_mixed_pattern() {
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
    }

    #[test]
    fn wle_with_no_answers_does_not_panic() {
        let bank = bank_n_items(3);
        let person = PersonState::new(1);
        let group = GroupParams::standard_normal(1);
        let result = estimate(&person, &bank, &group).unwrap();
        assert!(result.theta[0].is_finite());
    }
}
