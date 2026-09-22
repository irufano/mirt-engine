//! EAP berbasis summed score (`EAPsum`). Padanan `mirt::EAPsum()`
//! (`mirt/R/fscores.internal.R:777-899`, jalur non-`bfactor` non-`discrete`).
//!
//! **Keputusan desain penting** (diverifikasi empiris terhadap R
//! sungguhan, bukan cuma dibaca dari dokumentasi): fungsi ini beroperasi
//! hanya atas item yang **SUDAH DIJAWAB** (`person.answered_items()`),
//! bukan seluruh `ItemBank` + `NA` untuk item belum dijawab seperti pola
//! `mirt::fscores(response.pattern=...)`. Saya bangun dua model R berbeda
//! (satu berisi HANYA item yang "dijawab", satu berisi seluruh bank dengan
//! sisanya `NA` di `response.pattern`) dan hasil `EAPsum` keduanya **identik
//! ke banyak digit signifikan** — jadi `mirt` sendiri secara efektif
//! mengabaikan item ber-respons `NA` dari rekursi Lord & Wingersky, bukan
//! menyertakannya. Mengabaikan item belum terjawab karenanya adalah port
//! yang benar sekaligus natural untuk `PersonState` yang memang cuma
//! menyimpan daftar item terjawab.
//!
//! Algoritma: rekursi Lord & Wingersky (1984) — konvolusi distribusi
//! kategori tiap item terjawab menghasilkan `P(total_score=s | theta_g)`
//! untuk tiap titik grid `g` (grid sama seperti EAP grid-based, lih.
//! `quadrature::build_grid`), lalu EAP per kategori skor total
//! (`s = 0..=max_score`) memakai formula sama seperti EAP biasa tapi
//! "likelihood" diganti `P(score=s | theta_g)`.

use crate::bank::{GroupParams, ItemBank};
use crate::error::MirtCatError;
use crate::irt::ItemResponseModel;
use crate::item_selection::PersonState;
use crate::theta::{SEVector, ThetaVector};

use super::density::log_dmvnorm;
use super::quadrature::build_grid;

/// Satu baris pemetaan skor total -> `(theta, SE)`.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoreEstimate {
    /// Skor total (jumlah kategori respons benar/terpilih) baris ini.
    pub sum_score: usize,
    /// Estimasi `theta` EAP untuk `sum_score`.
    pub theta: ThetaVector,
    /// SE dari `theta` untuk `sum_score`.
    pub se: SEVector,
}

pub(crate) struct EapSumResult {
    /// Seluruh kategori skor mungkin (`0..=max_score`), terurut menaik.
    pub by_score: Vec<ScoreEstimate>,
    /// Estimasi untuk skor total peserta SEKARANG (hasil utama yang
    /// dikembalikan `ability_estimation::estimate`).
    pub current: ScoreEstimate,
    pub n_quadrature_points: usize,
}

