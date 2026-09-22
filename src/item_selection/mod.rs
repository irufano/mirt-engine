//! Fase 2 — Algoritma Pemilihan Item (Item Selection) khusus `nfact > 1`.
//!
//! Formula tiap kriteria diverifikasi numerik terhadap `mirtCAT`
//! (R 4.6.1/mirtCAT 1.14, via `computeCriteria()` pada item bank sintetis) —
//! lih. `PLAN.md` §Fase 2 "Riset yang sudah dilakukan" untuk rincian &
//! rujukan akademik tiap kriteria:
//!
//! - `Drule`/`DPrule`: `det(total_info)` ([`criteria::d_rule`]).
//! - `Trule`/`TPrule`: `sum_i weights[i] * total_info[i,i]` (trace berbobot,
//!   diagonal saja — [`criteria::t_rule`]).
//! - `Arule`/`APrule`: `-sum_i weights[i] * pseudo_inverse(total_info)[i,i]`
//!   (pseudo-invers, bukan invers biasa — matriks informasi dari satu item
//!   dikotomus tunggal selalu rank 1, umum singular di awal tes —
//!   [`criteria::a_rule`]).
//! - `Erule`/`EPrule`: `min_eigenvalue(total_info)` ([`criteria::e_rule`]).
//! - `Wrule`/`WPrule`: `weights' * total_info * weights` (bentuk kuadratik
//!   penuh, beda dari `Trule` yang cuma diagonal — [`criteria::w_rule`]).
//! - `KL`/`KLn`: Kullback-Leibler pointwise pada item kandidat sendiri
//!   (bukan kumulatif), `KLn` memakai `delta * sqrt(n_answered)`
//!   ([`criteria::kl`]).
//! - `seq`/`random`: baseline non-informasi ([`criteria::seq`],
//!   [`criteria::random`]).
//!
//! `total_info` untuk kriteria D/T/A/E/W-rule = `person.info_thetas(bank)` +
//! info Fisher item kandidat pada `person.theta()`, ditambah
//! `inverse(gcov)` bila varian `*Prule` (`prior: true`).
//!
//! **Catatan penting soal `info_thetas`**: `PersonState` menyimpan daftar
//! index item yang sudah dijawab, **bukan** matriks akumulasi yang
//! di-update inkremental. `info_thetas(bank)` menghitung ULANG dari nol tiap
//! dipanggil: jumlah info Fisher seluruh item terjawab, dievaluasi di
//! `theta` **saat ini** (bukan di theta saat masing-masing item dijawab).
//! Ini meniru persis `Person$Update.info_mats` di `mirtCAT`
//! (dikonfirmasi lewat inspeksi source terpasang, R 4.6.1/mirtCAT 1.14):
//! tiap kali dipanggil, ia menjumlahkan `FI(item, Theta = thetas_terbaru)`
//! untuk seluruh item yang sudah dijawab — bukan mengakumulasi kontribusi
//! item lama yang dihitung di theta lama (yang jadi "basi"/stale begitu
//! theta diestimasi ulang setelah item berikutnya). Karena estimasi ulang
//! `theta` antar-item adalah pekerjaan Fase 3 (belum ada), keputusan desain
//! ini memastikan Fase 2 tetap benar begitu Fase 3 terpasang, tanpa perlu
//! desain ulang `PersonState`.

pub mod criteria;

use rand::Rng;

use crate::bank::{GroupParams, ItemBank};
use crate::error::MirtCatError;
use crate::irt::{cumulative_information, ItemResponseModel};
use crate::theta::{InfoMatrix, SEVector, ThetaVector};

/// Index item di dalam `ItemBank`.
pub type ItemIndex = usize;

