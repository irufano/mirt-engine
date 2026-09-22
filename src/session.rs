//! Fase 8 — Integrasi End-to-End: `McatSession` menyatukan siklus adaptif
//! penuh Fase 1–7 (theta awal → pilih item → sajikan → terima respons →
//! estimasi ulang → cek stop → ... → hasil akhir) jadi satu orkestrator yang
//! bisa dipanggil per-request/per-langkah dari luar (mis. web API).
//!
//! Ground truth urutan pipeline: `mirtCAT/R/run_pattern.R::run_local()$fn`
//! (loop per-peserta) dan `mirtCAT/R/findNextCATItem.R::findNextCATItem()`
//! (urutan kerja nyata pemilihan item), dibaca langsung dari source
//! checkout (`../mirtCAT/R/`), bukan hanya dari sketsa `PLAN.md`:
//!
//! - Item **pertama** sesi selalu `start_item` **paksa** — tidak pernah
//!   lewat kriteria/content balancing/exposure control sama sekali
//!   (`if(all(is.na(person$responses)) && start) return(design@start_item)`
//!   di R). [`McatSession::next_item`] meniru ini persis: `person.n_answered()
//!   == 0` mengembalikan `config.start_item` langsung.
//! - Loop per-item (item ke-2 dst.): rekam respons → estimasi ulang theta
//!   (Fase 3) → evaluasi stop (Fase 4) → **kalau stop, berhenti** →
//!   [`McatSession::next_item`] lagi. Di dalam `findNextCATItem()`: skor
//!   SELURUH kandidat dihitung dulu, BARU disempitkan content balancing,
//!   BARU exposure control jalan di atas hasil sempitan. Modul
//!   `content_balancing` (Fase 6) sudah membuktikan lewat
//!   `narrowed_pool_composes_directly_with_rank_candidates` bahwa
//!   menyempitkan SEBELUM ranking (urutan di sini) = menyempitkan SESUDAH
//!   ranking (urutan R) secara numerik identik — jadi urutan yang dipakai:
//!   **kandidat (belum dijawab & valid) → `narrow_candidates` (bila content
//!   balancing aktif) → `rank_candidates`/`select_next_item` (kriteria) →
//!   `select_with_exposure` (bila exposure control aktif, hanya kriteria
//!   berskor) atau argmax langsung**.
//! - `Criterion::Seq` melewati content balancing & exposure control SAMA
//!   SEKALI (dicegah di konstruksi lewat
//!   `ContentBalancingConfig::validate_with_criterion`, dan
//!   `rank_candidates`/`select_with_exposure` sudah error untuk `Seq`/
//!   `Random` — lih. catatan modul `exposure_control` poin 1).
//!   `Criterion::Random` MASIH lewat content balancing (bila aktif) tapi
//!   TIDAK exposure control.
//!
//! **Deviasi dari sketsa awal `PLAN.md`** (pola sama seperti Fase 2–5, yang
//! masing-masing punya catatan serupa begitu diimplementasikan):
//!
//! 1. **`McatSession` TIDAK menyimpan `&ItemBank`/`&GroupParams`.** Sketsa
//!    menyebut "memegang ... referensi ItemBank+GroupParams", tapi
//!    `ItemBank` berisi `Box<dyn ItemResponseModel>` (trait object, tidak
//!    bisa `Serialize`; field lifetime-bound bikin session tidak bisa
//!    diserialisasi ke JSON/Redis/DB) — bertentangan langsung dengan tujuan
//!    "stateless-friendly" yang dinyatakan fase ini sendiri. Solusi: ikuti
//!    pola yang SUDAH dipakai tiap fase sebelumnya (`select_next_item`,
//!    `estimate`, `evaluate`, `select_with_exposure`, `narrow_candidates` —
//!    semua menerima `&ItemBank`/`&GroupParams` per-panggilan, bukan field
//!    tersimpan). [`McatSession::next_item`]/[`McatSession::submit_response`]
//!    menerima `bank`/`group` sebagai parameter.
//! 2. **`rng` juga parameter per-panggilan**, bukan field `McatSession` —
//!    pola sama seperti `select_next_item`/`select_with_exposure` (Fase
//!    2/5): reproducibility lewat seed eksternal, dan menghindari
//!    dependency serde untuk `StdRng` yang tidak perlu.
//! 3. **`finalize(&self) -> McatResult`, bukan `finalize(self)`** — konsumsi
//!    `self` tidak perlu (`McatResult::from_person` hanya butuh
//!    `&PersonState` + `&StopDecision`); borrow lebih berguna untuk API
//!    HTTP (endpoint hasil bisa dipanggil berkali-kali, bahkan sebelum
//!    sesi selesai) tanpa clone manual di pemanggil.
//! 4. **`InitialThetaDebug` di field terpisah** (`initial_theta_debug`),
//!    bukan masuk [`StepDebug`] — sketsa `PLAN.md` sendiri tidak
//!    menyertakan varian `InitialTheta` di situ (dihasilkan sekali di
//!    `new()`, bukan per langkah `next_item`/`submit_response`).
//! 5. **`InfoMatrix` (`src/theta.rs`) dapat `impl Serialize/Deserialize`
//!    manual** (pola sama seperti `ThetaVector`/`SEVector`, `Vec<Vec<f64>>`
//!    row-major) supaya `ItemSelectionDebug` (field `Option<InfoMatrix>`)
//!    ikut serde tanpa pengecualian — `McatSession` termasuk `debug_log`
//!    bisa diserialisasi utuh, tidak ada state yang diam-diam hilang saat
//!    disimpan ke store eksternal.

