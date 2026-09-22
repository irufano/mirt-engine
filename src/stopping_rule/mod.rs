//! Fase 4 — Stopping Rule pada MCAT: kapan sesi tes berhenti.
//!
//! Semantik **"ALL dimensi"** untuk kriteria presisi (`min_SEM`,
//! `delta_thetas`, klasifikasi `CI`) — konsekuensi langsung dari `theta`
//! berupa vektor pada MCAT: makin banyak `nfact`, makin sulit syarat "all"
//! terpenuhi secara statistik, sehingga `max_items`/`max_time` sering jadi
//! kondisi berhenti praktis untuk model dimensi tinggi (lih.
//! `docs/panduan/04-stopping-rule-overview.md`).
//!
//! Ground truth: `mirtCAT::Design$Update.stop_now`
//! (`mirtCAT/R/DesignClass.R:312-381`, dikonfirmasi identik pada R
//! 4.6.1/mirtCAT 1.14 terpasang & checkout source 1.14.3). Mode klasifikasi
//! `SPRT` (Wald, 1945) **berbeda semantik** dari `CI`: bukan "all dimensi",
//! melainkan satu uji rasio log-likelihood sekuensial gabungan (lih.
//! [`sprt`]).
//!
//! `.Object@stage > 1L` di R (terkait transisi tahap preCAT) diabaikan di
//! sini -- preCAT di luar scope crate ini (belum diimplementasikan), dan
//! tanpa preCAT `stage` selalu `2L` di R (selalu memenuhi `> 1L`).
//!
//! **Deviasi dari sketsa desain awal `PLAN.md`** (persis pola deviasi yang
//! sudah terjadi di Fase 2 [`select_next_item` menambah parameter `rng`
//! yang tidak ada di sketsa awal] dan Fase 3 [modul berubah total setelah
//! riset source `mirt`]):
//! - Parameter `test_state` pada sketsa `evaluate(config, person_state,
//!   test_state)` dipecah jadi `bank: &ItemBank` (dibutuhkan mode `Sprt`,
//!   yang menghitung log-likelihood atas item terjawab) dan
//!   `in_unfinished_constraint_block: bool`. Constraint (`ordered`/
//!   `unordered` block) adalah tanggung jawab Fase 6 (Content Balancing)
//!   yang belum diimplementasikan -- membangun tipe constraint penuh
//!   sekarang adalah scaffolding prematur. Guard `allow_constrain_breaks`
//!   R hanya butuh SATU fakta dari subsistem itu (apakah item terakhir ada
//!   di tengah blok yang belum tuntas), jadi cukup diteruskan sebagai
//!   `bool` oleh pemanggil; begitu Fase 6 ada, pemanggil cukup mengisi
//!   `bool` ini dari state constraint sungguhan tanpa perlu desain ulang
//!   `evaluate`.
//! - [`CustomStop`] sengaja jadi parameter terpisah (`Option<&dyn
//!   CustomStop>`), BUKAN field `StoppingConfig` -- `dyn CustomStop` tidak
//!   bisa `#[derive(Clone, PartialEq)]`, sedangkan seluruh config Fase 1-3
//!   (`InitialThetaConfig`, `SelectionConfig`, `EstimationConfig`) memakai
//!   pola derive itu; menambah field trait-object akan merusak pola
//!   tersebut untuk satu field yang jarang dipakai.
//!
//! **Reuse eksplisit** (Prinsip Desain #1 PLAN.md -- jangan duplikasi):
//! [`sprt::evaluate`] memakai [`crate::ability_estimation::log_likelihood_at`]
//! (Fase 3, sudah teruji) untuk `LL0`/`LL1`, dan [`classify_ci::evaluate`]
//! memakai [`crate::ability_estimation::quadrature::inverse_normal_cdf`]
//! (padanan `qnorm()`, sudah ada) -- tidak menulis ulang keduanya maupun
//! menambah dependency statistik baru.

mod classify_ci;
mod delta_theta;
mod max_items_time;
mod sem;
mod sprt;

use std::time::Duration;

use crate::bank::ItemBank;
use crate::error::MirtCatError;
use crate::item_selection::PersonState;
use crate::theta::{SEVector, ThetaVector};

/// Keputusan klasifikasi satu dimensi (`Ci`) atau gabungan (`Sprt`) --
/// padanan string `'above cutoff'`/`'below cutoff'`/`'no decision'` R.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ClassifyDecision {
    AboveCutoff,
    BelowCutoff,
    NoDecision,
}

