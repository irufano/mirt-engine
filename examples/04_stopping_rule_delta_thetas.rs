//! Contoh Fase 4: stopping rule `delta_thetas` — sesi berhenti begitu
//! perubahan `theta` antar-langkah TERAKHIR (`|Δtheta|`) di bawah ambang di
//! **seluruh** dimensi (`all()`), independen dari mode klasifikasi
//! (`docs/teori/04-stopping-rule-delta-thetas.md`). Trayektori theta
//! disintesis (konvergen geometris ke titik tetap, dim2 berosilasi lebih
//! lama sebelum stabil) untuk mengisolasi mekanisme ini dari item
//! selection/ability estimation (Fase 2/3).
//!
//! Jalankan dengan:
//! ```sh
//! cargo run --example 04_stopping_rule_delta_thetas
//! ```

use mirt_engine::irt::Dichotomous;
use mirt_engine::stopping_rule::{evaluate, StoppingConfig};
use mirt_engine::{ItemBank, ItemMeta, PersonState, SEVector, ThetaVector};

fn build_bank() -> ItemBank {
    let mut bank = ItemBank::new();
    for i in 0..40 {
        bank.push(
            Box::new(Dichotomous::m2pl(vec![1.0, 1.0], 0.0).unwrap()),
            ItemMeta::new(i),
        )
        .unwrap();
    }
    bank
}

fn main() {
    let bank = build_bank();
    let config = StoppingConfig {
        min_items: 3,
        max_items: bank.len(),
        delta_thetas: vec![0.01, 0.01],
        debug: true,
        ..Default::default()
    };
    let mut person = PersonState::new(2);

    println!("Stopping rule: delta_thetas=[0.01, 0.01] (semantik ALL dimensi), min_items=3");
    println!("Dim1 konvergen mulus, dim2 BEROSILASI beberapa langkah sebelum menetap.");
    println!();

    let mut deltas_printed = Vec::new();
    for k in 1..=bank.len() {
        person.record_answer(k - 1, 1);
        let theta1 = 1.0 * (1.0 - 0.7_f64.powi(k as i32));
        // Dim2: konvergen tapi berosilasi teredam (tanda berganti) supaya
        // |Δtheta| belum kecil di beberapa langkah awal walau dim1 sudah stabil.
        let theta2 = -0.5 + 0.3 * (0.6_f64.powi(k as i32)) * (if k % 2 == 0 { 1.0 } else { -1.0 });
        person.set_estimate(
            ThetaVector::from_vec(vec![theta1, theta2]),
            SEVector::from_vec(vec![0.5, 0.5]), // SE tetap -- bukan fokus contoh ini
        );

        let (decision, debug) = evaluate(&config, &person, &bank, false, None).unwrap();
        let debug = debug.unwrap();
        let delta_str = if debug.delta_theta_values.is_empty() {
            "(belum ada previous_theta)".to_string()
        } else {
            format!(
                "[{:.4}, {:.4}]",
                debug.delta_theta_values[0], debug.delta_theta_values[1]
            )
        };
        deltas_printed.push(delta_str.clone());
        println!(
            "item ke-{k:>2}: theta=[{:>7.4}, {:>7.4}] |Δtheta|={delta_str:<22} met_delta_thetas={:?} stop_now={}",
            person.theta()[0],
            person.theta()[1],
            decision.met_delta_thetas,
            decision.stop_now,
        );

        if decision.stop_now {
            assert!(
                decision.terminated_early,
                "harus berhenti karena presisi (delta_thetas), bukan max_items/waktu"
            );
            println!();
            println!(
                "Berhenti di item ke-{k} -- |Δtheta| KEDUA dimensi akhirnya di bawah 0.01 pada \
                 langkah yang SAMA. Perhatikan deret di atas: `met_delta_thetas` sempat berisi \
                 [false, true] (satu dimensi sudah stabil, satu belum) selama beberapa langkah \
                 sebelum keduanya `true` bersamaan -- semantik all() menahan tes tetap lanjut \
                 sampai KEDUA dimensi stabil serentak."
            );
            break;
        }
    }
}