use rand::Rng;

use crate::ability_estimation::{self, AbilityEstimationDebug, EstimationConfig};
use crate::bank::{GroupParams, ItemBank};
use crate::content_balancing::{self, ContentBalancingConfig, ContentBalancingDebug};
use crate::error::MirtCatError;
use crate::exposure_control::{self, ExposureControlConfig, ExposureControlDebug};
use crate::initial_theta::{self, InitialThetaConfig, InitialThetaDebug};
use crate::item_selection::{
    self, Criterion, ItemIndex, ItemSelectionDebug, PersonState, SelectionConfig,
};
use crate::result::McatResult;
use crate::stopping_rule::{self, CustomStop, StopDecision, StoppingConfig};

/// Konfigurasi satu sesi MCAT — gabungan config Fase 1–6 + parameter
/// orkestrasi (`start_item`, `debug`) yang belum dimiliki fase manapun
/// sebelumnya.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SessionConfig {
    pub initial_theta: InitialThetaConfig,
    pub selection: SelectionConfig,
    pub estimation: EstimationConfig,
    pub stopping: StoppingConfig,
    /// `None` — exposure control nonaktif (padanan `mirtCAT` `exposure_type
    /// == 'none'`, lih. catatan modul `exposure_control`).
    pub exposure: Option<ExposureControlConfig>,
    /// `None` — content balancing nonaktif.
    pub content_balancing: Option<ContentBalancingConfig>,
    /// Item pertama sesi, disajikan TANPA lewat kriteria/content
    /// balancing/exposure control — padanan `design@start_item` R
    /// (lih. catatan modul).
    pub start_item: ItemIndex,
    /// Saklar tunggal jejak debug (prinsip desain #7 `PLAN.md`) — diteruskan
    /// otomatis ke SELURUH sub-config Fase 1–6 saat [`McatSession::new`],
    /// menimpa apa pun yang diset manual pada sub-config individual.
    /// Default `false`.
    pub debug: bool,
}

