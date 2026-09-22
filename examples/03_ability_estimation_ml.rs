//! Contoh Fase 3: ML (Maximum Likelihood) tanpa prior, termasuk kasus
//! respons ekstrem (semua benar/semua salah) untuk menunjukkan potensi
//! divergensi ke `±infinity` — diverifikasi identik dengan
//! `mirt::fscores(method='ML')` sungguhan (lih.
//! `docs/teori/03-ability-estimation-ml.md`).
//!
//! Jalankan dengan:
//! ```sh
//! cargo run --example 03_ability_estimation_ml
//! ```

use mirtcat_engine::ability_estimation::estimate;
use mirtcat_engine::irt::Dichotomous;
use mirtcat_engine::{EstimationConfig, GroupParams, ItemBank, ItemMeta, Method, PersonState};

fn build_bank() -> ItemBank {
    let params: [(f64, f64); 6] = [
        (1.0, 0.0),
        (1.1, 0.1),
        (0.9, -0.1),
        (1.2, 0.2),
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

fn run(label: &str, responses: &[usize], bank: &ItemBank, group: &GroupParams) {
    let mut person = PersonState::new(1);
    for (idx, &r) in responses.iter().enumerate() {
        person.record_answer(idx, r);
    }
    let config = EstimationConfig {
        method: Method::Ml,
        debug: true,
        ..Default::default()
    };
    let ((theta, se), debug) = estimate(&config, &person, bank, group).unwrap();
    let debug = debug.unwrap();
    println!(
        "{label}: respons={responses:?} -> theta={}, se={} (iterasi={:?})",
        theta[0],
        se.as_slice()[0],
        debug.iterations
    );
}

fn main() {
    let bank = build_bank();
    let group = GroupParams::standard_normal(1);

    println!("ML murni (tanpa prior) -- theta = argmax log L(theta):");
    println!();

    run(
        "Pola moderat (4 benar, 2 salah)",
        &[1, 0, 1, 1, 0, 1],
        &bank,
        &group,
    );
    run(
        "Pola sedikit di bawah rata-rata (2 benar, 4 salah)",
        &[1, 0, 0, 1, 0, 0],
        &bank,
        &group,
    );

    println!();
    println!("== Kasus ekstrem: semua benar ==");
    run("Semua benar (6/6)", &[1, 1, 1, 1, 1, 1], &bank, &group);
    println!(
        "  -> theta=+infinity, se=NaN: TIDAK ada batas atas kemampuan yang bisa \"cukup\" \
         menjelaskan respons sempurna -- likelihood terus naik seiring theta -> infinity."
    );

    println!();
    println!("== Kasus ekstrem: semua salah ==");
    run("Semua salah (0/6)", &[0, 0, 0, 0, 0, 0], &bank, &group);
    println!("  -> theta=-infinity, se=NaN (simetri dari kasus semua-benar di atas).");

    println!();
    println!(
        "Bandingkan dengan `cargo run --example 03_ability_estimation_wle`: WLE (koreksi bias \
         Warm) TETAP TERBATAS untuk kedua pola ekstrem di atas -- itulah motivasi WLE dipakai \
         alih-alih ML mentah pada tahap awal CAT (respons sedikit, risiko pola ekstrem tinggi)."
    );
}
