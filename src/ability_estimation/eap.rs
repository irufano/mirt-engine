//! EAP (Expected A Posteriori) — grid-based & QMC-based. Padanan `mirt::EAP()`
//! (`mirt/R/fscores.internal.R:1121-1165`) & pembangunan grid/bobot di
//! `fscores.internal.R:836-847`.
//!
//! - Grid-based: bobot = `dmvnorm(grid; gmeans, gcov)`.
//! - QMC-based: titik dari `quadrature::build_qmc_points` (awan kuasi-`N(0,I)`,
//!   **tidak** diskalakan ke `gmeans`/`gcov`), bobot **seragam** — persis
//!   `mirt::QMC_quad`/`W <- if(QMC) rep(1, nrow(Theta)) else den_fun(...)`.
//!
//! Hanya memakai item yang **sudah dijawab** (`person.answered_items()` +
//! `person.responses()`) — item yang belum dijawab tidak berkontribusi ke
//! likelihood (setara `as.logical(patdata)` di R yang cuma memilih kategori
//! item yang terobservasi).

use crate::bank::{GroupParams, ItemBank};
use crate::error::MirtCatError;
use crate::item_selection::PersonState;
use crate::theta::{SEVector, ThetaVector};

use super::density::log_dmvnorm;
use super::quadrature::{build_grid, build_qmc_points};
use super::log_likelihood_at;

/// Hasil EAP + jumlah titik kuadratur dipakai (untuk `AbilityEstimationDebug`).
pub(crate) struct EapResult {
    pub theta: ThetaVector,
    pub se: SEVector,
    pub n_quadrature_points: usize,
}

/// EAP grid-based (`qmc=false`) atau QMC-based (`qmc=true`). `quadpts` =
/// jumlah titik per dimensi (grid, total `quadpts^nfact`) atau jumlah titik
/// total (QMC).
pub(crate) fn estimate(
    person: &PersonState,
    bank: &ItemBank,
    group: &GroupParams,
    quadpts: usize,
    qmc: bool,
) -> Result<EapResult, MirtCatError> {
    let nfact = person.nfact();

    let (grid, log_weights) = if qmc {
        let points = build_qmc_points(nfact, quadpts)?;
        let log_w = vec![0.0; points.len()]; // seragam -- konstanta, hilang saat dinormalisasi
        (points, log_w)
    } else {
        let points = build_grid(nfact, quadpts)?;
        let mut log_w = Vec::with_capacity(points.len());
        for p in &points {
            log_w.push(log_dmvnorm(p, group.gmeans(), group.gcov())?);
        }
        (points, log_w)
    };
    let n = grid.len();

    let log_lik = log_likelihood_per_grid_point(person, bank, &grid)?;

    let max_lw = log_lik
        .iter()
        .zip(&log_weights)
        .map(|(l, w)| l + w)
        .fold(f64::NEG_INFINITY, f64::max);
    let mut nc = 0.0;
    let mut num = vec![0.0; nfact];
    let mut expl_w_cache = Vec::with_capacity(n);
    for g in 0..n {
        let lw = log_lik[g] + log_weights[g];
        let expl_w = (lw - max_lw).exp();
        expl_w_cache.push(expl_w);
        nc += expl_w;
        for j in 0..nfact {
            num[j] += grid[g][j] * expl_w;
        }
    }
    if nc == 0.0 || !nc.is_finite() {
        return Err(MirtCatError::InvalidParameter(
            "ability_estimation::eap::estimate: normalizing constant nol/non-finite \
             -- pertimbangkan method Map"
                .into(),
        ));
    }
    let theta_hat: Vec<f64> = num.iter().map(|v| v / nc).collect();

    // SE dari momen kedua posterior tertimbang (hanya diagonal), persis
    // `SE <- sqrt(diag(vcov))` di `EAP()` R.
    let mut var = vec![0.0; nfact];
    for g in 0..n {
        for j in 0..nfact {
            var[j] += (grid[g][j] - theta_hat[j]).powi(2) * expl_w_cache[g];
        }
    }
    let se: Vec<f64> = var.iter().map(|v| (v / nc).sqrt()).collect();

    Ok(EapResult {
        theta: ThetaVector::from_vec(theta_hat),
        se: SEVector::from_vec(se),
        n_quadrature_points: n,
    })
}

