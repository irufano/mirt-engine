//! Fase 3 — Algoritma Estimasi Kemampuan (Ability Estimation), khusus
//! `nfact > 1` (walau seluruh method juga valid untuk `nfact == 1`).
//!
//! Estimasi ulang **seluruh vektor** `theta` sekaligus (bukan
//! dimensi-per-dimensi) setiap kali respons baru masuk, mengikuti perilaku
//! `mirtCAT`. Ground truth algoritma diverifikasi langsung dari source
//! `mirt`/`mirtCAT` terpasang (R 4.6.1 / mirt 1.47 / mirtCAT 1.14, source
//! diunduh & dibaca dari CRAN untuk memastikan port ini setia — lih.
//! `PLAN.md` §Fase 3):
//!
//! - `mirtCAT::Design$Update.thetas` (`DesignClass.R:57-94`) ternyata
//!   **hanya memanggil `mirt::fscores()`** langsung — ground truth
//!   algoritma sesungguhnya ada di `mirt/R/fscores.internal.R`, bukan di
//!   `mirtCAT`.
//! - `Eap { qmc }`: grid-based & QMC-based ([`eap`]). Default `method`
//!   (`Method::default()`) adalah [`Map`](Method::Map), persis default
//!   `mirtCAT` (`criteria_estimator <- 'MAP'`).
//! - `Map`: optimisasi `log L(theta) + log dmvnorm(theta; gmeans, gcov)`
//!   ([`map`]).
//! - `Ml`: optimisasi `log L(theta)` murni, tanpa prior ([`ml`]) — divergen
//!   ke `+-infinity` pada pola respons ekstrem (diverifikasi empiris
//!   identik dengan `mirt::fscores(method='ML')` sungguhan).
//! - `Wle`: optimisasi `log L(theta) + 0.5*log(det(info_kumulatif(theta)))`
//!   ([`wle`]) — koreksi bias Warm (1989) digeneralisasi multidimensional.
//! - `EapSum`: EAP berbasis summed score lewat rekursi Lord & Wingersky
//!   (1984) ([`eap_sum`]), dibatasi ke item yang **sudah dijawab** (lih.
//!   catatan modul `eap_sum` untuk verifikasi empiris kenapa ini setara
//!   dengan perilaku `mirt`).
//! - `Fixed`: identity murni, tidak mengestimasi ulang ([`fixed`]).
//!
//! `Ml`/`Map`/`Wle` dioptimasi lewat satu optimizer Newton-Raphson generik
//! dengan gradien & Hessian **finite-difference** ([`optim`]) — `nlm()` R
//! yang dipakai `mirt::MAP.mirt`/`WLE.mirt` juga tidak diberi gradien
//! analitik (memakai turunan numerik internal secara default), jadi ini
//! port yang setia di level algoritma (sama-sama konvergen ke optimum
//! fungsi objektif yang identik) tanpa perlu mengubah trait
//! [`crate::irt::ItemResponseModel`].

pub mod eap_sum;

mod density;
mod eap;
mod fixed;
mod map;
mod ml;
mod optim;
pub mod quadrature;
mod wle;

use nalgebra::DMatrix;

use crate::bank::{GroupParams, ItemBank};
use crate::error::MirtCatError;
use crate::item_selection::PersonState;
use crate::theta::{SEVector, ThetaVector};

pub use eap_sum::ScoreEstimate;

/// Metode estimasi kemampuan — enum tertutup persis method valid `mirtCAT`
/// (`fscores(method=...)`, dibatasi ke yang dipakai `Design$Update.thetas`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Method {
    /// EAP (Expected A Posteriori). `qmc=true` memakai titik Quasi-Monte
    /// Carlo (disarankan untuk `nfact > 4`, lih. peringatan di [`estimate`]);
    /// `qmc=false` memakai grid kuadratur reguler.
    Eap { qmc: bool },
    /// MAP (Maximum A Posteriori) — default `mirtCAT`.
    #[default]
    Map,
    /// ML (Maximum Likelihood), tanpa prior — divergen pada pola respons
    /// ekstrem (semua benar/semua salah untuk seluruh item terjawab).
    Ml,
    /// WLE (Weighted Likelihood Estimation) — koreksi bias Warm (1989).
    Wle,
    /// EAP berbasis summed score (Lord & Wingersky, 1984).
    EapSum,
    /// Identity — tidak melakukan estimasi ulang (kuesioner non-adaptif).
    Fixed,
}