/// Kriteria seleksi item — enum **tertutup** berisi persis kriteria yang
/// valid untuk `nfact > 1` di `mirtCAT`
/// (lih. `mirtCAT/R/DesignClass.R:113-117`). Kriteria yang tidak valid untuk
/// multidimensional (`MI`, `MEPV`, `MLWI`, `MPWI`, `MEI`, seluruh `IKL*`)
/// sengaja **tidak** dimodelkan: karena enum ini tertutup, mengonstruksi
/// kriteria tidak valid mustahil dilakukan, sehingga validasi terjadi di
/// compile time, bukan runtime (lih. `PLAN.md` §Fase 2 "Keputusan desain").
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Criterion {
    /// Baseline non-adaptif: item bank disajikan berurutan (default,
    /// mengikuti default `criteria = "seq"` di `mirtCAT`).
    #[default]
    Seq,
    /// Baseline exposure-neutral: pilih uniform random dari kandidat.
    Random,
    /// Kullback-Leibler pointwise (Chang & Ying, 1996), interval `delta` tetap.
    Kl { delta: f64 },
    /// Sama seperti `Kl`, tapi `delta` diskalakan `delta * sqrt(n_answered)`.
    KlN { delta: f64 },
    /// D-optimality: maksimalkan determinan info Fisher kumulatif.
    DRule { prior: bool },
    /// Maksimalkan trace (potensial berbobot) info Fisher kumulatif.
    TRule { prior: bool },
    /// Minimalkan trace (potensial berbobot) kovarians asimtotik — nilai
    /// kriteria sudah dinegasikan agar konsisten dengan argmax.
    ARule { prior: bool },
    /// Maksimalkan eigenvalue minimum info Fisher kumulatif.
    ERule { prior: bool },
    /// Maksimalkan bentuk kuadratik berbobot kustom `weights' * I * weights`.
    WRule { prior: bool },
}

/// Konfigurasi seleksi item.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SelectionConfig {
    pub criterion: Criterion,
    /// Bobot per-dimensi, dipakai `TRule`/`ARule`/`WRule` (+ varian `prior`).
    /// `None` → default `vec![1.0; nfact]`. Diabaikan kriteria lain.
    pub weights: Option<Vec<f64>>,
    /// Aktifkan jejak debug (prinsip desain #7 `PLAN.md`). Default `false`.
    pub debug: bool,
}

impl SelectionConfig {
    /// Konstruktor yang memvalidasi panjang `weights` (bila diberikan)
    /// terhadap `nfact`. Error `MirtCatError::DimensionMismatch` bila tidak
    /// cocok — pola sama seperti `weights` di `initial_theta::InitialThetaConfig`.
    pub fn new(
        criterion: Criterion,
        weights: Option<Vec<f64>>,
        nfact: usize,
        debug: bool,
    ) -> Result<Self, MirtCatError> {
        if let Some(w) = &weights {
            if w.len() != nfact {
                return Err(MirtCatError::DimensionMismatch {
                    expected: nfact,
                    actual: w.len(),
                    context: "SelectionConfig::new: weights",
                });
            }
        }
        Ok(Self {
            criterion,
            weights,
            debug,
        })
    }
}

/// State satu peserta yang relevan untuk seleksi item & estimasi kemampuan:
/// estimasi `theta`/SE saat ini dan daftar item yang sudah dijawab (index +
/// kategori respons) — dipakai lintas fase berikutnya (estimasi kemampuan,
/// stopping rule, dst.).
///
/// Sengaja **tidak** menyimpan matriks informasi kumulatif langsung sebagai
/// field — lih. catatan modul di atas soal kenapa `info_thetas` harus
/// dihitung ulang dari daftar item terjawab tiap dipanggil, bukan
/// diakumulasi inkremental.
#[derive(Debug, Clone, PartialEq)]
pub struct PersonState {
    theta: ThetaVector,
    se: SEVector,
    answered_items: Vec<ItemIndex>,
    /// Kategori respons per item terjawab, paralel dengan `answered_items`
    /// (index `i` di sini adalah kategori respons untuk `answered_items[i]`).
    /// Dibutuhkan Fase 3 (likelihood butuh tahu kategori yang benar-benar
    /// dijawab, bukan cuma item mana yang diadministrasikan).
    responses: Vec<usize>,
}

impl PersonState {
    /// State awal: `theta`/SE nol, belum ada item terjawab. `SE` nol di sini
    /// hanya placeholder mentah (sama seperti `theta` nol) — pemakaian nyata
    /// semestinya lewat `from_theta_and_se` dengan hasil `initial_theta::initialize`.
    pub fn new(nfact: usize) -> Self {
        Self {
            theta: ThetaVector::zeros(nfact),
            se: SEVector::zeros(nfact),
            answered_items: Vec::new(),
            responses: Vec::new(),
        }
    }

    /// State awal dengan `theta` tertentu (mis. hasil `initial_theta::initialize`),
    /// SE nol placeholder (lih. catatan `new`).
    pub fn from_theta(theta: ThetaVector) -> Self {
        let nfact = theta.nfact();
        Self {
            theta,
            se: SEVector::zeros(nfact),
            answered_items: Vec::new(),
            responses: Vec::new(),
        }
    }

