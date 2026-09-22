//! Contoh Fase 4: stopping rule mode klasifikasi `SPRT` (Sequential
//! Probability Ratio Test, Wald 1945) — **beda semantik** dari `CI`: bukan
//! "all dimensi", melainkan satu uji rasio log-likelihood sekuensial
//! gabungan antara hipotesis vektor `theta_lower` vs `theta_upper`,
//! dibandingkan terhadap batas `[a, b]` yang diturunkan dari `alpha`/`beta`
//! (`docs/teori/04-stopping-rule-sprt.md`). Item bank & pola respons nyata
//! dipakai di sini (bukan sintesis theta/SE langsung seperti contoh
//! `min_sem`/`delta_thetas`/`ci`) karena SPRT beroperasi langsung atas
//! `log_likelihood_at(person, bank, Theta)` -- perlu item & respons
//! sungguhan, bukan hanya `theta`/SE ringkasan.
//!
//! Jalankan dengan:
//! ```sh
//! cargo run --example 04_stopping_rule_sprt
//! ```

use mirtcat_engine::irt::Dichotomous;
use mirtcat_engine::stopping_rule::{evaluate, ClassifyDecision, ClassifyType, StoppingConfig};
use mirtcat_engine::{ItemBank, ItemMeta, PersonState, ThetaVector};

fn build_bank() -> ItemBank {
    let mut bank = ItemBank::new();
    for i in 0..20 {
        bank.push(
            Box::new(Dichotomous::m2pl(vec![1.2, 1.2], 0.0).unwrap()),
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

fn run_scenario(name: &str, responses: &[usize], bank: &ItemBank) {
    let lower = ThetaVector::from_vec(vec![-1.0, -1.0]);
    let upper = ThetaVector::from_vec(vec![1.0, 1.0]);
    let config = StoppingConfig {
        min_items: 1,
        max_items: bank.len(),
        classify: ClassifyType::Sprt {
            lower: lower.clone(),
            upper: upper.clone(),
            alpha: 0.05,
            beta: 0.05,
        },
        debug: true,
        ..Default::default()
    };

    println!("== Skenario: {name} ==");
    println!("theta_lower={:?} theta_upper={:?} alpha=beta=0.05", lower.as_slice(), upper.as_slice());

    let mut person = PersonState::new(2);
    for (k, &response) in responses.iter().enumerate() {
        person.record_answer(k, response);
        let (decision, debug) = evaluate(&config, &person, bank, false, None).unwrap();
        let debug = debug.unwrap();
        let (a, b) = debug.sprt_bounds.unwrap();
        let llr = debug.sprt_log_likelihood_ratio.unwrap();
        println!(
            "item ke-{:>2} (r={response}): LL_diff={llr:>8.4}  batas=[{a:>7.4}, {b:>7.4}]  keputusan={}  stop_now={}",
            k + 1,
            label(decision.classification[0]),
            decision.stop_now,
        );
        if decision.stop_now {
            if decision.terminated_early {
                println!("-> keputusan SPRT tercapai: {}", label(decision.classification[0]));
            } else {
                println!("-> item bank habis TANPA keputusan SPRT tercapai (LL_diff tidak pernah keluar batas [a, b])");
            }
            break;
        }
    }
    println!();
}

fn main() {
    let bank = build_bank();

    // Seluruh respons benar -- konsisten dengan hipotesis ATAS (theta tinggi).
    run_scenario("respons konsisten dengan theta_upper", &[1; 20], &bank);

    // Seluruh respons salah -- konsisten dengan hipotesis BAWAH (theta rendah).
    run_scenario("respons konsisten dengan theta_lower", &[0; 20], &bank);

    // Respons berselang-seling -- ambigu, LL_diff tetap di sekitar 0, tidak
    // pernah keluar batas [a, b] sampai item bank habis (SPRT sengaja TIDAK
    // memaksa keputusan -- beda dari CI/min_SEM yang bisa dipaksa lewat
    // max_items terpisah).
    let alternating: Vec<usize> = (0..20).map(|i| i % 2).collect();
    run_scenario("respons berselang-seling -- ambigu", &alternating, &bank);
}