/// Konfigurasi estimasi kemampuan.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EstimationConfig {
    pub method: Method,
    /// Jumlah titik kuadratur (grid: per-dimensi; QMC: total). `None` ->
    /// default dari tabel `nfact` ([`quadrature::default_quadpts`]) --
    /// dipakai method `Eap`/`EapSum` saja, diabaikan method lain. Sama
    /// seperti `mirtCAT`, default ini dipakai apa adanya walau `qmc=true`
    /// (bukan otomatis dibesarkan) -- QMC baru dapat manfaat penuhnya bila
    /// `quadpts` dinaikkan manual (lih. `docs/panduan/03-ability-estimation-eap.md`).
    pub quadpts: Option<usize>,
    /// Aktifkan jejak debug (prinsip desain #7 `PLAN.md`). Default `false`.
    pub debug: bool,
}

/// Jejak debug opsional dari [`estimate`] — hanya `Some` bila
/// `EstimationConfig::debug == true`; tidak memengaruhi `(ThetaVector,
/// SEVector)` yang dikembalikan (prinsip desain #7). Field bernilai `None`
/// untuk method yang tidak relevan (mis. `iterations` untuk `Eap`/`Fixed`,
/// `n_quadrature_points` untuk `Ml`/`Map`/`Wle`/`Fixed`).
#[derive(Debug, Clone, PartialEq)]
pub struct AbilityEstimationDebug {
    pub method: Method,
    pub n_quadrature_points: Option<usize>,
    pub iterations: Option<usize>,
    pub log_likelihood_trace: Option<Vec<f64>>,
    pub converged: Option<bool>,
}

/// Log-likelihood satu titik `theta`: `sum` atas item terjawab
/// (`person.answered_items()`/`person.responses()`) dari
/// `log P_{response}(theta)`. Item belum dijawab tidak berkontribusi.
/// Dipakai [`eap`] (per titik grid), [`map`], [`ml`], [`wle`] (di dalam
/// closure objective optimizer).
pub(crate) fn log_likelihood_at(
    person: &PersonState,
    bank: &ItemBank,
    theta: &ThetaVector,
) -> Result<f64, MirtCatError> {
    let mut ll = 0.0;
    for (&item_idx, &response) in person.answered_items().iter().zip(person.responses()) {
        let item = bank.item(item_idx).ok_or_else(|| {
            MirtCatError::InvalidParameter(format!(
                "ability_estimation: index item {item_idx} di luar jangkauan ItemBank"
            ))
        })?;
        let probs = item.probabilities(theta)?;
        let p = probs.get(response).ok_or_else(|| {
            MirtCatError::InvalidParameter(format!(
                "ability_estimation: kategori respons {response} di luar jangkauan item \
                 {item_idx} ({} kategori)",
                probs.len()
            ))
        })?;
        ll += p.max(1e-300).ln();
    }
    Ok(ll)
}

/// Validasi seluruh item/kategori respons di `person` terhadap `bank`
/// sebelum optimisasi dimulai — supaya closure objective di `map`/`ml`/`wle`
/// bisa memakai `.expect(...)` (index & kategori sudah dijamin valid) tanpa
/// perlu closure fallible (`optim::maximize` butuh closure infallible).
pub(crate) fn validate_answers(person: &PersonState, bank: &ItemBank) -> Result<(), MirtCatError> {
    for (&item_idx, &response) in person.answered_items().iter().zip(person.responses()) {
        let item = bank.item(item_idx).ok_or_else(|| {
            MirtCatError::InvalidParameter(format!(
                "ability_estimation: index item {item_idx} di luar jangkauan ItemBank"
            ))
        })?;
        if response >= item.n_categories() {
            return Err(MirtCatError::InvalidParameter(format!(
                "ability_estimation: kategori respons {response} di luar jangkauan item \
                 {item_idx} ({} kategori)",
                item.n_categories()
            )));
        }
    }
    Ok(())
}

