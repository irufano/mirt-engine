//! Fase 7 — Hasil Bersih Akhir untuk Theta pada MCAT.
//!
//! Ground truth: `mirtCAT_post_internal()` (`mirtCAT/R/mirtCAT_preamble.R:245-285`),
//! yang merangkum `Person` (state internal sesi) jadi `list` kelas `mirtCAT`
//! yang dikembalikan ke pemanggil di akhir sesi.
//!
//! **Reuse eksplisit** (Prinsip Desain #1 `PLAN.md`): sketsa desain awal
//! `PLAN.md` mengusulkan `result::ClassifyDecision` sebagai enum baru --
//! ternyata **duplikat**. `stopping_rule::ClassifyDecision` (Fase 4) sudah
//! persis enum yang dibutuhkan (`AboveCutoff`/`BelowCutoff`/`NoDecision`,
//! padanan string R `'above cutoff'`/`'below cutoff'`/`'no decision'`) --
//! dipakai ulang langsung lewat `crate::stopping_rule::ClassifyDecision`,
//! bukan didefinisikan ulang. Pola sama untuk `info_thetas`: sketsa awal
//! mengusulkan accessor baru, padahal `PersonState::info_thetas(&self, bank)`
//! (Fase 2, `src/item_selection/mod.rs`) sudah persis aksesor yang dimaksud
//! -- lih. §"Accessor `info_thetas`" di bawah.
//!
//! **Deviasi dari `thetas_history` R**: R menyisipkan baris `theta`
//! pra-tes (sebelum item manapun dijawab) sebagai baris pertama
//! `thetas_history`/`thetas_SE_history` (`PersonClass.R$initialize`), lalu
//! `rbind` satu baris per re-estimasi -- hasil akhir R panjang
//! `N_item + 1`. `PersonState::thetas_history()`/`thetas_se_history()`
//! (`src/item_selection/mod.rs`) sengaja TIDAK menyertakan baris pra-tes itu
//! -- panjangnya persis `N_item`, sejalan 1:1 dengan `items_answered`/
//! `responses`, lebih bersih dipakai konsumen API (judul fase ini "Hasil
//! **Bersih** Akhir") daripada replikasi off-by-one R. Lih.
//! `docs/teori/07-hasil-akhir-theta.md`.
//!
//! **Deviasi `classification` untuk mode `Sprt`**: `StopDecision::classification`
//! (Fase 4) panjangnya bervariasi sesuai `ClassifyType` -- kosong (`None`),
//! `nfact` (`Ci`, per-dimensi), atau `1` (`Sprt`, satu keputusan gabungan;
//! `mirtCAT` sendiri menimpa `person$classify_decision` jadi skalar untuk
//! mode ini, lih. `DesignClass.R:327-329`). `McatResult::classification`
//! SELALU panjang `nfact` (spesifikasi desain fase ini): kosong dinormalisasi
//! jadi `vec![NoDecision; nfact]`, `Sprt` (panjang 1) di-*broadcast* ke
//! seluruh dimensi (keputusan gabungan berlaku sama untuk tiap dimensi,
//! bukan didekomposisi -- SPRT inheren satu uji hipotesis gabungan, bukan
//! per-dimensi), `Ci` (sudah panjang `nfact`) diteruskan apa adanya.

use crate::item_selection::PersonState;
use crate::stopping_rule::{ClassifyDecision, StopDecision};
use crate::theta::{SEVector, ThetaVector};

