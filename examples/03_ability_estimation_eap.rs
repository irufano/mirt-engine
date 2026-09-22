//! Contoh Fase 3: EAP (Expected A Posteriori) grid-based vs QMC-based pada
//! pola respons yang sama — bandingkan theta/SE & waktu komputasi relatif
//! antar mode integrasi (`eap.rs` menaungi keduanya, sama seperti
//! `KL`/`KLn` di Fase 2 satu modul untuk satu keluarga algoritma).
//!
//! Jalankan dengan:
//! ```sh
//! cargo run --example 03_ability_estimation_eap
//! ```

use std::time::Instant;

use mirtcat_engine::ability_estimation::estimate;
use mirtcat_engine::irt::Dichotomous;
use mirtcat_engine::{EstimationConfig, GroupParams, ItemBank, ItemMeta, Method, PersonState};

fn build_bank() -> ItemBank {
    let params: [([f64; 2], f64); 6] = [
        ([1.2, 0.3], 0.2),
        ([0.3, 1.2], -0.1),
        ([0.9, 0.9], 0.0),
        ([1.0, 0.4], 0.3),
        ([0.4, 1.0], -0.2),
        ([0.8, 0.8], 0.1),
    ];
    let mut bank = ItemBank::new();
    for (i, (a, d)) in params.into_iter().enumerate() {
        bank.push(
            Box::new(Dichotomous::m2pl(a.to_vec(), d).unwrap()),
            ItemMeta::new(i),
        )
        .unwrap();
    }
    bank
}

fn main() {
    let bank = build_bank();
    let group = GroupParams::standard_normal(2);
    let mut person = PersonState::new(2);
    for (idx, response) in [1, 0, 1, 1, 0, 1].into_iter().enumerate() {
        person.record_answer(idx, response);
    }

    println!("== EAP grid-based (quadpts=31/dimensi, total 31^2=961 titik) ==");
    let config_grid = EstimationConfig {
        method: Method::Eap { qmc: false },
        debug: true,
        ..Default::default()
    };
    let t0 = Instant::now();
    let ((theta_grid, se_grid), debug_grid) =
        estimate(&config_grid, &person, &bank, &group).unwrap();
    let dt_grid = t0.elapsed();
    println!("  theta = {:?}", theta_grid.as_slice());
    println!("  se    = {:?}", se_grid.as_slice());
    println!(
        "  n_quadrature_points = {:?}, waktu = {dt_grid:?}",
        debug_grid.unwrap().n_quadrature_points
    );

    println!();
    println!("== EAP QMC-based (leaped-Halton, quadpts=961 titik total -- disamakan jumlah titik grid di atas) ==");
    let config_qmc = EstimationConfig {
        method: Method::Eap { qmc: true },
        quadpts: Some(961),
        debug: true,
    };
    let t0 = Instant::now();
    let ((theta_qmc, se_qmc), debug_qmc) = estimate(&config_qmc, &person, &bank, &group).unwrap();
    let dt_qmc = t0.elapsed();
    println!("  theta = {:?}", theta_qmc.as_slice());
    println!("  se    = {:?}", se_qmc.as_slice());
    println!(
        "  n_quadrature_points = {:?}, waktu = {dt_qmc:?}",
        debug_qmc.unwrap().n_quadrature_points
    );

    println!();
    println!(
        "Selisih |theta_grid - theta_qmc| = [{:.4}, {:.4}] -- kecil untuk nfact=2 \
         (QMC lebih unggul justru di dimensi TINGGI, lih. docs/teori/03-ability-estimation-eap.md)",
        (theta_grid.as_slice()[0] - theta_qmc.as_slice()[0]).abs(),
        (theta_grid.as_slice()[1] - theta_qmc.as_slice()[1]).abs()
    );

    println!();
    println!("== EAP QMC-based dengan quadpts DEFAULT nfact=2 (31, bukan 961) ==");
    println!(
        "Catatan penting: default `quadpts` SAMA untuk grid & QMC (tabel nfact `mirtCAT`), \
         TIDAK otomatis dibesarkan untuk QMC -- QMC baru unggul bila `quadpts` dinaikkan \
         manual (lih. docs/panduan/03-ability-estimation-eap.md)."
    );
    let config_qmc_default = EstimationConfig {
        method: Method::Eap { qmc: true },
        debug: true,
        ..Default::default()
    };
    let ((theta_qmc_default, _), debug_qmc_default) =
        estimate(&config_qmc_default, &person, &bank, &group).unwrap();
    println!(
        "  n_quadrature_points = {:?}, theta = {:?} (lebih noisy dibanding 961 titik di atas)",
        debug_qmc_default.unwrap().n_quadrature_points,
        theta_qmc_default.as_slice()
    );
}