/// Jejak debug satu langkah `next_item()`/`submit_response()` — agregasi
/// `Option<XxxDebug>` Fase 1–6 yang `Some` pada langkah itu. Hanya terisi
/// bila `SessionConfig::debug == true` (prinsip desain #7); tidak pernah
/// memengaruhi `ItemIndex`/`StopDecision`/`McatResult` yang dikembalikan.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StepDebug {
    Selection(ItemSelectionDebug),
    Estimation(AbilityEstimationDebug),
    Stopping(stopping_rule::StoppingRuleDebug),
    Exposure(ExposureControlDebug),
    ContentBalancing(ContentBalancingDebug),
}

/// Orkestrator siklus adaptif MCAT penuh — lih. dokumentasi modul untuk
/// urutan pipeline & deviasi dari sketsa awal `PLAN.md`.
///
/// **Sengaja TIDAK memegang `ItemBank`/`GroupParams`** — keduanya
/// diteruskan per-panggilan ke [`Self::next_item`]/[`Self::submit_response`]
/// (lih. deviasi #1 dokumentasi modul), supaya `McatSession` tetap bisa
/// `Serialize`/`Deserialize` utuh (feature `serde`) untuk disimpan antar
/// request HTTP (cookie/DB/Redis).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct McatSession {
    person: PersonState,
    config: SessionConfig,
    /// Diindeks per `ItemIndex` bank, panjang `bank.len()` saat `new()` --
    /// state exposure control Sympson-Hetter (item yang ditolak jadi
    /// `false` permanen sepanjang sesi, lih. catatan modul
    /// `exposure_control`).
    valid_items: Vec<bool>,
    /// Item hasil `next_item()` terakhir yang BELUM dijawab --
    /// `submit_response()` menolak `item` yang tidak cocok dengan ini
    /// (mencegah state korup).
    pending_item: Option<ItemIndex>,
    last_stop_decision: StopDecision,
    initial_theta_debug: Option<InitialThetaDebug>,
    debug_log: Vec<StepDebug>,
}

impl McatSession {
    /// Mulai sesi baru: inisialisasi `theta`/SE awal (Fase 1, reuse
    /// `initial_theta::initialize`), validasi kombinasi content
    /// balancing/kriteria (Fase 6, reuse
    /// `ContentBalancingConfig::validate_with_criterion`), dan sinkronkan
    /// `config.debug` ke seluruh sub-config.
    ///
    /// `person_index` diteruskan ke `initial_theta::initialize` (dipakai
    /// hanya untuk `ThetaStart::PerPersonMatrix`, lih. Fase 1).
    ///
    /// Error bila `config.start_item >= bank.len()`, kombinasi content
    /// balancing + `Criterion::Seq`, atau error konstruksi Fase 1 (dimensi
    /// `ThetaStart`/`weights` tidak cocok `group.nfact()`).
    pub fn new(
        mut config: SessionConfig,
        bank: &ItemBank,
        group: &GroupParams,
        person_index: usize,
    ) -> Result<Self, MirtCatError> {
        config.initial_theta.debug = config.debug;
        config.selection.debug = config.debug;
        config.estimation.debug = config.debug;
        config.stopping.debug = config.debug;
        if let Some(exposure) = &mut config.exposure {
            exposure.debug = config.debug;
        }
        if let Some(cb) = &mut config.content_balancing {
            cb.debug = config.debug;
        }

        if let Some(cb) = &config.content_balancing {
            cb.validate_with_criterion(config.selection.criterion)?;
        }

        if config.start_item >= bank.len() {
            return Err(MirtCatError::InvalidParameter(format!(
                "McatSession::new: start_item {} di luar jangkauan ItemBank (panjang {})",
                config.start_item,
                bank.len()
            )));
        }

        let nfact = group.nfact();
        let (theta0, se0, _weights, initial_theta_debug) =
            initial_theta::initialize(&config.initial_theta, group, person_index)?;
        let person = PersonState::from_theta_and_se(theta0, se0);

        Ok(Self {
            person,
            config,
            valid_items: vec![true; bank.len()],
            pending_item: None,
            last_stop_decision: StopDecision {
                stop_now: false,
                met_sem: vec![false; nfact],
                met_delta_thetas: vec![false; nfact],
                met_classify: None,
                classification: Vec::new(),
                terminated_early: false,
            },
            initial_theta_debug,
            debug_log: Vec::new(),
        })
    }

