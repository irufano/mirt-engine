//! `mirtcat-engine` — mesin administrasi tes adaptif multidimensional
//! (Multidimensional Computerized Adaptive Testing), mengacu pada perilaku
//! `mirt`/`mirtCAT` (R), dipersempit ke skenario `nfact > 1`.
//!
//! Engine ini **tidak** melakukan kalibrasi model IRT — ia menerima
//! parameter item yang sudah terkalibrasi (lih. [`irt`]) dan menjalankan
//! siklus adaptif di atasnya (item selection, ability estimation, stopping
//! rule, exposure control, content balancing — fase-fase berikutnya).
//!
//! Status implementasi saat ini: **Fase 6 — content balancing**. Lihat
//! `PLAN.md` di root crate untuk peta fase lengkap.

pub mod ability_estimation;
pub mod bank;
pub mod content_balancing;
pub mod error;
pub mod exposure_control;
pub mod initial_theta;
pub mod irt;
pub mod item_selection;
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
pub use stopping_rule::{
    evaluate as evaluate_stopping_rule, ClassifyDecision, ClassifyType, CustomStop, StopDecision,
    StoppingConfig, StoppingRuleDebug,
};
pub use theta::{InfoMatrix, SEVector, ThetaVector};
