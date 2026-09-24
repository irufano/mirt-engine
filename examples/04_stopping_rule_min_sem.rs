//! Contoh Fase 4: stopping rule `min_SEM` — sesi berhenti begitu SE
//! **seluruh** dimensi (`all()`) turun di bawah ambang, bukan salah satu
//! saja (`docs/teori/04-stopping-rule-min-sem.md`).
//!
//! Trayektori `theta`/SE di sini disintesis langsung (dim1 menurun cepat,
//! dim2 lambat) supaya fokus HANYA pada mekanisme stopping rule itu sendiri
//! (`stopping_rule::evaluate`), terpisah dari detail item selection/ability
//! estimation (Fase 2/3, sudah dicontohkan tersendiri) — pola yang sama
//! seperti contoh Fase 2/3 yang memakai skenario terkontrol, bukan selalu
//! simulasi tertutup penuh.
//!
//! Jalankan dengan:
//! ```sh
//! cargo run --example 04_stopping_rule_min_sem
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
        min_sem: vec![0.4, 0.4],
        debug: true,
        ..Default::default()
    };
    let mut person = PersonState::new(2);

    println!("Stopping rule: min_SEM=[0.4, 0.4] (semantik ALL dimensi), min_items=3");
    println!("Dim1 diprogram konvergen CEPAT (SE -> 0), dim2 LAMBAT -- tes harus menunggu dim2.");
    println!();

    for k in 1..=bank.len() {
        person.record_answer(k - 1, 1);
        // Sintesis SE menurun geometris per-dimensi dengan laju berbeda; theta
        // konvergen ke target tetap. Ini BUKAN hasil ability_estimation
        // sungguhan -- murni untuk mengisolasi perilaku stopping rule.
        let se1 = 1.0 * 0.92_f64.powi(k as i32);
        let se2 = 1.0 * 0.97_f64.powi(k as i32);
        let theta1 = 1.2 * (1.0 - 0.85_f64.powi(k as i32));
        let theta2 = -0.6 * (1.0 - 0.85_f64.powi(k as i32));
        person.set_estimate(
            ThetaVector::from_vec(vec![theta1, theta2]),
            SEVector::from_vec(vec![se1, se2]),
        );

        let (decision, debug) = evaluate(&config, &person, &bank, false, None).unwrap();
        let debug = debug.unwrap();
        println!(
            "item ke-{k:>2}: theta=[{:>6.3}, {:>6.3}] se=[{:>6.3}, {:>6.3}] met_SEM={:?} stop_now={}",
            person.theta()[0],
            person.theta()[1],
            debug.se_values[0],
            debug.se_values[1],
            decision.met_sem,
            decision.stop_now,
        );

        if decision.stop_now {
            assert!(
                decision.terminated_early,
                "harus berhenti karena presisi (min_SEM), bukan max_items/waktu"
            );
            println!();
            println!(
                "Berhenti di item ke-{k} -- KEDUA dimensi akhirnya di bawah 0.4 SECARA BERSAMAAN. \
                 Perhatikan baris-baris sebelumnya: begitu dim1 sudah met_SEM=true tapi dim2 masih \
                 false, tes TETAP LANJUT -- semantik all(), bukan any()."
            );
            break;
        }
    }
}