    /// Pilih item selanjutnya. Item pertama sesi (`person.n_answered() ==
    /// 0`) selalu `config.start_item`, TANPA lewat kriteria/content
    /// balancing/exposure control (lih. dokumentasi modul). Selain itu:
    /// kandidat (belum dijawab & `valid_items`) → content balancing (bila
    /// aktif, dilewati untuk `Criterion::Seq`) → kriteria (+ exposure
    /// control bila aktif, hanya kriteria berskor -- `Seq`/`Random` tidak
    /// pernah masuk exposure control, lih. catatan modul
    /// `exposure_control`).
    ///
    /// Error bila sesi sudah berhenti (`stop_now == true` pada evaluasi
    /// terakhir), kandidat habis, atau error dari fase yang dipanggil
    /// (Fase 2/5/6).
    pub fn next_item(
        &mut self,
        bank: &ItemBank,
        group: &GroupParams,
        rng: &mut impl Rng,
    ) -> Result<ItemIndex, MirtCatError> {
        if self.last_stop_decision.stop_now {
            return Err(MirtCatError::InvalidParameter(
                "McatSession::next_item: sesi sudah berhenti (stop_now == true)".into(),
            ));
        }

        if self.person.n_answered() == 0 {
            let chosen = self.config.start_item;
            self.pending_item = Some(chosen);
            return Ok(chosen);
        }

        let candidates: Vec<ItemIndex> = (0..bank.len())
            .filter(|&idx| self.valid_items[idx] && !self.person.answered_items().contains(&idx))
            .collect();
        if candidates.is_empty() {
            return Err(MirtCatError::InvalidParameter(
                "McatSession::next_item: kandidat habis -- tidak ada item tersisa untuk \
                 disajikan"
                    .into(),
            ));
        }

        let criterion = self.config.selection.criterion;

        let (candidates, cb_debug) = if criterion == Criterion::Seq {
            (candidates, None)
        } else if let Some(cb) = &self.config.content_balancing {
            content_balancing::narrow_candidates(cb, self.person.answered_items(), &candidates)?
        } else {
            (candidates, None)
        };

        let (chosen, sel_debug, exp_debug) = match criterion {
            Criterion::Seq | Criterion::Random => {
                let (chosen, sel_debug) = item_selection::select_next_item(
                    &self.config.selection,
                    &candidates,
                    &self.person,
                    bank,
                    group,
                    rng,
                )?;
                (chosen, sel_debug, None)
            }
            _ => {
                if let Some(exposure) = &self.config.exposure {
                    let scores = item_selection::rank_candidates(
                        &self.config.selection,
                        &candidates,
                        &self.person,
                        bank,
                        group,
                    )?;
                    let position = self.person.n_answered();
                    let (chosen, exp_debug) = exposure_control::select_with_exposure(
                        exposure,
                        &scores,
                        position,
                        &mut self.valid_items,
                        rng,
                    )?;
                    (chosen, None, exp_debug)
                } else {
                    let (chosen, sel_debug) = item_selection::select_next_item(
                        &self.config.selection,
                        &candidates,
                        &self.person,
                        bank,
                        group,
                        rng,
                    )?;
                    (chosen, sel_debug, None)
                }
            }
        };

        self.pending_item = Some(chosen);

        if let Some(d) = cb_debug {
            self.debug_log.push(StepDebug::ContentBalancing(d));
        }
        if let Some(d) = sel_debug {
            self.debug_log.push(StepDebug::Selection(d));
        }
        if let Some(d) = exp_debug {
            self.debug_log.push(StepDebug::Exposure(d));
        }

        Ok(chosen)
    }