    /// State awal dengan `theta` & SE tertentu (mis. hasil
    /// `initial_theta::initialize`, yang mengembalikan keduanya sekaligus).
    pub fn from_theta_and_se(theta: ThetaVector, se: SEVector) -> Self {
        Self {
            theta,
            se,
            answered_items: Vec::new(),
            responses: Vec::new(),
        }
    }

    pub fn nfact(&self) -> usize {
        self.theta.nfact()
    }

    pub fn theta(&self) -> &ThetaVector {
        &self.theta
    }

    pub fn se(&self) -> &SEVector {
        &self.se
    }

    /// Perbarui estimasi `theta` saja (dipakai test/eksperimen yang tidak
    /// peduli SE). Untuk hasil `ability_estimation::estimate`, pakai
    /// `set_estimate` yang memperbarui `theta` & SE sekaligus.
    pub fn set_theta(&mut self, theta: ThetaVector) {
        self.theta = theta;
    }

    /// Perbarui `theta` & SE sekaligus — dipakai hasil `ability_estimation::estimate`
    /// (Fase 3).
    pub fn set_estimate(&mut self, theta: ThetaVector, se: SEVector) {
        self.theta = theta;
        self.se = se;
    }

    pub fn n_answered(&self) -> usize {
        self.answered_items.len()
    }

    pub fn answered_items(&self) -> &[ItemIndex] {
        &self.answered_items
    }

    /// Kategori respons per item terjawab, paralel dengan `answered_items()`.
    pub fn responses(&self) -> &[usize] {
        &self.responses
    }

    /// Catat satu item terjawab (index `ItemBank`) beserta kategori respons
    /// yang dipilih (index kategori, 0-based -- mis. 0/1 untuk item
    /// dikotomus). Tidak memvalidasi index/kategori terhadap bank di sini —
    /// validasi terjadi lazily saat `info_thetas()`/estimasi kemampuan
    /// dipanggil (butuh akses `&ItemBank`).
    pub fn record_answer(&mut self, item_index: ItemIndex, response: usize) {
        self.answered_items.push(item_index);
        self.responses.push(response);
    }

    /// Matriks informasi Fisher kumulatif dari seluruh item terjawab,
    /// dihitung ulang di `theta` **saat ini** (bukan diakumulasi bertahap —
    /// lih. catatan modul di atas). Memakai [`cumulative_information`]
    /// (Fase 0) yang sudah ada & teruji.
    ///
    /// Error bila ada index item terjawab yang di luar jangkauan `bank`.
    pub fn info_thetas(&self, bank: &ItemBank) -> Result<InfoMatrix, MirtCatError> {
        let mut items = Vec::with_capacity(self.answered_items.len());
        for &idx in &self.answered_items {
            let item = bank.item(idx).ok_or_else(|| {
                MirtCatError::InvalidParameter(format!(
                    "PersonState::info_thetas: index item {idx} di luar jangkauan ItemBank"
                ))
            })?;
            items.push(item);
        }
        cumulative_information(self.nfact(), items, &self.theta)
    }
}

/// Jejak debug opsional dari [`select_next_item`] — hanya `Some` bila
/// `SelectionConfig::debug == true`; tidak memengaruhi item yang dipilih
/// (prinsip desain #7 `PLAN.md`).
#[derive(Debug, Clone, PartialEq)]
pub struct ItemSelectionDebug {
    /// Nilai kriteria mentah tiap kandidat (index item bank, nilai kriteria).
    /// Kosong untuk `Seq`/`Random` (tidak ada nilai kriteria untuk keduanya).
    pub candidate_scores: Vec<(ItemIndex, f64)>,
    pub chosen: ItemIndex,
    /// Snapshot `info_thetas` sebelum item terpilih ditambahkan — `Some`
    /// hanya untuk kriteria D/T/A/E/W-rule.
    pub info_before: Option<InfoMatrix>,
    /// Snapshot `info_thetas` seandainya item terpilih ditambahkan — `Some`
    /// hanya untuk kriteria D/T/A/E/W-rule.
    pub info_after: Option<InfoMatrix>,
}