/// SE dari Hessian di titik optimum: `sqrt(diag(inverse(-hessian)))` —
/// `-hessian` adalah observed information (Hessian fungsi yang
/// DIMAKSIMALKAN harus negatif definit di titik maksimum sejati), inversnya
/// adalah kovarians asimtotik. Padanan `sqrt(diag(solve(hessian)))` di
/// `mirt::MAP()`/`ML()`/`WLE()` (yang meminimalkan objective negasi, jadi
/// Hessian mereka sudah positif definit tanpa negasi eksplisit). `NaN` per
/// dimensi bila Hessian tidak invertible atau varians ternyata negatif
/// (Hessian bukan negatif definit -- optimizer belum genuinely konvergen ke
/// maksimum, mis. pola respons yang mendekati divergen).
pub(crate) fn se_from_hessian(hessian: &DMatrix<f64>, nfact: usize) -> SEVector {
    let neg_hessian = -hessian.clone();
    match neg_hessian.try_inverse() {
        Some(cov) => {
            let diag: Vec<f64> = (0..nfact)
                .map(|i| {
                    let v = cov[(i, i)];
                    if v > 0.0 {
                        v.sqrt()
                    } else {
                        f64::NAN
                    }
                })
                .collect();
            SEVector::from_vec(diag)
        }
        None => SEVector::from_vec(vec![f64::NAN; nfact]),
    }
}

/// Estimasi ulang `(theta, SE)` peserta sesuai `config.method`, mengikuti
/// `mirtCAT::Design$Update.thetas` (yang mendelegasikan penuh ke
/// `mirt::fscores()`).
///
/// Untuk `method: Eap { qmc: false }` dengan `person.nfact() > 4`,
/// mengeluarkan peringatan (`log::warn!`) & menyarankan `Map` — persis
/// `mirtCAT_preamble.R:184-191` (`nfact > 4 && method=='EAP' &&
/// !fscores_args$QMC`) — bukan error keras, sesuai perilaku R yang cuma
/// `warning()`.
///
/// Error bila ada index item terjawab di luar jangkauan `bank`, kategori
/// respons di luar jangkauan kategori item terkait, atau `quadpts`
/// tidak valid (method `Eap`/`EapSum`).
pub fn estimate(
    config: &EstimationConfig,
    person: &PersonState,
    bank: &ItemBank,
    group: &GroupParams,
) -> Result<((ThetaVector, SEVector), Option<AbilityEstimationDebug>), MirtCatError> {
    let nfact = person.nfact();

    match config.method {
        Method::Fixed => {
            let (theta, se) = fixed::estimate(person);
            let debug = config.debug.then_some(AbilityEstimationDebug {
                method: config.method,
                n_quadrature_points: None,
                iterations: None,
                log_likelihood_trace: None,
                converged: None,
            });
            Ok(((theta, se), debug))
        }
        Method::Eap { qmc } => {
            if nfact > 4 && !qmc {
                log::warn!(
                    "method EAP tanpa Quasi-Monte Carlo (QMC) tidak disarankan untuk model \
                     dimensi tinggi (nfact={nfact} > 4) -- pertimbangkan method Map, atau \
                     aktifkan Method::Eap {{ qmc: true }}"
                );
            }
            let quadpts = config
                .quadpts
                .unwrap_or_else(|| quadrature::default_quadpts(nfact));
            let result = eap::estimate(person, bank, group, quadpts, qmc)?;
            let debug = config.debug.then_some(AbilityEstimationDebug {
                method: config.method,
                n_quadrature_points: Some(result.n_quadrature_points),
                iterations: None,
                log_likelihood_trace: None,
                converged: None,
            });
            Ok(((result.theta, result.se), debug))
        }
        Method::Map => {
            let result = map::estimate(person, bank, group)?;
            let debug = config.debug.then_some(AbilityEstimationDebug {
                method: config.method,
                n_quadrature_points: None,
                iterations: Some(result.iterations),
                log_likelihood_trace: Some(result.log_likelihood_trace.clone()),
                converged: Some(result.converged),
            });
            Ok(((result.theta, result.se), debug))
        }
        Method::Ml => {
            let result = ml::estimate(person, bank, group)?;
            let debug = config.debug.then_some(AbilityEstimationDebug {
                method: config.method,
                n_quadrature_points: None,
                iterations: Some(result.iterations),
                log_likelihood_trace: Some(result.log_likelihood_trace.clone()),
                converged: Some(result.converged),
            });
            Ok(((result.theta, result.se), debug))
        }
        Method::Wle => {
            let result = wle::estimate(person, bank, group)?;
            let debug = config.debug.then_some(AbilityEstimationDebug {
                method: config.method,
                n_quadrature_points: None,
                iterations: Some(result.iterations),
                log_likelihood_trace: Some(result.log_likelihood_trace.clone()),
                converged: Some(result.converged),
            });
            Ok(((result.theta, result.se), debug))
        }
        Method::EapSum => {
            let quadpts = config
                .quadpts
                .unwrap_or_else(|| quadrature::default_quadpts(nfact));
            let result = eap_sum::estimate(person, bank, group, quadpts)?;
            let debug = config.debug.then_some(AbilityEstimationDebug {
                method: config.method,
                n_quadrature_points: Some(result.n_quadrature_points),
                iterations: None,
                log_likelihood_trace: None,
                converged: None,
            });
            Ok(((result.current.theta, result.current.se), debug))
        }
    }
}