    /// Terima respons untuk `item`, estimasi ulang theta (Fase 3), lalu
    /// evaluasi stopping rule (Fase 4).
    ///
    /// `custom_stop` diteruskan apa adanya ke `stopping_rule::evaluate`
    /// (parameter per-panggilan, bukan field tersimpan -- pola sama seperti
    /// Fase 4, lih. catatan modul `stopping_rule` soal kenapa `dyn
    /// CustomStop` tidak cocok jadi field yang perlu `Clone`/`PartialEq`).
    ///
    /// Error `MirtCatError::InvalidParameter` bila `item` bukan hasil
    /// [`Self::next_item`] terakhir (mencegah state korup -- item yang
    /// direkam tidak sinkron dengan item yang benar-benar disajikan), atau
    /// error dari Fase 3/4.
    pub fn submit_response(
        &mut self,
        item: ItemIndex,
        response: usize,
        bank: &ItemBank,
        group: &GroupParams,
        custom_stop: Option<&dyn CustomStop>,
    ) -> Result<StopDecision, MirtCatError> {
        if self.pending_item != Some(item) {
            return Err(MirtCatError::InvalidParameter(format!(
                "McatSession::submit_response: item {item} bukan hasil next_item() terakhir \
                 ({:?}) -- panggil next_item() dulu sebelum submit_response()",
                self.pending_item
            )));
        }

        self.person.record_answer(item, response);

        let ((theta, se), est_debug) =
            ability_estimation::estimate(&self.config.estimation, &self.person, bank, group)?;
        self.person.set_estimate(theta, se);

        // `in_unfinished_constraint_block = false` -- constraint block
        // (`ordered`/`unordered`) di luar scope Fase 6 (belum
        // diimplementasikan), sama seperti precedent Fase 4/5 yang
        // menerima fakta ini sebagai `bool` dari pemanggil.
        let (decision, stop_debug) = stopping_rule::evaluate(
            &self.config.stopping,
            &self.person,
            bank,
            false,
            custom_stop,
        )?;

        self.last_stop_decision = decision.clone();
        self.pending_item = None;

        if let Some(d) = est_debug {
            self.debug_log.push(StepDebug::Estimation(d));
        }
        if let Some(d) = stop_debug {
            self.debug_log.push(StepDebug::Stopping(d));
        }

        Ok(decision)
    }

    /// Rakit `McatResult` (Fase 7) dari state sesi saat ini -- bisa
    /// dipanggil berkali-kali, termasuk sebelum sesi benar-benar berhenti
    /// (mis. untuk snapshot "hasil sejauh ini").
    pub fn finalize(&self) -> McatResult {
        McatResult::from_person(&self.person, &self.last_stop_decision)
    }

    pub fn person(&self) -> &PersonState {
        &self.person
    }

    pub fn config(&self) -> &SessionConfig {
        &self.config
    }

    /// Item yang sedang menunggu jawaban (hasil `next_item()` terakhir
    /// yang belum di-`submit_response()`), `None` bila belum ada
    /// `next_item()` dipanggil atau respons terakhir sudah diterima.
    pub fn pending_item(&self) -> Option<ItemIndex> {
        self.pending_item
    }

    pub fn last_stop_decision(&self) -> &StopDecision {
        &self.last_stop_decision
    }

    /// `true` bila evaluasi stopping rule terakhir memutuskan berhenti --
    /// padanan singkat `last_stop_decision().stop_now`.
    pub fn is_finished(&self) -> bool {
        self.last_stop_decision.stop_now
    }

    pub fn initial_theta_debug(&self) -> Option<&InitialThetaDebug> {
        self.initial_theta_debug.as_ref()
    }