/// Pilih item selanjutnya dari `candidates` (index `ItemBank` yang belum
/// dijawab) sesuai `config.criterion`.
///
/// `group` dipakai varian `*Prule` (sumber `inverse(gcov)` sebagai informasi
/// prior); diabaikan kriteria lain. `rng` dipakai kriteria `Random`;
/// diabaikan kriteria lain — parameter ini ditambahkan di atas sketsa awal
/// PLAN.md supaya `Random` bisa diuji deterministik & dipakai dengan seed
/// tetap (lih. `PLAN.md` §Fase 2 "Keputusan desain" #3).
///
/// Error bila `candidates` kosong, ada index kandidat di luar `bank`, atau
/// panjang `weights` (default maupun manual) tidak cocok `nfact`.
pub fn select_next_item(
    config: &SelectionConfig,
    candidates: &[ItemIndex],
    person: &PersonState,
    bank: &ItemBank,
    group: &GroupParams,
    rng: &mut impl Rng,
) -> Result<(ItemIndex, Option<ItemSelectionDebug>), MirtCatError> {
    if candidates.is_empty() {
        return Err(MirtCatError::InvalidParameter(
            "select_next_item: candidates kosong -- tidak ada item tersisa untuk dipilih".into(),
        ));
    }

    let get_item = |idx: ItemIndex| -> Result<&dyn ItemResponseModel, MirtCatError> {
        bank.item(idx).ok_or_else(|| {
            MirtCatError::InvalidParameter(format!(
                "select_next_item: index item {idx} di luar jangkauan ItemBank"
            ))
        })
    };

    match config.criterion {
        Criterion::Seq => {
            let chosen = criteria::seq::select(candidates);
            let debug = config.debug.then_some(ItemSelectionDebug {
                candidate_scores: Vec::new(),
                chosen,
                info_before: None,
                info_after: None,
            });
            Ok((chosen, debug))
        }
        Criterion::Random => {
            let chosen = criteria::random::select(candidates, rng);
            let debug = config.debug.then_some(ItemSelectionDebug {
                candidate_scores: Vec::new(),
                chosen,
                info_before: None,
                info_after: None,
            });
            Ok((chosen, debug))
        }
        Criterion::Kl { delta } | Criterion::KlN { delta } => {
            let effective_delta = if matches!(config.criterion, Criterion::KlN { .. }) {
                criteria::kl::scaled_delta(delta, person.n_answered())
            } else {
                delta
            };
            let mut scores = Vec::with_capacity(candidates.len());
            for &idx in candidates {
                let item = get_item(idx)?;
                let s = criteria::kl::score(item, person.theta(), effective_delta)?;
                scores.push((idx, s));
            }
            let chosen = argmax(&scores)?;
            let debug = config.debug.then_some(ItemSelectionDebug {
                candidate_scores: scores,
                chosen,
                info_before: None,
                info_after: None,
            });
            Ok((chosen, debug))
        }
        Criterion::DRule { prior }
        | Criterion::TRule { prior }
        | Criterion::ARule { prior }
        | Criterion::ERule { prior }
        | Criterion::WRule { prior } => {
            let nfact = person.nfact();
            let weights = match &config.weights {
                Some(w) => {
                    if w.len() != nfact {
                        return Err(MirtCatError::DimensionMismatch {
                            expected: nfact,
                            actual: w.len(),
                            context: "select_next_item: weights",
                        });
                    }
                    w.clone()
                }
                None => vec![1.0; nfact],
            };
            let prior_info = if prior {
                Some(group.gcov().inverse()?)
            } else {
                None
            };
            let info_before = person.info_thetas(bank)?;

            let candidate_total = |idx: ItemIndex| -> Result<InfoMatrix, MirtCatError> {
                let item = get_item(idx)?;
                let mut total = info_before.clone() + item.information(person.theta())?;
                if let Some(pinfo) = &prior_info {
                    total += pinfo;
                }
                Ok(total)
            };

            let mut scores = Vec::with_capacity(candidates.len());
            for &idx in candidates {
                let total = candidate_total(idx)?;
                let s = match config.criterion {
                    Criterion::DRule { .. } => criteria::d_rule::score(&total),
                    Criterion::TRule { .. } => criteria::t_rule::score(&total, &weights)?,
                    Criterion::ARule { .. } => criteria::a_rule::score(&total, &weights)?,
                    Criterion::ERule { .. } => criteria::e_rule::score(&total),
                    Criterion::WRule { .. } => criteria::w_rule::score(&total, &weights)?,
                    _ => unreachable!("cabang match sudah dibatasi ke D/T/A/E/W-rule"),
                };
                scores.push((idx, s));
            }
            let chosen = argmax(&scores)?;

            let debug = if config.debug {
                let info_after = candidate_total(chosen)?;
                Some(ItemSelectionDebug {
                    candidate_scores: scores,
                    chosen,
                    info_before: Some(info_before),
                    info_after: Some(info_after),
                })
            } else {
                None
            };
            Ok((chosen, debug))
        }
    }
}