/// Mode klasifikasi -- enum tertutup persis 3 mode valid `mirtCAT`
/// (`classify_type`: `'none'`/`'CI'`/`'SPRT'`).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ClassifyType {
    /// Tidak ada klasifikasi -- stopping presisi murni lewat `min_SEM`.
    #[default]
    None,
    /// Klasifikasi berbasis confidence interval per-dimensi terhadap
    /// `cutoffs` (z-score vs `alpha`, lih. [`classify_ci`]).
    Ci { cutoffs: Vec<f64>, alpha: f64 },
    /// Uji rasio log-likelihood sekuensial Wald antara hipotesis `lower`
    /// vs `upper` (lih. [`sprt`]). `alpha`/`beta` menurunkan batas `[a, b]`
    /// (padanan `sprt_alpha`/`sprt_beta` R, default `0.05` masing-masing).
    Sprt {
        lower: ThetaVector,
        upper: ThetaVector,
        alpha: f64,
        beta: f64,
    },
}

/// Konfigurasi stopping rule.
#[derive(Debug, Clone, PartialEq)]
pub struct StoppingConfig {
    /// Jumlah item minimum sebelum kriteria presisi (`min_SEM`/
    /// `delta_thetas`/klasifikasi) mulai dievaluasi. Default `1` (padanan
    /// default `min_items` R).
    pub min_items: usize,
    /// Jumlah item maksimum -- begitu tercapai, tes berhenti terlepas dari
    /// presisi. Tidak ada default universal yang masuk akal (R men-default
    /// ke ukuran bank; `Default::default()` di sini memakai `usize::MAX`,
    /// pemanggil disarankan selalu override sesuai ukuran bank sungguhan).
    pub max_items: usize,
    /// Batas waktu total -- `None` berarti tidak terbatas (padanan
    /// `max_time = Inf` R).
    pub max_time: Option<Duration>,
    /// Ambang SE per-dimensi untuk `met_SEM` (hanya dipakai bila
    /// `classify == ClassifyType::None`). Panjang `1` (broadcast ke
    /// `nfact`) atau persis `nfact`, sama seperti `min_SEM` R -- lih.
    /// [`broadcast`]. Default `vec![0.3]` (padanan default R).
    pub min_sem: Vec<f64>,
    /// Ambang `|Δtheta|` per-dimensi untuk `met_delta_thetas`. Aturan
    /// panjang sama seperti `min_sem`. Default `vec![0.0]` (nonaktif,
    /// padanan default R `rep(0, nfact)` -- `|Δtheta| < 0` tidak pernah
    /// terpenuhi).
    pub delta_thetas: Vec<f64>,
    /// Mode klasifikasi. Default `ClassifyType::None`.
    pub classify: ClassifyType,
    /// `false` (default) -- mencegah berhenti di tengah blok
    /// `ordered`/`unordered` constraint yang belum selesai (lih. parameter
    /// `in_unfinished_constraint_block` pada [`evaluate`]).
    pub allow_constrain_breaks: bool,
    /// Aktifkan jejak debug (Prinsip Desain #7). Default `false`.
    pub debug: bool,
}

impl Default for StoppingConfig {
    fn default() -> Self {
        Self {
            min_items: 1,
            max_items: usize::MAX,
            max_time: None,
            min_sem: vec![0.3],
            delta_thetas: vec![0.0],
            classify: ClassifyType::default(),
            allow_constrain_breaks: false,
            debug: false,
        }
    }
}

/// Hasil evaluasi stopping rule satu langkah.
#[derive(Debug, Clone, PartialEq)]
pub struct StopDecision {
    pub stop_now: bool,
    /// Hanya relevan bila `classify == ClassifyType::None`; `vec![false;
    /// nfact]` (nilai default R yang tidak pernah ditimpa) untuk mode
    /// klasifikasi lain.
    pub met_sem: Vec<bool>,
    /// Selalu dievaluasi bila `n_answered >= min_items`, independen dari
    /// `classify` (persis R) -- `vec![false; nfact]` bila belum ada
    /// estimasi sebelumnya untuk dibandingkan.
    pub met_delta_thetas: Vec<bool>,
    /// `Some(per-dimensi)` untuk `Ci`, `Some(vec![stop])` (1 elemen,
    /// keputusan gabungan) untuk `Sprt`, `None` untuk `ClassifyType::None`.
    pub met_classify: Option<Vec<bool>>,
    /// `Ci`: panjang `nfact`. `Sprt`: panjang `1` (keputusan gabungan,
    /// BUKAN per-dimensi -- persis R yang skalar untuk mode ini).
    /// `ClassifyType::None`: kosong.
    pub classification: Vec<ClassifyDecision>,
    /// `true` hanya bila berhenti dipicu kriteria presisi (`min_SEM`/
    /// `delta_thetas`/klasifikasi) -- **bukan** `max_items`/`max_time`/
    /// `CustomStop`, persis semantik `person$terminated_early` R.
    pub terminated_early: bool,
}

