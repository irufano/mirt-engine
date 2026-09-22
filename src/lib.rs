//! `mirtcat-engine` — mesin administrasi tes adaptif multidimensional
//! (Multidimensional Computerized Adaptive Testing), mengacu pada perilaku
//! `mirt`/`mirtCAT` (R), dipersempit ke skenario `nfact > 1`.
//!
//! Engine ini **tidak** melakukan kalibrasi model IRT — ia menerima
//! parameter item yang sudah terkalibrasi (lih. [`irt`]) dan menjalankan
//! siklus adaptif di atasnya (item selection, ability estimation, stopping
//! rule, exposure control, content balancing — fase-fase berikutnya).
//!
//! Status implementasi saat ini: **Fase 1 — theta awal**. Lihat `PLAN.md`
//! di root crate untuk peta fase lengkap.

pub mod bank;
pub mod error;
pub mod initial_theta;
pub mod irt;
pub mod theta;

pub use bank::{GroupParams, ItemBank, ItemMeta};
pub use error::MirtCatError;
pub use initial_theta::{InitialThetaConfig, InitialThetaDebug, ThetaStart};
pub use theta::{InfoMatrix, SEVector, ThetaVector};