    /// Jejak debug append-only seluruh langkah `next_item()`/
    /// `submit_response()` -- selalu kosong bila `config.debug == false`.
    pub fn debug_log(&self) -> &[StepDebug] {
        &self.debug_log
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bank::ItemMeta;
    use crate::content_balancing::ContentCategory;
    use crate::irt::Dichotomous;
    use crate::item_selection::Criterion;
    use crate::stopping_rule::ClassifyType;
    use crate::ThetaVector;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use std::collections::HashMap;

    /// Bank sintetis 2 dimensi, 10 item -- pola sama seperti `bank_2d` di
    /// modul-modul fase sebelumnya, diperluas.
    fn bank_2d_10items() -> ItemBank {
        let mut bank = ItemBank::new();
        for i in 0..10 {
            let dim_strong = i % 2;
            let mut a = vec![0.3; 2];
            a[dim_strong] = 1.5;
            bank.push(
                Box::new(Dichotomous::m2pl(a, 0.0).unwrap()),
                ItemMeta::new(i),
            )
            .unwrap();
        }
        bank
    }

    fn group_2d() -> GroupParams {
        GroupParams::standard_normal(2)
    }

    fn simple_config() -> SessionConfig {
        SessionConfig {
            selection: SelectionConfig {
                criterion: Criterion::DRule { prior: false },
                ..Default::default()
            },
            estimation: EstimationConfig::default(), // Map
            stopping: StoppingConfig {
                min_items: 3,
                max_items: 10,
                min_sem: vec![0.4],
                ..Default::default()
            },
            start_item: 0,
            ..Default::default()
        }
    }

    /// Jalankan sesi sampai selesai, respons selalu kategori `1` (benar) --
    /// dipakai beberapa test yang cuma butuh siklus berjalan, tidak peduli
    /// pola respons sesungguhnya.
    fn run_to_completion(
        session: &mut McatSession,
        bank: &ItemBank,
        group: &GroupParams,
        rng: &mut impl Rng,
    ) {
        loop {
            let item = session.next_item(bank, group, rng).unwrap();
            let decision = session.submit_response(item, 1, bank, group, None).unwrap();
            if decision.stop_now {
                break;
            }
        }
    }

    #[test]
    fn full_cycle_from_new_to_finalize_yields_consistent_result() {
        let bank = bank_2d_10items();
        let group = group_2d();
        let mut session = McatSession::new(simple_config(), &bank, &group, 0).unwrap();
        let mut rng = StdRng::seed_from_u64(1);

        run_to_completion(&mut session, &bank, &group, &mut rng);

        let result = session.finalize();
        assert!(session.is_finished());
        assert_eq!(result.thetas_history.len(), result.items_answered.len());
        assert_eq!(result.thetas_se_history.len(), result.items_answered.len());
        assert!(!result.items_answered.is_empty());
    }

    #[test]
    fn first_next_item_is_always_start_item_regardless_of_criterion() {
        let bank = bank_2d_10items();
        let group = group_2d();
        let mut rng = StdRng::seed_from_u64(2);

        for criterion in [
            Criterion::Seq,
            Criterion::Random,
            Criterion::DRule { prior: false },
        ] {
            let config = SessionConfig {
                selection: SelectionConfig {
                    criterion,
                    ..Default::default()
                },
                start_item: 4,
                ..Default::default()
            };
            let mut session = McatSession::new(config, &bank, &group, 0).unwrap();
            let item = session.next_item(&bank, &group, &mut rng).unwrap();
            assert_eq!(
                item, 4,
                "criterion {criterion:?} harus tetap start_item di langkah pertama"
            );
        }
    }

    #[test]
    fn submit_response_rejects_item_that_is_not_the_pending_item() {
        let bank = bank_2d_10items();
        let group = group_2d();
        let mut session = McatSession::new(simple_config(), &bank, &group, 0).unwrap();
        let mut rng = StdRng::seed_from_u64(3);

        let item = session.next_item(&bank, &group, &mut rng).unwrap();
        let wrong_item = if item == 0 { 1 } else { 0 };
        let err = session
            .submit_response(wrong_item, 1, &bank, &group, None)
            .unwrap_err();
        assert!(matches!(err, MirtCatError::InvalidParameter(_)));

        // item yang benar tetap bisa dipakai setelahnya.
        assert!(session
            .submit_response(item, 1, &bank, &group, None)
            .is_ok());
    }

    #[test]
    fn submit_response_without_prior_next_item_errors() {
        let bank = bank_2d_10items();
        let group = group_2d();
        let mut session = McatSession::new(simple_config(), &bank, &group, 0).unwrap();
        let err = session
            .submit_response(0, 1, &bank, &group, None)
            .unwrap_err();
        assert!(matches!(err, MirtCatError::InvalidParameter(_)));
    }

    #[test]
    fn next_item_after_session_finished_errors() {
        let bank = bank_2d_10items();
        let group = group_2d();
        let config = SessionConfig {
            selection: SelectionConfig {
                criterion: Criterion::DRule { prior: false },
                ..Default::default()
            },
            stopping: StoppingConfig {
                max_items: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut session = McatSession::new(config, &bank, &group, 0).unwrap();
        let mut rng = StdRng::seed_from_u64(4);

        let item = session.next_item(&bank, &group, &mut rng).unwrap();
        let decision = session
            .submit_response(item, 1, &bank, &group, None)
            .unwrap();
        assert!(decision.stop_now); // max_items=1 -> berhenti setelah 1 item

        let err = session.next_item(&bank, &group, &mut rng).unwrap_err();
        assert!(matches!(err, MirtCatError::InvalidParameter(_)));
    }

    #[test]
    fn seq_plus_content_balancing_is_rejected_at_construction() {
        let bank = bank_2d_10items();
        let group = group_2d();
        let mut target = HashMap::new();
        target.insert(ContentCategory::from("A"), 1.0);
        let cb = ContentBalancingConfig::new(vec![ContentCategory::from("A"); 10], target, false)
            .unwrap();
        let config = SessionConfig {
            selection: SelectionConfig {
                criterion: Criterion::Seq,
                ..Default::default()
            },
            content_balancing: Some(cb),
            ..Default::default()
        };
        let err = McatSession::new(config, &bank, &group, 0).unwrap_err();
        assert_eq!(err, MirtCatError::ContentBalancingIncompatibleWithSeq);
    }

    #[test]
    fn start_item_out_of_range_errors_at_construction() {
        let bank = bank_2d_10items();
        let group = group_2d();
        let config = SessionConfig {
            start_item: 999,
            ..Default::default()
        };
        let err = McatSession::new(config, &bank, &group, 0).unwrap_err();
        assert!(matches!(err, MirtCatError::InvalidParameter(_)));
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serialize_then_deserialize_mid_session_continues_identically() {
        let bank = bank_2d_10items();
        let group = group_2d();
        let config = simple_config();

        let mut original = McatSession::new(config.clone(), &bank, &group, 0).unwrap();
        let mut rng_a = StdRng::seed_from_u64(42);
        for _ in 0..3 {
            let item = original.next_item(&bank, &group, &mut rng_a).unwrap();
            original
                .submit_response(item, 1, &bank, &group, None)
                .unwrap();
        }

        let json = serde_json::to_string(&original).unwrap();
        let mut restored: McatSession = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);

        // Lanjutkan KEDUA sesi dengan RNG yang di-seed identik dari titik
        // ini -- hasil akhir harus sama persis.
        let mut rng_b = StdRng::seed_from_u64(777);
        let mut rng_c = StdRng::seed_from_u64(777);
        run_to_completion(&mut original, &bank, &group, &mut rng_b);
        run_to_completion(&mut restored, &bank, &group, &mut rng_c);

        assert_eq!(original.finalize(), restored.finalize());
        assert_eq!(original, restored);
    }

    #[test]
    fn debug_false_keeps_log_empty_and_debug_true_does_not_change_outcome() {
        let bank = bank_2d_10items();
        let group = group_2d();

        let config_no_debug = simple_config();
        let mut session_a = McatSession::new(config_no_debug, &bank, &group, 0).unwrap();
        let mut rng_a = StdRng::seed_from_u64(9);
        run_to_completion(&mut session_a, &bank, &group, &mut rng_a);
        assert!(session_a.debug_log().is_empty());
        assert!(session_a.initial_theta_debug().is_none());

        let config_debug = SessionConfig {
            debug: true,
            ..simple_config()
        };
        let mut session_b = McatSession::new(config_debug, &bank, &group, 0).unwrap();
        let mut rng_b = StdRng::seed_from_u64(9);
        run_to_completion(&mut session_b, &bank, &group, &mut rng_b);
        assert!(!session_b.debug_log().is_empty());
        assert!(session_b.initial_theta_debug().is_some());

        assert_eq!(session_a.finalize(), session_b.finalize());
    }

    #[test]
    fn selected_item_always_stays_within_content_balancing_narrowed_pool() {
        let bank = bank_2d_10items();
        let group = group_2d();
        let categories: Vec<ContentCategory> = (0..10)
            .map(|i| if i % 2 == 0 { "Aljabar" } else { "Geometri" }.into())
            .collect();
        let mut target = HashMap::new();
        target.insert(ContentCategory::from("Aljabar"), 1.0);
        target.insert(ContentCategory::from("Geometri"), 0.0);
        let cb = ContentBalancingConfig::new(categories, target, false).unwrap();

        let config = SessionConfig {
            selection: SelectionConfig {
                criterion: Criterion::DRule { prior: false },
                ..Default::default()
            },
            content_balancing: Some(cb),
            exposure: Some(
                ExposureControlConfig::new(
                    exposure_control::ExposureMode::SympsonHetter(vec![1.0; 10]),
                    10,
                    false,
                )
                .unwrap(),
            ),
            stopping: StoppingConfig {
                max_items: 5,
                ..Default::default()
            },
            start_item: 1, // item ganjil (Geometri), item selanjutnya harus Aljabar (target=1.0)
            ..Default::default()
        };
        let mut session = McatSession::new(config, &bank, &group, 0).unwrap();
        let mut rng = StdRng::seed_from_u64(5);

        let first = session.next_item(&bank, &group, &mut rng).unwrap();
        assert_eq!(first, 1);
        session
            .submit_response(first, 1, &bank, &group, None)
            .unwrap();

        let second = session.next_item(&bank, &group, &mut rng).unwrap();
        assert_eq!(
            second % 2,
            0,
            "target Aljabar=1.0 -- item kedua harus dari kategori Aljabar (index genap)"
        );
    }

    #[test]
    #[cfg(feature = "serde")]
    fn initial_theta_debug_serializes_when_present() {
        let bank = bank_2d_10items();
        let group = group_2d();
        let config = SessionConfig {
            debug: true,
            ..simple_config()
        };
        let session = McatSession::new(config, &bank, &group, 0).unwrap();
        let json = serde_json::to_string(&session).unwrap();
        let back: McatSession = serde_json::from_str(&json).unwrap();
        assert_eq!(session, back);
        assert!(back.initial_theta_debug().is_some());
    }

    #[test]
    fn classify_type_ci_config_is_accepted_and_theta0_is_zero() {
        let bank = bank_2d_10items();
        let group = group_2d();
        let config = SessionConfig {
            selection: SelectionConfig {
                criterion: Criterion::DRule { prior: false },
                ..Default::default()
            },
            stopping: StoppingConfig {
                classify: ClassifyType::Ci {
                    cutoffs: vec![0.0, 0.0],
                    alpha: 0.05,
                },
                min_items: 2,
                max_items: 10,
                ..Default::default()
            },
            ..Default::default()
        };
        let session = McatSession::new(config, &bank, &group, 0).unwrap();
        assert_eq!(session.person().theta(), &ThetaVector::zeros(2));
    }
}