/// Jejak debug opsional dari [`evaluate`] -- hanya `Some` bila
/// `StoppingConfig::debug == true`; tidak memengaruhi [`StopDecision`] yang
/// dikembalikan (Prinsip Desain #7).
#[derive(Debug, Clone, PartialEq)]
pub struct StoppingRuleDebug {
    pub se_values: SEVector,
    /// `|Δtheta|` mentah per-dimensi -- kosong bila belum ada
    /// `previous_theta` untuk dibandingkan.
    pub delta_theta_values: Vec<f64>,
    /// `Some` hanya untuk mode `Sprt`.
    pub sprt_log_likelihood_ratio: Option<f64>,
    /// `Some((a, b))` hanya untuk mode `Sprt`.
    pub sprt_bounds: Option<(f64, f64)>,
}

/// Hook kustom pengguna untuk menghentikan sesi -- padanan `customStop` R.
/// Hasil `check()` **menimpa total** `stop_now` yang sudah dihitung
/// [`evaluate`] (lih. catatan di `evaluate`), persis `stop_now <-
/// customStop(...)` R yang dieksekusi paling akhir tanpa syarat.
///
/// Blanket impl untuk `Fn(&PersonState) -> bool` supaya closure biasa bisa
/// dipakai langsung tanpa perlu struct pembungkus.
pub trait CustomStop {
    fn check(&self, person: &PersonState) -> bool;
}

impl<F> CustomStop for F
where
    F: Fn(&PersonState) -> bool,
{
    fn check(&self, person: &PersonState) -> bool {
        self(person)
    }
}

/// Broadcast/validasi panjang vektor per-dimensi (`min_sem`/`delta_thetas`):
/// panjang `1` disebar ke `nfact`, panjang `nfact` dipakai apa adanya,
/// selain itu error -- persis pesan error R `"... criteria is not a
/// suitable length"` (`DesignClass.R:275-276`).
fn broadcast(values: &[f64], nfact: usize, context: &'static str) -> Result<Vec<f64>, MirtCatError> {
    match values.len() {
        1 => Ok(vec![values[0]; nfact]),
        n if n == nfact => Ok(values.to_vec()),
        n => Err(MirtCatError::InvalidParameter(format!(
            "{context}: panjang harus 1 (broadcast ke nfact) atau {nfact} (nfact), didapat {n}"
        ))),
    }
}

