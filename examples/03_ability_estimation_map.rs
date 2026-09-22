//! Contoh Fase 3: MAP (Maximum A Posteriori) dengan prior `N(gmeans, gcov)`
//! — tunjukkan efek prior sempit vs lebar terhadap penarikan theta ke arah
//! mean (`docs/teori/03-ability-estimation-map.md`).
//!
//! Jalankan dengan:
//! ```sh
//! cargo run --example 03_ability_estimation_map
//! ```

use mirtcat_engine::ability_estimation::estimate;
use mirtcat_engine::irt::Dichotomous;
use mirtcat_engine::theta::InfoMatrix;
use mirtcat_engine::{
    EstimationConfig, GroupParams, ItemBank, ItemMeta, Method, PersonState, ThetaVector,
};
use nalgebra::DMatrix;

fn build_bank() -> ItemBank {
    let params: [(f64, f64); 6] = [
        (1.2, 0.0),
        (1.0, 0.1),
        (0.9, -0.1),
        (1.1, 0.2),
        (0.8, -0.2),
        (1.0, 0.0),
    ];
    let mut bank = ItemBank::new();
    for (i, (a, d)) in params.into_iter().enumerate() {
        bank.push(
            Box::new(Dichotomous::m2pl(vec![a], d).unwrap()),
            ItemMeta::new(i),
        )
        .unwrap();
    }
    bank
}

fn run(label: &str, group: &GroupParams, person: &PersonState, bank: &ItemBank) {
    let config = EstimationConfig {
        method: Method::Map,
        debug: true,
        ..Default::default()
    };
    let ((theta, se), debug) = estimate(&config, person, bank, group).unwrap();
    let debug = debug.unwrap();
    println!(
        "{label}: theta={:.4}, se={:.4} (gmeans={:.2}, gcov_diag={:.2}, iterasi={:?}, konvergen={:?})",
        theta[0],
        se[0],
        group.gmeans()[0],
        group.gcov().get(0, 0),
        debug.iterations,
        debug.converged
    );
}

fn main() {
    let bank = build_bank();
    let mut person = PersonState::new(1);
    // 4 benar, 2 salah -- pola moderat, bukan ekstrem, supaya efek prior
    // (bukan cuma likelihood mendominasi) terlihat jelas.
    for (idx, response) in [1, 0, 1, 1, 0, 1].into_iter().enumerate() {
        person.record_answer(idx, response);
    }

    println!("Pola respons: 4 benar, 2 salah dari 6 item (a≈0.8-1.2, d≈±0.2)");
    println!("gmeans = 2.0 di seluruh skenario (prior menarik ke arah tinggi, berlawanan arah likelihood)");
    println!();

    let gmeans = ThetaVector::from_vec(vec![2.0]);

    let narrow_cov = InfoMatrix::from_matrix(DMatrix::from_row_slice(1, 1, &[0.1])).unwrap();
    let narrow_group = GroupParams::new(gmeans.clone(), narrow_cov).unwrap();
    run("Prior SEMPIT (var=0.1)", &narrow_group, &person, &bank);

    let moderate_cov = InfoMatrix::from_matrix(DMatrix::from_row_slice(1, 1, &[1.0])).unwrap();
    let moderate_group = GroupParams::new(gmeans.clone(), moderate_cov).unwrap();
    run("Prior SEDANG (var=1.0)", &moderate_group, &person, &bank);

    let wide_cov = InfoMatrix::from_matrix(DMatrix::from_row_slice(1, 1, &[100.0])).unwrap();
    let wide_group = GroupParams::new(gmeans, wide_cov).unwrap();
    run(
        "Prior LEBAR (var=100, hampir tak berinformasi)",
        &wide_group,
        &person,
        &bank,
    );

    println!();
    println!(
        "Pengamatan: prior SEMPIT menarik theta paling dekat ke gmeans=2.0 (mendominasi \
         likelihood); prior LEBAR membiarkan likelihood mendominasi (theta mendekati hasil ML \
         murni -- bandingkan `cargo run --example 03_ability_estimation_ml`)."
    );

    println!();
    println!("== Tanpa item terjawab -- MAP harus persis gmeans (posterior = prior) ==");
    let group_zero =
        GroupParams::new(ThetaVector::from_vec(vec![0.6]), InfoMatrix::identity(1)).unwrap();
    let empty_person = PersonState::new(1);
    run("Belum ada item dijawab", &group_zero, &empty_person, &bank);
}