/// Hasil akhir sesi tes MCAT — padanan `list` kelas `'mirtCAT'` yang
/// dikembalikan `mirtCAT()` di R (`mirtCAT_post_internal()`), dipersempit ke
/// field yang relevan untuk `nfact > 1` (lih. modul).
///
/// **Sengaja tidak menyimpan `info_thetas`** (matriks informasi kumulatif) —
/// lih. §"Accessor `info_thetas`" di dokumentasi modul.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct McatResult {
    /// Estimasi `theta` akhir, panjang `nfact`.
    pub thetas: ThetaVector,
    /// SE akhir, panjang `nfact` -- baris terakhir `thetas_se_history`
    /// (identik `person.se()` saat `finalize`, lih. `from_person`).
    pub se_thetas: SEVector,
    /// Riwayat `theta` tiap re-estimasi, satu baris per item terjawab
    /// (lih. catatan deviasi di dokumentasi modul).
    pub thetas_history: Vec<ThetaVector>,
    /// Riwayat SE, sejalan dengan `thetas_history`.
    pub thetas_se_history: Vec<SEVector>,
    /// Keputusan klasifikasi PER-DIMENSI, panjang selalu `nfact`, default
    /// `NoDecision` (lih. catatan deviasi mode `Sprt` di dokumentasi modul).
    pub classification: Vec<ClassifyDecision>,
    /// `["Theta_1", "Theta_2", ..., "Theta_nfact"]`.
    pub column_names: Vec<String>,
    /// Index item (urutan `ItemBank`) yang terjawab, urut waktu penyajian.
    pub items_answered: Vec<usize>,
    /// Kategori respons per item terjawab, sejalan dengan `items_answered`.
    /// `Option` (bukan `i32` polos) -- ruang untuk item yang diadministrasikan
    /// tapi tidak dinilai (`items_not_scored`, di luar scope fase ini);
    /// `PersonState` saat ini tidak pernah mencatat slot semacam itu, jadi
    /// selalu `Some` (lih. `from_person`).
    pub responses: Vec<Option<i32>>,
    /// `true` bila sesi berhenti karena kriteria presisi (bukan
    /// `max_items`/`max_time`) -- diteruskan apa adanya dari `StopDecision`
    /// terakhir (Fase 4).
    pub terminated_early: bool,
}

impl McatResult {
    /// `["Theta_1", ..., "Theta_nfact"]` — padanan
    /// `paste0('Theta_', 1L:nfact)` R (`mirtCAT_preamble.R:276-277`).
    pub fn column_names(nfact: usize) -> Vec<String> {
        (1..=nfact).map(|i| format!("Theta_{i}")).collect()
    }

    /// Rakit `McatResult` dari `PersonState` final + `StopDecision` terakhir
    /// (hasil `stopping_rule::evaluate` di langkah yang memutuskan sesi
    /// berhenti) — padanan `mirtCAT_post_internal()`.
    pub fn from_person(person: &PersonState, stop_decision: &StopDecision) -> Self {
        let nfact = person.nfact();
        Self {
            thetas: person.theta().clone(),
            se_thetas: person.se().clone(),
            thetas_history: person.thetas_history().to_vec(),
            thetas_se_history: person.thetas_se_history().to_vec(),
            classification: normalize_classification(&stop_decision.classification, nfact),
            column_names: Self::column_names(nfact),
            items_answered: person.answered_items().to_vec(),
            responses: person.responses().iter().map(|&r| Some(r as i32)).collect(),
            terminated_early: stop_decision.terminated_early,
        }
    }
}

