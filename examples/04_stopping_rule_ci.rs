//! Contoh Fase 4: stopping rule mode klasifikasi `CI` — keputusan
//! `'above cutoff'`/`'below cutoff'`/`'no decision'` per-dimensi (z-score
//! terhadap `cutoffs` via `classify_alpha`), status `'no decision'`
//! bertahan sampai SEMUA dimensi terklasifikasi (`docs/teori/04-stopping-rule-ci.md`).
//! Bank & trayektori disintesis (2 dimensi, satu menjauh cepat dari cutoff,
//! satu lambat) untuk mengisolasi mekanisme klasifikasi ini.
//!
//! Jalankan dengan:
//! ```sh
//! cargo run --example 04_stopping_rule_ci
//! ```

use mirt_engine::irt::Dichotomous;
use mirt_engine::stopping_rule::{evaluate, ClassifyDecision, ClassifyType, StoppingConfig};
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

fn label(d: ClassifyDecision) -> &'static str {
    match d {
        ClassifyDecision::AboveCutoff => "above cutoff",
        ClassifyDecision::BelowCutoff => "below cutoff",
        ClassifyDecision::NoDecision => "no decision",
    }
}

fn main() {
    let bank = build_bank();
    let cutoffs = vec![0.0, 0.0];
    let config = StoppingConfig {
        min_items: 3,
        max_items: bank.len(),
        classify: ClassifyType::Ci {
            cutoffs: cutoffs.clone(),
            alpha: 0.05, // padanan classify_CI = 0.90
        },
        debug: true,
        ..Default::default()
    };
    let mut person = PersonState::new(2);

    println!("Stopping rule: classify CI, cutoffs={cutoffs:?}, alpha=0.05 (classify_CI=0.90)");
    println!("Dim1 menjauh CEPAT dari cutoff (segera 'above cutoff'), dim2 LAMBAT.");
    println!();

    for k in 1..=bank.len() {
        person.record_answer(k - 1, 1);
        let theta1 = 1.5 * (1.0 - 0.75_f64.powi(k as i32));
        let theta2 = -(1.0 - 0.90_f64.powi(k as i32));
        let se1 = 0.4 + 0.6 * 0.85_f64.powi(k as i32);
        let se2 = 0.4 + 0.6 * 0.95_f64.powi(k as i32);
        person.set_estimate(
            ThetaVector::from_vec(vec![theta1, theta2]),
            SEVector::from_vec(vec![se1, se2]),
        );

        let (decision, _) = evaluate(&config, &person, &bank, false, None).unwrap();
        let classification: Vec<&str> = decision.classification.iter().map(|&d| label(d)).collect();
        println!(
            "item ke-{k:>2}: theta=[{:>6.3}, {:>6.3}] se=[{:>5.3}, {:>5.3}] met_classify={:?} classification={classification:?} stop_now={}",
            person.theta()[0],
            person.theta()[1],
            person.se()[0],
            person.se()[1],
            decision.met_classify,
            decision.stop_now,
        );

        if decision.stop_now {
            assert!(
                decision.terminated_early,
                "harus berhenti karena semua dimensi terklasifikasi, bukan max_items/waktu"
            );
            println!();
            println!(
                "Berhenti di item ke-{k} -- KEDUA dimensi akhirnya terklasifikasi (bukan lagi \
                 'no decision') secara BERSAMAAN. Perhatikan baris-baris sebelumnya: begitu dim1 \
                 sudah 'above cutoff' tapi dim2 masih 'no decision', tes TETAP LANJUT."
            );
            break;
        }
    }
}