/// Index dengan nilai kriteria maksimum; pada seri (tie), pilih yang
/// **pertama muncul** — sama seperti `which.max()` di R yang dipakai
/// `findNextCATItem.R` (`index[which.max(crit)][1L]`).
fn argmax(scores: &[(ItemIndex, f64)]) -> Result<ItemIndex, MirtCatError> {
    scores
        .iter()
        .copied()
        .fold(None::<(ItemIndex, f64)>, |best, (idx, val)| match best {
            Some((_, best_val)) if best_val >= val => best,
            _ => Some((idx, val)),
        })
        .map(|(idx, _)| idx)
        .ok_or_else(|| {
            MirtCatError::InvalidParameter(
                "select_next_item: tidak ada kandidat untuk argmax".into(),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bank::ItemMeta;
    use crate::irt::Dichotomous;
    use approx::assert_relative_eq;
    use nalgebra::DMatrix;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn bank_2d() -> ItemBank {
        let mut bank = ItemBank::new();
        // item0: kuat di dim1, item1: kuat di dim2, item2: seimbang.
        bank.push(
            Box::new(Dichotomous::m2pl(vec![2.0, 0.1], 0.0).unwrap()),
            ItemMeta::new(0),
        )
        .unwrap();
        bank.push(
            Box::new(Dichotomous::m2pl(vec![0.1, 2.0], 0.0).unwrap()),
            ItemMeta::new(1),
        )
        .unwrap();
        bank.push(
            Box::new(Dichotomous::m2pl(vec![1.0, 1.0], 0.0).unwrap()),
            ItemMeta::new(2),
        )
        .unwrap();
        bank
    }

    #[test]
    fn selection_config_new_rejects_weights_length_mismatch() {
        let err =
            SelectionConfig::new(Criterion::TRule { prior: false }, Some(vec![1.0]), 2, false)
                .unwrap_err();
        assert_eq!(
            err,
            MirtCatError::DimensionMismatch {
                expected: 2,
                actual: 1,
                context: "SelectionConfig::new: weights",
            }
        );
    }

    #[test]
    fn selection_config_default_is_seq_with_no_weights() {
        let config = SelectionConfig::default();
        assert_eq!(config.criterion, Criterion::Seq);
        assert_eq!(config.weights, None);
        assert!(!config.debug);
    }

    #[test]
    fn person_state_record_answer_accumulates_info_and_count() {
        let mut bank = ItemBank::new();
        bank.push(
            Box::new(Dichotomous::m2pl(vec![1.0, 0.5], 0.0).unwrap()),
            ItemMeta::new(0),
        )
        .unwrap();
        let item = bank.item(0).unwrap();

        let mut person = PersonState::new(2);
        assert_eq!(person.n_answered(), 0);
        assert_relative_eq!(person.info_thetas(&bank).unwrap().trace(), 0.0);

        person.record_answer(0, 1);
        assert_eq!(person.n_answered(), 1);
        assert_eq!(person.answered_items(), &[0]);
        let expected = item.information(person.theta()).unwrap();
        assert_relative_eq!(
            person.info_thetas(&bank).unwrap().get(0, 0),
            expected.get(0, 0),
            epsilon = 1e-12
        );

        person.record_answer(0, 1);
        assert_eq!(person.n_answered(), 2);
        assert_relative_eq!(
            person.info_thetas(&bank).unwrap().get(0, 0),
            2.0 * expected.get(0, 0),
            epsilon = 1e-12
        );
    }

    #[test]
    fn person_state_info_thetas_recomputes_at_current_theta_not_answer_time_theta() {
        // Meniru `Person$Update.info_mats` mirtCAT: info item terjawab
        // dievaluasi ULANG di theta TERBARU, bukan "dibekukan" di theta saat
        // item itu dijawab.
        let mut bank = ItemBank::new();
        bank.push(
            Box::new(Dichotomous::m2pl(vec![1.0, 0.5], 0.2).unwrap()),
            ItemMeta::new(0),
        )
        .unwrap();
        let item = bank.item(0).unwrap();

        let mut person = PersonState::new(2);
        person.record_answer(0, 1);
        let info_at_zero = person.info_thetas(&bank).unwrap();

        person.set_theta(ThetaVector::from_vec(vec![0.7, -0.4]));
        let info_after_theta_change = person.info_thetas(&bank).unwrap();
        let expected_at_new_theta = item.information(person.theta()).unwrap();

        assert_relative_eq!(
            info_after_theta_change.get(0, 0),
            expected_at_new_theta.get(0, 0),
            epsilon = 1e-12
        );
        assert!((info_after_theta_change.get(0, 0) - info_at_zero.get(0, 0)).abs() > 1e-9);
    }

    #[test]
    fn person_state_info_thetas_errors_on_out_of_bank_answered_index() {
        let bank = bank_2d();
        let mut person = PersonState::new(2);
        person.record_answer(999, 1);
        assert!(person.info_thetas(&bank).is_err());
    }

    #[test]
    fn select_next_item_errors_on_empty_candidates() {
        let bank = bank_2d();
        let person = PersonState::new(2);
        let group = GroupParams::standard_normal(2);
        let config = SelectionConfig::default();
        let mut rng = StdRng::seed_from_u64(0);
        assert!(select_next_item(&config, &[], &person, &bank, &group, &mut rng).is_err());
    }

    #[test]
    fn seq_picks_smallest_remaining_index() {
        let bank = bank_2d();
        let person = PersonState::new(2);
        let group = GroupParams::standard_normal(2);
        let config = SelectionConfig::default(); // Criterion::Seq
        let mut rng = StdRng::seed_from_u64(0);
        let (chosen, debug) =
            select_next_item(&config, &[2, 0, 1], &person, &bank, &group, &mut rng).unwrap();
        assert_eq!(chosen, 0);
        assert!(debug.is_none());
    }

    #[test]
    fn random_only_ever_picks_from_candidates_and_is_seed_reproducible() {
        let bank = bank_2d();
        let person = PersonState::new(2);
        let group = GroupParams::standard_normal(2);
        let config = SelectionConfig {
            criterion: Criterion::Random,
            ..Default::default()
        };
        let mut rng_a = StdRng::seed_from_u64(99);
        let mut rng_b = StdRng::seed_from_u64(99);
        for _ in 0..10 {
            let (a, _) =
                select_next_item(&config, &[0, 1, 2], &person, &bank, &group, &mut rng_a).unwrap();
            let (b, _) =
                select_next_item(&config, &[0, 1, 2], &person, &bank, &group, &mut rng_b).unwrap();
            assert_eq!(a, b);
            assert!([0, 1, 2].contains(&a));
        }
    }

    #[test]
    fn e_rule_accounts_for_accumulated_info_not_just_the_candidate_alone() {
        // Info Fisher satu item dikotomus tunggal selalu rank 1, jadi tanpa
        // info kumulatif SEMUA kandidat akan punya eigenvalue minimum ~0
        // (tidak ada yang menang). Uji ini membuktikan `select_next_item`
        // benar-benar menjumlahkan `person.info_thetas()` + info kandidat
        // (bukan cuma info kandidat sendiri) sebelum mengevaluasi Erule --
        // nilai referensi dihitung tangan (lih. komentar di bawah).
        let mut bank = bank_2d();
        bank.push(
            Box::new(Dichotomous::m2pl(vec![1.5, 0.3], 0.0).unwrap()),
            ItemMeta::new(3),
        )
        .unwrap();
        let mut person = PersonState::new(2);
        person.record_answer(3, 1); // seed: sudah dijawab, bukan kandidat

        let group = GroupParams::standard_normal(2);
        let config = SelectionConfig {
            criterion: Criterion::ERule { prior: false },
            ..Default::default()
        };
        let mut rng = StdRng::seed_from_u64(0);
        let (chosen, debug) =
            select_next_item(&config, &[0, 1, 2], &person, &bank, &group, &mut rng).unwrap();
        // Seed sudah kaya informasi dim1 (a=[1.5,0.3]); dari 3 kandidat,
        // item1 (a=[0.1,2.0], kuat di dim2) paling melengkapi eigenvalue
        // minimum gabungan (hitungan tangan: min_eig gabungan candidate0
        // ~0.008, candidate1 ~0.513, candidate2 ~0.090 -- candidate1 menang
        // telak karena paling menambal dimensi yang lemah).
        assert_eq!(chosen, 1);
        assert!(debug.is_none());
    }

    #[test]
    fn debug_true_contains_all_candidate_scores_without_changing_chosen_item() {
        let bank = bank_2d();
        let person = PersonState::new(2);
        let group = GroupParams::standard_normal(2);
        let config_no_debug = SelectionConfig {
            criterion: Criterion::DRule { prior: false },
            ..Default::default()
        };
        let mut rng = StdRng::seed_from_u64(0);
        let (chosen_a, debug_a) = select_next_item(
            &config_no_debug,
            &[0, 1, 2],
            &person,
            &bank,
            &group,
            &mut rng,
        )
        .unwrap();
        assert!(debug_a.is_none());

        let config_debug = SelectionConfig {
            criterion: Criterion::DRule { prior: false },
            debug: true,
            ..Default::default()
        };
        let (chosen_b, debug_b) =
            select_next_item(&config_debug, &[0, 1, 2], &person, &bank, &group, &mut rng).unwrap();
        assert_eq!(chosen_a, chosen_b);

        let debug = debug_b.expect("debug=true harus menghasilkan Some");
        assert_eq!(debug.chosen, chosen_b);
        assert_eq!(debug.candidate_scores.len(), 3);
        assert!(debug.info_before.is_some());
        assert!(debug.info_after.is_some());
    }

    #[test]
    fn prior_variant_differs_from_non_prior_when_gcov_is_not_identity() {
        let bank = bank_2d();
        let person = PersonState::new(2);
        let gcov =
            InfoMatrix::from_matrix(DMatrix::from_row_slice(2, 2, &[0.25, 0.0, 0.0, 4.0])).unwrap();
        let group = GroupParams::new(ThetaVector::zeros(2), gcov).unwrap();

        let config_plain = SelectionConfig {
            criterion: Criterion::DRule { prior: false },
            debug: true,
            ..Default::default()
        };
        let config_prior = SelectionConfig {
            criterion: Criterion::DRule { prior: true },
            debug: true,
            ..Default::default()
        };
        let mut rng = StdRng::seed_from_u64(0);
        let (_, debug_plain) =
            select_next_item(&config_plain, &[0, 1, 2], &person, &bank, &group, &mut rng).unwrap();
        let (_, debug_prior) =
            select_next_item(&config_prior, &[0, 1, 2], &person, &bank, &group, &mut rng).unwrap();

        let plain_scores = debug_plain.unwrap().candidate_scores;
        let prior_scores = debug_prior.unwrap().candidate_scores;
        for ((_, plain_val), (_, prior_val)) in plain_scores.iter().zip(prior_scores.iter()) {
            assert!((plain_val - prior_val).abs() > 1e-9);
        }
    }

    #[test]
    fn kln_uses_wider_effective_delta_as_n_answered_grows() {
        let bank = bank_2d();
        let group = GroupParams::standard_normal(2);
        let config = SelectionConfig {
            criterion: Criterion::KlN { delta: 0.1 },
            debug: true,
            ..Default::default()
        };
        let mut rng = StdRng::seed_from_u64(0);

        let mut person_fresh = PersonState::new(2);
        let (_, debug_fresh) =
            select_next_item(&config, &[0], &person_fresh, &bank, &group, &mut rng).unwrap();

        for _ in 0..4 {
            person_fresh.record_answer(1, 1); // n_answered saja yang relevan untuk KLn
        }
        let (_, debug_after_4) =
            select_next_item(&config, &[0], &person_fresh, &bank, &group, &mut rng).unwrap();

        let score_fresh = debug_fresh.unwrap().candidate_scores[0].1;
        let score_after_4 = debug_after_4.unwrap().candidate_scores[0].1;
        // n_answered=0 -> delta efektif 0 -> KL persis 0; n_answered=4 -> delta*2.
        assert_relative_eq!(score_fresh, 0.0, epsilon = 1e-12);
        assert!(score_after_4 > score_fresh);
    }

    #[test]
    fn out_of_bank_candidate_index_errors_not_panics() {
        let bank = bank_2d();
        let person = PersonState::new(2);
        let group = GroupParams::standard_normal(2);
        let config = SelectionConfig {
            criterion: Criterion::DRule { prior: false },
            ..Default::default()
        };
        let mut rng = StdRng::seed_from_u64(0);
        assert!(select_next_item(&config, &[42], &person, &bank, &group, &mut rng).is_err());
    }
}