/// Normalisasi `StopDecision::classification` (panjang bervariasi, lih.
/// `stopping_rule::StopDecision`) jadi selalu panjang `nfact` -- lih. catatan
/// deviasi `Sprt` di dokumentasi modul.
fn normalize_classification(raw: &[ClassifyDecision], nfact: usize) -> Vec<ClassifyDecision> {
    match raw.len() {
        0 => vec![ClassifyDecision::NoDecision; nfact],
        n if n == nfact => raw.to_vec(),
        _ => vec![raw[0]; nfact],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stopping_rule::ClassifyType;

    fn person_with_history(nfact: usize, n_items: usize) -> PersonState {
        let mut person =
            PersonState::from_theta_and_se(ThetaVector::zeros(nfact), SEVector::zeros(nfact));
        for i in 0..n_items {
            person.record_answer(i, 1);
            person.set_estimate(
                ThetaVector::from_vec(vec![0.1 * (i as f64 + 1.0); nfact]),
                SEVector::from_vec(vec![1.0 / (i as f64 + 2.0); nfact]),
            );
        }
        person
    }

    fn no_stop(nfact: usize) -> StopDecision {
        StopDecision {
            stop_now: false,
            met_sem: vec![false; nfact],
            met_delta_thetas: vec![false; nfact],
            met_classify: None,
            classification: Vec::new(),
            terminated_early: false,
        }
    }

    #[test]
    fn column_names_generates_theta_1_through_nfact() {
        assert_eq!(
            McatResult::column_names(3),
            vec!["Theta_1", "Theta_2", "Theta_3"]
        );
    }

    #[test]
    fn from_person_fills_thetas_history_matrix_with_n_item_rows() {
        let person = person_with_history(3, 5);
        let result = McatResult::from_person(&person, &no_stop(3));

        assert_eq!(result.thetas_history.len(), 5);
        for row in &result.thetas_history {
            assert_eq!(row.nfact(), 3);
        }
        assert_eq!(result.thetas_se_history.len(), 5);
        assert_eq!(result.thetas, *person.theta());
        assert_eq!(result.se_thetas, *person.se());
        assert_eq!(result.items_answered, vec![0, 1, 2, 3, 4]);
        assert_eq!(result.responses, vec![Some(1); 5]);
        assert_eq!(result.column_names, McatResult::column_names(3));
    }

    #[test]
    fn classification_defaults_to_no_decision_for_every_dimension_when_classify_type_none() {
        let person = person_with_history(3, 2);
        let result = McatResult::from_person(&person, &no_stop(3));
        assert_eq!(result.classification, vec![ClassifyDecision::NoDecision; 3]);
    }

    #[test]
    fn classification_passes_through_per_dimension_decisions_for_ci_mode() {
        let person = person_with_history(2, 1);
        let mut stop = no_stop(2);
        stop.classification = vec![ClassifyDecision::AboveCutoff, ClassifyDecision::NoDecision];
        stop.met_classify = Some(vec![true, false]);
        let result = McatResult::from_person(&person, &stop);
        assert_eq!(
            result.classification,
            vec![ClassifyDecision::AboveCutoff, ClassifyDecision::NoDecision]
        );
    }

    #[test]
    fn classification_broadcasts_single_sprt_decision_to_every_dimension() {
        let person = person_with_history(3, 1);
        let mut stop = no_stop(3);
        stop.classification = vec![ClassifyDecision::BelowCutoff];
        let result = McatResult::from_person(&person, &stop);
        assert_eq!(
            result.classification,
            vec![ClassifyDecision::BelowCutoff; 3]
        );
    }

    #[test]
    fn terminated_early_is_carried_over_from_stop_decision() {
        let person = person_with_history(2, 1);
        let mut stop = no_stop(2);
        stop.terminated_early = true;
        let result = McatResult::from_person(&person, &stop);
        assert!(result.terminated_early);
    }

    #[test]
    fn info_thetas_is_not_a_field_of_mcat_result_structurally() {
        // Pattern destructuring EXHAUSTIVE (tanpa `..`) -- gagal kompilasi
        // bila `McatResult` punya field lain (mis. `info_thetas`) di luar
        // daftar ini, atau bila salah satu field di bawah tidak ada.
        let person = person_with_history(2, 1);
        let result = McatResult::from_person(&person, &no_stop(2));
        let McatResult {
            thetas: _,
            se_thetas: _,
            thetas_history: _,
            thetas_se_history: _,
            classification: _,
            column_names: _,
            items_answered: _,
            responses: _,
            terminated_early: _,
        } = result;

        // `info_thetas` tetap dapat diakses -- BUKAN lewat `McatResult`,
        // melainkan `PersonState::info_thetas()` (Fase 2, sudah ada
        // sebelumnya) selama `person`/`bank` masih hidup.
        let bank = {
            use crate::bank::{ItemBank, ItemMeta};
            use crate::irt::Dichotomous;
            let mut bank = ItemBank::new();
            bank.push(
                Box::new(Dichotomous::m2pl(vec![1.0, 1.0], 0.0).unwrap()),
                ItemMeta::new(0),
            )
            .unwrap();
            bank
        };
        assert!(person.info_thetas(&bank).is_ok());
    }

    #[test]
    fn from_person_handles_zero_answered_items_without_panicking() {
        let person = PersonState::new(2);
        let result = McatResult::from_person(&person, &no_stop(2));
        assert!(result.thetas_history.is_empty());
        assert!(result.items_answered.is_empty());
        assert_eq!(result.se_thetas, *person.se());
        assert_eq!(result.classification, vec![ClassifyDecision::NoDecision; 2]);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn mcat_result_serializes_and_deserializes_round_trip() {
        let person = person_with_history(3, 4);
        let mut stop = no_stop(3);
        stop.classification = vec![
            ClassifyDecision::AboveCutoff,
            ClassifyDecision::NoDecision,
            ClassifyDecision::BelowCutoff,
        ];
        stop.terminated_early = true;
        let result = McatResult::from_person(&person, &stop);

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"column_names\""));
        assert!(json.contains("Theta_1"));

        let back: McatResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back, result);
    }

    #[test]
    fn classify_type_default_still_produces_no_decision_vector() {
        // Sanity check dokumentasi: `ClassifyType::default()` == `None`.
        assert_eq!(ClassifyType::default(), ClassifyType::None);
    }
}
