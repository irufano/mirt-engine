//! Implementasi tiap kriteria seleksi item — satu file per algoritma
//! (lih. `PLAN.md` §Fase 2 "Desain Modul Rust"). Fungsi di sini murni
//! (`total_info`/`theta`/`weights` sudah dibentuk oleh caller), dispatcher
//! ada di `item_selection::select_next_item` (`../mod.rs`).

pub(crate) mod a_rule;
pub(crate) mod d_rule;
pub(crate) mod e_rule;
pub(crate) mod kl;
pub(crate) mod random;
pub(crate) mod seq;
pub(crate) mod t_rule;
pub(crate) mod w_rule;