/// `log L(theta_g) = sum` atas item terjawab `log P_{response}(theta_g)`,
/// untuk tiap titik grid `g`. Item belum dijawab tidak berkontribusi.
/// Memakai `log_likelihood_at` (dipakai juga `map.rs`/`ml.rs`/`wle.rs`)
/// sekali per titik grid.
fn log_likelihood_per_grid_point(
    person: &PersonState,
    bank: &ItemBank,
    grid: &[ThetaVector],
) -> Result<Vec<f64>, MirtCatError> {
    grid.iter()
        .map(|theta_g| log_likelihood_at(person, bank, theta_g))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bank::ItemMeta;
    use crate::irt::Dichotomous;
    use approx::assert_relative_eq;

    fn bank_1item_1d(a: f64, d: f64) -> ItemBank {
        let mut bank = ItemBank::new();
        bank.push(Box::new(Dichotomous::m2pl(vec![a], d).unwrap()), ItemMeta::new(0))
            .unwrap();
        bank
    }

    /// Sanity check dasar 1D: item simetris (`a=1, d=0`) + prior `N(0,1)`
    /// simetris -> posterior mean untuk respons benar HARUS persis negatif
    /// dari posterior mean untuk respons salah (simetri analitik integral,
    /// bukan cuma tanda). Dihitung ulang independen lewat brute-force
    /// Riemann sum langsung di test ini (bukan lewat `build_grid` produksi)
    /// untuk verifikasi nilai numerik, bukan cuma struktur.
    #[test]
    fn eap_grid_1d_matches_independent_brute_force_and_is_symmetric() {
        let bank = bank_1item_1d(1.5, 0.0);
        let group = GroupParams::standard_normal(1);

        let mut person_correct = PersonState::new(1);
        person_correct.record_answer(0, 1);
        let mut person_incorrect = PersonState::new(1);
        person_incorrect.record_answer(0, 0);

        let quadpts = 401; // grid halus untuk mendekati integral kontinu
        let correct = estimate(&person_correct, &bank, &group, quadpts, false).unwrap();
        let incorrect = estimate(&person_incorrect, &bank, &group, quadpts, false).unwrap();

        assert_relative_eq!(correct.theta[0], -incorrect.theta[0], epsilon = 1e-9);
        assert!(correct.theta[0] > 0.0, "respons benar harus menarik theta positif");

        // Brute-force independen: integral kontinu didekati Riemann sum
        // langsung (bukan reuse `build_grid`), dibandingkan terhadap hasil
        // produksi di atas.
        let item = bank.item(0).unwrap();
        let n = 20001;
        let lo = -6.0;
        let hi = 6.0;
        let step = (hi - lo) / (n - 1) as f64;
        let mut num = 0.0;
        let mut den = 0.0;
        for i in 0..n {
            let t = lo + step * i as f64;
            let theta = ThetaVector::from_vec(vec![t]);
            let p1 = item.probabilities(&theta).unwrap()[1];
            let prior = (-0.5 * t * t).exp(); // proporsional N(0,1), konstanta hilang saat normalisasi
            let w = p1 * prior;
            num += t * w;
            den += w;
        }
        let expected_theta = num / den;
        assert_relative_eq!(correct.theta[0], expected_theta, epsilon = 1e-3);
    }

    #[test]
    fn eap_returns_prior_mean_when_no_items_answered() {
        let bank = bank_1item_1d(1.0, 0.0);
        let mean = ThetaVector::from_vec(vec![0.4]);
        let group = GroupParams::new(mean.clone(), crate::theta::InfoMatrix::identity(1)).unwrap();
        let person = PersonState::new(1);
        let result = estimate(&person, &bank, &group, 201, false).unwrap();
        assert_relative_eq!(result.theta[0], 0.4, epsilon = 1e-6);
    }

    #[test]
    fn qmc_and_grid_agree_approximately_on_same_scenario() {
        let bank = bank_1item_1d(1.2, 0.0);
        let group = GroupParams::standard_normal(1);
        let mut person = PersonState::new(1);
        person.record_answer(0, 1);

        let grid_result = estimate(&person, &bank, &group, 401, false).unwrap();
        let qmc_result = estimate(&person, &bank, &group, 4000, true).unwrap();

        assert_relative_eq!(grid_result.theta[0], qmc_result.theta[0], epsilon = 0.05);
    }

    #[test]
    fn n_quadrature_points_matches_grid_and_qmc_sizes() {
        let bank = bank_1item_1d(1.0, 0.0);
        let group = GroupParams::standard_normal(2);
        let mut bank2d = ItemBank::new();
        bank2d
            .push(
                Box::new(Dichotomous::m2pl(vec![1.0, 0.5], 0.0).unwrap()),
                ItemMeta::new(0),
            )
            .unwrap();
        let _ = bank;
        let mut person = PersonState::new(2);
        person.record_answer(0, 1);

        let grid_result = estimate(&person, &bank2d, &group, 5, false).unwrap();
        assert_eq!(grid_result.n_quadrature_points, 25);

        let qmc_result = estimate(&person, &bank2d, &group, 123, true).unwrap();
        assert_eq!(qmc_result.n_quadrature_points, 123);
    }

    #[test]
    fn out_of_bank_answered_item_errors_not_panics() {
        let bank = bank_1item_1d(1.0, 0.0);
        let group = GroupParams::standard_normal(1);
        let mut person = PersonState::new(1);
        person.record_answer(999, 1);
        assert!(estimate(&person, &bank, &group, 21, false).is_err());
    }
}