/// Evaluasi stopping rule untuk `person` saat ini.
///
/// `bank` dibutuhkan mode `ClassifyType::Sprt` (log-likelihood atas item
/// terjawab); diabaikan mode lain. `in_unfinished_constraint_block` dan
/// `custom_stop` -- lih. catatan deviasi desain di dokumentasi modul ini.
///
/// **Urutan evaluasi PERSIS `Update.stop_now` R** (`DesignClass.R:312-381`):
/// 1. Bila `person.n_answered() >= config.min_items`: evaluasi
///    `min_SEM`/`Ci`/`Sprt` sesuai `config.classify`, LALU (independen)
///    `delta_thetas` -- keduanya bisa memicu `stop_now` & `terminated_early`.
/// 2. `max_items` tercapai -> `stop_now = true` (tidak set `terminated_early`).
/// 3. `max_time` tercapai -> `stop_now = true` (tidak set `terminated_early`).
/// 4. `!allow_constrain_breaks && in_unfinished_constraint_block` -> paksa
///    `stop_now = false`, MENIMPA langkah 1-3.
/// 5. `custom_stop` ada -> **timpa total** `stop_now` dengan hasilnya --
///    bisa membatalkan ATAU memaksakan stop sekalipun langkah 1-4
///    berkata sebaliknya (termasuk `max_items`!), persis R.
///
/// Error bila panjang `min_sem`/`delta_thetas` bukan `1` atau `nfact`,
/// panjang `Ci::cutoffs`/`Sprt::lower`/`Sprt::upper` bukan persis `nfact`,
/// atau index item terjawab di luar jangkauan `bank` (mode `Sprt`).
pub fn evaluate(
    config: &StoppingConfig,
    person: &PersonState,
    bank: &ItemBank,
    in_unfinished_constraint_block: bool,
    custom_stop: Option<&dyn CustomStop>,
) -> Result<(StopDecision, Option<StoppingRuleDebug>), MirtCatError> {
    let nfact = person.nfact();
    let min_sem = broadcast(&config.min_sem, nfact, "StoppingConfig::min_sem")?;
    let delta_thetas = broadcast(&config.delta_thetas, nfact, "StoppingConfig::delta_thetas")?;

    let mut stop_now = false;
    let mut terminated_early = false;
    let mut met_sem = vec![false; nfact];
    let mut met_classify: Option<Vec<bool>> = None;
    let mut classification: Vec<ClassifyDecision> = Vec::new();
    let mut met_delta_thetas = vec![false; nfact];

    let mut debug_sprt_llr: Option<f64> = None;
    let mut debug_sprt_bounds: Option<(f64, f64)> = None;

    if person.n_answered() >= config.min_items {
        match &config.classify {
            ClassifyType::Sprt {
                lower,
                upper,
                alpha,
                beta,
            } => {
                lower.validate_nfact(nfact, "StoppingConfig::classify Sprt.lower")?;
                upper.validate_nfact(nfact, "StoppingConfig::classify Sprt.upper")?;
                let outcome = sprt::evaluate(person, bank, lower, upper, *alpha, *beta)?;
                met_classify = Some(vec![outcome.stop_now]);
                classification = vec![outcome.decision];
                if outcome.stop_now {
                    stop_now = true;
                    terminated_early = true;
                }
                if config.debug {
                    debug_sprt_llr = Some(outcome.log_likelihood_ratio);
                    debug_sprt_bounds = Some(outcome.bounds);
                }
            }
            ClassifyType::Ci { cutoffs, alpha } => {
                if cutoffs.len() != nfact {
                    return Err(MirtCatError::DimensionMismatch {
                        expected: nfact,
                        actual: cutoffs.len(),
                        context: "StoppingConfig::classify Ci.cutoffs",
                    });
                }
                let (met, decisions) = classify_ci::evaluate(person.theta(), person.se(), cutoffs, *alpha);
                let all_met = met.iter().all(|&m| m);
                met_classify = Some(met);
                classification = decisions;
                if all_met {
                    stop_now = true;
                    terminated_early = true;
                }
            }
            ClassifyType::None => {
                let met = sem::evaluate(person.se(), &min_sem);
                let all_met = met.iter().all(|&m| m);
                met_sem = met;
                if all_met {
                    stop_now = true;
                    terminated_early = true;
                }
            }
        }

        met_delta_thetas = delta_theta::evaluate(person.theta(), person.previous_theta(), &delta_thetas);
        if met_delta_thetas.iter().all(|&m| m) {
            stop_now = true;
            terminated_early = true;
        }
    }

    if max_items_time::max_items_reached(person.n_answered(), config.max_items) {
        stop_now = true;
    }
    if max_items_time::max_time_reached(person.total_time(), config.max_time) {
        stop_now = true;
    }

    if !config.allow_constrain_breaks && in_unfinished_constraint_block {
        stop_now = false;
    }

    if let Some(custom) = custom_stop {
        stop_now = custom.check(person);
    }

    let decision = StopDecision {
        stop_now,
        met_sem,
        met_delta_thetas,
        met_classify,
        classification,
        terminated_early,
    };

    let debug = config.debug.then(|| StoppingRuleDebug {
        se_values: person.se().clone(),
        delta_theta_values: match person.previous_theta() {
            Some(prev) => (0..nfact).map(|i| (person.theta()[i] - prev[i]).abs()).collect(),
            None => Vec::new(),
        },
        sprt_log_likelihood_ratio: debug_sprt_llr,
        sprt_bounds: debug_sprt_bounds,
    });

    Ok((decision, debug))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bank::ItemMeta;
    use crate::irt::Dichotomous;

    fn bank_1item_2d() -> ItemBank {
        let mut bank = ItemBank::new();
        bank.push(
            Box::new(Dichotomous::m2pl(vec![1.0, 1.0], 0.0).unwrap()),
            ItemMeta::new(0),
        )
        .unwrap();
        bank
    }

    fn person_with_se(se: Vec<f64>) -> PersonState {
        let nfact = se.len();
        let mut person = PersonState::from_theta_and_se(ThetaVector::zeros(nfact), SEVector::from_vec(se));
        person.record_answer(0, 1);
        person
    }

    #[test]
    fn default_config_has_min_items_one_and_disabled_delta_thetas() {
        let config = StoppingConfig::default();
        assert_eq!(config.min_items, 1);
        assert_eq!(config.min_sem, vec![0.3]);
        assert_eq!(config.delta_thetas, vec![0.0]);
        assert_eq!(config.classify, ClassifyType::None);
        assert!(!config.allow_constrain_breaks);
        assert!(!config.debug);
    }

    #[test]
    fn met_sem_stops_only_when_all_dimensions_below_threshold() {
        let bank = bank_1item_2d();
        let config = StoppingConfig {
            min_sem: vec![0.3, 0.3],
            ..Default::default()
        };

        let person_one_unmet = person_with_se(vec![0.2, 0.35]);
        let (decision, _) = evaluate(&config, &person_one_unmet, &bank, false, None).unwrap();
        assert!(!decision.stop_now);
        assert_eq!(decision.met_sem, vec![true, false]);

        let person_all_met = person_with_se(vec![0.2, 0.25]);
        let (decision, _) = evaluate(&config, &person_all_met, &bank, false, None).unwrap();
        assert!(decision.stop_now);
        assert!(decision.terminated_early);
        assert_eq!(decision.met_sem, vec![true, true]);
    }

    #[test]
    fn met_delta_thetas_uses_all_semantics() {
        let bank = bank_1item_2d();
        let config = StoppingConfig {
            min_sem: vec![f64::NEG_INFINITY, f64::NEG_INFINITY], // matikan jalur SEM (tidak pernah met)
            delta_thetas: vec![0.05, 0.05],
            ..Default::default()
        };

        let mut person = PersonState::from_theta(ThetaVector::from_vec(vec![0.0, 0.0]));
        person.record_answer(0, 1);
        person.set_theta(ThetaVector::from_vec(vec![0.01, 0.20])); // dim2 melampaui delta
        let (decision, _) = evaluate(&config, &person, &bank, false, None).unwrap();
        assert!(!decision.stop_now);
        assert_eq!(decision.met_delta_thetas, vec![true, false]);

        person.set_theta(ThetaVector::from_vec(vec![0.015, 0.201])); // sekarang keduanya < 0.05
        let (decision, _) = evaluate(&config, &person, &bank, false, None).unwrap();
        assert!(decision.stop_now);
        assert!(decision.terminated_early);
        assert_eq!(decision.met_delta_thetas, vec![true, true]);
    }

    #[test]
    fn ci_classify_two_dimensions_one_confident_one_not_then_both() {
        let bank = bank_1item_2d();
        let config = StoppingConfig {
            classify: ClassifyType::Ci {
                cutoffs: vec![0.0, 0.0],
                alpha: 0.05,
            },
            ..Default::default()
        };

        let mut person = PersonState::from_theta_and_se(
            ThetaVector::from_vec(vec![3.0, 0.05]),
            SEVector::from_vec(vec![0.3, 1.0]),
        );
        person.record_answer(0, 1);
        let (decision, _) = evaluate(&config, &person, &bank, false, None).unwrap();
        assert!(!decision.stop_now);
        assert_eq!(decision.met_classify, Some(vec![true, false]));
        assert_eq!(
            decision.classification,
            vec![ClassifyDecision::AboveCutoff, ClassifyDecision::NoDecision]
        );

        person.set_estimate(
            ThetaVector::from_vec(vec![3.0, -3.0]),
            SEVector::from_vec(vec![0.3, 0.3]),
        );
        let (decision, _) = evaluate(&config, &person, &bank, false, None).unwrap();
        assert!(decision.stop_now);
        assert!(decision.terminated_early);
        assert_eq!(decision.met_classify, Some(vec![true, true]));
        assert_eq!(
            decision.classification,
            vec![ClassifyDecision::AboveCutoff, ClassifyDecision::BelowCutoff]
        );
    }

    fn bank_5items_2d() -> ItemBank {
        let mut bank = ItemBank::new();
        for i in 0..5 {
            bank.push(
                Box::new(Dichotomous::m2pl(vec![1.5, 1.5], 0.0).unwrap()),
                ItemMeta::new(i),
            )
            .unwrap();
        }
        bank
    }

    #[test]
    fn sprt_decisions_match_consistent_and_ambiguous_response_patterns() {
        let bank = bank_5items_2d();
        let config = StoppingConfig {
            classify: ClassifyType::Sprt {
                lower: ThetaVector::from_vec(vec![-1.5, -1.5]),
                upper: ThetaVector::from_vec(vec![1.5, 1.5]),
                alpha: 0.05,
                beta: 0.05,
            },
            ..Default::default()
        };

        let mut person_upper = PersonState::new(2);
        for i in 0..5 {
            person_upper.record_answer(i, 1);
        }
        let (decision, _) = evaluate(&config, &person_upper, &bank, false, None).unwrap();
        assert!(decision.stop_now);
        assert!(decision.terminated_early);
        assert_eq!(decision.classification, vec![ClassifyDecision::AboveCutoff]);

        let mut person_lower = PersonState::new(2);
        for i in 0..5 {
            person_lower.record_answer(i, 0);
        }
        let (decision, _) = evaluate(&config, &person_lower, &bank, false, None).unwrap();
        assert!(decision.stop_now);
        assert_eq!(decision.classification, vec![ClassifyDecision::BelowCutoff]);

        // `min_items: 0` supaya klasifikasi tetap dievaluasi walau belum ada
        // item terjawab (LL_diff = 0, ambigu, dalam batas [a,b]).
        let config_no_min_items = StoppingConfig {
            min_items: 0,
            ..config
        };
        let person_ambiguous = PersonState::new(2);
        let (decision, _) = evaluate(&config_no_min_items, &person_ambiguous, &bank, false, None).unwrap();
        assert!(!decision.stop_now);
        assert_eq!(decision.classification, vec![ClassifyDecision::NoDecision]);
    }

    #[test]
    fn max_items_stops_regardless_of_precision() {
        let bank = bank_1item_2d();
        let config = StoppingConfig {
            max_items: 1,
            min_sem: vec![f64::NEG_INFINITY, f64::NEG_INFINITY], // presisi tidak akan pernah terpenuhi
            ..Default::default()
        };
        let person = person_with_se(vec![5.0, 5.0]);
        let (decision, _) = evaluate(&config, &person, &bank, false, None).unwrap();
        assert!(decision.stop_now);
        assert!(!decision.terminated_early); // stop karena max_items, bukan presisi
    }

    #[test]
    fn max_time_stops_regardless_of_precision() {
        let bank = bank_1item_2d();
        let config = StoppingConfig {
            max_time: Some(Duration::from_secs(30)),
            min_sem: vec![f64::NEG_INFINITY, f64::NEG_INFINITY],
            ..Default::default()
        };
        let mut person = person_with_se(vec![5.0, 5.0]);
        person.record_item_time(Duration::from_secs(31));
        let (decision, _) = evaluate(&config, &person, &bank, false, None).unwrap();
        assert!(decision.stop_now);
        assert!(!decision.terminated_early);
    }

    #[test]
    fn custom_stop_closure_is_honored_and_can_override_everything() {
        let bank = bank_1item_2d();
        // Presisi terpenuhi & max_items terpenuhi -> stop_now akan true tanpa customStop.
        let config = StoppingConfig {
            max_items: 1,
            min_sem: vec![0.3, 0.3],
            ..Default::default()
        };
        let person = person_with_se(vec![0.1, 0.1]);

        let never_stop = |_: &PersonState| false;
        let (decision, _) = evaluate(&config, &person, &bank, false, Some(&never_stop)).unwrap();
        assert!(!decision.stop_now, "customStop harus MENIMPA total, bahkan membatalkan max_items");

        let always_stop = |_: &PersonState| true;
        let config_loose = StoppingConfig {
            min_sem: vec![f64::NEG_INFINITY, f64::NEG_INFINITY],
            ..Default::default()
        };
        let (decision, _) = evaluate(&config_loose, &person, &bank, false, Some(&always_stop)).unwrap();
        assert!(decision.stop_now, "customStop harus bisa memaksa stop walau presisi/limit belum tercapai");
    }

    #[test]
    fn allow_constrain_breaks_false_prevents_stop_mid_unfinished_block() {
        let bank = bank_1item_2d();
        let config = StoppingConfig {
            min_sem: vec![0.3, 0.3],
            allow_constrain_breaks: false,
            ..Default::default()
        };
        let person = person_with_se(vec![0.1, 0.1]); // presisi terpenuhi -> stop_now akan true

        let (decision, _) = evaluate(&config, &person, &bank, true, None).unwrap();
        assert!(!decision.stop_now, "harus dicegah karena masih di tengah blok constraint");

        let (decision, _) = evaluate(&config, &person, &bank, false, None).unwrap();
        assert!(decision.stop_now, "tidak di tengah blok constraint -> boleh stop");

        let config_allowed = StoppingConfig {
            allow_constrain_breaks: true,
            ..config
        };
        let (decision, _) = evaluate(&config_allowed, &person, &bank, true, None).unwrap();
        assert!(decision.stop_now, "allow_constrain_breaks=true -> guard tidak berlaku");
    }

    #[test]
    fn debug_false_yields_none_and_debug_true_does_not_change_decision() {
        let bank = bank_1item_2d();
        let config_no_debug = StoppingConfig {
            min_sem: vec![0.3, 0.3],
            ..Default::default()
        };
        let person = person_with_se(vec![0.1, 0.1]);
        let (decision_a, debug_a) = evaluate(&config_no_debug, &person, &bank, false, None).unwrap();
        assert!(debug_a.is_none());

        let config_debug = StoppingConfig {
            debug: true,
            ..config_no_debug
        };
        let (decision_b, debug_b) = evaluate(&config_debug, &person, &bank, false, None).unwrap();
        assert_eq!(decision_a, decision_b);
        let debug = debug_b.expect("debug=true harus menghasilkan Some");
        assert_eq!(debug.se_values, *person.se());
        assert!(debug.sprt_log_likelihood_ratio.is_none());
        assert!(debug.sprt_bounds.is_none());
    }

    #[test]
    fn debug_true_fills_sprt_fields_for_sprt_mode() {
        let bank = bank_5items_2d();
        let config = StoppingConfig {
            classify: ClassifyType::Sprt {
                lower: ThetaVector::from_vec(vec![-1.5, -1.5]),
                upper: ThetaVector::from_vec(vec![1.5, 1.5]),
                alpha: 0.05,
                beta: 0.05,
            },
            debug: true,
            ..Default::default()
        };
        let mut person = PersonState::new(2);
        for i in 0..5 {
            person.record_answer(i, 1);
        }
        let (_, debug) = evaluate(&config, &person, &bank, false, None).unwrap();
        let debug = debug.unwrap();
        assert!(debug.sprt_log_likelihood_ratio.is_some());
        assert!(debug.sprt_bounds.is_some());
    }

    #[test]
    fn min_sem_broadcasts_single_value_to_nfact() {
        let bank = bank_1item_2d();
        let config = StoppingConfig {
            min_sem: vec![0.3], // broadcast ke nfact=2
            ..Default::default()
        };
        let person = person_with_se(vec![0.25, 0.25]);
        let (decision, _) = evaluate(&config, &person, &bank, false, None).unwrap();
        assert!(decision.stop_now);
    }

    #[test]
    fn min_sem_invalid_length_errors() {
        let bank = bank_1item_2d();
        let config = StoppingConfig {
            min_sem: vec![0.3, 0.3, 0.3], // panjang 3, nfact=2 -> invalid
            ..Default::default()
        };
        let person = person_with_se(vec![0.1, 0.1]);
        assert!(evaluate(&config, &person, &bank, false, None).is_err());
    }

    #[test]
    fn below_min_items_never_stops_from_precision_alone() {
        let bank = bank_1item_2d();
        let config = StoppingConfig {
            min_items: 5,
            min_sem: vec![10.0, 10.0], // longgar sekali, pasti met bila dievaluasi
            ..Default::default()
        };
        let person = person_with_se(vec![0.01, 0.01]);
        let (decision, _) = evaluate(&config, &person, &bank, false, None).unwrap();
        assert!(!decision.stop_now);
        assert_eq!(decision.met_sem, vec![false, false]); // belum dievaluasi sama sekali
    }
}
