//! `Method::Fixed` — identity murni, tidak melakukan estimasi ulang. Dipakai
//! untuk kuesioner non-adaptif (semua item wajib dijawab, theta/SE
//! dibiarkan seperti nilai terakhir) — persis cabang `method != 'fixed'`
//! yang dilewati di `mirtCAT::Design$Update.thetas`
//! (`mirtCAT/R/DesignClass.R:87-89`, dipanggil `person$Update_thetas`
//! dengan `person$thetas`/SE riwayat terakhir apa adanya, tanpa panggilan
//! `fscores()` sama sekali).

use crate::item_selection::PersonState;
use crate::theta::{SEVector, ThetaVector};

/// Kembalikan `(theta, SE)` `PersonState` apa adanya, tanpa perubahan.
pub(crate) fn estimate(person: &PersonState) -> (ThetaVector, SEVector) {
    (person.theta().clone(), person.se().clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theta::SEVector as SE;

    #[test]
    fn fixed_does_not_change_theta_or_se() {
        let theta = ThetaVector::from_vec(vec![0.3, -0.7]);
        let se = SE::from_vec(vec![0.4, 0.5]);
        let person = PersonState::from_theta_and_se(theta.clone(), se.clone());
        let (out_theta, out_se) = estimate(&person);
        assert_eq!(out_theta, theta);
        assert_eq!(out_se, se);
    }

    #[test]
    fn fixed_ignores_answered_items() {
        let theta = ThetaVector::from_vec(vec![0.1]);
        let se = SE::from_vec(vec![0.2]);
        let mut person = PersonState::from_theta_and_se(theta.clone(), se.clone());
        person.record_answer(0, 1);
        person.record_answer(1, 0);
        let (out_theta, out_se) = estimate(&person);
        assert_eq!(out_theta, theta);
        assert_eq!(out_se, se);
    }
}