/// Pemetaan LENGKAP skor total (`0..=max_score`) -> `(theta, SE)` hasil
/// `EapSum` — pelengkap [`estimate`] (yang cuma mengembalikan skor peserta
/// sekarang) untuk kebutuhan menampilkan/mengaudit seluruh tabel skor
/// sekaligus (mis. contoh & dokumentasi). `quadpts` mengikuti aturan
/// default yang sama seperti [`estimate`] bila `config.quadpts == None`.
///
/// Error bila `config.method` bukan [`Method::EapSum`], atau kondisi error
/// yang sama seperti [`estimate`] (index/kategori tidak valid, dst.).
pub fn eap_sum_score_table(
    config: &EstimationConfig,
    person: &PersonState,
    bank: &ItemBank,
    group: &GroupParams,
) -> Result<Vec<ScoreEstimate>, MirtCatError> {
    if config.method != Method::EapSum {
        return Err(MirtCatError::InvalidParameter(
            "ability_estimation::eap_sum_score_table: config.method harus Method::EapSum".into(),
        ));
    }
    let nfact = person.nfact();
    let quadpts = config
        .quadpts
        .unwrap_or_else(|| quadrature::default_quadpts(nfact));
    let result = eap_sum::estimate(person, bank, group, quadpts)?;
    Ok(result.by_score)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bank::ItemMeta;
    use crate::irt::Dichotomous;
    use std::sync::{Mutex, OnceLock};

    fn bank_1item(a: f64, d: f64) -> ItemBank {
        let mut bank = ItemBank::new();
        bank.push(Box::new(Dichotomous::m2pl(vec![a], d).unwrap()), ItemMeta::new(0))
            .unwrap();
        bank
    }

    #[test]
    fn default_method_is_map() {
        assert_eq!(EstimationConfig::default().method, Method::Map);
    }

    #[test]
    fn debug_false_yields_none_for_every_method() {
        let bank = bank_1item(1.0, 0.0);
        let group = GroupParams::standard_normal(1);
        let mut person = PersonState::new(1);
        person.record_answer(0, 1);

        for method in [
            Method::Eap { qmc: false },
            Method::Map,
            Method::Ml,
            Method::Wle,
            Method::EapSum,
            Method::Fixed,
        ] {
            let config = EstimationConfig {
                method,
                ..Default::default()
            };
            let (_, debug) = estimate(&config, &person, &bank, &group).unwrap();
            assert!(debug.is_none(), "method {method:?} harus None saat debug=false");
        }
    }

    fn bank_mixed_3items() -> ItemBank {
        let mut bank = ItemBank::new();
        for (i, (a, d)) in [(1.0, 0.0), (1.1, -0.2), (0.9, 0.3)].into_iter().enumerate() {
            bank.push(Box::new(Dichotomous::m2pl(vec![a], d).unwrap()), ItemMeta::new(i))
                .unwrap();
        }
        bank
    }

    #[test]
    fn debug_true_does_not_change_main_values_and_fills_relevant_fields() {
        // Pola respons campuran (bukan 1 item / semua-benar) supaya `Ml`
        // menghasilkan estimasi FINITE -- `assert_eq!` di bawah gagal untuk
        // `NaN == NaN` (IEEE754), jadi skenario divergen diuji terpisah di
        // `ml::tests`/`wle::tests`, bukan di sini.
        let bank = bank_mixed_3items();
        let group = GroupParams::standard_normal(1);
        let mut person = PersonState::new(1);
        person.record_answer(0, 1);
        person.record_answer(1, 0);
        person.record_answer(2, 1);

        for method in [Method::Map, Method::Ml, Method::Wle] {
            let config_a = EstimationConfig {
                method,
                ..Default::default()
            };
            let config_b = EstimationConfig {
                method,
                debug: true,
                ..Default::default()
            };
            let ((theta_a, se_a), debug_a) = estimate(&config_a, &person, &bank, &group).unwrap();
            let ((theta_b, se_b), debug_b) = estimate(&config_b, &person, &bank, &group).unwrap();
            assert_eq!(theta_a, theta_b, "method {method:?}");
            assert_eq!(se_a, se_b, "method {method:?}");
            assert!(debug_a.is_none());
            let debug = debug_b.unwrap();
            assert!(debug.iterations.is_some());
            assert!(debug.log_likelihood_trace.is_some());
            assert!(debug.converged.is_some());
            assert!(debug.n_quadrature_points.is_none());
        }

        for method in [Method::Eap { qmc: false }, Method::EapSum] {
            let config = EstimationConfig {
                method,
                debug: true,
                ..Default::default()
            };
            let (_, debug) = estimate(&config, &person, &bank, &group).unwrap();
            let debug = debug.unwrap();
            assert!(debug.n_quadrature_points.is_some());
            assert!(debug.iterations.is_none());
        }
    }

    #[test]
    fn fixed_debug_true_yields_all_none_fields() {
        let bank = bank_1item(1.0, 0.0);
        let group = GroupParams::standard_normal(1);
        let person = PersonState::new(1);
        let config = EstimationConfig {
            method: Method::Fixed,
            debug: true,
            ..Default::default()
        };
        let (_, debug) = estimate(&config, &person, &bank, &group).unwrap();
        let debug = debug.unwrap();
        assert!(debug.n_quadrature_points.is_none());
        assert!(debug.iterations.is_none());
        assert!(debug.log_likelihood_trace.is_none());
        assert!(debug.converged.is_none());
    }

    #[test]
    fn out_of_bank_answered_item_errors_for_every_method_not_panics() {
        let bank = bank_1item(1.0, 0.0);
        let group = GroupParams::standard_normal(1);
        let mut person = PersonState::new(1);
        person.record_answer(999, 1);

        for method in [
            Method::Eap { qmc: false },
            Method::Map,
            Method::Ml,
            Method::Wle,
            Method::EapSum,
        ] {
            let config = EstimationConfig {
                method,
                ..Default::default()
            };
            assert!(
                estimate(&config, &person, &bank, &group).is_err(),
                "method {method:?} harus error, bukan panic"
            );
        }
    }

    /// Test capturing logger sederhana untuk menguji `log::warn!` -- persis
    /// instruksi PLAN.md "dites via capture log/tracing, bukan panic".
    struct CapturingLogger;
    static CAPTURED: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

    impl log::Log for CapturingLogger {
        fn enabled(&self, _metadata: &log::Metadata) -> bool {
            true
        }
        fn log(&self, record: &log::Record) {
            CAPTURED
                .get_or_init(|| Mutex::new(Vec::new()))
                .lock()
                .unwrap()
                .push(format!("{}", record.args()));
        }
        fn flush(&self) {}
    }

    static LOGGER_INIT: OnceLock<()> = OnceLock::new();
    /// Mengunci seluruh critical section "clear buffer -> estimate() ->
    /// baca buffer" -- logger `log` bersifat global per-proses, dan test
    /// Rust berjalan paralel lintas thread secara default, jadi tanpa lock
    /// ini dua test logger bisa saling meracau buffer satu sama lain.
    static LOG_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn install_capturing_logger() {
        LOGGER_INIT.get_or_init(|| {
            log::set_logger(&CapturingLogger).expect("logger cuma dipasang sekali per proses");
            log::set_max_level(log::LevelFilter::Warn);
        });
    }

    fn nfact5_bank() -> ItemBank {
        let mut bank = ItemBank::new();
        for i in 0..5 {
            bank.push(
                Box::new(
                    Dichotomous::m2pl(
                        (0..5).map(|j| if i == j { 1.0 } else { 0.0 }).collect(),
                        0.0,
                    )
                    .unwrap(),
                ),
                ItemMeta::new(i),
            )
            .unwrap();
        }
        bank
    }

    /// Menggabungkan kedua skenario (warns tanpa QMC, tidak warn dengan QMC)
    /// dalam satu test supaya urutan clear/estimate/baca buffer log tidak
    /// bisa diselingi test lain yang juga memakai logger global.
    #[test]
    fn eap_nfact_above_4_warns_only_without_qmc() {
        let _guard = LOG_TEST_LOCK.lock().unwrap();
        install_capturing_logger();
        let bank = nfact5_bank();
        let group = GroupParams::standard_normal(5);
        let person = PersonState::new(5);

        CAPTURED.get_or_init(|| Mutex::new(Vec::new())).lock().unwrap().clear();
        let config_no_qmc = EstimationConfig {
            method: Method::Eap { qmc: false },
            quadpts: Some(3),
            ..Default::default()
        };
        estimate(&config_no_qmc, &person, &bank, &group).unwrap();
        {
            let captured = CAPTURED.get().unwrap().lock().unwrap();
            assert!(
                captured
                    .iter()
                    .any(|msg| msg.contains("QMC") || msg.contains("EAP")),
                "harus ada peringatan EAP+nfact>4 tanpa QMC, captured={captured:?}"
            );
        }

        CAPTURED.get().unwrap().lock().unwrap().clear();
        let config_qmc = EstimationConfig {
            method: Method::Eap { qmc: true },
            quadpts: Some(50),
            ..Default::default()
        };
        estimate(&config_qmc, &person, &bank, &group).unwrap();
        {
            let captured = CAPTURED.get().unwrap().lock().unwrap();
            assert!(captured.is_empty(), "qmc=true tidak boleh memicu peringatan");
        }
    }

    #[test]
    fn eap_sum_score_table_rejects_non_eap_sum_method() {
        let bank = bank_1item(1.0, 0.0);
        let group = GroupParams::standard_normal(1);
        let person = PersonState::new(1);
        let config = EstimationConfig {
            method: Method::Map,
            ..Default::default()
        };
        assert!(eap_sum_score_table(&config, &person, &bank, &group).is_err());
    }

    #[test]
    fn eap_sum_score_table_returns_full_mapping() {
        let bank = bank_1item(1.0, 0.0);
        let group = GroupParams::standard_normal(1);
        let mut person = PersonState::new(1);
        person.record_answer(0, 1);
        let config = EstimationConfig {
            method: Method::EapSum,
            ..Default::default()
        };
        let table = eap_sum_score_table(&config, &person, &bank, &group).unwrap();
        assert_eq!(table.len(), 2); // skor 0 & 1 untuk 1 item dikotomus
        assert_eq!(table[0].sum_score, 0);
        assert_eq!(table[1].sum_score, 1);
    }
}
