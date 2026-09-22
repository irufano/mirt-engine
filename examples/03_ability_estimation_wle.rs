//! Contoh Fase 3: WLE (Weighted Likelihood Estimation) dengan koreksi bias
//! Warm (1989), dibandingkan dengan ML pada kasus respons ekstrem yang sama
//! untuk menunjukkan bias lebih kecil (`docs/teori/03-ability-estimation-wle.md`).
//!
//! Jalankan dengan:
//! ```sh
//! cargo run --example 03_ability_estimation_wle
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

fn run(
    method_name: &str,
    method: Method,
    responses: &[usize],
    bank: &ItemBank,
    group: &GroupParams,
) {
    let mut person = PersonState::new(1);
    for (idx, &r) in responses.iter().enumerate() {
        person.record_answer(idx, r);
    }
    let config = EstimationConfig {
        method,
        ..Default::default()
    };
    let ((theta, se), _) = estimate(&config, &person, bank, group).unwrap();
    println!(
        "  {method_name}: theta={}, se={}",
        theta[0],
        se.as_slice()[0]
    );
}

fn main() {
    let bank = build_bank();
    let group = GroupParams::standard_normal(1);

    println!(
        "WLE mengoreksi bias Warm dengan memaksimalkan \
         `log L(theta) + 0.5*log(det(info_kumulatif(theta)))`, bukan `log L(theta)` murni \
         seperti ML -- suku kedua mencegah estimasi meledak pada pola respons ekstrem."
    );
    println!();

    println!("== Pola ekstrem: semua benar (6/6) ==");
    run("ML ", Method::Ml, &[1, 1, 1, 1, 1, 1], &bank, &group);
    run("WLE", Method::Wle, &[1, 1, 1, 1, 1, 1], &bank, &group);
    println!("  -> ML divergen ke +infinity; WLE tetap TERBATAS (bias lebih kecil).");

    println!();
    println!("== Pola ekstrem: semua salah (0/6) ==");
    run("ML ", Method::Ml, &[0, 0, 0, 0, 0, 0], &bank, &group);
    run("WLE", Method::Wle, &[0, 0, 0, 0, 0, 0], &bank, &group);
    println!("  -> simetri: ML divergen ke -infinity; WLE tetap terbatas.");

    println!();
    println!("== Pola moderat (4 benar, 2 salah) -- ML & WLE harus berdekatan ==");
    run("ML ", Method::Ml, &[1, 0, 1, 1, 0, 1], &bank, &group);
    run("WLE", Method::Wle, &[1, 0, 1, 1, 0, 1], &bank, &group);
    println!(
        "  -> jauh dari batas ekstrem, koreksi bias Warm kecil -- ML & WLE hampir sama \
         (WLE mendekati ML tak-bias asimtotik seiring respons makin informatif/moderat)."
    );
}