pub(crate) fn estimate(
    person: &PersonState,
    bank: &ItemBank,
    group: &GroupParams,
    quadpts: usize,
) -> Result<EapSumResult, MirtCatError> {
    let nfact = person.nfact();
    let items: Vec<&dyn ItemResponseModel> = person
        .answered_items()
        .iter()
        .map(|&idx| {
            bank.item(idx).ok_or_else(|| {
                MirtCatError::InvalidParameter(format!(
                    "ability_estimation::eap_sum: index item {idx} di luar jangkauan ItemBank"
                ))
            })
        })
        .collect::<Result<_, _>>()?;

    let grid = build_grid(nfact, quadpts)?;
    let n = grid.len();
    let mut log_weights = Vec::with_capacity(n);
    for p in &grid {
        log_weights.push(log_dmvnorm(p, group.gmeans(), group.gcov())?);
    }

    let max_score: usize = items.iter().map(|it| it.n_categories() - 1).sum();
    let n_scores = max_score + 1;

    // l1[g][s] = P(total_score=s | theta_g), lewat konvolusi Lord-Wingersky
    // (Lord & Wingersky, 1984): mulai dari distribusi "skor 0 pasti"
    // (P(0)=1), lalu tiap item mengonvolusi kategori responsnya sendiri ke
    // distribusi skor parsial.
    let mut l1: Vec<Vec<f64>> = Vec::with_capacity(n);
    for theta_g in &grid {
        let mut dist = vec![1.0_f64];
        for item in &items {
            let probs = item.probabilities(theta_g)?;
            let mut next = vec![0.0; dist.len() + probs.len() - 1];
            for (s, &p_s) in dist.iter().enumerate() {
                if p_s == 0.0 {
                    continue;
                }
                for (k, &p_k) in probs.iter().enumerate() {
                    next[s + k] += p_s * p_k;
                }
            }
            dist = next;
        }
        l1.push(dist);
    }

    let mut by_score = Vec::with_capacity(n_scores);
    // `s` dipakai sebagai nilai index literal (`ScoreEstimate::sum_score`),
    // bukan cuma untuk mengakses elemen -- iterator `enumerate()` tidak
    // menyederhanakan di sini.
    #[allow(clippy::needless_range_loop)]
    for s in 0..n_scores {
        let log_l1_w: Vec<f64> = (0..n)
            .map(|g| l1[g][s].max(1e-300).ln() + log_weights[g])
            .collect();
        let max_lw = log_l1_w.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        let mut nc = 0.0;
        let mut num = vec![0.0; nfact];
        let mut expl_w_cache = vec![0.0; n];
        for g in 0..n {
            let w = (log_l1_w[g] - max_lw).exp();
            expl_w_cache[g] = w;
            nc += w;
            for j in 0..nfact {
                num[j] += grid[g][j] * w;
            }
        }
        let theta_hat: Vec<f64> = num.iter().map(|v| v / nc).collect();

        let mut var = vec![0.0; nfact];
        for g in 0..n {
            for j in 0..nfact {
                var[j] += (grid[g][j] - theta_hat[j]).powi(2) * expl_w_cache[g];
            }
        }
        let se: Vec<f64> = var.iter().map(|v| (v / nc).sqrt()).collect();

        by_score.push(ScoreEstimate {
            sum_score: s,
            theta: ThetaVector::from_vec(theta_hat),
            se: SEVector::from_vec(se),
        });
    }

    let current_score: usize = person.responses().iter().sum();
    let current = by_score.get(current_score).cloned().ok_or_else(|| {
        MirtCatError::InvalidParameter(format!(
            "ability_estimation::eap_sum: skor total {current_score} di luar jangkauan \
                 0..={max_score} -- kategori respons tercatat tidak valid untuk item terkait"
        ))
    })?;

    Ok(EapSumResult {
        by_score,
        current,
        n_quadrature_points: n,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bank::ItemMeta;
    use crate::irt::Dichotomous;
    use approx::assert_relative_eq;

    fn bank_2items_1d() -> ItemBank {
        let mut bank = ItemBank::new();
        bank.push(
            Box::new(Dichotomous::m2pl(vec![1.2], 0.0).unwrap()),
            ItemMeta::new(0),
        )
        .unwrap();
        bank.push(
            Box::new(Dichotomous::m2pl(vec![0.9], 0.1).unwrap()),
            ItemMeta::new(1),
        )
        .unwrap();
        bank
    }

    /// Cross-check terhadap `eap::estimate` murni: untuk SATU item
    /// dikotomus, rekursi Lord-Wingersky trivial mereduksi ke likelihood
    /// item itu sendiri -- skor 1 (benar) harus PERSIS sama dengan hasil
    /// EAP grid-based untuk pola respons "item 0 dijawab benar", dan skor 0
    /// sama dengan "dijawab salah".
    #[test]
    fn eap_sum_single_item_matches_plain_eap_grid() {
        let mut bank = ItemBank::new();
        bank.push(
            Box::new(Dichotomous::m2pl(vec![1.3], 0.0).unwrap()),
            ItemMeta::new(0),
        )
        .unwrap();
        let group = GroupParams::standard_normal(1);
        let quadpts = 61;

        let mut person_correct = PersonState::new(1);
        person_correct.record_answer(0, 1);
        let sum_result = estimate(&person_correct, &bank, &group, quadpts).unwrap();
        let plain_result =
            super::super::eap::estimate(&person_correct, &bank, &group, quadpts, false).unwrap();

        assert_relative_eq!(
            sum_result.by_score[1].theta[0],
            plain_result.theta[0],
            epsilon = 1e-9
        );
        assert_relative_eq!(
            sum_result.by_score[1].se[0],
            plain_result.se[0],
            epsilon = 1e-9
        );

        let mut person_incorrect = PersonState::new(1);
        person_incorrect.record_answer(0, 0);
        let sum_result_incorrect = estimate(&person_incorrect, &bank, &group, quadpts).unwrap();
        let plain_incorrect =
            super::super::eap::estimate(&person_incorrect, &bank, &group, quadpts, false).unwrap();
        assert_relative_eq!(
            sum_result_incorrect.by_score[0].theta[0],
            plain_incorrect.theta[0],
            epsilon = 1e-9
        );
    }

    #[test]
    fn eap_sum_theta_increases_monotonically_with_score_category() {
        let bank = bank_2items_1d();
        let group = GroupParams::standard_normal(1);
        let mut person = PersonState::new(1);
        person.record_answer(0, 1);
        person.record_answer(1, 1);
        let result = estimate(&person, &bank, &group, 61).unwrap();

        assert_eq!(result.by_score.len(), 3); // skor 0, 1, 2
        assert!(result.by_score[0].theta[0] < result.by_score[1].theta[0]);
        assert!(result.by_score[1].theta[0] < result.by_score[2].theta[0]);
    }

    #[test]
    fn current_maps_to_actual_answered_sum_score() {
        let bank = bank_2items_1d();
        let group = GroupParams::standard_normal(1);
        let mut person = PersonState::new(1);
        person.record_answer(0, 1);
        person.record_answer(1, 0);
        let result = estimate(&person, &bank, &group, 61).unwrap();
        assert_eq!(result.current.sum_score, 1);
        assert_eq!(result.current.theta, result.by_score[1].theta);
    }

    #[test]
    fn eap_sum_with_no_answers_has_single_zero_score_category() {
        let bank = bank_2items_1d();
        let group = GroupParams::standard_normal(1);
        let person = PersonState::new(1);
        let result = estimate(&person, &bank, &group, 61).unwrap();
        assert_eq!(result.by_score.len(), 1);
        assert_eq!(result.current.sum_score, 0);
    }
}
