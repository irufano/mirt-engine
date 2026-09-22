//! Fondasi model respons item (IRT) multidimensional — Fase 0.
//!
//! Menyediakan trait [`model::ItemResponseModel`] dan dua implementasi
//! konkret ([`dichotomous::Dichotomous`] untuk M2PL/M3PL/M4PL,
//! [`graded::GradedResponse`] untuk graded response model politomus),
//! ditambah agregasi informasi Fisher kumulatif ([`information`]) yang
//! menjadi dasar seluruh fase berikutnya (item selection, ability
//! estimation, dst.).

pub mod dichotomous;
pub mod graded;
pub mod information;
pub mod model;
pub(crate) mod probability;

pub use dichotomous::Dichotomous;
pub use graded::GradedResponse;
pub use information::cumulative_information;
pub use model::ItemResponseModel;
