//! Contoh Fase 3: `Method::Fixed` sebagai identity (theta/SE input tidak
//! berubah), dipakai untuk kuesioner non-adaptif
//! (`docs/teori/03-ability-estimation-fixed.md`).
//!
//! Jalankan dengan:
//! ```sh
//! cargo run --example 03_ability_estimation_fixed
//! ```

use mirtcat_engine::ability_estimation::estimate;
use mirtcat_engine::irt::Dichotomous;
use mirtcat_engine::theta::SEVector;
use mirtcat_engine::{EstimationConfig, GroupParams, ItemBank, ItemMeta, Method, PersonState, ThetaVector};

fn build_bank() -> ItemBank {
    let params: [(f64, f64); 4] = [(1.0, 0.0), (0.9, 0.1), (1.1, -0.1), (0.8, 0.2)];
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

fn main() {
    let bank = build_bank();
    let group = GroupParams::standard_normal(1);

    // Skenario kuesioner NON-adaptif: theta/SE sudah ditentukan di muka
    // (mis. hasil kalibrasi sebelumnya), dan seluruh item disajikan tanpa
    // estimasi ulang di antaranya.
    let fixed_theta = ThetaVector::from_vec(vec![0.42]);
    let fixed_se = SEVector::from_vec(vec![0.3]);
    let mut person = PersonState::from_theta_and_se(fixed_theta.clone(), fixed_se.clone());

    println!(
        "theta/SE awal (mis. dari kalibrasi sebelumnya, TIDAK dari `initial_theta`): \
         theta={:.4}, se={:.4}",
        fixed_theta[0], fixed_se[0]
    );
    println!();

    let config = EstimationConfig {
        method: Method::Fixed,
        debug: true,
        ..Default::default()
    };

    for (idx, response) in [1, 0, 1, 1].into_iter().enumerate() {
        person.record_answer(idx, response);
        let ((theta, se), debug) = estimate(&config, &person, &bank, &group).unwrap();
        println!(
            "setelah item ke-{} dijawab ({}): theta={:.4}, se={:.4} -- TIDAK berubah",
            idx + 1,
            if response == 1 { "benar" } else { "salah" },
            theta[0],
            se.as_slice()[0]
        );
        assert_eq!(theta, fixed_theta, "Fixed harus identity persis");
        assert_eq!(se, fixed_se, "Fixed harus identity persis");
        let debug = debug.unwrap();
        assert!(debug.n_quadrature_points.is_none());
        assert!(debug.iterations.is_none());
    }

    println!();
    println!(
        "Seluruh 4 item dijawab, theta/SE tetap persis nilai awal di setiap langkah -- \
         `Fixed` tidak pernah memanggil optimisasi/kuadratur apa pun (semua field \
         AbilityEstimationDebug bernilai None, dites di atas)."
    );
}
