//! `mirt-engine` — mesin administrasi tes adaptif multidimensional
//! (Multidimensional Computerized Adaptive Testing), mengacu pada perilaku
//! `mirt`/`mirtCAT` (R), dipersempit ke skenario `nfact > 1`.
//!
//! Engine ini **tidak** melakukan kalibrasi model IRT — ia menerima
//! parameter item yang sudah terkalibrasi (lih. [`irt`]) dan menjalankan
//! siklus adaptif di atasnya (item selection, ability estimation, stopping
//! rule, exposure control, content balancing — fase-fase berikutnya).
//!
//! Status implementasi: **Fase 0–8 selesai** — fondasi IRT, siklus adaptif
//! penuh (theta awal, seleksi item, estimasi kemampuan, stopping rule,
//! exposure control, content balancing, hasil akhir), serta hardening
//! rilis (Fase 8: property test, benchmark, audit keamanan, dokumentasi
//! API lengkap). Lihat `PLAN.md` di root crate untuk peta fase lengkap &
//! `CHANGELOG.md` untuk ringkasan tiap fase.

#![deny(missing_docs)]

pub mod ability_estimation;
pub mod bank;
pub mod content_balancing;
pub mod error;
pub mod exposure_control;
pub mod initial_theta;
pub mod irt;
pub mod item_selection;
pub mod result;
pub mod stopping_rule;
pub mod theta;

pub use ability_estimation::{AbilityEstimationDebug, EstimationConfig, Method, ScoreEstimate};
pub use bank::{GroupParams, ItemBank, ItemMeta};
pub use content_balancing::{
    narrow_candidates, ContentBalancingConfig, ContentBalancingDebug, ContentCategory,
};
pub use error::MirtCatError;
pub use exposure_control::{
    select_with_exposure, ExposureControlConfig, ExposureControlDebug, ExposureMode,
};
pub use initial_theta::{InitialThetaConfig, InitialThetaDebug, ThetaStart};
pub use item_selection::{
    rank_candidates, select_next_item, Criterion, ItemIndex, ItemSelectionDebug, PersonState,
    SelectionConfig,
};
pub use result::McatResult;
pub use stopping_rule::{
    evaluate as evaluate_stopping_rule, ClassifyDecision, ClassifyType, CustomStop, StopDecision,
    StoppingConfig, StoppingRuleDebug,
};
pub use theta::{InfoMatrix, SEVector, ThetaVector};
